# Context Classes

The Context class provides access to task execution metadata and enables efficient resource management through thread-persistent storage. Use it when you need to share resources across multiple task executions in the same worker thread.

## Overview

The `Context` class provides a dual-layer storage system:

- **Worker Layer (`thread_storage`)**: A dictionary that persists across all tasks executed by the same worker thread. Use this for heavy resource pooling (e.g., database engines, HTTP clients) to avoid re-initialization overhead.
- **Task Layer (`metadata`)**: Isolated metadata for each task execution via ContextVars. Provides read-only access to the current task's unique identity and execution state.

## Basic Usage

To use the Context feature, decorate your task function with `@fluxqueue.task_with_context()` instead of `@fluxqueue.task()`. The function must accept a context parameter as its first argument, typed as `Context` or a subclass of `Context`:

```python
from fluxqueue import FluxQueue, Context

fluxqueue = FluxQueue()

@fluxqueue.task_with_context()
def process_data_task(ctx: Context, data: str):
    # Access task metadata
    print(f"Task ID: {ctx.metadata.task_id}")
    print(f"Retry count: {ctx.metadata.retries}")
    print(f"Max retries: {ctx.metadata.max_retries}")
    print(f"Enqueued at: {ctx.metadata.enqueued_at}")

    # Process the data
    process_data(data)
```

When you enqueue the task, the context parameter is automatically injected by the worker and is not part of the function's public signature:

```python
# Just call with your regular arguments - no context needed!
process_data_task("some data")
```

## Task Metadata

The `metadata` property provides read-only access to task execution information:

- `task_id`: Unique identifier for the current task execution
- `retries`: Number of times this task has been retried
- `max_retries`: Maximum number of retry attempts allowed
- `enqueued_at`: ISO 8601 timestamp of when the task was originally enqueued

This metadata is isolated per task using Python's `ContextVar`, ensuring data integrity even when multiple tasks execute concurrently on the same thread.

## Custom Context Classes

You can subclass `Context` to create domain-specific contexts that provide convenient access to resources. This is especially useful for database connections, HTTP clients, or other resources that benefit from pooling:

```python
from contextlib import asynccontextmanager
from fluxqueue import FluxQueue, Context
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker, AsyncSession

class DbContext(Context):
    """
    Custom context for managing database connections.

    This context reuses database engines across tasks in the same worker thread,
    significantly reducing connection overhead.
    """
    def __init__(self) -> None:
        super().__init__()

    def _get_local_session(self):
        """
        Get or create a session maker for this worker thread.
        """
        session = self.thread_storage.get("session")
        if session is None:
            engine = create_async_engine(DATABASE_URL)
            session = async_sessionmaker(
                bind=engine, expire_on_commit=False
            )
            self.thread_storage["session"] = session
        return session

    @asynccontextmanager
    async def session_context(self):
        """
        Context manager for database sessions with automatic commit/rollback.
        """
        local_session = self._get_local_session()
        async with local_session() as session:
            try:
                yield session
                await session.commit()
            except Exception:
                await session.rollback()
                raise

# Use the custom context
@fluxqueue.task_with_context()
async def create_user_task(ctx: DbContext, email: str, username: str):
    async with ctx.session_context() as db_session:
        user = User(email=email, username=username)
        db_session.add(user)
        # Session commits automatically on success, rolls back on exception
```

## Benefits

Using Context classes provides several key benefits:

1. **Resource Efficiency**: Share expensive resources (like database engines) across multiple tasks in the same worker thread, reducing initialization overhead.

2. **Task Isolation**: Each task gets its own isolated metadata via ContextVars, ensuring data integrity even with concurrent execution.

3. **Type Safety**: Full type hint support means your IDE and type checkers understand the context structure.

4. **Clean API**: The context parameter is automatically injected, so your task functions have a clean public interface without exposing implementation details.

5. **Flexibility**: Subclass `Context` to create domain-specific contexts tailored to your application's needs.

!!! tip "Avoiding Event Loop Issues in Multi-threaded Environments"

    FluxQueue's multi-threaded Tokio runtime can cause issues with async database libraries like `asyncpg` that throw errors such as "got future pending attached to a different loop" when resources are shared across threads.

    By using the Context class with `thread_storage`, each executor in the worker gets its own database engine instance. Combined with Python's context managers, you can safely acquire database sessions and run queries without event loop conflicts. This ensures that each worker thread maintains its own isolated database connection pool, preventing cross-thread resource sharing issues.

## Async and Sync Support

Context classes work seamlessly with both synchronous and asynchronous tasks:

```python
# Synchronous task with context
@fluxqueue.task_with_context()
def sync_task(ctx: Context, data: str):
    print(f"Processing {data} in task {ctx.metadata.task_id}")
    process(data)

# Async task with context
@fluxqueue.task_with_context()
async def async_task(ctx: Context, data: str):
    print(f"Processing {data} in task {ctx.metadata.task_id}")
    await process_async(data)
```

## Best Practices

1. **Use thread_storage for expensive resources**: Database engines, HTTP clients, and connection pools are perfect candidates for thread storage.

2. **Keep metadata read-only**: The metadata property is read-only for a reason - don't try to modify it.

3. **Subclass for domain logic**: Create custom context classes when you have domain-specific resources or logic that multiple tasks share.

4. **One context parameter per task**: Each task function should have exactly one context parameter, typed as `Context` or a subclass.

5. **Initialize resources lazily**: Check if resources exist in `thread_storage` before creating them to avoid unnecessary initialization.
