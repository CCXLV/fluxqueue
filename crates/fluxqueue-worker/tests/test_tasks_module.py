def task1():
    pass


task1.task_name = "task-1"  # type: ignore
task1.queue = "default"  # type: ignore


def task2(x: int, y: int):
    print(x + y)


task2.task_name = "task-2"  # type: ignore
task2.queue = "default"  # type: ignore


async def async_task(x: int, y: int):
    print(x + y)


async_task.task_name = "async-task"  # type: ignore
async_task.queue = "default"  # type: ignore


def high_priority_task():
    pass


high_priority_task.task_name = "high-priority-task"  # type: ignore
high_priority_task.queue = "high-priority"  # type: ignore


def regular_function():
    pass
