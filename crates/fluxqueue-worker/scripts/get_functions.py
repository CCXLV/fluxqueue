import importlib
import inspect
import sys
from pathlib import Path


def list_functions(module_path: str, queue: str, module_dir: str | None = None):
    if module_dir:
        module_dir_path = Path(module_dir).resolve()
        if str(module_dir_path) not in sys.path:
            sys.path.insert(0, str(module_dir_path))

    module = importlib.import_module(module_path)
    funcs = {}
    for _name, obj in inspect.getmembers(module):
        task_name = getattr(obj, "task_name", None)
        task_queue = getattr(obj, "queue", None)
        if not task_queue or task_queue != queue:
            continue

        if inspect.isfunction(obj) or (inspect.isbuiltin(obj) and task_name):
            if funcs.get(task_name):
                raise ValueError(f"Task name '{task_name}' is duplicated")

            original_func = getattr(obj, "__wrapped__", obj)
            funcs[task_name] = original_func
    return funcs
