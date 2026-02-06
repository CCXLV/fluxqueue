# Worker

The worker is the process that actually **executes your tasks**.  
The Python client only enqueues tasks into Redis - the worker pulls them out, runs your Python functions, handles retries, and updates task state.

This page explains:

- What the worker is and how it fits into FluxQueue
- How to run it using the `fluxqueue` CLI
- Configuration (CLI flags and environment variables)
- How task discovery works
- How task execution, retries, and dead tasks are handled

---

## What the Worker Is

The worker is a standalone Rust-powered process (invoked through the `fluxqueue` CLI) that:

- Connects to Redis
- Watches one queue (e.g. `default`, `urgent`)
- Discovers your task functions from a Python module
- Pulls tasks from Redis and executes the corresponding Python functions
- Handles retries and optionally preserves dead tasks for debugging

You run it via the dedicated Python CLI (`fluxqueue-cli`), which exposes a Python-friendly interface:

```bash
fluxqueue start --tasks-module-path myapp.tasks --queue default
```

For advanced use cases, you can also run the underlying Rust binary `fluxqueue-worker` with the same flags, but the `fluxqueue` CLI is the recommended entry point for Python projects.

---

## Running the Worker

Basic usage:

```bash
fluxqueue start --tasks-module-path myapp.tasks --queue default
```

The worker runs in the **foreground**.  
If you want multiple workers (for different queues or higher throughput), run each one in its **own terminal**:

```bash
# Terminal 1 – default queue
fluxqueue start --tasks-module-path myapp.tasks --queue default

# Terminal 2 – urgent queue
fluxqueue start --tasks-module-path myapp.tasks --queue urgent
```

Press `Ctrl+C` in a worker terminal to trigger **graceful shutdown** (it finishes current work and cleans up executor metadata in Redis).

---

## CLI Options and Environment Variables

The worker accepts several options (via CLI flags or env vars):

All worker options are available via:

```bash
fluxqueue start --help
```

### `--concurrency` / `-c`

- **Flag**: `-c, --concurrency`
- **Env**: `FLUXQUEUE_CONCURRENCY`
- **Default**: `4`
- **What it does**: Number of tasks processed in parallel.

Each worker process spawns `concurrency` independent executors that:

- Reserve tasks from Redis
- Execute them concurrently
- Report completion or failure

If you want more throughput from a single worker process, increase `--concurrency`.  
You can also run multiple worker processes, each with its own `--concurrency`.

### `--redis-url` / `-r`

- **Flag**: `-r, --redis-url`
- **Env**: `FLUXQUEUE_REDIS_URL`
- **Default**: `redis://127.0.0.1:6379`
- **What it does**: Redis connection URL.

Example:

```bash
fluxqueue start \
  --redis-url redis://localhost:6379 \
  --tasks-module-path myapp.tasks \
  --queue default
```

### `--tasks-module-path` / `-t`

- **Flag**: `-t, --tasks-module-path`
- **Env**: `FLUXQUEUE_TASKS_MODULE_PATH`
- **Required**: yes
- **What it does**: Python module path where your task functions are exposed.

This should be the **module path**, not a file path:

- `myapp.tasks` → imports `myapp/tasks/__init__.py`
- `tasks` → imports `tasks.py`

The worker imports that module and inspects it to find callables that have a `task_name` and `queue` attribute (these are normally added by the `@fluxqueue.task()` decorator).  
In practice you follow the pattern described in the [Defining and Exposing Tasks](defininig_and_exposing_tasks.md) tutorial: a dedicated tasks module plus `__init__.py` with `__all__` to expose the functions you want the worker to see.

### `--queue` / `-q`

- **Flag**: `-q, --queue`
- **Env**: `FLUXQUEUE_QUEUE`
- **Default**: `default`
- **What it does**: Name of the queue to pull tasks from.

Use different queues to separate workloads:

- `default` - general tasks
- `urgent` - high-priority notifications
- `reports` - heavy reporting jobs

Example:

```bash
fluxqueue start --tasks-module-path myapp.tasks --queue urgent
```

### `--save-dead-tasks`

- **Flag**: `--save-dead-tasks`
- **Env**: _(none, flag only)_
- **Default**: off
- **What it does**: When enabled, tasks that use all retries are stored in a **dead-tasks list** in Redis instead of being dropped.

This is useful for:

- Checking later why a task failed
- Keeping failed tasks while you fix the problem

---

## How Task Discovery Works

At startup, the worker needs to know:

- Which Python functions are tasks
- Which queue each task belongs to

The workflow is:

The worker imports your tasks module (from `--tasks-module-path`).
Then it runs a small Python helper (`get_functions.py`) against that module and looks for callables with a `task_name` and `queue` attribute (set by the `@fluxqueue.task()` decorator).
Next it filters those callables by the **target queue** (the `--queue` you passed).
Finally it registers the remaining functions in an internal `TaskRegistry`, keyed by `task_name`.

Because the worker imports your module once at startup and looks only at the exposed objects, the recommended pattern is:

- Put task definitions in a dedicated module (and submodules).
- Expose them via `__init__.py` with `__all__`.

This keeps discovery predictable and avoids walking your entire codebase.

---

## How Task Execution Works

Once running, each worker executor:

First it asks Redis for the next task and moves it into a **processing** list for that executor. This keeps tasks visible and makes abandoned tasks detectable.

Then it deserializes the task payload from Redis using the shared Rust core to rebuild arguments (`args`) and keyword arguments (`kwargs`).

It looks up the task function in `TaskRegistry` by name and executes the Python function. It reconstructs `*args` and `**kwargs`, calls the function inside `tokio::task::spawn_blocking`, and if the result is awaitable it runs it with `asyncio.run`.

If the task succeeds, the executor removes it from the processing list in Redis.

If it fails, the executor logs the error and marks the task as **failed** in Redis.

---

## Retries and Dead Tasks

Each task has:

- `retries` – how many times it has been tried
- `max_retries` – max allowed attempts (set by the decorator)

The **janitor** component of the worker:

It periodically checks Redis for failed tasks.

For each failed task, if `retries < max_retries`, it puts the task back on the main queue so it can be tried again.

If `retries == max_retries`, it removes the task from the queue, and if `--save-dead-tasks` is set, it moves the task into a **dead-tasks list** in Redis for later inspection.

This separation lets you:

- Keep queues clean of permanently failing tasks.
- Still retain them for inspection when you need to debug.

---

## Executors, Heartbeats, and Janitor

For each worker process:

- **Executors**:
  - One per `--concurrency` slot.
  - Each has a unique executor ID.
  - Registers itself in Redis and pulls tasks.

- **Heartbeat**:
  - Janitor periodically updates an “executor heartbeat” in Redis for all executor IDs.
  - This makes it possible (now or in the future) to detect stale or dead executors.

- **Janitor loop**:
  - Checks failed tasks and handles retries / dead tasks.
  - Maintains executor heartbeats in Redis.
  - Responds to shutdown signals cleanly.

On shutdown (`Ctrl+C`):

- The main process signals all executors and the janitor via a shared channel.
- Executors finish their current iteration and exit.
- The worker cleans up executor registry entries in Redis.

---

## Recommended Deployment Patterns

Some practical patterns:

- **Single queue, single worker** – simplest:
  - One worker process with `--queue default`, `--concurrency` tuned to your workload.

- **Multiple queues by priority**:
  - Separate workers for `default`, `urgent`, `reports`, etc.
  - Each queue can have different concurrency and be scaled independently.

- **Horizontal scaling**:
  - Run multiple worker processes for the same queue on one or more machines.
  - Redis ensures tasks are distributed across workers.

Example (two workers for `default`, one for `urgent`):

```bash
# Machine 1
fluxqueue start --tasks-module-path myapp.tasks --queue default --concurrency 4

# Machine 2
fluxqueue start --tasks-module-path myapp.tasks --queue default --concurrency 4
fluxqueue start --tasks-module-path myapp.tasks --queue urgent --concurrency 2
```

---

## Summary

The worker is where FluxQueue’s performance characteristics show up:

- Rust handles Redis I/O, task deserialization, and scheduling.
- Python stays focused on your business logic.
- Queues, concurrency, and retries are configurable per worker.

Once your tasks are defined and exposed correctly, the worker can scale from local development to production simply by adjusting flags and running more processes.  
For details on defining tasks and exposing them to the worker, see [**Defining and Exposing Tasks**](defininig_and_exposing_tasks.md) in the tutorial.
