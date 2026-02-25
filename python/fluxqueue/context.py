import inspect
import threading
from collections.abc import Callable, Coroutine
from contextvars import ContextVar
from typing import Any, Concatenate, ParamSpec, TypeVar, cast, get_type_hints, overload

from ._core import FluxQueueCore
from ._task import _task_decorator
from .models import TaskMetadata

P = ParamSpec("P")


class Context:
    __fluxqueue_context__: str | None = None

    def __init__(self) -> None:
        self._thread_local = threading.local()
        self._metadata_var: ContextVar[TaskMetadata] = ContextVar(
            "task_metadata", default=None
        )

    @property
    def thread_storage(self) -> dict[str, Any]:
        """
        Retrieves the thread-persistent storage dictionary.

        Returns a dictionary that persists across all tasks executed by the current worker.
        Used for storing long-lived resources like database engines and connection
        pools to avoid re-initialization overhead.
        """
        if not hasattr(self._thread_local, "storage"):
            self._thread_local.storage = {}

        return self._thread_local.storage

    @property
    def metadata(self) -> TaskMetadata:
        """
        Returns metadata isolated to the current task.

        Returns a TaskMetadata instance containing execution details like
        retry counts and task IDs. This property uses ContextVars to ensure
        data isolation during concurrent task execution on the same thread.
        """
        return self._metadata_var.get()

    async def _run_async_task(
        self, func: Callable, metadata: TaskMetadata, args, kwargs
    ):
        """
        This function is for internal use only.
        """
        token = self._metadata_var.set(metadata)
        try:
            await func(*args, **kwargs)
        finally:
            self._metadata_var.reset(token)

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
