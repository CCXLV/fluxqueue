import os
from collections.abc import Iterator
from dataclasses import dataclass
from typing import Annotated

import pytest
from redis import Redis
from testcontainers.redis import RedisContainer

from fluxqueue import FluxQueue

REDIS_VERSION = os.getenv("REDIS_VERSION") if os.getenv("REDIS_VERSION") else "latest"


@dataclass
class TestEnv:
    fluxqueue: FluxQueue
    redis_client: Redis


TestEnvFixture = Annotated[TestEnv, pytest.fixture]


@pytest.fixture
def test_env():
    redis_client: Redis = Redis()
    redis_client.flushall()

    fluxqueue: FluxQueue = FluxQueue()

    try:
        yield TestEnv(fluxqueue=fluxqueue, redis_client=redis_client)
    finally:
        redis_client.close()
