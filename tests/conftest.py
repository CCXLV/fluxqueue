from collections.abc import Iterator
from dataclasses import dataclass
from typing import Annotated

import pytest
from redis import Redis
from testcontainers.redis import RedisContainer

from fluxqueue import FluxQueue


@dataclass
class TestEnv:
    fluxqueue: FluxQueue
    redis_client: Redis


TestEnvFixture = Annotated[TestEnv, pytest.fixture]


@pytest.fixture(scope="session")
def redis_container() -> Iterator[RedisContainer]:
    with RedisContainer("redis:8.4.0") as container:
        yield container


@pytest.fixture
def test_env(redis_container: RedisContainer):
    host = redis_container.get_container_host_ip()
    port = int(redis_container.get_exposed_port(redis_container.port))

    redis_url: str = f"redis://{host}:{port}"

    redis_client: Redis = Redis(host=host, port=port)
    redis_client.flushall()

    fluxqueue: FluxQueue = FluxQueue(redis_url=redis_url)

    try:
        yield TestEnv(fluxqueue=fluxqueue, redis_client=redis_client)
    finally:
        redis_client.close()
