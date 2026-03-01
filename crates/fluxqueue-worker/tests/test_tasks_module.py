from .main import fluxqueue


@fluxqueue.task()
def task_1():
    pass


@fluxqueue.task()
def task_2(x: int, y: int):
    print(x + y)


@fluxqueue.task()
async def async_task(x: int, y: int):
    print(x + y)


@fluxqueue.task(queue="high-priority")
def high_priority_task():
    pass


def regular_function():
    pass
