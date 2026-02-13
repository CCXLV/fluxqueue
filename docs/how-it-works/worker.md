# FluxQueue Worker

The FluxQueue worker is a Rust binary which internally runs a `tokio` multi-threaded runtime. The `--concurrency` argument it takes actually determines the number of `tokio` tasks it is going to spawn. That's why even with a single worker and higher `concurrency` it performs well with high concurrency while keeping memory usage low.

## Executor Loop

An executor loop is a `tokio` task that the runtime spawns for a given worker. For each unit of `concurrency`, the worker creates one executor loop that:

- Dequeues the next task from Redis and marks it as "processing".
- Looks up the corresponding Python function in the task registry.
- Executes the function and waits for it to finish.
- On success, removes the task from the processing list.
- On failure, marks the task as failed so it can be retried by the janitor loop.

The task registry itself is a shared, thread-safe registry of task functions, so all executors can look up Python callables by name without copying them. This lets multiple executor loops perform fast, concurrent lookups from the same set of registered tasks.

## Janitor Loop

In addition to the executor loops, the worker also runs a single janitor loop as another `tokio` task. The janitor loop is responsible for:

- Periodically checking Redis for tasks that have failed.
- Deciding whether a failed task should be retried or considered "dead" based on its `retries` and `max_retries` fields.
- Requeuing failed tasks back into the main queue if they still have retries left.
- Optionally pushing tasks that have used all retries into a "dead tasks" list when `--save-dead-tasks` is enabled. These dead tasks are kept only for inspection and debugging and are not reprocessed by the worker.
- Sending heartbeat updates for all executors so they can be tracked as healthy by the system.

This separation lets executor loops focus purely on running user code, while the janitor loop handles retries, cleanup, and health tracking in the background.

## Graceful Shutdown

When you stop the worker with `Ctrl+C`:

- The worker broadcasts a shutdown signal to all executor loops and the janitor loop.
- Each loop finishes the task it is currently processing but does not take new work from Redis.
- After all loops exit, the worker removes executor registrations from Redis and then terminates.
- The main task queue is not removed, so a restarted worker continues processing any remaining tasks in the queue.

## Handling Async Tasks

When a task is dispatched, the worker first calls the corresponding Python function with its deserialized `args` and `kwargs`. If the function returns a regular value, it is treated as a **synchronous task** and completes on the executor loop itself.

If the function returns a coroutine (i.e. an `async def` in Python), it is treated as an **asynchronous task**. In that case:

- The coroutine object is sent to a dedicated Python dispatcher.
- The dispatcher converts the Python coroutine into a Rust `Future` using `pyo3_async_runtimes::tokio::into_future`.
- That future is then polled to completion on the Tokio runtime, so the async Python code runs concurrently with other tasks.

This design lets you freely mix sync and async Python task functions while keeping the Rust worker fully async and non-blocking.

## Python Dispatcher

The Python dispatcher is a small component that owns a dedicated Python event loop and hides the complexity of driving async Python code from Rust. Internally, it:

- Runs a background thread that initializes Python and starts a Tokio runtime bound to the Python interpreter.
- Listens on an internal channel for incoming Python coroutine objects sent by executor loops.
- For each coroutine, wraps it into a Rust `Future` via `pyo3_async_runtimes::tokio::into_future` and awaits it on the dispatcher’s Tokio runtime.

Because all async Python work is funneled through this dispatcher, executor loops still acquire the Python GIL when calling into Python code, but they do not need to manage a Python async event loop themselves. They send the coroutine to the dispatcher over an internal `tokio::sync::mpsc` channel and then await the result like any other async Rust function.
