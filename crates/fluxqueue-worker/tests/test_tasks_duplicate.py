from .main import fluxqueue


@fluxqueue.task(name="task-1")
def task_1():
    pass


@fluxqueue.task(name="task-1")
def task_2():
    pass
