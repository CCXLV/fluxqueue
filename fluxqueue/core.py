import inspect
from collections.abc import Callable
from functools import wraps
from typing import Any, ParamSpec, TypeAlias, cast

from .fluxqueue_core import FluxQueueCore
from .utils import get_task_name

P = ParamSpec("P")
TaskDecorator: TypeAlias = Callable[[Callable[P, Any]], Callable[P, Any]]


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
    ) -> TaskDecorator[P]:
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

        def decorator(func: Callable[P, Any]) -> Callable[P, Any]:
            is_async = inspect.iscoroutinefunction(func)
            task_name = get_task_name(func, name)

            cast(Any, func).task_name = task_name
            cast(Any, func).queue = queue

            if is_async:

                @wraps(func)
                async def async_wrapper(
                    *args: P.args, **kwargs: P.kwargs
                ) -> None:
                    await self._core._enqueue_async(
                        task_name, queue, max_retries, args, kwargs
                    )
                    return None

                return async_wrapper
            else:

                @wraps(func)
                def sync_wrapper(*args: P.args, **kwargs: P.kwargs) -> None:
                    self._core._enqueue(
                        task_name, queue, max_retries, args, kwargs
                    )
                    return None

                return sync_wrapper

        return decorator
