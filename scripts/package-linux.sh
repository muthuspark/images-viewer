#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
architecture="$(dpkg --print-architecture)"
if [[ "$architecture" != amd64 ]]; then
  echo "Linux packaging currently supports amd64 only." >&2
  exit 1
fi
cargo build --release --locked
appdir="$PWD/target/package/AppDir"
debdir="$PWD/target/package/deb"
mkdir -p target/bundle
rm -rf "$appdir" "$debdir"
install -Dm755 target/release/image-viewer "$appdir/usr/bin/image-viewer"
install -Dm644 assets/image-viewer.desktop "$appdir/usr/share/applications/image-viewer.desktop"
install -Dm644 assets/icon.png "$appdir/usr/share/icons/hicolor/512x512/apps/image-viewer.png"
install -Dm644 LICENSE "$appdir/usr/share/doc/image-viewer/copyright"
install -Dm644 LICENSE-APACHE "$appdir/usr/share/doc/image-viewer/LICENSE-APACHE"
install -Dm644 NOTICE "$appdir/usr/share/doc/image-viewer/NOTICE"
install -Dm644 assets/licenses/libjpeg-turbo-LICENSE.md "$appdir/usr/share/doc/image-viewer/libjpeg-turbo-LICENSE.md"
install -Dm644 assets/licenses/libjpeg-turbo-README.ijg "$appdir/usr/share/doc/image-viewer/libjpeg-turbo-README.ijg"
install -Dm644 assets/licenses/turbojpeg-LICENSE "$appdir/usr/share/doc/image-viewer/turbojpeg-LICENSE"
cp -a "$appdir" "$debdir"
mkdir -p "$debdir/DEBIAN"
cat > "$debdir/DEBIAN/control" <<CONTROL
Package: image-viewer
Version: $version
Architecture: amd64
Maintainer: Muthukrishnan
Section: graphics
Priority: optional
Depends: libasound2, libfontconfig1, libfreetype6, libwayland-client0, libx11-6, libxcb1, libxkbcommon0, libxkbcommon-x11-0, libvulkan1
Description: A minimal native GPUI gallery for local images
CONTROL
dpkg-deb --root-owner-group --build "$debdir" "target/bundle/image-viewer_${version}_amd64.deb"
if [[ -z "${LINUXDEPLOY:-}" ]]; then
  echo "Set LINUXDEPLOY to linuxdeploy-x86_64.AppImage to also build an AppImage."
else
  export LDAI_OUTPUT="$PWD/target/bundle/image-viewer_${version}_amd64.AppImage"
  export LINUXDEPLOY_OUTPUT_VERSION="$version"
  export APPIMAGE_EXTRACT_AND_RUN=1
  "$LINUXDEPLOY" --appimage-extract-and-run --appdir "$appdir" --output appimage
fi
(cd target/bundle && find . -maxdepth 1 -type f \( -name "*.deb" -o -name "*.AppImage" \) -printf "%f\n" | sort | xargs sha256sum > SHA256SUMS)
