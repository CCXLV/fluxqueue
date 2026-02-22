import importlib
import inspect
import sys
from pathlib import Path


def get_registry(module_path: str, queue: str, module_dir: str | None = None):
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
            registry["tasks"][task_name] = original_func
        elif inspect.isclass(obj):
            context_name = getattr(obj, "__fluxqueue_context__", None)
            if not context_name:
                continue

            if registry["contexts"].get(context_name):
                raise ValueError(f"Context '{context_name}' is duplicated")

            registry["contexts"][context_name] = obj

    return registry
