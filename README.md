<p align="center">
  <img src="https://fluxqueue.ccxlv.dev/images/logo_full.png" alt="FluxQueue" />
</p>

<div align="center">

**A lightweight, resource-efficient, high-throughput task queue for Python, written in Rust.**

[![CI](https://github.com/ccxlv/fluxqueue/actions/workflows/tests.yml/badge.svg)](https://github.com/ccxlv/fluxqueue/actions/workflows/tests.yml)
![Python versions](https://img.shields.io/badge/python-3.11%20%7C%203.12%20%7C%203.13%20%7C%203.14-%2334D058)

[Documentation](https://fluxqueue.ccxlv.dev)

</div>

---

## Overview

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
def send_email(to_email: str):
    with email_context() as email_client:
        send_email(to_email, email_client)
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
async def send_email_task(to_email: str):
    async with email_context() as email_client:
        await send_email(to_email, email_client)
```

Running the async function in an async context will also enqueue the task.

### Tasks with Context

FluxQueue provides a `Context` class that gives you access to task metadata and allows you to manage thread-persistent resources. Use `task_with_context()` decorator to enable this feature:

```python
from fluxqueue import FluxQueue, Context

fluxqueue = FluxQueue()

@fluxqueue.task_with_context()
def process_data_task(ctx: Context, data: str):
    # Access task metadata
    print(f"Task ID: {ctx.metadata.task_id}")
    print(f"Retry count: {ctx.metadata.retries}")

    process_data(data)
```

You can also subclass `Context` to create custom contexts with domain-specific resources. This example shows how to create a `DbContext` that manages database connections efficiently by reusing them across tasks in the same worker thread:

```python
from contextlib import asynccontextmanager
from fluxqueue import FluxQueue, Context
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker, AsyncSession

class DbContext(Context):
    def __init__(self) -> None:
        super().__init__()

    def _get_local_session(self):
        if "session" not in self.thread_storage:
            engine = create_async_engine(DATABASE_URL)
            self.thread_storage["session"] = async_sessionmaker(
                bind=engine, expire_on_commit=False
            )
        return self.thread_storage["session"]

    @asynccontextmanager
    async def session_context(self):
        local_session = self._get_local_session()
        async with local_session() as session:
            try:
                yield session
                await session.commit()
            except Exception:
                await session.rollback()
                raise

@fluxqueue.task_with_context()
async def create_user_task(ctx: DbContext, email: str, username: str):
    async with ctx.session_context() as db_session:
        user = User(email=email, username=username)
        db_session.add(user)
```

The context parameter is automatically injected by the worker and is not part of the function's public signature when enqueueing:

```python
# Just call with your regular arguments
create_user_task("user@example.com", "johndoe")
```

## Installing the worker

In order the tasks to be executed you need to run a FluxQueue worker. You need to install the worker on your system, recommended way of doing that is using `fluxqueue-cli` which comes with the `[cli]` extra:

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
