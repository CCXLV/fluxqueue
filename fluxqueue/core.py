import inspect
from collections.abc import Callable, Coroutine
from functools import wraps
from typing import Any, ParamSpec, TypeVar, cast

from .fluxqueue_core import FluxQueueCore
from .utils import get_task_name

P = ParamSpec("P")
R = TypeVar("R")


class FluxQueue:
    def __init__(self, redis_url: str | None = "redis://127.0.0.1:6379"):
        self._core = FluxQueueCore(redis_url=redis_url)

    def task(
        self, *, name: str | None = None, queue: str = "default", max_retries: int = 3
    ) -> Callable[[Callable[P, R]], Callable[P, None]]:
        """
        Decorator for wrapping a function to be enqueued in the fluxqueue.
        """

        def decorator(func: Callable[P, R]) -> Callable[P, None]:
            task_name = get_task_name(func, name)
            cast(Any, func).task_name = task_name
            cast(Any, func).queue = queue

            if inspect.iscoroutinefunction(func):
                raise TypeError(
                    "fluxqueue.task can only decorate sync functions, use Asyncfluxqueue instead"
                )

            @wraps(func)
            def sync_wrapper(*args: P.args, **kwargs: P.kwargs) -> None:
                self._core._enqueue(task_name, queue, max_retries, args, kwargs)
                return None

            return sync_wrapper

        return decorator


class AsyncFluxQueue:
    def __init__(self, redis_url: str | None = "redis://127.0.0.1:6379"):
        self._core = FluxQueueCore(redis_url=redis_url)

    def task(
        self, *, name: str | None = None, queue: str = "default", max_retries: int = 3
    ) -> Callable[
        [Callable[P, Coroutine[Any, Any, R]]],
        Callable[P, Coroutine[Any, Any, None]],
    ]:
        """
        Decorator for wrapping a function to be enqueued in the fluxqueue.
        """

        def decorator(
            func: Callable[P, Coroutine[Any, Any, R]],
        ) -> Callable[P, Coroutine[Any, Any, None]]:
            task_name = get_task_name(func, name)
            cast(Any, func).task_name = task_name
            cast(Any, func).queue = queue

            if not inspect.iscoroutinefunction(func):
                raise TypeError(
                    "Asyncfluxqueue.task can only decorate async (coroutine) functions"
                )

            @wraps(func)
            async def wrapper(*args: P.args, **kwargs: P.kwargs) -> None:
                await self._core._enqueue_async(
                    task_name, queue, max_retries, args, kwargs
                )
                return None

            return wrapper

        return decorator
