from fastqueue import FastQueue

fastqueue = FastQueue()


@fastqueue.task()
def test_name(name: str):
    print(f"Name: {name}")


test_name("George")
test_name("John Doe")

fastqueue.close()
