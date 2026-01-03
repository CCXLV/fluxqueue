from collections.abc import Callable
from typing import TypeVar

T = TypeVar("T")

class FastQueueCore:
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

    def shutdown(self) -> None:
        """
        Wait for all queued tasks to complete and shut down the worker.

        This method blocks until all tasks have finished executing, then
        permanently shuts down the worker. The queue cannot be used after
        calling this method.

        Note: This is primarily useful for testing purposes. In production,
        the queue should remain active to handle tasks asynchronously.
        """
        ...
