import os
from dataclasses import dataclass
from typing import Annotated

import pytest
from redis import Redis

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
    fluxqueue: FluxQueue = FluxQueue()

    try:
        yield TestEnv(fluxqueue=fluxqueue, redis_client=redis_client)
    finally:
        redis_client.close()
