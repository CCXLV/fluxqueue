import pytest
from fluxqueue import FluxQueue

from .main import fluxqueue

invalid_fluxqueue = FluxQueue(redis_url="redis://localhost:123")


@invalid_fluxqueue.task()
def invalid_task():
    print("Invalid Task")


@fluxqueue.task()
def say_hello(name: str):
    print("Hello ", name)


def test_redis_connection():
    with pytest.raises(RuntimeError):
        invalid_task()

    assert say_hello("George") == None


@pytest.mark.asyncio
async def test_async_task():
    @fluxqueue.task()
    async def async_hello(name: str):
        print("Async Hello ", name)

    async_hello_result = await async_hello("Async George")
    assert async_hello_result == None
