# Image Viewer

A minimal native desktop gallery written in Rust with [GPUI](https://www.gpui.rs/). Photos occupy the main canvas, with a compact expandable folder tree alongside it. The application renders directly through GPUI, without a browser or WebView.

## Features

- Browse local folders from Home, with expandable filesystem navigation.
- Selecting a folder includes its images and images up to four subfolder levels below it (selected folder = depth 0; depth 4 included; depth 5 excluded). Directory symlinks are skipped. Unavailable subfolders are skipped while accessible images remain visible.
- View JPEG, PNG, WebP, GIF, BMP, and SVG images.
- Resize thumbnails with + and −; the mouse wheel scrolls normally.
- Virtualized gallery rows, bounded memory caches, and reusable 512-pixel still thumbnails on disk.
- Directory scans and image decoding run in the background. Selecting another folder cancels the previous recursive scan.
- Click an image to view it fitted to the window. Escape or × closes it; Left and Right move between images.
- Tab and Shift+Tab navigate controls; Enter opens a folder or image. + and − resize thumbnails; F5 refreshes the selected folder.
- Optionally pass a folder path when starting the application.

## Development

Install a current stable Rust toolchain. On Debian/Ubuntu, install the native dependencies:

```sh
sudo apt install build-essential pkg-config cmake nasm clang libasound2-dev \
  libfontconfig1-dev libfreetype6-dev libwayland-dev libx11-xcb-dev \
  libxkbcommon-x11-dev libvulkan-dev libssl-dev libxcb-shape0-dev libxcb-xfixes0-dev
cargo run --locked
# Or start in a specific folder:
cargo run --locked -- /path/to/photos
```

CMake, a C compiler, and NASM are needed to build the bundled static libjpeg-turbo library on x86-64. The resulting executable requires no separate TurboJPEG installation.

Linux requires a working X11 or Wayland session and a Vulkan-capable graphics driver. Linux is the supported packaging target; other platforms are not validated.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

The executable is `target/release/image-viewer`. Node.js, React, Tauri, and WebKit are no longer required.

## Thumbnail performance

Only visible rows request image previews. At most four thumbnail jobs run concurrently, and evicted queued jobs skip decoding. Resizing the grid preserves decoded previews. JPEG previews use a SIMD-enabled libjpeg-turbo decoder with fractional downscaling during decoding. Other formats and unsupported JPEGs use the standard image decoder. Image codecs are optimized even in development builds.

Generated raster thumbnails are saved under `$XDG_CACHE_HOME/image-viewer/thumbnails-v1` (usually `~/.cache/image-viewer/thumbnails-v1`). The cache key includes the source path, size, and modification time. Changed files regenerate their previews; corrupt or unavailable cache files fall back to the source image. The disk cache is periodically trimmed around 512 MiB and can be deleted safely. SVG previews use GPUI's loader.

The first visit still decodes uncached source images, with JPEGs decoded near the required thumbnail resolution. Later visits, including after restarting the app, load the small saved previews. For the fastest initial decoding, use `cargo run --release --locked -- /path/to/photos`.

Run the synthetic 24-megapixel JPEG timing check with:

```sh
cargo test --locked benchmark_cold_and_warm -- --ignored --nocapture
```

## Linux packages

```sh
scripts/package-linux.sh
# To also generate an AppImage using linuxdeploy:
LINUXDEPLOY=/absolute/path/linuxdeploy-x86_64.AppImage scripts/package-linux.sh
```

Packages and `SHA256SUMS` are written to `target/bundle/`. Install the Debian package with `sudo apt install ./target/bundle/*.deb`, or make the AppImage executable and run it. The AppImage supports `--appimage-extract-and-run` on systems without FUSE. Both formats still require a compatible graphics driver.

Versioned builds are published to [GitHub Releases](https://github.com/muthuspark/images-viewer/releases) when a stable `v<major>.<minor>.<patch>` tag matching `Cargo.toml` is pushed. Set the version in `Cargo.toml`, refresh `Cargo.lock` with `cargo check`, run the checks above, then commit and tag the release. Never reuse a published tag. Releases contain an AppImage, a Debian installer, checksums, and GitHub provenance attestations.

Verify a download with `sha256sum --check --ignore-missing SHA256SUMS`. Provenance can be checked with `gh attestation verify <installer> --repo muthuspark/images-viewer`.

## License

[MIT](LICENSE). The image-cache implementation adapts the Apache-2.0 GPUI example; see [NOTICE](NOTICE).

JPEG thumbnail decoding uses libjpeg-turbo. This software is based in part on the work of the Independent JPEG Group. See the bundled [libjpeg-turbo licenses](assets/licenses/libjpeg-turbo-LICENSE.md).
