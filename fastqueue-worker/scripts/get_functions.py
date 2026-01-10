import importlib
import inspect


# TODO: Change the dict type since we also need to get max_retries of the task
# which also causes the get_task_functions in worker.rs to change
def list_functions(module_path: str, queue: str):
    module = importlib.import_module(module_path)
    funcs = {}
    for _name, obj in inspect.getmembers(module):
        task_name = getattr(obj, "task_name", None)
        task_queue = getattr(obj, "queue", None)
        if not task_queue or task_queue != queue:
            continue

        if inspect.isfunction(obj) or (inspect.isbuiltin(obj) and task_name):
            funcs[task_name] = obj
    return funcs
