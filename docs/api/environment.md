# Environment Variables (Worker)

These environment variables configure the worker and are equivalent to the CLI flags documented in the worker tutorial:

- `FLUXQUEUE_CONCURRENCY` – number of tasks processed in parallel.
- `FLUXQUEUE_REDIS_URL` – Redis connection URL.
- `FLUXQUEUE_TASKS_MODULE_PATH` – Python module path where tasks are exposed.
- `FLUXQUEUE_QUEUE` – queue name to listen on.

See the [**Worker**](../tutorial/worker.md) tutorial for detailed behavior and examples.
