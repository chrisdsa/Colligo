# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2025-11-28

### Added

- More info on progress bar. Now indicate the step being executed, ex: resolving deltas, etc.

### Changed

- Configure repository to filter out blog when using `--light`. This config stays afterward, even for full sync.
- Trim values when parsing the manifest.

### Fixed

- Add missing --tags arg in git fetch
- Add missing --progress arg in git fetch

## [0.5.0] - 2025-11-26

### Changed

- Updated dependencies.
- Use git init instead of git clone for new repositories.

## [0.4.2] - 2025-03-21

### Changed

- Now using Tokio task instead of threads.
- Improved the code.
- Update rust version in CI.

### Fixed

- The progress bar progression. It is now much more responsive.

## [0.4.1] - 2024-08-27

### Added

- Sanity check steps for GitHub Actions.

### Removed

- GitLab CI/CD configuration.

### Changed

- Now using GitHub to make it easier for users to contribute.
- Version is now generated from the version in Cargo.toml and the git commit sha.

## [0.4.0] - 2024-08-12

### Added

- Add `status` argument to list all projects in the manifest and indicate if they are modified.
- Add `list` argument to list all projects in the manifest.

## [0.3.1] - 2024-05-03

### Fixed

- Fix bug where commit message or path with the word "error" would be considered an error when executing git command.

## [0.3.0] - 2024-02-20

### Added

- Progress bar when synchronizing the projects.
- Lightweight clone.
- Quiet mode.
- Force option to overwrite local changes.
- Copy a directory recursively action.
- Delete repository action.

### Changed

- When synchronizing the projects, using fetch, checkout, merge instead of fetch, checkout, pull.
- Improve error messages. Now indicating where the error occurred.
- Error message are message from git with "error" or "fatal" in the message.
- Testing if the repository is dirty instead of relying on git error message.
- Add the copydir and delete_project action to the default manifest.
- Changed the default manifest default parameter from "gitlab.com" to "hostname.com".

## [0.2.0] - 2023-08-04

### Changed

- Renamed project to Colligo. Motivation: The name is more unique and does not
  conflict with other projects or utilities (like manifest in Ubuntu).

### Fixed

- Fix version not generated correctly when deployed on GitHub.

## [0.1.1] - 2023-08-02

### Fixed

- Fix bug where --version argument was not working.

## [0.1.0] - 2023-08-02

### Added

- Download repositories described in a manifest using SSH(default) or HTTPS.
- Pin manifest to current commit id.
- Debug mode. It is currently not exhaustive.
- Support for Linux and Windows.
