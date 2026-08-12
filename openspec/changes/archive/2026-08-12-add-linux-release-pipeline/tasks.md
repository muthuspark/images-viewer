## 1. Linux Bundle Configuration

- [x] 1.1 Restrict Tauri bundle targets to AppImage and Debian and add the Linux package metadata needed for identifiable release artifacts.
- [x] 1.2 Confirm the existing Linux icon input is valid and that the configured application version is aligned across the npm and Cargo manifests and lockfiles.

## 2. Release Version Validation

- [x] 2.1 Add a repository script that validates a `v<semver>` release tag against `tauri.conf.json`, `package.json`, `package-lock.json`, `Cargo.toml`, and the application entry in `Cargo.lock`.
- [x] 2.2 Add automated checks covering both a matching tag and representative malformed or mismatched tags.

## 3. GitHub Release Automation

- [x] 3.1 Add a tag-triggered Ubuntu 22.04 workflow with per-tag concurrency, least-privilege release and attestation permissions, npm lockfile installation, Rust setup/cache, and the required Tauri Linux system packages.
- [x] 3.2 Configure the workflow to validate the tag and build only x86-64 AppImage and Debian bundles with the committed lockfiles.
- [x] 3.3 Configure draft GitHub Release creation and versioned installer uploads, then generate and upload `SHA256SUMS` covering both installers.
- [x] 3.4 Add GitHub provenance attestations for both installers and publish the draft only after every required build and integrity step succeeds.

## 4. Documentation

- [x] 4.1 Update the README to state Linux-only distribution support and explain AppImage and Debian download, installation, and SHA-256 verification.
- [x] 4.2 Document the maintainer release sequence, version synchronization, immutable tag convention, supported formats, and intentionally unsupported distribution targets.

## 5. Verification

- [x] 5.1 Run the frontend build, locked Cargo checks, release-version validation tests, and a local Linux Tauri bundle build for both configured formats.
- [x] 5.2 Validate the GitHub Actions workflow syntax and inspect the generated artifact names and checksum inputs against the release specification.
