use image::{
    ImageEncoder, RgbaImage,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::UNIX_EPOCH,
};

const EDGE: u32 = 512;
const DISK_BUDGET: u64 = 512 * 1024 * 1024;

pub fn workers() -> &'static async_lock::Semaphore {
    static WORKERS: OnceLock<async_lock::Semaphore> = OnceLock::new();
    WORKERS.get_or_init(|| {
        async_lock::Semaphore::new(
            std::thread::available_parallelism()
                .map_or(2, usize::from)
                .clamp(1, 4),
        )
    })
}

pub fn load(path: &Path) -> Result<RgbaImage, gpui::ImageCacheError> {
    let cache = dirs::cache_dir().map(|root| root.join("image-viewer/thumbnails-v1"));
    load_cached(path, cache.as_deref(), decode)
}

fn cache_path(source: &Path, root: &Path) -> io::Result<PathBuf> {
    let metadata = fs::metadata(source)?;
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    let mut key = blake3::Hasher::new();
    key.update(source.as_os_str().as_encoded_bytes());
    key.update(&metadata.len().to_le_bytes());
    key.update(&modified.as_nanos().to_le_bytes());
    Ok(root.join(format!("{}.png", key.finalize().to_hex())))
}

fn load_cached(
    source: &Path,
    root: Option<&Path>,
    decode: impl FnOnce(&Path) -> Result<RgbaImage, gpui::ImageCacheError>,
) -> Result<RgbaImage, gpui::ImageCacheError> {
    let cached = root.and_then(|root| cache_path(source, root).ok());
    if let Some(path) = &cached
        && let Ok(mut reader) = image::ImageReader::open(path)
    {
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(EDGE);
        limits.max_image_height = Some(EDGE);
        limits.max_alloc = Some(u64::from(EDGE * EDGE * 8));
        reader.limits(limits);
        if let Ok(image) = reader.decode() {
            return Ok(image.into_rgba8());
        }
    }
    let pixels = decode(source)?;
    // A full/read-only cache must never prevent viewing the original image.
    if let Some(path) = cached {
        let _ = store(&path, &pixels);
    }
    Ok(pixels)
}

fn decode(path: &Path) -> Result<RgbaImage, gpui::ImageCacheError> {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg"))
        && let Some(pixels) = decode_jpeg(path)
    {
        return Ok(pixels);
    }
    decode_standard(path)
}

fn decode_standard(path: &Path) -> Result<RgbaImage, gpui::ImageCacheError> {
    let image = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?;
    let image = if image.width() > EDGE || image.height() > EDGE {
        image.thumbnail(EDGE, EDGE)
    } else {
        image
    };
    Ok(image.into_rgba8())
}

fn decode_jpeg(path: &Path) -> Option<RgbaImage> {
    let data = fs::read(path).ok()?;
    let mut decoder = turbojpeg::Decompressor::new().ok()?;
    let header = decoder.read_header(&data).ok()?;
    let longest = header.width.max(header.height);
    if longest == 0 {
        return None;
    }
    // Decode only as many pixels as the thumbnail needs. Keep the final
    // dimensions based on the original header, avoiding rounding-induced stretch.
    let scale = if header.is_lossless {
        turbojpeg::ScalingFactor::ONE
    } else {
        [
            turbojpeg::ScalingFactor::ONE_EIGHTH,
            turbojpeg::ScalingFactor::ONE_QUARTER,
            turbojpeg::ScalingFactor::ONE_HALF,
            turbojpeg::ScalingFactor::ONE,
        ]
        .into_iter()
        .find(|scale| scale.scale(longest) >= EDGE as usize)
        .unwrap_or(turbojpeg::ScalingFactor::ONE)
    };
    decoder.set_scaling_factor(scale).ok()?;
    let scaled = header.scaled(scale);
    let length = scaled.width.checked_mul(scaled.height)?.checked_mul(4)?;
    if length > 256 * 1024 * 1024 {
        return None;
    }
    let mut output = turbojpeg::Image {
        pixels: vec![0; length],
        width: scaled.width,
        height: scaled.height,
        pitch: scaled.width.checked_mul(4)?,
        format: turbojpeg::PixelFormat::RGBA,
    };
    decoder.decompress(&data, output.as_deref_mut()).ok()?;
    let pixels = RgbaImage::from_raw(
        scaled.width.try_into().ok()?,
        scaled.height.try_into().ok()?,
        output.pixels,
    )?;
    if longest <= EDGE as usize {
        return Some(pixels);
    }
    let width = ((header.width * EDGE as usize + longest / 2) / longest).max(1) as u32;
    let height = ((header.height * EDGE as usize + longest / 2) / longest).max(1) as u32;
    Some(
        image::DynamicImage::ImageRgba8(pixels)
            .thumbnail_exact(width, height)
            .into_rgba8(),
    )
}

fn store(path: &Path, pixels: &RgbaImage) -> Result<(), Box<dyn std::error::Error>> {
    let root = path.parent().ok_or("Missing thumbnail cache directory")?;
    fs::create_dir_all(root)?;
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    PngEncoder::new_with_quality(file.as_file_mut(), CompressionType::Fast, FilterType::Sub)
        .write_image(
            pixels.as_raw(),
            pixels.width(),
            pixels.height(),
            image::ExtendedColorType::Rgba8,
        )?;
    file.persist(path)?;
    static WRITES: AtomicUsize = AtomicUsize::new(0);
    if WRITES.fetch_add(1, Ordering::Relaxed).is_multiple_of(256) {
        prune(root);
    }
    Ok(())
}

fn prune(root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            if entry.path().extension()? != "png" {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            Some((metadata.modified().ok()?, metadata.len(), entry.path()))
        })
        .collect();
    let mut total: u64 = entries.iter().map(|(_, size, _)| size).sum();
    if total <= DISK_BUDGET {
        return;
    }
    entries.sort_by_key(|(modified, _, _)| *modified);
    for (_, size, path) in entries {
        if fs::remove_file(path).is_ok() {
            total = total.saturating_sub(size);
        }
        if total <= DISK_BUDGET * 3 / 4 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_hit_skips_source_decode_and_preserves_alpha_and_color() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("image.png");
        let original = RgbaImage::from_pixel(800, 400, image::Rgba([23, 72, 164, 100]));
        original.save(&source).unwrap();
        let cache = temp.path().join("cache");
        let first = load_cached(&source, Some(&cache), decode).unwrap();
        let second = load_cached(&source, Some(&cache), |_| {
            panic!("cache hit decoded original")
        })
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(second.dimensions(), (512, 256));
        assert_eq!(second.get_pixel(0, 0).0, [23, 72, 164, 100]);
    }

    #[test]
    fn changed_source_and_corrupt_cache_are_regenerated() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("image.png");
        let cache = temp.path().join("cache");
        RgbaImage::new(800, 400).save(&source).unwrap();
        load_cached(&source, Some(&cache), decode).unwrap();
        let old_key = cache_path(&source, &cache).unwrap();
        fs::write(&old_key, b"broken").unwrap();
        assert_eq!(
            load_cached(&source, Some(&cache), decode)
                .unwrap()
                .dimensions(),
            (512, 256)
        );
        RgbaImage::new(32, 16).save(&source).unwrap();
        assert_ne!(cache_path(&source, &cache).unwrap(), old_key);
        assert_eq!(
            load_cached(&source, Some(&cache), decode)
                .unwrap()
                .dimensions(),
            (32, 16)
        );
        let blocked = temp.path().join("not-a-directory");
        fs::write(&blocked, b"file").unwrap();
        assert!(load_cached(&source, Some(&blocked), decode).is_ok());
    }

    #[test]
    fn jpeg_thumbnail_preserves_color_and_handles_grayscale() {
        let temp = tempfile::tempdir().unwrap();
        let color = temp.path().join("color.jpg");
        image::RgbImage::from_pixel(2400, 1600, image::Rgb([200, 40, 20]))
            .save(&color)
            .unwrap();
        let pixels = decode(&color).unwrap();
        assert_eq!(pixels.dimensions(), (512, 341));
        assert!(pixels.get_pixel(0, 0)[0] > 190);
        let gray = temp.path().join("gray.jpg");
        image::GrayImage::from_pixel(1200, 800, image::Luma([80]))
            .save(&gray)
            .unwrap();
        assert_eq!(decode(&gray).unwrap().dimensions(), (512, 341));
    }

    #[test]
    fn turbojpeg_preserves_dimensions_and_small_images() {
        let temp = tempfile::tempdir().unwrap();
        for (width, height) in [(2401, 1603), (1603, 2401), (32, 16), (1, 1200)] {
            let path = temp.path().join(format!("{width}x{height}.JPG"));
            image::RgbImage::from_pixel(width, height, image::Rgb([200, 40, 20]))
                .save(&path)
                .unwrap();
            let turbo = decode_jpeg(&path).expect("JPEG should use the native decoder");
            let standard = decode_standard(&path).unwrap();
            assert_eq!(turbo.dimensions(), standard.dimensions());
            let pixel = turbo.get_pixel(0, 0);
            assert_eq!(pixel[3], 255);
            for channel in 0..3 {
                assert!(pixel[channel].abs_diff(standard.get_pixel(0, 0)[channel]) <= 3);
            }
        }
    }

    #[test]
    fn turbojpeg_handles_progressive_and_grayscale() {
        let temp = tempfile::tempdir().unwrap();
        for (format, channels) in [
            (turbojpeg::PixelFormat::RGB, 3),
            (turbojpeg::PixelFormat::GRAY, 1),
        ] {
            let pixels = vec![80u8; 1201 * 801 * channels];
            let input = turbojpeg::Image {
                pixels: pixels.as_slice(),
                width: 1201,
                height: 801,
                pitch: 1201 * channels,
                format,
            };
            let mut compressor = turbojpeg::Compressor::new().unwrap();
            compressor.set_progressive(true).unwrap();
            compressor
                .set_subsamp(if channels == 1 {
                    turbojpeg::Subsamp::Gray
                } else {
                    turbojpeg::Subsamp::Sub2x2
                })
                .unwrap();
            let bytes = compressor.compress_to_vec(input).unwrap();
            let path = temp.path().join(format!("progressive-{channels}.jpeg"));
            fs::write(&path, &bytes).unwrap();
            assert!(
                turbojpeg::Decompressor::new()
                    .unwrap()
                    .read_header(&bytes)
                    .unwrap()
                    .is_progressive
            );
            let pixels = decode_jpeg(&path).expect("progressive JPEG fast path failed");
            assert_eq!(
                pixels.dimensions(),
                decode_standard(&path).unwrap().dimensions()
            );
            assert!(pixels.get_pixel(0, 0)[0].abs_diff(80) <= 3);
        }
    }

    #[test]
    fn cmyk_jpegs_use_the_standard_color_conversion() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("cmyk.jpg");
        let pixels = [255u8, 0, 0, 255].repeat(64 * 32);
        let input = turbojpeg::Image {
            pixels: pixels.as_slice(),
            width: 64,
            height: 32,
            pitch: 64 * 4,
            format: turbojpeg::PixelFormat::CMYK,
        };
        let mut compressor = turbojpeg::Compressor::new().unwrap();
        compressor.set_subsamp(turbojpeg::Subsamp::None).unwrap();
        fs::write(&path, compressor.compress_to_vec(input).unwrap()).unwrap();
        assert!(decode_jpeg(&path).is_none());
        assert_eq!(decode(&path).unwrap(), decode_standard(&path).unwrap());
    }

    #[test]
    fn unsupported_jpeg_fast_path_falls_back_and_corruption_returns_error() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("actually-png.jpg");
        image::RgbImage::from_pixel(32, 16, image::Rgb([30, 60, 90]))
            .save_with_format(&path, image::ImageFormat::Png)
            .unwrap();
        assert!(decode_jpeg(&path).is_none());
        assert_eq!(decode(&path).unwrap().dimensions(), (32, 16));
        fs::write(&path, b"broken jpeg").unwrap();
        assert!(decode_jpeg(&path).is_none());
        assert!(decode(&path).is_err());
    }

    #[test]
    fn pruning_keeps_disk_usage_bounded() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..3 {
            fs::File::create(temp.path().join(format!("{index}.png")))
                .unwrap()
                .set_len(300 * 1024 * 1024)
                .unwrap();
        }
        let unrelated = temp.path().join("keep.txt");
        fs::write(&unrelated, b"keep").unwrap();
        prune(temp.path());
        let total: u64 = fs::read_dir(temp.path())
            .unwrap()
            .flatten()
            .filter(|entry| entry.path().extension().is_some_and(|e| e == "png"))
            .map(|entry| entry.metadata().unwrap().len())
            .sum();
        assert!(total <= DISK_BUDGET * 3 / 4);
        assert!(unrelated.exists());
    }

    #[test]
    #[ignore = "manual thumbnail timing; run with --ignored --nocapture"]
    fn benchmark_cold_and_warm() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("camera.jpg");
        image::RgbImage::from_fn(6000, 4000, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        })
        .save(&source)
        .unwrap();
        let start = std::time::Instant::now();
        std::hint::black_box(decode_standard(&source).unwrap());
        let baseline = start.elapsed();
        let start = std::time::Instant::now();
        std::hint::black_box(decode_jpeg(&source).expect("TurboJPEG fast path failed"));
        let turbo = start.elapsed();
        let cache = temp.path().join("cache");
        let start = std::time::Instant::now();
        std::hint::black_box(load_cached(&source, Some(&cache), decode).unwrap());
        let cold = start.elapsed();
        let start = std::time::Instant::now();
        std::hint::black_box(load_cached(&source, Some(&cache), decode).unwrap());
        eprintln!(
            "24 MP JPEG: standard thumbnail {baseline:?}, TurboJPEG thumbnail {turbo:?}, cold cached thumbnail {cold:?}, disk hit {:?}",
            start.elapsed()
        );
    }
}
