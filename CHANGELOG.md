# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

<!-- next-header -->

## [Unreleased] - ReleaseDate

### Added
- new argument `--format` to support multiple Zotero export formats

## [0.3.0] - 2025-09-14

### Added
- implemented pagination - can now export library with arbitrary size

### Removed
- argument `-u, --user-id` - not needed anymore, the id is now fetched with the api key
- the short versions for all arguments: `-a, -f, -i` - for better readability, only long versions are supported

## [0.2.4] - 2025-09-12

### Changed
- only re-export the library when it was changed since the last export
- now exporting 100 items instead of 25 (pagination is not yet implemented!)

## [0.2.3] - 2025-09-11

### Added
- releases for multiple platforms

### Fixed
- no error if the output file does not exist yet - it will just be created

## [0.2.2] - 2025-09-11

### Changed
- program is now self-contained - it is not required to have openssl installed anymore

## [0.2.1] - 2025-09-11

### Added
- documentation

## [0.2.0] - 2025-09-11  

### Added
- new argument "interval" for periodic exports

### Changed
- made file argument obligatory

## [0.1.0] - 2025-09-10

### Added
- simple executable that fetches a zotero lib once in biblatex format

<!-- next-url -->
[Unreleased]: https://github.com/assert-rs/predicates-rs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/assert-rs/predicates-rs/compare/v0.2.4...v0.3.0
[0.2.4]: https://github.com/assert-rs/predicates-rs/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/assert-rs/predicates-rs/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/assert-rs/predicates-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/assert-rs/predicates-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/assert-rs/predicates-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/fabiofranke/zotex/compare/a9179286c9c33a5113a2d0414d58a2f2854da6e5...v0.1.0
