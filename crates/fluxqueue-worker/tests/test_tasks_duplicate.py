def task1():
    """First task with name task-1."""
    pass


task1.task_name = "task-1"  # type: ignore
task1.queue = "default"  # type: ignore


def task2():
    """Second task with same name task-1 (duplicate)."""
    pass


task2.task_name = "task-1"  # type: ignore
task2.queue = "default"  # type: ignore
