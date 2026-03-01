# Release Notes

All notable changes to FluxQueue will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-03-01

### Added

- **Context Classes**: New `Context` class for accessing task metadata and managing thread-persistent resources ([#114](https://github.com/CCXLV/fluxqueue/pull/114))
  - `@fluxqueue.task_with_context()` decorator for tasks that need context access
  - `Context.metadata` property for accessing task execution information (task ID, retry count, etc.)
  - `Context.thread_storage` property for sharing resources across tasks in the same worker thread
  - Support for custom context classes by subclassing `Context`
  - Prevents event loop issues in multi-threaded environments (e.g., asyncpg "got future pending attached to a different loop" errors)
  - Works with both synchronous and asynchronous tasks
- Client library version check before worker initialization ([#128](https://github.com/CCXLV/fluxqueue/pull/128))
- Tests to cover the context feature ([#130](https://github.com/CCXLV/fluxqueue/pull/130))

### Changed

- Moved function calls to dispatcher ([#116](https://github.com/CCXLV/fluxqueue/pull/116))
- Improved executor startup speed ([#117](https://github.com/CCXLV/fluxqueue/pull/117))
- Made startup logs more clear ([#118](https://github.com/CCXLV/fluxqueue/pull/118))
- Improved examples documentation ([#119](https://github.com/CCXLV/fluxqueue/pull/119))
- Fixed worker installation instructions in README ([#120](https://github.com/CCXLV/fluxqueue/pull/120))
- Bumped pyo3 dependency versions ([#121](https://github.com/CCXLV/fluxqueue/pull/121))
- Improved client docstrings ([#122](https://github.com/CCXLV/fluxqueue/pull/122))
- Improved `_run_async_task` docstring for clarity ([#126](https://github.com/CCXLV/fluxqueue/pull/126))
- Various fixes and updates ([#124](https://github.com/CCXLV/fluxqueue/pull/124))

### Fixed

- Fixed tasks with the base Context class ([#123](https://github.com/CCXLV/fluxqueue/pull/123))
- Fixed client library version check for older workers ([#129](https://github.com/CCXLV/fluxqueue/pull/129))
- Reverted disallowing `_Context` as context name ([#127](https://github.com/CCXLV/fluxqueue/pull/127))

## [0.2.1] - 2026-02-21

### Changed

- Moved examples to docs ([#110](https://github.com/CCXLV/fluxqueue/pull/110))
- Moved core to crates directory ([#111](https://github.com/CCXLV/fluxqueue/pull/111))
- Renamed Rust core module ([#112](https://github.com/CCXLV/fluxqueue/pull/112))
- Improved task decorator type inference ([#113](https://github.com/CCXLV/fluxqueue/pull/113))

## [0.2.0] - 2026-02-17

### Added

- Logs after task finishes ([#80](https://github.com/CCXLV/fluxqueue/pull/80))
- Python version badges in README and classifiers/keywords in pyproject.toml ([#84](https://github.com/CCXLV/fluxqueue/pull/84))
- Unit and integration tests ([#90](https://github.com/CCXLV/fluxqueue/pull/90))
- Support for Windows and macOS ([#94](https://github.com/CCXLV/fluxqueue/pull/94))
- Tests status badge in README ([#92](https://github.com/CCXLV/fluxqueue/pull/92))

### Changed

- Updated way of running async task functions ([#83](https://github.com/CCXLV/fluxqueue/pull/83))
- Improved README clarity and documentation ([#85](https://github.com/CCXLV/fluxqueue/pull/85), [#86](https://github.com/CCXLV/fluxqueue/pull/86), [#87](https://github.com/CCXLV/fluxqueue/pull/87))
- Updated get_functions.py script to raise exception on duplication ([#78](https://github.com/CCXLV/fluxqueue/pull/78))
- Bumped worker version to 0.2.0-beta.4 ([#95](https://github.com/CCXLV/fluxqueue/pull/95))
- Updated pyproject description ([#108](https://github.com/CCXLV/fluxqueue/pull/108))

### Fixed

- Fixed executors heartbeat flaw on startup ([#82](https://github.com/CCXLV/fluxqueue/pull/82))
- Fixed test_path_to_module_path to work on Windows ([#93](https://github.com/CCXLV/fluxqueue/pull/93))
- Fixed various workflow files (release-worker, build-worker, publish workflows) ([#96](https://github.com/CCXLV/fluxqueue/pull/96), [#97](https://github.com/CCXLV/fluxqueue/pull/97), [#98](https://github.com/CCXLV/fluxqueue/pull/98), [#99](https://github.com/CCXLV/fluxqueue/pull/99), [#100](https://github.com/CCXLV/fluxqueue/pull/100), [#101](https://github.com/CCXLV/fluxqueue/pull/101), [#102](https://github.com/CCXLV/fluxqueue/pull/102), [#105](https://github.com/CCXLV/fluxqueue/pull/105), [#106](https://github.com/CCXLV/fluxqueue/pull/106))
- Fixed license name in pyproject.toml ([#107](https://github.com/CCXLV/fluxqueue/pull/107))
- Fixed pyproject classifiers ([#109](https://github.com/CCXLV/fluxqueue/pull/109))

[0.3.0]: https://github.com/CCXLV/fluxqueue/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/CCXLV/fluxqueue/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/CCXLV/fluxqueue/compare/v0.2.0-beta.4...v0.2.0
