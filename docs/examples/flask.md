# Flask Integration Example

This example shows how to integrate FluxQueue with a Flask application to offload background work from your HTTP handlers.

## Prerequisites

- **Flask** installed
- A running **Redis** instance
- FluxQueue installed with the worker

## Project Structure

For a minimal Flask example, you can use:

```bash
my_app/
  __init__.py
  app.py
  tasks/
    __init__.py
    tasks.py
```

## Define Tasks

Create `tasks/tasks.py` and define your FluxQueue instance and tasks:

```py
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()


@fluxqueue.task()
def send_welcome_email(email: str) -> None:
    # Replace this with your real email sending logic
    print(f"Sending welcome email to {email}")
```

## Expose the Task

In order to expose the task we modify `my_app/tasks/__init__.py` like this:

```py
__all__ = ["send_welcome_email"]

from .tasks import send_welcome_email
```

## Create the Flask App

In `app.py`, wire FluxQueue into your Flask routes:

```py
from flask import Flask, request, jsonify

from .tasks import send_welcome_email

app = Flask(__name__)


@app.post("/signup")
def signup():
    email = request.form["email"]

    send_welcome_email(email=email)

    return jsonify({"status": "ok"})
```

## Run the Worker

Start the FluxQueue worker in a separate process so it can execute tasks:

```bash
fluxqueue start --tasks-module-path my_app.tasks --queue default
```

## Run the Flask App

Run the Flask development server:

```bash
flask --app my_app.app run --debug
```

Now when you `POST` to `/signup`, the `send_welcome_email` task will be enqueued and processed by the worker.
