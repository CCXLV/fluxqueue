import os

from dotenv import load_dotenv
from redis import Redis

from fluxqueue import FluxQueue

load_dotenv()

REDIS_PORT = (os.getenv("REDIS_PORT")) or 6379

fluxqueue = FluxQueue(redis_url=f"redis://localhost:{REDIS_PORT}")
redis_client = Redis(port=int(REDIS_PORT))
