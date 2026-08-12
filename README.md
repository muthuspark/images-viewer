# Image Viewer

A minimal desktop gallery for browsing images directly from local folders. The interface keeps filesystem navigation compact and gives the image grid most of the available space.

## Features

- Browse the local filesystem with an expandable folder tree.
- View JPEG, PNG, WebP, GIF, BMP, and SVG images.
- Resize thumbnails with the mouse wheel over the gallery.
- Load thumbnails lazily for responsive browsing.
- Run as a native Linux desktop application through Tauri.

## Download

Versioned Linux x86-64 builds are available from [GitHub Releases](https://github.com/muthuspark/images-viewer/releases). Each release provides:

- An AppImage for portable use across supported Linux desktops.
- A Debian package for Debian, Ubuntu, Mint, and derivatives.
- A `SHA256SUMS` manifest covering both installers.

This project does not currently distribute macOS, Windows, RPM, Snap, Flatpak, ARM, or auto-update packages.

### Verify a download

Download `SHA256SUMS` and the installer into the same directory, then run:

```sh
sha256sum --check --ignore-missing SHA256SUMS
```

GitHub also records build-provenance attestations for both installers. With the GitHub CLI installed, an installer can be verified against this repository with:

```sh
gh attestation verify ./*.AppImage --repo muthuspark/images-viewer
```

### Run the AppImage

```sh
chmod +x ./*.AppImage
./*.AppImage
```

Some distributions require the FUSE 2 compatibility package to run AppImages. The AppImage can instead be started without FUSE using its `--appimage-extract-and-run` option.

### Install the Debian package

```sh
sudo apt install ./*_amd64.deb
```

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

Create the supported Linux AppImage and Debian bundles with:

```sh
npm run tauri build
```

The generated bundles are written beneath `src-tauri/target/release/bundle/`.

## Maintainer release process

Releases use stable semantic versions and immutable annotated tags in the form `v<major>.<minor>.<patch>`. Before creating a tag:

1. Choose the next version and update `package.json` plus `package-lock.json`:

   ```sh
   npm version 0.2.0 --no-git-tag-version
   ```

2. Set the same version in `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`, then refresh the application entry in `Cargo.lock`:

   ```sh
   cargo check --manifest-path src-tauri/Cargo.toml
   ```

3. Validate all five version declarations and run the local checks:

   ```sh
   npm run validate:release-version -- v0.2.0
   npm run test:release-version
   npm run build
   cargo check --manifest-path src-tauri/Cargo.toml --locked
   npm run tauri build -- --bundles appimage,deb -- --locked
   ```

4. Commit the version changes, merge them to `main`, and create the release tag on that commit:

   ```sh
   git tag -a v0.2.0 -m "Image Viewer v0.2.0"
   git push origin v0.2.0
   ```

The tag triggers the Linux release workflow. It publishes the GitHub Release only after both installers, their checksums, and their provenance attestations succeed. Never move, delete, or reuse a pushed release tag; fix a failed shared release with a new patch version.

## License

This project is available under the [MIT License](LICENSE).
