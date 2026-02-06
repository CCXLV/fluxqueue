import pytest

from .main import fluxqueue, redis_client


def test_sync_task():
    @fluxqueue.task()
    def say_hello(name: str):
        print("Hello ", name)

    result = say_hello("George")
    redis_result = redis_client.lrange("fluxqueue:queue:default", 0, -1)
    redis_string = redis_result[0].decode("utf-8", errors="ignore")  # type: ignore

    assert result is None
    assert "George" in redis_string

    redis_client.flushdb()


@pytest.mark.asyncio
async def test_async_task():
    @fluxqueue.task()
    async def async_hello(name: str):
        print("Async Hello ", name)

    result = await async_hello("Async George")
    redis_result = redis_client.lrange("fluxqueue:queue:default", 0, -1)
    redis_string = redis_result[0].decode("utf-8", errors="ignore")  # type: ignore

    assert result is None
    assert "Async George" in redis_string

    redis_client.flushdb()
