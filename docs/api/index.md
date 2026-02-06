# API Reference

This page documents the public Python API exposed by FluxQueue.

---

## `fluxqueue.FluxQueue`

```python
from fluxqueue import FluxQueue
```

### Constructor

```python
FluxQueue(redis_url: str | None = "redis://127.0.0.1:6379")
```

- **redis_url**: Redis connection URL.  
  Examples:
  - `redis://127.0.0.1:6379`
  - `redis://:password@hostname:6379/0`

Creates a new queue client that can enqueue tasks into Redis.

### `.task()` decorator

```python
fluxqueue = FluxQueue()

@fluxqueue.task(
    name: str | None = None,
    queue: str = "default",
    max_retries: int = 3,
)
def my_task(...):
    ...
```

The `.task()` method returns a decorator that wraps a function so that calling it enqueues a task instead of running it immediately.

Supported function types:

- Synchronous functions (`def ...`)
- Asynchronous functions (`async def ...`)

#### Parameters

- **name**: Optional custom task name.
  - If omitted, the name is derived from the function name.
  - You can use this to keep a stable task name even if you rename the function.
- **queue**: Name of the queue to push tasks to.
  - Default: `"default"`.
  - Use different queues for different kinds of workloads.
- **max_retries**: Maximum number of retries for a task before it is treated as dead.
  - Default: `3`.

#### Behavior

- Calling the decorated function:
  - Serializes `*args` and `**kwargs`.
  - Enqueues a task into Redis.
  - Returns `None` (for both sync and async functions).
- The actual execution happens in the worker process, not in your application process.

#### Examples

Sync task:

```python
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()

@fluxqueue.task()
def send_email(to: str, subject: str, body: str) -> None:
    # This code runs in the worker
    ...

# In your app code:
send_email("user@example.com", "Welcome", "Hello!")
```

Async task:

```python
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()

@fluxqueue.task(queue="notifications", max_retries=5)
async def send_notification(user_id: int, message: str) -> None:
    # This code runs in the worker
    ...

# In an async context:
await send_notification(123, "Hello")
```

Custom task name:

```python
@fluxqueue.task(name="send-welcome-email")
def send_welcome_email(user_id: int) -> None:
    ...
```

The worker will refer to this task as `send-welcome-email` regardless of the Python function name.

---

## Internal Core (`fluxqueue.fluxqueue_core`)

Most users do **not** need to interact with the Rust-backed core directly, but for completeness:

```python
from fluxqueue.fluxqueue_core import FluxQueueCore
```

!!! warning
`fluxqueue.fluxqueue_core` is an internal API. It may change without notice and it bypasses the higher-level task decorator ergonomics. Prefer using `FluxQueue.task()` unless you have a specific reason to work at the core level.

### `FluxQueueCore`

```python
FluxQueueCore(redis_url: str | None = "redis://127.0.0.1:6379")
```

Low-level interface used by `FluxQueue` to push tasks into Redis.

#### Methods

- `_enqueue(name: str, queue_name: str, max_retries: int, *args: Any, **kwargs: Any) -> None`
- `_enqueue_async(name: str, queue_name: str, max_retries: int, *args: Any, **kwargs: Any) -> Awaitable[None]`

These methods:

- Serialize the task payload.
- Push it into the appropriate Redis list for the queue.

They are considered an **internal** API and are only documented to clarify how `FluxQueue` works under the hood. Prefer using `FluxQueue.task()` instead of calling these directly.

---

## Environment Variables (Worker)

These environment variables configure the worker and are equivalent to the CLI flags documented in the worker tutorial:

- `FLUXQUEUE_CONCURRENCY` – number of tasks processed in parallel.
- `FLUXQUEUE_REDIS_URL` – Redis connection URL.
- `FLUXQUEUE_TASKS_MODULE_PATH` – Python module path where tasks are exposed.
- `FLUXQUEUE_QUEUE` – queue name to listen on.

See the [**Worker**](../tutorial/worker.md) tutorial for detailed behavior and examples.
