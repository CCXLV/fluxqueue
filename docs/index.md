<style>
.md-content .md-typeset h1 { display: none; }
</style>
<p align="center">
  <img src="images/logo_full.png" alt="FluxQueue" />
</p>

Welcome to FluxQueue documentation. FluxQueue is a lightweight, resource-efficient, high-throughput task queue for Python, written in Rust.

## Quick Links

- [Installation](tutorial/installation.md) - Get started with FluxQueue
- [Quick Start](tutorial/quickstart.md) - Your first task queue
- [Defining and Exposing Tasks](tutorial/defininig_and_exposing_tasks.md) - Organize and expose tasks for the worker
- [Worker Setup](tutorial/worker.md) - Deploy and run workers
- [How it Works](how-it-works/index.md) - Learn more about how FluxQueue actually works
- [Examples](examples/index.md) - FluxQueue examples with **Flask**, **Django**, **FastAPI**
- [API Reference](api/index.md) - Complete API documentation

## What is FluxQueue?

FluxQueue is a task queue for Python that gets out of your way. Built on a multi-threaded Tokio runtime, FluxQueue delivers high throughput while maintaining low memory usage. The Rust core ensures minimal overhead and dependencies, making it an efficient solution for background task processing. Tasks are managed through Redis.

## Key Features

- **Lightweight**: Minimal dependencies, low memory footprint, and low CPU usage even at high concurrency
- **High Throughput**: Rust-powered core for efficient task enqueueing and processing
- **Redis-Backed**: Reliable task persistence and distribution
- **Async & Sync**: Support for both synchronous and asynchronous Python functions
- **Retry Mechanism**: Built-in automatic retry with configurable limits
- **Multiple Queues**: Organize tasks across different queues
- **Simple API**: Decorator-based interface that feels natural in Python
- **Type Safe**: Full type hints support
- **Context Classes**: Access task metadata and manage thread-persistent resources with the Context class

## Requirements

- Python 3.11, 3.12, 3.13 or 3.14
- Redis server

## Getting Started

Ready to start using FluxQueue? Head over to the [Installation](tutorial/installation.md) guide to get set up, then check out the [Quick Start](tutorial/quickstart.md) guide to create your first task.
