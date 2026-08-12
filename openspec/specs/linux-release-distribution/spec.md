# Linux Release Distribution Specification

## Purpose

Define the supported, reproducible, and verifiable process for distributing versioned Linux builds through GitHub Releases.

## Requirements

### Requirement: Version-tagged release trigger
The release system SHALL start Linux distribution builds only for an explicitly pushed version tag and SHALL reject a tag that does not exactly match the repository's application version declarations.

#### Scenario: Matching version tag
- **WHEN** a maintainer pushes a tag in the form `v<version>` and every application manifest declares `<version>`
- **THEN** the Linux release build proceeds for that tagged commit

#### Scenario: Version mismatch
- **WHEN** the pushed tag differs from any checked-in application version declaration
- **THEN** the workflow fails before building or publishing installers

### Requirement: Reproducible Linux build inputs
The release system SHALL build from the tagged commit on an Ubuntu 22.04 x86-64 runner using the committed npm and Cargo lockfiles.

#### Scenario: Locked dependency installation
- **WHEN** the release job prepares the frontend and Rust build
- **THEN** it installs npm dependencies with lockfile enforcement and uses the committed Cargo lockfile

### Requirement: Supported Linux artifacts
Each successful release SHALL provide an x86-64 AppImage and an x86-64 Debian package built by Tauri from the same tagged commit.

#### Scenario: Successful bundle build
- **WHEN** Tauri completes the release build
- **THEN** both an AppImage artifact and a Debian package artifact exist and contain the validated application version in their release metadata

### Requirement: Atomic public release
The release system SHALL keep a GitHub Release in draft state until every required installer and integrity step succeeds.

#### Scenario: Complete release
- **WHEN** both installers, their checksum manifest, and their provenance attestations have been created successfully
- **THEN** the workflow publishes the GitHub Release for the triggering tag

#### Scenario: Failed release build
- **WHEN** any required build, checksum, attestation, or upload step fails
- **THEN** no incomplete release is made public

### Requirement: Artifact integrity information
The release system SHALL publish SHA-256 checksums for every installer and SHALL create GitHub build-provenance attestations for the installers.

#### Scenario: User verifies a download
- **WHEN** a user downloads an installer from a published release
- **THEN** the same release provides a checksum manifest covering that installer and GitHub contains provenance tied to the repository workflow

### Requirement: Least-privilege and duplicate-run controls
The release workflow SHALL declare only the GitHub permissions required to publish releases and attest artifacts, and SHALL prevent concurrent runs from publishing the same tag.

#### Scenario: Duplicate release invocation
- **WHEN** two workflow runs target the same version tag
- **THEN** the configured concurrency policy prevents simultaneous publication attempts

### Requirement: Linux distribution documentation
Repository documentation SHALL describe Linux as the supported desktop distribution target, explain AppImage and Debian installation and checksum verification, and document the maintainer's tag-based release sequence.

#### Scenario: User selects an installer
- **WHEN** a Linux user reads the project documentation
- **THEN** the user can determine which published artifact fits their system and how to install or run it

#### Scenario: Maintainer creates a release
- **WHEN** a maintainer reads the release documentation
- **THEN** the maintainer can update versions, validate the project, and trigger a release using an immutable version tag
