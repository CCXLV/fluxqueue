import importlib
import inspect


def list_functions(module_path: str, queue: str):
    module = importlib.import_module(module_path)
    funcs = {}
    for _name, obj in inspect.getmembers(module):
        task_name = getattr(obj, "task_name", None)
        task_queue = getattr(obj, "queue", None)
        if not task_queue or task_queue != queue:
            continue

        if inspect.isfunction(obj) or (inspect.isbuiltin(obj) and task_name):
            # Get the original function from the wrapper
            # The @wraps decorator stores the original function in __wrapped__
            original_func = getattr(obj, "__wrapped__", obj)
            funcs[task_name] = original_func
    return funcs
