import logging

from fastqueue import FastQueue

logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s"
)

fastqueue = FastQueue(redis_url="redis://127.0.0.1:6380")


@fastqueue.task()
def test_name():
    print(f"Name: Giorgi")


test_name()

fastqueue.close()
