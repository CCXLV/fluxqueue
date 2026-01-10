from tests._core import fastqueue


@fastqueue.task(name="name-print", queue="test")
def print_name(name: str):
    print(f"Name: {name}")


@fastqueue.task()
def send_hello():
    print("Hello!")
