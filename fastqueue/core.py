import inspect
from collections.abc import Callable, Coroutine
from functools import wraps
from typing import ParamSpec, TypeVar

from .fastqueue_core import FastQueueCore

P = ParamSpec("P")
R = TypeVar("R")


class FastQueue:
    def __init__(self, redis_url: str | None = "redis://127.0.0.1:6379"):
        self._core = FastQueueCore(redis_url=redis_url)

    def task(
        self, *, name: str | None = None
    ) -> Callable[[Callable[P, R]], Callable[P, None]]:
        """
        Decorator for wrapping a function to be enqueued in the fastqueue.
        """

        def decorator(func: Callable[P, R]) -> Callable[P, None]:
            task_name = name if name else func.__name__

            if inspect.iscoroutinefunction(func):
                raise TypeError(
                    "FastQueue.task can only decorate sync functions, use AsyncFastQueue instead"
                )

            @wraps(func)
            def sync_wrapper(*args: P.args, **kwargs: P.kwargs) -> None:
                self._core._enqueue(task_name, args, kwargs)
                return None

            return sync_wrapper

        return decorator


class AsyncFastQueue:
    def __init__(self, redis_url: str | None = "redis://127.0.0.1:6379"):
        self._core = FastQueueCore(redis_url=redis_url)

    async def task(
        self, *, name: str | None = None
    ) -> Coroutine[Callable[[Callable[P, R]], Callable[P, None]]]:
        """
        Decorator for wrapping a function to be enqueued in the fastqueue.
        """

        async def decorator(func: Callable[P, R]) -> Coroutine[Callable[P, R]]:
            task_name = name if name else func.__name__

            if not inspect.iscoroutinefunction(func):
                raise TypeError(
                    "AsyncFastQueue.task can only decorate async (coroutine) functions"
                )

            @wraps(func)
            async def wrapper(
                *args: P.args, **kwargs: P.kwargs
            ) -> Coroutine[None]:
                await self._core._enqueue_async(task_name, args, kwargs)
                return None

            return wrapper

        return decorator
