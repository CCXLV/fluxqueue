from collections.abc import Callable
from typing import TypeVar

T = TypeVar("T")

class FastQueueCore:
    """
    High-performance task queue backed by Rust.
    """

    def __init__(self, redis_url: str | None) -> None:
        """
        Initialize a new task queue.
        """
        ...

    def register_task(self, name: str, func: Callable[..., T]) -> None:
        """
        Register a new task at startup
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
        Sync shutdown
        """
        ...

    async def async_shutdown(self) -> None:
        """
        Async shutdown
        """
        ...
