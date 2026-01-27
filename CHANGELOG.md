# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-01-27

### Fixed
- `compare` command now validates file existence before checking for twiggy, providing correct error messages

### Changed
- MSRV bumped to 1.88 for cargo-platform compatibility
- Removed 365 lines of unused plugin infrastructure from analyzer module

### Improved
- Test coverage increased from 82% to 91% (+188 test functions)

### Dependencies
- toml_edit: 0.23.9 → 0.24.0
- criterion: 0.7.0 → 0.8.0
- actions/cache: 4 → 5
- actions/checkout: 4 → 6

## [0.1.0] - 2025-11-08

Initial beta release (pre-1.0 API may change).

**Note**: This is a beta release. APIs and command-line interfaces are subject to change before 1.0.0.
