import os

from dotenv import load_dotenv

from fluxqueue import FluxQueue

load_dotenv()

fluxqueue = FluxQueue(redis_url=os.getenv("REDIS_URL"))
