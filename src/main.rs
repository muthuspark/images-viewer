mod filesystem;
mod image_cache;
mod thumbnails;

use filesystem::Folder;
use gpui::{prelude::*, *};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const SIDEBAR: f32 = 220.;
const GAP: f32 = 8.;

struct Viewer {
    roots: Vec<Folder>,
    expanded: HashSet<PathBuf>,
    directories: HashMap<PathBuf, Result<Vec<Folder>, String>>,
    scan_cancelled: Arc<AtomicBool>,
    pending: HashSet<PathBuf>,
    selected: PathBuf,
    images: Vec<PathBuf>,
    opened: Option<usize>,
    thumbnail_size: f32,
    loading: bool,
    error: Option<String>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
}

#[derive(Clone)]
enum Command {
    Toggle(PathBuf),
    Resize(f32),
    Close,
}

fn control(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    command: Command,
    cx: &mut Context<Viewer>,
) -> Stateful<Div> {
    let keyboard_command = command.clone();
    div()
        .id(id)
        .tab_stop(true)
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .size(px(30.))
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(0xd6cfc4))
        .hover(|s| s.bg(rgb(0xe5ded3)))
        .focus(|s| s.border_color(rgb(0x8c694b)))
        .child(label.into())
        .on_click(cx.listener(move |this, _, window, cx| this.command(command.clone(), window, cx)))
        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.key == "enter" || event.keystroke.key == "space" {
                this.command(keyboard_command.clone(), window, cx);
                cx.stop_propagation();
            }
        }))
}

fn failed_image() -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(rgb(0x8a8074))
        .child("Image unavailable")
        .into_any_element()
}

impl Viewer {
    fn command(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        match command {
            Command::Toggle(path) => self.toggle(path, cx),
            Command::Resize(amount) => self.resize(amount, window, cx),
            Command::Close => {
                self.opened = None;
                cx.notify();
            }
        }
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let selected = std::env::args_os()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or(home.clone());
        let selected = selected.canonicalize().unwrap_or(selected);
        let mut roots = vec![Folder {
            name: "Home".into(),
            path: home.clone(),
        }];
        if !selected.starts_with(&home) {
            roots.insert(
                0,
                Folder {
                    name: selected.to_string_lossy().into_owned(),
                    path: selected.clone(),
                },
            );
        }
        let focus = cx.focus_handle();
        focus.focus(window);
        let mut this = Self {
            roots,
            expanded: HashSet::from([if selected.starts_with(&home) {
                home.clone()
            } else {
                selected.clone()
            }]),
            directories: HashMap::new(),
            scan_cancelled: Arc::new(AtomicBool::new(false)),
            pending: HashSet::new(),
            selected: selected.clone(),
            images: vec![],
            opened: None,
            thumbnail_size: 190.,
            loading: true,
            error: None,
            focus,
            scroll: UniformListScrollHandle::new(),
        };
        if selected.starts_with(&home) {
            for ancestor in selected
                .ancestors()
                .skip(1)
                .take_while(|p| p.starts_with(&home))
            {
                this.expanded.insert(ancestor.to_path_buf());
            }
        }
        for path in this.expanded.clone() {
            this.load_folders(path, cx);
        }
        this.select(selected, cx);
        this
    }

    fn load_folders(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.pending.insert(path.clone()) {
            return;
        }
        let work_path = path.clone();
        let task = cx.background_executor().spawn(async move {
            filesystem::read_folders(&work_path)
                .map_err(|e| format!("This folder cannot be opened: {e}"))
        });
        cx.spawn(async move |weak, cx| {
            let contents = task.await;
            let _ = weak.update(cx, |this, cx| {
                this.pending.remove(&path);
                this.directories.insert(path.clone(), contents);
                cx.notify();
            });
        })
        .detach();
    }

    fn select(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.scan_cancelled.store(true, Ordering::Relaxed);
        let cancelled = Arc::new(AtomicBool::new(false));
        self.scan_cancelled = cancelled.clone();
        self.selected = path.clone();
        self.opened = None;
        self.images.clear();
        self.error = None;
        self.loading = true;
        self.scroll.scroll_to_item(0, ScrollStrategy::Top);
        self.load_folders(path.clone(), cx);
        let task_cancelled = cancelled.clone();
        let task = cx.background_executor().spawn(async move {
            filesystem::read_images_recursive(&path, &task_cancelled)
                .map_err(|e| format!("This folder cannot be opened: {e}"))
        });
        cx.spawn(async move |weak, cx| {
            let images = task.await;
            let _ = weak.update(cx, |this, cx| {
                // A new selection or refresh supersedes the previous scan, even
                // when the folder path is the same.
                if !Arc::ptr_eq(&this.scan_cancelled, &cancelled) {
                    return;
                }
                this.loading = false;
                match images {
                    Ok(images) => this.images = images,
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn toggle(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.expanded.remove(&path) {
            self.expanded.insert(path.clone());
            if !self.directories.contains_key(&path) {
                self.load_folders(path, cx);
            }
        }
        cx.notify();
    }

    fn tree_rows(&self, folders: &[Folder], depth: usize, rows: &mut Vec<(Folder, usize)>) {
        for folder in folders {
            rows.push((folder.clone(), depth));
            if self.expanded.contains(&folder.path)
                && let Some(Ok(folders)) = self.directories.get(&folder.path)
            {
                self.tree_rows(folders, depth + 1, rows);
            }
        }
    }

    fn resize(&mut self, amount: f32, window: &Window, cx: &mut Context<Self>) {
        let maximum = (f32::from(window.viewport_size().width) - SIDEBAR - 32.).max(140.);
        self.thumbnail_size = (self.thumbnail_size + amount).clamp(140., maximum);
        cx.notify();
    }
}

impl Drop for Viewer {
    fn drop(&mut self) {
        self.scan_cancelled.store(true, Ordering::Relaxed);
    }
}

impl Render for Viewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let available = (f32::from(window.viewport_size().width) - SIDEBAR - 24.).max(1.);
        let columns = ((available + GAP) / (self.thumbnail_size + GAP))
            .floor()
            .max(1.) as usize;
        let tile_size = ((available - GAP * (columns - 1) as f32) / columns as f32).max(1.);
        let rows = self.images.len().div_ceil(columns);
        let mut tree = vec![];
        self.tree_rows(&self.roots, 0, &mut tree);
        let sidebar = div()
            .w(px(SIDEBAR))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(rgb(0xebe5db))
            .border_r_1()
            .border_color(rgb(0xdcd4c8))
            .child(
                div()
                    .px_4()
                    .py_4()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Folders"),
            )
            .child(
                div()
                    .id("tree")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(tree.into_iter().enumerate().map(|(ix, (folder, depth))| {
                        let path = folder.path;
                        let toggle_path = path.clone();
                        let key_path = path.clone();
                        let expanded = self.expanded.contains(&path);
                        div()
                            .id(("folder", ix))
                            .flex()
                            .items_center()
                            .h(px(30.))
                            .pl(px(8. + depth as f32 * 16.))
                            .when(self.selected == path, |s| s.bg(rgb(0xded3c3)))
                            .child(
                                control(
                                    ("toggle", ix),
                                    if expanded { "⌄" } else { "›" },
                                    Command::Toggle(toggle_path),
                                    cx,
                                )
                                .w(px(22.))
                                .border_0(),
                            )
                            .child(
                                div()
                                    .id(("label", ix))
                                    .tab_stop(true)
                                    .cursor_pointer()
                                    .flex_1()
                                    .min_w_0()
                                    .text_sm()
                                    .truncate()
                                    .focus(|s| s.bg(rgb(0xd4c5b1)))
                                    .child(folder.name)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.select(path.clone(), cx)
                                    }))
                                    .on_key_down(cx.listener(
                                        move |this, event: &KeyDownEvent, _, cx| {
                                            if event.keystroke.key == "enter" {
                                                this.select(key_path.clone(), cx);
                                                cx.stop_propagation();
                                            }
                                            if event.keystroke.key == "right"
                                                || event.keystroke.key == "left"
                                            {
                                                this.toggle(key_path.clone(), cx);
                                                cx.stop_propagation();
                                            }
                                        },
                                    )),
                            )
                    })),
            );
        let gallery = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(48.))
                    .flex_none()
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child(control("smaller", "−", Command::Resize(-40.), cx))
                    .child(control("larger", "+", Command::Resize(40.), cx)),
            )
            .child(if self.loading || self.images.is_empty() {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(rgb(0x817668))
                    .child(if self.loading {
                        "Loading images…".into()
                    } else {
                        self.error
                            .clone()
                            .unwrap_or_else(|| "No images in this folder".into())
                    })
                    .into_any_element()
            } else {
                uniform_list(
                    "gallery",
                    rows,
                    cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|row| {
                                div()
                                    .id(("row", row))
                                    .flex()
                                    .gap(px(GAP))
                                    .pb(px(GAP))
                                    .children(
                                        (row * columns
                                            ..((row + 1) * columns).min(this.images.len()))
                                            .map(|index| {
                                                div()
                                                    .id(("tile", index))
                                                    .tab_stop(true)
                                                    .cursor_pointer()
                                                    .size(px(tile_size))
                                                    .overflow_hidden()
                                                    .bg(rgb(0xe5ded3))
                                                    .border_1()
                                                    .border_color(rgb(0xe5ded3))
                                                    .focus(|s| s.border_color(rgb(0x8c694b)))
                                                    .child(
                                                        img(this.images[index].clone())
                                                            .size_full()
                                                            .object_fit(ObjectFit::Cover)
                                                            .with_fallback(failed_image),
                                                    )
                                                    .on_click(cx.listener(move |this, _, window, cx| {
                                                        this.focus.focus(window);
                                                        this.opened = Some(index);
                                                        cx.notify();
                                                    }))
                                                    .on_key_down(cx.listener(
                                                        move |this, event: &KeyDownEvent, window, cx| {
                                                            if event.keystroke.key == "enter"
                                                                || event.keystroke.key == "space"
                                                            {
                                                                this.focus.focus(window);
                                                                this.opened = Some(index);
                                                                cx.notify();
                                                                cx.stop_propagation();
                                                            }
                                                        },
                                                    ))
                                            }),
                                    )
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .track_scroll(self.scroll.clone())
                .flex_1()
                .min_h_0()
                .w_full()
                .into_any_element()
            });
        let content = div()
            .relative()
            .size_full()
            .bg(rgb(0xf3efe8))
            .text_color(rgb(0x453d34))
            .font_family("sans-serif")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => this.opened = None,
                    "right" if this.opened.is_some() => {
                        this.opened = this.opened.map(|i| (i + 1).min(this.images.len() - 1))
                    }
                    "left" if this.opened.is_some() => {
                        this.opened = this.opened.map(|i| i.saturating_sub(1))
                    }
                    "+" | "=" => this.resize(40., window, cx),
                    "-" => this.resize(-40., window, cx),
                    "f5" => this.select(this.selected.clone(), cx),
                    "tab" if this.opened.is_none() => {
                        if event.keystroke.modifiers.shift {
                            window.focus_prev();
                        } else {
                            window.focus_next();
                        }
                    }
                    _ => return,
                }
                cx.notify();
            }))
            .child(
                div().flex().size_full().child(sidebar).child(
                    div().flex_1().min_w_0().px_3().pb_3().child(
                        gpui::image_cache(image_cache::thumbnail_cache(
                            "photos",
                            (columns
                                * ((f32::from(window.viewport_size().height) / (tile_size + GAP))
                                    .ceil() as usize
                                    + 2))
                                .max(96),
                        ))
                        .size_full()
                        .child(gallery),
                    ),
                ),
            )
            .when_some(self.opened, |root, index| {
                root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(rgb(0x25211d))
                        .p_5()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            img(self.images[index].clone())
                                .size_full()
                                .object_fit(ObjectFit::Contain)
                                .with_fallback(failed_image),
                        )
                        .child(
                            control("close", "×", Command::Close, cx)
                                .absolute()
                                .top_3()
                                .right_3()
                                .bg(rgb(0xebe5db)),
                        ),
                )
            });
        gpui::image_cache(image_cache::bounded_image_cache("full-size", 2))
            .size_full()
            .child(content)
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1100.), px(760.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(480.), px(320.))),
                app_id: Some("image-viewer".into()),
                titlebar: Some(TitlebarOptions {
                    title: Some("Image Viewer".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Viewer::new(window, cx)),
        )
        .expect("Cannot open Image Viewer window");
        cx.activate(true);
    });
}
