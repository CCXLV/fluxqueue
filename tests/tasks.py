from tests._core import fastqueue, async_fastqueue


@fastqueue.task(name="name-print")
def print_name(name: str, *, email: str):
    print(f"Name: {name}")
    print("Your Email: ", email)


@async_fastqueue.task()
async def send_hello(name: str, *, email: str):
    print("Hello from async ", name)
    print("Your Email: ", email)
