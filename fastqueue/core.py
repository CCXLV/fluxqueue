from collections.abc import Callable
from functools import partial, wraps
from typing import Any

from .fastqueue_core import FastQueueCore


class FastQueue(FastQueueCore):
    def __new__(cls, max_workers: int = 10):
        return super().__new__(cls, max_workers)

    def task(self, func: Callable) -> Callable:
        """
        Decorator for wrapping a function to be enqueued in the fastqueue.
        """

        @wraps(func)
        def wrapper(*args, **kwargs):
            self.enqueue(partial[Any](func, *args, **kwargs))
            return None  # fire-and-forget

        return wrapper
