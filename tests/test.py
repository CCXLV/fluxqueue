import logging

from fastqueue import FastQueue

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

fastqueue = FastQueue()


@fastqueue.task()
def test_name():
    print(f"Name: Giorgi")


test_name()

fastqueue.close()
