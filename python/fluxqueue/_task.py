import inspect
from collections.abc import Callable, Coroutine
from functools import wraps
from typing import Any, ParamSpec, cast, get_type_hints, overload

from ._core import FluxQueueCore
from .utils import get_task_name

P = ParamSpec("P")


@overload
def _task_decorator(
    func: Callable[P, None],
    *,
    name: str | None,
    queue: str,
    max_retries: int,
    core: FluxQueueCore,
) -> Callable[P, None]: ...


@overload
def _task_decorator(
    func: Callable[P, Coroutine[Any, Any, None]],
    *,
    name: str | None,
    queue: str,
    max_retries: int,
    core: FluxQueueCore,
) -> Callable[P, Coroutine[Any, Any, None]]: ...


def _task_decorator(
    func: Callable[P, None | Coroutine[Any, Any, None]],
    *,
    name: str | None,
    queue: str,
    max_retries: int,
    core: FluxQueueCore,
) -> Callable[P, None | Coroutine[Any, Any, None]]:
    type_hints = get_type_hints(func)
    return_type = type_hints.get("return")

    if return_type and return_type is not type(None):
        raise TypeError(f"Task function must return None, got {return_type}")

    is_async = inspect.iscoroutinefunction(func)
    task_name = get_task_name(func, name)

    cast(Any, func).task_name = task_name
    cast(Any, func).queue = queue

    if is_async:

        @wraps(func)
        async def async_wrapper(*args: P.args, **kwargs: P.kwargs) -> None:
            await core._enqueue_async(task_name, queue, max_retries, args, kwargs)
            return None

        return async_wrapper
    else:

        @wraps(func)
        def sync_wrapper(*args: P.args, **kwargs: P.kwargs) -> None:
            core._enqueue(task_name, queue, max_retries, args, kwargs)
            return None

        return sync_wrapper
