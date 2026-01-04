import inspect
from collections.abc import Callable
from functools import partial, wraps
from typing import Any, ParamSpec, TypeVar

from .fastqueue_core import FastQueueCore

P = ParamSpec("P")
R = TypeVar("R")


class FastQueue(FastQueueCore):
    def __new__(cls, workers: int = 10):
        return super().__new__(cls, workers)

    def task(self, func: Callable[P, R]) -> Callable[P, None]:
        """
        Decorator for wrapping a function to be enqueued in the fastqueue.
        """
        if inspect.iscoroutinefunction(func):

            @wraps(func)
            async def async_wrapper(*args, **kwargs):
                self.enqueue(partial[Any](func, *args, **kwargs))
                return None

            return async_wrapper
        else:

            @wraps(func)
            def sync_wrapper(*args, **kwargs):
                self.enqueue(partial[Any](func, *args, **kwargs))
                return None  # fire-and-forget

            return sync_wrapper

    def close(self) -> None:
        """
        Gracefully finish all pending tasks and shut down the queue.

        This method is asynchronous and non-blocking. It stops the queue
        from accepting new work and waits for current tasks in the
        buffer to complete before exiting.

        Best used in application shutdown hooks (e.g., FastAPI lifespan)
        to ensure no background tasks are lost during deployment.
        """
        self._shutdown()

    async def aclose(self) -> None:
        """
        Gracefully finish all pending tasks and shut down the queue.

        This method is asynchronous and non-blocking. It stops the queue
        from accepting new work and waits for current tasks in the
        buffer to complete before exiting.

        Best used in application shutdown hooks (e.g., FastAPI lifespan)
        to ensure no background tasks are lost during deployment.
        """
        await self._async_shutdown()
