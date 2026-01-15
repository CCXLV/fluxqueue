from fastqueue import FastQueue, AsyncFastQueue

REDIS_URL = "redis://127.0.0.1:6380"

fastqueue = FastQueue(redis_url=REDIS_URL)
async_fastqueue = AsyncFastQueue(redis_url=REDIS_URL)
