from __future__ import annotations

import inspect
from collections.abc import Callable, Coroutine
from datetime import timedelta
from functools import wraps
from typing import TYPE_CHECKING, Any, ParamSpec, cast, get_type_hints, overload

from .schedule import cron
from .utils import get_task_name

if TYPE_CHECKING:
    from ._core import FluxQueueCore

P = ParamSpec("P")


def _task_wrapper(
    func: Callable[P, None | Coroutine[Any, Any, None]],
    *,
    task_name: str,
    queue: str,
    max_retries: int,
    core: FluxQueueCore,
) -> Callable[P, None | Coroutine[Any, Any, None]]:
    if inspect.iscoroutinefunction(func):

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

    task_name = get_task_name(func, name)

    cast(Any, func).fluxqueue = True
    cast(Any, func).task_name = task_name
    cast(Any, func).queue = queue

    wrapped_task = _task_wrapper(
        func, task_name=task_name, queue=queue, max_retries=max_retries, core=core
    )

    def defer(delay: timedelta,  *args: P.args, **kwargs: P.kwargs):
        pass

    def _cron(
        expression: str | None = None,
        minute: int | str = "*",
        hour: int | str = "*",
        day_of_month: int | str = "*",
        month: int | str = "*",
        day_of_week: int | str = "*",
        *args: P.args,
        **kwargs: P.kwargs,
    ) -> Callable[P, None | Coroutine[Any, Any, None]]:
        cron(
            expression,
            minute=minute,
            hour=hour,
            day_of_month=day_of_month,
            month=month,
            day_of_week=day_of_week,
        )(func)
        return wrapped_task

    cast(Any, wrapped_task).defer = defer
    cast(Any, wrapped_task).cron = _cron

    return wrapped_task
