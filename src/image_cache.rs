// Adapted from GPUI 0.2.2 image_gallery example (Apache-2.0).
use futures::FutureExt;
use gpui::{
    App, AppContext, Asset, AssetLogger, Context, ElementId, Entity, ImageAssetLoader, ImageCache,
    ImageCacheProvider, Window, hash,
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub fn bounded_image_cache(id: impl Into<ElementId>, max_items: usize) -> SimpleLruCacheProvider {
    SimpleLruCacheProvider {
        id: id.into(),
        max_items,
        thumbnails: false,
    }
}

pub fn thumbnail_cache(id: impl Into<ElementId>, max_items: usize) -> SimpleLruCacheProvider {
    SimpleLruCacheProvider {
        id: id.into(),
        max_items,
        thumbnails: true,
    }
}

pub struct SimpleLruCacheProvider {
    id: ElementId,
    max_items: usize,
    thumbnails: bool,
}

impl ImageCacheProvider for SimpleLruCacheProvider {
    fn provide(&mut self, window: &mut Window, cx: &mut App) -> gpui::AnyImageCache {
        window
            .with_global_id(self.id.clone(), |global_id, window| {
                window.with_element_state::<Entity<SimpleLruCache>, _>(
                    global_id,
                    |lru_cache, _window| {
                        let lru_cache = lru_cache.unwrap_or_else(|| {
                            cx.new(|cx| SimpleLruCache::new(self.max_items, self.thumbnails, cx))
                        });
                        // A resize changes the number of visible tiles. Preserve decoded
                        // previews instead of throwing away the entire cache.
                        lru_cache.update(cx, |cache, _| {
                            cache.max_items = cache.max_items.max(self.max_items);
                        });
                        (lru_cache.clone(), lru_cache)
                    },
                )
            })
            .into()
    }
}

struct SimpleLruCache {
    max_items: usize,
    thumbnails: bool,
    usages: Vec<u64>,
    cache: HashMap<u64, CachedImage>,
}

struct CachedImage {
    item: gpui::ImageCacheItem,
    cancelled: Arc<AtomicBool>,
}

impl CachedImage {
    fn get(&mut self) -> Option<Result<Arc<gpui::RenderImage>, gpui::ImageCacheError>> {
        self.item.get()
    }
}

impl Drop for CachedImage {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl SimpleLruCache {
    fn new(max_items: usize, thumbnails: bool, cx: &mut Context<Self>) -> Self {
        cx.on_release(|simple_cache, cx| {
            for (_, mut item) in std::mem::take(&mut simple_cache.cache) {
                if let Some(Ok(image)) = item.get() {
                    cx.drop_image(image, None);
                }
            }
        })
        .detach();

        Self {
            max_items,
            thumbnails,
            usages: Vec::with_capacity(max_items),
            cache: HashMap::with_capacity(max_items),
        }
    }
}

impl ImageCache for SimpleLruCache {
    fn load(
        &mut self,
        resource: &gpui::Resource,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Result<Arc<gpui::RenderImage>, gpui::ImageCacheError>> {
        assert_eq!(self.usages.len(), self.cache.len());
        assert!(self.cache.len() <= self.max_items);

        let hash = hash(resource);

        if let Some(item) = self.cache.get_mut(&hash) {
            let current_ix = self
                .usages
                .iter()
                .position(|item| *item == hash)
                .expect("cache and usages must stay in sync");
            self.usages.remove(current_ix);
            self.usages.insert(0, hash);

            return item.get();
        }

        let resource = resource.clone();
        let fallback = AssetLogger::<ImageAssetLoader>::load(resource.clone(), cx);
        let thumbnails = self.thumbnails;
        let cancelled = Arc::new(AtomicBool::new(false));
        let job_cancelled = cancelled.clone();
        let task = cx
            .background_executor()
            .spawn(async move {
                let _permit = if thumbnails {
                    Some(crate::thumbnails::workers().acquire().await)
                } else {
                    None
                };
                // Evicted jobs can still be held by completion observers. Skip them
                // before touching the source, so scrolling does not queue old decodes.
                if job_cancelled.load(Ordering::Relaxed) {
                    return Err(gpui::ImageCacheError::Asset(
                        "Thumbnail no longer visible".into(),
                    ));
                }
                if thumbnails
                    && let gpui::Resource::Path(path) = &resource
                    && !path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
                {
                    let mut pixels = crate::thumbnails::load(path)?;
                    for pixel in pixels.pixels_mut() {
                        pixel.0.swap(0, 2);
                    }
                    return Ok(Arc::new(gpui::RenderImage::new(smallvec::smallvec![
                        image::Frame::new(pixels)
                    ])));
                }
                fallback.await
            })
            .shared();
        if self.usages.len() == self.max_items {
            let oldest = self.usages.pop().unwrap();
            let mut image = self
                .cache
                .remove(&oldest)
                .expect("cache and usages must be in sync");
            if let Some(Ok(image)) = image.get() {
                cx.drop_image(image, Some(window));
            }
        }
        self.cache.insert(
            hash,
            CachedImage {
                item: gpui::ImageCacheItem::Loading(task.clone()),
                cancelled: cancelled.clone(),
            },
        );
        self.usages.insert(0, hash);

        let entity = window.current_view();
        window
            .spawn(cx, {
                async move |cx| {
                    _ = task.await;
                    if !cancelled.load(Ordering::Relaxed) {
                        cx.on_next_frame(move |_, cx| {
                            cx.notify(entity);
                        });
                    }
                }
            })
            .detach();

        None
    }
}
