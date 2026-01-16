from flask import Flask
from fastapi import FastAPI

from tests.tasks import print_name, send_hello

# app = Flask(__name__)
fastapp = FastAPI()

# @app.route("/<name>")
# async def index(name: str):
#     await send_hello(name)
#     return {"message": "Printed name"}, 200


@fastapp.get("/{name}")
async def _index(name: str):
    await send_hello(name, email="test@test.com")
    return {"message": "Printed name"}, 200


# if __name__ == "__main__":
#     app.run(debug=True)
