<p align="center">
  <img src="../public/logo_full.png" alt="FluxQueue" />
</p>

# FluxQueue Documentation

Welcome to FluxQueue documentation. FluxQueue is a blazingly fast, lightweight task queue for Python, powered by Rust.

<!--
## Quick Links

- [Installation](installation.md) - Get started with FluxQueue
- [Quick Start](quickstart.md) - Your first task queue
- [Worker Setup](worker.md) - Deploy and run workers
- [API Reference](api.md) - Complete API documentation
-->

## What is FluxQueue?

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

## Getting Started

Ready to start using FluxQueue? Head over to the [Installation](installation.md) guide to get set up, then check out the [Quick Start](quickstart.md) guide to create your first task.
