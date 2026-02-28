import importlib
import inspect
import sys
from pathlib import Path
from typing import get_type_hints

from fluxqueue import Context


def get_registry(  # noqa: C901
    module_path: str, queue: str, module_dir: str | None = None
):
    if module_dir:
        module_dir_path = Path(module_dir).resolve()
        if str(module_dir_path) not in sys.path:
            sys.path.insert(0, str(module_dir_path))

    module = importlib.import_module(module_path)
    registry = {"tasks": {}, "contexts": {}}
    for _name, obj in inspect.getmembers(module):
        if inspect.isfunction(obj):
            task_name = getattr(obj, "task_name", None)
            task_queue = getattr(obj, "queue", None)
            if not task_queue or task_queue != queue:
                continue

            if registry["tasks"].get(task_name):
                raise ValueError(f"Task '{task_name}' is duplicated")

            original_func = getattr(obj, "__wrapped__", obj)

            hints = get_type_hints(original_func)
            sig = inspect.signature(original_func)
            context_params = {
                name: hints[name]
                for name in sig.parameters
                if name in hints
                and isinstance(hints[name], type)
                and issubclass(hints[name], Context)
            }
            if not context_params:
                context_name = None
            else:
                context = context_params[next(iter(context_params))]
                context_name = getattr(context, "__fluxqueue_context__", None)

            registry["tasks"][task_name] = {
                "func": original_func,
                "context_name": context_name,
            }
        elif inspect.isclass(obj):
            if not issubclass(obj, Context):
                continue

            context_name = getattr(obj, "__fluxqueue_context__", None)
            if registry["contexts"].get(context_name):
                raise ValueError(f"Context '{context_name}' is duplicated")

            registry["contexts"][context_name] = obj

    return registry
