from fastapi import FastAPI

from tests.tasks import print_name, send_hello

fastapp = FastAPI()

@fastapp.get("/{name}")
async def _index(name: str):
    await send_hello(name, email="test@test.com")
    return {"message": "Printed name"}, 200
