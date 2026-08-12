## Context

Image Viewer is a Tauri 2 desktop application with committed npm and Cargo lockfiles, but it has no GitHub Actions workflows, tags, or releases. Its bundle configuration currently requests every platform target even though the intended distribution scope is Linux. The repository is public, so GitHub-hosted Linux runners and public-repository artifact attestations are available.

The release mechanism must produce understandable downloads for both portable Linux use and Debian-family installation while keeping the operational burden appropriate for a small application.

## Goals / Non-Goals

**Goals:**

- Build Linux x86-64 AppImage and Debian bundles from a version tag on a native Linux runner.
- Guarantee that a release tag agrees with all checked-in application version declarations.
- Publish complete releases rather than exposing partially built releases.
- Provide checksums and GitHub provenance attestations for the downloadable bundles.
- Make the supported platforms, formats, and release procedure explicit in repository documentation.

**Non-Goals:**

- Building or supporting macOS or Windows artifacts.
- Publishing installers through GitHub Packages.
- Producing RPM, Snap, Flatpak, AUR, or Linux ARM artifacts.
- Adding application auto-update behavior or updater signing.
- Adding GPG/AppImage signatures, a Linux package repository, or automated semantic version selection.

## Decisions

### Use GitHub Releases as the distribution surface

Installers will be attached to a GitHub Release whose tag is the application version. GitHub Packages is not used because it is a package-registry service rather than an arbitrary desktop-installer catalog. Temporary workflow artifacts are not the primary distribution channel because they have workflow-oriented access and retention semantics.

### Trigger releases from explicit semantic-version tags

The workflow will run for tags matching `v*`, then validate that the tag is exactly `v<version>` for a stable semantic version declared by the repository. The release process remains intentional: versions are updated in a normal commit before the corresponding tag is pushed. Prerelease automation is excluded from the initial change.

The Tauri configuration remains the authoritative application version, while `package.json`, `package-lock.json`, `Cargo.toml`, and `Cargo.lock` are kept aligned and checked by the workflow. This makes version drift a release-blocking error rather than silently naming bundles inconsistently.

### Build once on Ubuntu 22.04 x86-64

A single native `ubuntu-22.04` job will install Tauri's Linux prerequisites, restore Node and Rust caches, install frontend dependencies with `npm ci`, and invoke `tauri-apps/tauri-action` for AppImage and Debian targets. Ubuntu 22.04 is deliberately older than the newest hosted runner to avoid unnecessarily increasing the runtime system-library baseline.

One Linux job is preferable to a matrix because only one architecture is in scope. It also simplifies atomic draft-release publication and checksum collection.

### Publish a draft and reveal it only after verification

The Tauri action will upload the two installers to a draft release. Subsequent steps will generate a `SHA256SUMS` file from the exact uploaded build outputs, attest the installers with GitHub's OIDC-backed attestation action, upload the checksum file, and publish the release only after every step succeeds. A failed run therefore leaves a draft that maintainers can inspect or delete rather than a public partial release.

The workflow will receive only the permissions it requires: `contents: write`, `id-token: write`, and `attestations: write`. Release concurrency will be keyed by tag to prevent duplicate publication attempts.

### Keep the initial Linux format set deliberately small

The AppImage is the portable cross-distribution download; the Debian package provides normal installation on Debian, Ubuntu, Mint, and derivatives. RPM and ARM builds add runner, test, and support surface without demonstrated demand, so they can be proposed independently later.

### Configure and document Linux as the supported distribution target

Tauri bundle targets will be limited to `appimage` and `deb`. Existing 512x512 RGBA icon input is sufficient for the initial Linux bundles. Bundle metadata and README content will identify Linux support, installation commands, AppImage execution requirements, artifact verification, and the maintainer release procedure.

## Risks / Trade-offs

- **AppImage compatibility still depends on baseline system components such as glibc** → Build on Ubuntu 22.04 and document the supported baseline; move to an older dedicated build image only if real compatibility reports require it.
- **Unsigned Linux artifacts require users to establish trust separately** → Publish SHA-256 checksums and GitHub provenance attestations tied to the repository workflow.
- **A failed rerun can encounter an existing draft release or assets** → Use per-tag concurrency and make upload behavior idempotent or explicitly replace same-named draft assets.
- **Four version-bearing files can drift** → Add a release validation script or workflow step that fails before compilation and document the required version-bump sequence.
- **Tag deletion and recreation could change the source associated with a version** → Document tags as immutable and avoid workflow behavior that force-updates tags or published assets.
- **AppImage and Debian bundles do not cover every Linux packaging preference** → State the supported formats plainly and evaluate RPM, Flatpak, or ARM only from concrete demand.

## Migration Plan

1. Add and validate the Linux bundle metadata and explicit targets locally.
2. Add the release workflow and supporting version-validation logic.
3. Update README documentation to describe Linux-only support, installation, verification, and maintainer release steps.
4. Run ordinary build checks on the change without creating a public release.
5. After merge, bump the version if needed and push the first immutable `v<version>` tag to exercise the production workflow.
6. If the production run fails, leave the draft unpublished, fix the workflow in a new commit, and use a new version tag if the original tag was already shared.

## Open Questions

None for the initial implementation. Additional architectures, formats, auto-updating, and automated version selection require separate product decisions.
