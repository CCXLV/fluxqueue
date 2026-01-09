import importlib
import inspect
import types


def get_functions(module_path: str):
    """
    Given a module path like "os.path" or "my_package.module",
    return a dictionary of function names and function objects.
    """
    # Import the module dynamically
    module = importlib.import_module(module_path)

    functions = {}

    # Iterate through attributes of the module
    for name, obj in inspect.getmembers(module):
        if inspect.isfunction(obj) or inspect.isbuiltin(obj):
            # Only functions
            functions[name] = obj

    return functions


if __name__ == "__main__":
    funcs = get_functions("tests")
    print(list(funcs.keys()))
