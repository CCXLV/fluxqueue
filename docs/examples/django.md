# Django Integration Example

This example shows how to integrate FluxQueue with a Django project to offload background work from your views.

## Prerequisites

- A Django project (created with `django-admin startproject`)
- A running **Redis** instance
- FluxQueue installed with the worker

## Project Structure

For a minimal Django example, you can use:

```bash
myproject/
  manage.py
  myproject/
    __init__.py
    asgi.py
    settings.py
    urls.py
    wsgi.py
  app/
    __init__.py
    views.py
    tasks/
      __init__.py
      tasks.py
```

## Define Tasks

Create `app/tasks/tasks.py` and define your FluxQueue instance and tasks:

```py
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()


@fluxqueue.task()
async def send_welcome_email(email: str):
    # Replace this with your real email sending logic
    print(f"Sending welcome email to {email}")
```

## Expose the Task

In order to expose tasks we modify `myproject/app/tasks/__init__.py` like this:

```py
__all__ = ["send_welcome_email"]

from .tasks import send_welcome_email
```

## Use Tasks in Django Views

In `app/views.py`, enqueue tasks from your views instead of doing work inline:

```python
from django.http import JsonResponse

from .tasks import send_welcome_email


async def signup(request):
    email = request.POST["email"]

    await send_welcome_email(email=email)

    return JsonResponse({"status": "ok"})
```

Make sure your `signup` view is added in `urls.py` as usual.

## Run the Worker

Start the FluxQueue worker in a separate process so it can execute tasks:

```bash
fluxqueue start --tasks-module-path app.tasks --queue default
```

## Run the Django Development Server

Run Django with Uvicorn:

```bash
uvicorn myproject.asgi:application
```

Now when you submit the signup form that posts to `signup`, the `send_welcome_email` task will be enqueued and processed by the worker.
