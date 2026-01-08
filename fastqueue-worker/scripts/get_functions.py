import importlib
import inspect


def list_functions(module_path: str):
    module = importlib.import_module(module_path)
    return [
        name
        for name, obj in inspect.getmembers(module)
        if inspect.isfunction(obj) or inspect.isbuiltin(obj)
    ]
