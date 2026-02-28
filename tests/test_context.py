import pytest
from fluxqueue import Context

from .conftest import TestEnvFixture


def test_sync_task_with_context(test_env: TestEnvFixture):
    @test_env.fluxqueue.task_with_context()
    def task(ctx: Context, name: str):
        print(ctx.metadata)
        print("Hello ", name)

    result = task("George")
    redis_result = test_env.redis_client.lrange("fluxqueue:queue:default", 0, -1)

    assert result is None
    assert b"George" in redis_result[0]  # type: ignore

    test_env.redis_client.flushdb()


@pytest.mark.asyncio
async def test_async_task_with_context(test_env: TestEnvFixture):
    @test_env.fluxqueue.task_with_context()
    async def task(ctx: Context, name: str):
        print(ctx.metadata)
        print("Async Hello ", name)

    result = await task("Async George")
    redis_result = test_env.redis_client.lrange("fluxqueue:queue:default", 0, -1)

    assert result is None
    assert b"Async George" in redis_result[0]  # type: ignore

    test_env.redis_client.flushdb()


def test_task_with_context_but_without_argument(test_env: TestEnvFixture):
    with pytest.raises(TypeError):

        @test_env.fluxqueue.task_with_context()  # type: ignore
        def task(name: str):
            print("Hello ", name)

    with pytest.raises(TypeError):

        @test_env.fluxqueue.task_with_context()  # type: ignore
        async def async_task(name: str):
            print("Async Hello ", name)
