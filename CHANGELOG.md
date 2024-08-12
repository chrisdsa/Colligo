# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
