import importlib
import inspect


def list_functions(module_path: str):
    module = importlib.import_module(module_path)
    funcs = {}
    for _name, obj in inspect.getmembers(module):
        if inspect.isfunction(obj) or (
            inspect.isbuiltin(obj) and obj.task_name
        ):
            funcs[obj.task_name] = obj
    return funcs
