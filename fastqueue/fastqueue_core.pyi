from collections.abc import Callable
from typing import TypeVar

T = TypeVar("T")

class TaskQueue:
    """
    High-performance task queue backed by Rust.
    """

    def __init__(self, max_workers: int = 10) -> None:
        """
        Initialize a new task queue.
        """
        ...

    def enqueue(
        self,
        func: Callable[..., T],
    ) -> None:
        """
        Enqueue a function for background execution.
        """
        ...

    def wait(self) -> None:
        """
        Wait for all queued tasks to complete.

        This method blocks until all tasks currently in the queue
        have finished executing. It's typically called automatically
        via atexit when the program terminates.
        """
        ...
