# FastAPI Integration Example

This example shows how to integrate FluxQueue with a FastAPI application to enqueue background work from your HTTP handlers.

## Prerequisites

- **FastAPI** installed
- A running **Redis** instance
- FluxQueue installed with the worker

## Project Structure

For a minimal FastAPI example, you can use:

```bash
my_app/
  __init__.py
  main.py
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
async def send_welcome_email(email: str):
    # Replace this with your real email sending logic
    print(f"Sending welcome email to {email}")
```

## Expose the Task

In order to expose the task we modify `my_app/tasks/__init__.py` like this:

```py
__all__ = ["send_welcome_email"]

from .tasks import send_welcome_email
```

## Create the FastAPI App

In `main.py`, wire FluxQueue into your FastAPI routes:

```py
from fastapi import FastAPI

from .tasks import fluxqueue, send_welcome_email

app = FastAPI()


@app.post("/signup")
async def signup(email: str):
    await send_welcome_email(email=email)

    return {"status": "ok"}
```

## Run the Worker

Start the FluxQueue worker in a separate process so it can execute tasks:

```bash
fluxqueue start --tasks-module-path my_app.tasks --queue default
```

## Run the FastAPI App

Run the FastAPI server with Uvicorn:

```bash
uvicorn my_app.main:app --reload
```

Now when you `POST` to `/signup`, the `send_welcome_email` task will be enqueued and processed by the worker.
