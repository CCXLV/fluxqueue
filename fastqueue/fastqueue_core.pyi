from collections.abc import Callable
from typing import TypeVar

T = TypeVar("T")

class FastQueueCore:
    """
    High-performance task queue backed by Rust.
    """

    def __init__(self, workers: int = 10) -> None:
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

    def _shutdown(self) -> None: ...
    async def _async_shutdown(self) -> None: ...
