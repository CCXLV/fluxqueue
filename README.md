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

- Python 3.11, 3.12, 3.13 or 3,14
- Redis server
- Linux (Windows and macOS support coming soon)

## Installation

```bash
pip install fluxqueue[cli]
```

## Example

`tasks.py`

```py
from fluxqueue import FluxQueue

# Create a FluxQueue instance (defaults to redis://127.0.0.1:6379)
fluxqueue = FluxQueue()

# Define a task using the @fluxqueue.task decorator
@fluxqueue.task()
def send_email(to: str, subject: str, body: str):
    print(f"Sending email to {to}")
    print(f"Subject: {subject}")
    print(f"Body: {body}")
```

## Enqueue Tasks

Call the decorated function to enqueue it. The function returns immediately, the actual work happens in the background:

```python
# Enqueue the task
send_email("user@example.com", "Hello", "This is a test email")
```

The task is now in the queue, waiting to be processed by a worker.

## Async Tasks

FluxQueue supports async functions too. Just define an async function and use the same decorator:

```python
@fluxqueue.task()
async def process_data(data: dict):
    # Your async processing logic
    result = await some_async_operation(data)
    return result

# Enqueue it (use await in async contexts)
await process_data({"key": "value"})
```

## Installing the worker

In order the tasks to be executed you need to run a fluxqueue worker, you need to install the worker on your system with:

```bash
fluxqueue worker install
```

It picks the latest released worker based on your python version and installs it. You can also pass `--version` argument to specify the version you want to install.

## Running the worker

Running the worker is straightforward:

```bash
fluxqueue start --tasks-module-path myapp/tasks
```

In order the worker to disover your tasks you need to pass `--tasks-module-path` argument with the path to the tasks module. For more information please view the [documentation](https://fluxqueue.ccxlv.dev/tutorial/defininig_and_exposing_tasks).

## License

FluxQueue is licensed under the Apache-2.0 license. See [LICENSE](LICENSE) for details.
