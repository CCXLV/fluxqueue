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
    High-performance task queue backed by Rust.
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
        Decorator for wrapping a function to be enqueued in the fluxqueue.
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
