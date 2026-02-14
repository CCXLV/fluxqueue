import pytest

from .conftest import TestEnvFixture


def test_sync_task(test_env: TestEnvFixture):
    @test_env.fluxqueue.task()
    def say_hello(name: str):
        print("Hello ", name)

    result = say_hello("George")
    redis_result = test_env.redis_client.lrange("fluxqueue:queue:default", 0, -1)

    assert result is None
    assert b"George" in redis_result[0]  # type: ignore

    test_env.redis_client.flushdb()


@pytest.mark.asyncio
async def test_async_task(test_env: TestEnvFixture):
    @test_env.fluxqueue.task()
    async def async_hello(name: str):
        print("Async Hello ", name)

    result = await async_hello("Async George")
    redis_result = test_env.redis_client.lrange("fluxqueue:queue:default", 0, -1)

    assert result is None
    assert b"Async George" in redis_result[0]  # type: ignore

    test_env.redis_client.flushdb()
