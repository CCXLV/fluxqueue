<p align="center">
  <img src="https://fluxqueue.ccxlv.dev/images/logo_full.png" alt="FluxQueue" />
</p>

<div align="center">

**A lightweight, resource-efficient, high-throughput task queue for Python, written in Rust.**

[![CI](https://github.com/ccxlv/fluxqueue/actions/workflows/tests.yml/badge.svg)](https://github.com/ccxlv/fluxqueue/actions/workflows/tests.yml)
![Python versions](https://img.shields.io/badge/python-3.11%20%7C%203.12%20%7C%203.13%20%7C%203.14-%2334D058)

[Documentation](https://fluxqueue.ccxlv.dev) · [Benchmarks](https://github.com/CCXLV/fluxqueue-benchmarks)

</div>

---

## Overview

FluxQueue is a task queue for Python that gets out of your way. The Rust core makes the process fast with less overhead, least dependencies, and most importantly, less memory usage. Tasks are managed through Redis.

## Key Features

- **Lightweight**: Minimal dependencies, low memory footprint, and low CPU usage even at high concurrency
- **High Throughput**: Rust-powered core for efficient task enqueueing and processing
- **Redis-Backed**: Reliable task persistence and distribution
- **Async & Sync**: Support for both synchronous and asynchronous Python functions
- **Retry Mechanism**: Built-in automatic retry with configurable limits
- **Multiple Queues**: Organize tasks across different queues
- **Simple API**: Decorator-based interface that feels natural in Python
- **Type Safe**: Full type hints support

## Requirements

- Python 3.11, 3.12, 3.13 or 3.14
- Redis server

## Installation

```bash
pip install fluxqueue[cli]
```

## How to use FluxQueue

FluxQueue can be used very easily. For example here's how `myapp/tasks.py` could look like:

```py
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()

@fluxqueue.task()
def send_email(to_email: str, subject: str, body: str):
    with email_context() as email_client:
        message = EmailMessage()
        message["From"] = "test@example.com"
        message["To"] = to_email
        message["Subject"] = subject
        message.set_content(body)
        
        email_client.send_message(message)
```

### Enqueue Tasks

Call the decorated function to enqueue it. The function returns immediately, the actual work happens in the background:

```python
send_email("user@example.com", "Hello", "This is a test email")
```

The task is now in the queue, waiting to be processed by a worker.

### Async Tasks

FluxQueue supports async functions too. Just define an async function and use the same decorator:

```python
@fluxqueue.task()
async def send_email(data: dict):
    async with email_context() as email_client:
        message = EmailMessage()
        message["From"] = "test@example.com"
        message["To"] = to_email
        message["Subject"] = subject
        message.set_content(body)
        
        await email_client.send_message(message)
```

Running the async function in an async context will also enqueue the task.

## Installing the worker

In order the tasks to be executed you need to run a FluxQueue worker. You need to install the worker on your system, recommended way of doing that is using `fluxqueue-worker`:

```bash
fluxqueue worker install
```

It picks the latest released worker based on your python version and installs it. You can also pass `--version` argument to specify the version you want to install.

## Running the worker

Running the worker is straightforward:

```bash
fluxqueue start --tasks-module-path myapp/tasks
```

In order the worker to disover your tasks you need to pass `--tasks-module-path` argument with the path to the tasks module. For more information please view the [defining and exposing tasks](https://fluxqueue.ccxlv.dev/tutorial/defininig_and_exposing_tasks) documentation.

## License

FluxQueue is licensed under the Apache-2.0 license. See [LICENSE](LICENSE) for details.
