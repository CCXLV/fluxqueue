from collections.abc import Callable


def get_task_name(func: Callable, name: str | None = None) -> str:
    task_name = name if name else func.__name__
    return task_name.replace("_", "-")
