from collections.abc import Coroutine
from typing import ParamSpec, TypeVar

T = TypeVar("T")
P = ParamSpec("P")

class FastQueueCore:
    """
    High-performance task queue backed by Rust.
    """

    def __init__(self, redis_url: str | None) -> None:
        """
        Initialize a new task queue.
        """
        ...

    def _enqueue(self, name: str, *args: P.args, **kwargs: P.kwargs) -> None:
        """
        Enqueue a function for background execution.
        """
        ...

    async def _enqueue_async(
        self, name: str, *args: P.args, **kwargs: P.kwargs
    ) -> Coroutine[None]:
        """
        Enqueue a function for background execution.
        """
        ...
