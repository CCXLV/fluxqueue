import inspect
from collections.abc import Callable, Coroutine
from contextvars import ContextVar
from typing import Any, Concatenate, ParamSpec, TypeVar, cast, get_type_hints, overload

from ._core import FluxQueueCore
from ._task import _task_decorator

P = ParamSpec("P")


class Context:
    __fluxqueue_context__: str | None = None

    def __init__(self) -> None:
        self._thread_storage: ContextVar[dict[str, Any]] = ContextVar(
            "thread_storage", default=None
        )

    @property
    def thread_storage(self) -> dict[str, Any]:
        """
        Context-scoped storage dictionary.

        Returns a mutable dictionary associated with the current
        execution context. The storage is isolated per thread or
        async task using ContextVar, ensuring safe concurrent usage.

        The dictionary is initialized lazily on first access.
        """
        storage = self._thread_storage.get(None)

        if storage is None:
            storage = {}
            self._thread_storage.set(storage)

        return storage

    def __init_subclass__(cls) -> None:
        if not cls.__fluxqueue_context__:
            cls.__fluxqueue_context__ = cls.__name__


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

    wrapper = _task_decorator(
        cast(Any, func), name=name, queue=queue, max_retries=max_retries, core=core
    )

    cast(Any, wrapper).__signature__ = new_sig
    return wrapper
