## Why

The application has no repeatable way to produce or publish Linux installers, so users must build it locally and releases cannot be tied reliably to source versions. A Linux-only release pipeline will provide durable, versioned downloads without adding unsupported macOS or Windows distribution work.

## What Changes

- Add a tag-triggered GitHub Actions workflow that builds the Tauri application on Linux.
- Publish x86-64 AppImage and Debian package assets to a versioned GitHub Release.
- Validate release tags against the application version before building.
- Generate SHA-256 checksums and GitHub build-provenance attestations for published artifacts.
- Restrict Tauri bundle targets and project documentation to the supported Linux release formats.
- Document the release process and Linux installation choices.

## Capabilities

### New Capabilities

- `linux-release-distribution`: Defines version validation, reproducible Linux builds, release assets, integrity metadata, and release documentation.

### Modified Capabilities

None.

## Impact

- Adds a GitHub Actions release workflow and repository automation configuration.
- Changes Tauri bundle configuration and Linux package metadata.
- Updates README claims and installation/release documentation.
- Uses GitHub Releases and artifact attestations; it does not publish installers to GitHub Packages.
- Requires GitHub Actions permissions for release contents, OIDC identity tokens, and attestations.
