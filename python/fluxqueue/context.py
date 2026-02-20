import inspect
from collections.abc import Callable, Coroutine
from typing import Any, Concatenate, ParamSpec, TypeVar, cast, get_type_hints, overload

from ._core import FluxQueueCore
from ._task import _task_decorator

P = ParamSpec("P")


class Context:
    __fluxqueue_context__: str | None = None

    def __init_subclass__(cls) -> None:
        if not cls.__fluxqueue_context__:
            cls.__fluxqueue_context__ = cls.__name__
            # raise NotImplementedError(
            #     f"{cls.__name__} is not implemented properly, make sure to add '__fluxqueue_context__' attribute."
            # )


C = TypeVar("C", bound=Context)


@overload
def _with_context(
    func: Callable[Concatenate[C, P], None],
    *,
    name: str | None,
    queue: str,
    max_retries: int,
    core: FluxQueueCore,
) -> Callable[P, None]: ...


@overload
def _with_context(
    func: Callable[Concatenate[C, P], Coroutine[Any, Any, None]],
    *,
    name: str | None,
    queue: str,
    max_retries: int,
    core: FluxQueueCore,
) -> Callable[P, Coroutine[Any, Any, None]]: ...


def _with_context(
    func: Callable[Concatenate[C, P], None | Coroutine[Any, Any, None]],
    *,
    name: str | None,
    queue: str,
    max_retries: int,
    core: FluxQueueCore,
) -> Callable[P, None | Coroutine[Any, Any, None]]:
    sig = inspect.signature(func)
    hints = get_type_hints(func)

    all_param_names = list(sig.parameters.keys())

    context_params = {
        name: hints[name]
        for name in all_param_names
        if name in hints
        and isinstance(hints[name], type)
        and issubclass(hints[name], Context)
    }

    if len(context_params) != 1:
        raise TypeError(
            f"Expected exactly one context parameter, found {len(context_params)}: {list(context_params.keys())}"
        )

    non_context_params = [
        name for name in all_param_names if name not in context_params
    ]

    new_sig = sig.replace(parameters=[sig.parameters[n] for n in non_context_params])

    if inspect.iscoroutinefunction(func):
        wrapper = _task_decorator(
            cast(Any, func), name=name, queue=queue, max_retries=max_retries, core=core
        )
    else:
        wrapper = _task_decorator(
            cast(Any, func), name=name, queue=queue, max_retries=max_retries, core=core
        )

    cast(Any, wrapper).__signature__ = new_sig
    return wrapper
