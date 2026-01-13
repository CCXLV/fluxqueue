import importlib
import inspect


# TODO: get the functions based on the queue name
def list_functions(module_path: str):
    module = importlib.import_module(module_path)
    funcs = {}
    for _name, obj in inspect.getmembers(module):
        if inspect.isfunction(obj) or (
            inspect.isbuiltin(obj) and obj.task_name  # pyright: ignore
        ):
            funcs[obj.task_name] = obj  # pyright: ignore
    return funcs
