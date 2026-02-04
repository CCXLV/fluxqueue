from typing import Any, TypeVar

T = TypeVar("T")

class FluxQueueCore:
    """
    High-performance task queue backed by Rust.
    """

    def __init__(self, redis_url: str | None) -> None:
        """
        Initialize a new task queue.
        """
        ...

    def _enqueue(
        self,
        name: str,
        queue_name: str,
        max_retries: int,
        *args: Any,
        **kwargs: Any
    ) -> None:
        """
        Enqueue a function for background execution.
        """
        ...

    async def _enqueue_async(
        self,
        name: str,
        queue_name: str,
        max_retries: int,
        *args: Any,
        **kwargs: Any
    ) -> None:
        """
        Enqueue a function for background execution.
        """
        ...
