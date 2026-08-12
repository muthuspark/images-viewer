# Image Viewer

A minimal desktop gallery for browsing images directly from local folders. The interface keeps filesystem navigation compact and gives the image grid most of the available space.

## Features

- Browse the local filesystem with an expandable folder tree.
- View JPEG, PNG, WebP, GIF, BMP, and SVG images.
- Resize thumbnails with the mouse wheel over the gallery.
- Load thumbnails lazily for responsive browsing.
- Run on Linux, macOS, and Windows through Tauri.

## Development

Install [Node.js](https://nodejs.org/), [Rust](https://www.rust-lang.org/tools/install), and the [Tauri system prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform. Then run:

```sh
npm install
npm run tauri dev
```

To verify the frontend and Rust application without creating an installer:

```sh
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

## Production build

Create a platform-specific application bundle with:

```sh
npm run tauri build
```

The generated bundles are written beneath `src-tauri/target/release/bundle/`.

## License

This project is available under the [MIT License](LICENSE).
