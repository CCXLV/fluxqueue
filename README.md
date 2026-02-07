<p align="center">
  <img src="https://fluxqueue.ccxlv.dev/images/logo_full.png" alt="FluxQueue" />
</p>

<div align="center">

**A blazingly fast, lightweight task queue for Python, written in Rust.**

[Documentation](https://fluxqueue.ccxlv.dev)

</div>

---

## Overview

FluxQueue is a task queue for Python that gets out of your way. The Rust core makes the process fast with less overhead, least dependencies, and most importantly, less memory usage. Tasks are managed through Redis.

## Key Features

- **Fast**: Rust-powered core for efficient task enqueueing and processing
- **Lightweight**: Minimal dependencies and low memory footprint
- **Redis-Backed**: Reliable task persistence and distribution
- **Async & Sync**: Support for both synchronous and asynchronous Python functions
- **Retry Mechanism**: Built-in automatic retry with configurable limits
- **Multiple Queues**: Organize tasks across different queues
- **Simple API**: Decorator-based interface that feels natural in Python
- **Type Safe**: Full type hints support

## Requirements

- Python 3.11, 3.12, or 3.13
- Redis server
- Linux (Windows and macOS support coming soon)

## License

FluxQueue is licensed under the Apache-2.0 license. See [LICENSE](LICENSE) for details.
