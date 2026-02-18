from dataclasses import dataclass
from typing import Annotated

import pytest
from fluxqueue import FluxQueue
from redis import Redis


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
