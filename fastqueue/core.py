import inspect
from collections.abc import Callable
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
            self._core.register_task(task_name, func)

            if inspect.iscoroutinefunction(func):

                @wraps(func)
                async def async_wrapper(
                    *args: P.args, **kwargs: P.kwargs
                ) -> None:
                    self._core.enqueue(task_name, args, kwargs)
                    return None

                return async_wrapper
            else:

                @wraps(func)
                def sync_wrapper(*args: P.args, **kwargs: P.kwargs) -> None:
                    self._core.enqueue(task_name, args, kwargs)
                    return None

                return sync_wrapper

        return decorator

    def close(self) -> None:
        """
        Gracefully finish all pending tasks and shut down the queue.

        This method is synchronous and blocking. It stops the queue
        from accepting new tasks and waits for current tasks in the
        buffer to complete before exiting.

        Best used in application shutdown hooks (that does not suport
        async, otherwise use `aclose` instead) to ensure no background
        tasks are lost during process shutdown.
        """
        self._core.shutdown()

    async def aclose(self) -> None:
        """
        Gracefully finish all pending tasks and shut down the queue.

        This method is asynchronous and non-blocking. It stops the queue
        from accepting new tasks and waits for current tasks in the
        buffer to complete before exiting.

        Best used in application shutdown hooks (e.g., FastAPI lifespan)
        to ensure no background tasks are lost during process shutdown.
        """
        await self._core.async_shutdown()
