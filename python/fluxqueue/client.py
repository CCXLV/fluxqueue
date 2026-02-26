from collections.abc import Callable, Coroutine
from typing import Any, Concatenate, ParamSpec, cast, overload

from ._core import FluxQueueCore
from ._task import _task_decorator
from .context import C, _with_context

P = ParamSpec("P")


class FluxQueue:
    """
    High-level client for enqueueing Python callables as background tasks.

    It uses the Rust-backed core to push tasks into Redis and is intended to be
    the main entry point used from your application code.

    In most cases you create a single instance per application or service and
    reuse it. The `redis_url` parameter controls which Redis instance is used.
    """

    def __init__(self, redis_url: str | None = "redis://127.0.0.1:6379"):
        self._core = FluxQueueCore(redis_url=redis_url)

    def task(
        self,
        *,
        name: str | None = None,
        queue: str = "default",
        max_retries: int = 3,
    ):
        """
        Mark a function as a FluxQueue task.

        This returns a decorator. When you apply it to a function, calling that
        function will enqueue a task in Redis instead of running the function
        immediately. The actual work is done later by the worker.

        Parameters
        ----------
        `name`:
            Optional explicit task name. If not set, a name is derived from the
            function name.

        `queue`:
            Name of the queue to push tasks to. Defaults to `"default"`.

        `max_retries`:
            Maximum number of retries the worker will attempt for this task
            before treating it as dead.
        """

        @overload
        def decorator(func: Callable[P, None]) -> Callable[P, None]: ...

        @overload
        def decorator(
            func: Callable[P, Coroutine[Any, Any, None]],
        ) -> Callable[P, Coroutine[Any, Any, None]]: ...

        def decorator(
            func: Callable[P, None | Coroutine[Any, Any, None]],
        ) -> Callable[P, None | Coroutine[Any, Any, None]]:
            return _task_decorator(
                cast(Any, func),
                name=name,
                queue=queue,
                max_retries=max_retries,
                core=self._core,
            )

        return decorator

    def task_with_context(
        self,
        *,
        name: str | None = None,
        queue: str = "default",
        max_retries: int = 3,
    ):
        @overload
        def decorator(func: Callable[Concatenate[C, P], None]) -> Callable[P, None]: ...

        @overload
        def decorator(
            func: Callable[Concatenate[C, P], Coroutine[Any, Any, None]],
        ) -> Callable[P, Coroutine[Any, Any, None]]: ...

        def decorator(
            func: Callable[Concatenate[C, P], None | Coroutine[Any, Any, None]],
        ) -> Callable[P, None | Coroutine[Any, Any, None]]:
            return _with_context(
                cast(Any, func),
                name=name,
                queue=queue,
                max_retries=max_retries,
                core=self._core,
            )

        return decorator
