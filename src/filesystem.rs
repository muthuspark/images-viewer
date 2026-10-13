use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

/// The selected folder is depth zero; images at depth four are included.
pub const MAX_IMAGE_DEPTH: usize = 4;

#[derive(Clone, Debug)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
}

/// The navigation tree only needs immediate children, not a recursive scan.
pub fn read_folders(path: &Path) -> io::Result<Vec<Folder>> {
    let mut folders = Vec::new();
    for entry in fs::read_dir(path)?.flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            folders.push(Folder {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path(),
            });
        }
    }
    folders.sort_by_cached_key(|folder| (folder.name.to_lowercase(), folder.path.clone()));
    Ok(folders)
}

pub fn read_images_recursive(path: &Path, cancelled: &AtomicBool) -> io::Result<Vec<PathBuf>> {
    let mut images = Vec::new();
    let mut pending = vec![(path.to_path_buf(), 0)];
    while let Some((directory, depth)) = pending.pop() {
        check_cancelled(cancelled)?;
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if depth == 0 => return Err(error),
            // An unavailable descendant should not hide accessible images.
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            check_cancelled(cancelled)?;
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() && depth < MAX_IMAGE_DEPTH {
                pending.push((path, depth + 1));
            } else if kind.is_file() && is_image(&path) {
                images.push(path);
            }
            // file_type does not follow symlinks, preventing directory loops.
        }
    }
    check_cancelled(cancelled)?;
    images.sort_by_cached_key(|path| (path.to_string_lossy().to_lowercase(), path.clone()));
    Ok(images)
}

fn check_cancelled(cancelled: &AtomicBool) -> io::Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Folder scan cancelled",
        ))
    } else {
        Ok(())
    }
}

fn is_image(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        ["jpg", "jpeg", "png", "webp", "gif", "bmp", "svg"]
            .iter()
            .any(|e| s.eq_ignore_ascii_case(e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_lists_sorted_immediate_folders_only() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("zebra/nested")).unwrap();
        fs::create_dir(temp.path().join("Alpha")).unwrap();
        fs::write(temp.path().join("photo.png"), []).unwrap();
        let folders = read_folders(temp.path()).unwrap();
        assert_eq!(
            folders.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            ["Alpha", "zebra"]
        );
        assert!(read_folders(&temp.path().join("missing")).is_err());
    }

    #[test]
    fn recursive_gallery_includes_depth_four_but_excludes_depth_five() {
        let temp = tempfile::tempdir().unwrap();
        let mut directory = temp.path().to_path_buf();
        let mut expected = Vec::new();
        for depth in 0..=5 {
            fs::create_dir_all(&directory).unwrap();
            let image = directory.join(format!("depth-{depth}.JpEg"));
            fs::write(&image, []).unwrap();
            fs::write(directory.join("notes.txt"), []).unwrap();
            if depth <= MAX_IMAGE_DEPTH {
                expected.push(image);
            }
            directory.push("child");
        }
        let images = read_images_recursive(temp.path(), &AtomicBool::new(false)).unwrap();
        expected.sort_by_cached_key(|p| (p.to_string_lossy().to_lowercase(), p.clone()));
        assert_eq!(images, expected);
        // The depth limit starts again from whichever folder is selected.
        let deeper =
            read_images_recursive(&temp.path().join("child"), &AtomicBool::new(false)).unwrap();
        assert_eq!(deeper.len(), 5);
        assert!(
            deeper
                .iter()
                .any(|p| p.file_name().unwrap() == "depth-5.JpEg")
        );
    }

    #[test]
    fn recursive_gallery_filters_and_sorts_across_sibling_folders() {
        let temp = tempfile::tempdir().unwrap();
        for folder in ["zebra", "Alpha"] {
            fs::create_dir(temp.path().join(folder)).unwrap();
        }
        let expected = ["Alpha/A.svg", "Alpha/z.PNG", "b.JpEg", "zebra/B.webp"];
        for name in expected {
            fs::write(temp.path().join(name), []).unwrap();
        }
        fs::write(temp.path().join("notes.txt"), []).unwrap();
        fs::write(temp.path().join("no-extension"), []).unwrap();
        let images = read_images_recursive(temp.path(), &AtomicBool::new(false)).unwrap();
        assert_eq!(images, expected.map(|p| temp.path().join(p)));
    }

    #[test]
    fn recursive_scan_can_be_cancelled_and_reports_missing_root() {
        let temp = tempfile::tempdir().unwrap();
        let error = read_images_recursive(temp.path(), &AtomicBool::new(true)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(
            read_images_recursive(&temp.path().join("missing"), &AtomicBool::new(false)).is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn recursive_gallery_skips_symlink_loops_and_linked_files() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let child = temp.path().join("child");
        fs::create_dir(&child).unwrap();
        let photo = child.join("photo.png");
        fs::write(&photo, []).unwrap();
        symlink(temp.path(), child.join("loop")).unwrap();
        symlink(&photo, temp.path().join("linked.png")).unwrap();
        assert_eq!(
            read_images_recursive(temp.path(), &AtomicBool::new(false)).unwrap(),
            [photo]
        );
    }
}
