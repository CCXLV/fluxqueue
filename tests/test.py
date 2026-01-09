from flask import Flask

from tests.tasks import print_name

app = Flask(__name__)


@app.route("/<name>")
def index(name: str):
    print_name(name)
    return {"message": "Printed name"}, 200


if __name__ == "__main__":
    app.run(debug=True)
