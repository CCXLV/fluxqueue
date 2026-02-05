# Quick Start

Get up and running with FluxQueue in minutes. This guide shows you how to create tasks, enqueue them, and run a worker to process them.

## Create Your First Task

Start by creating a FluxQueue instance and defining a task function:

```python
from fluxqueue import FluxQueue

# Create a FluxQueue instance (defaults to redis://127.0.0.1:6379)
fluxqueue = FluxQueue()

# Define a task using the @fluxqueue.task decorator
@fluxqueue.task()
def send_email(to: str, subject: str, body: str):
    print(f"Sending email to {to}")
    print(f"Subject: {subject}")
    print(f"Body: {body}")
    # Your email sending logic here
```

## Enqueue Tasks

Call the decorated function to enqueue it. The function returns immediately, the actual work happens in the background:

```python
# Enqueue the task
send_email("user@example.com", "Hello", "This is a test email")
```

The task is now in the queue, waiting to be processed by a worker.

## Async Tasks

FluxQueue supports async functions too. Just define an async function and use the same decorator:

```python
@fluxqueue.task()
async def process_data(data: dict):
    # Your async processing logic
    result = await some_async_operation(data)
    return result

# Enqueue it (use await in async contexts)
await process_data({"key": "value"})
```

## Task Options

You can customize tasks with options:

```python
@queue.task(
    name="custom-task-name",  # Custom task name
    queue="high-priority",     # Use a different queue
    max_retries=5              # Retry up to 5 times on failure
)
def important_task(data: str):
    # Critical task logic
    pass
```

## Run the Worker

To process tasks, you need to run a worker. Make sure Redis is running, then start the worker:

```bash
fluxqueue start --tasks-module-path myapp.tasks --queue default
```

Or use the binary directly:

```bash
fluxqueue-worker --tasks-module-path myapp.tasks --queue default
```

The `--tasks-module-path` tells the worker where to find your task functions. It should be the module path where your tasks are defined (e.g., `myapp.tasks` if your tasks are in `myapp/tasks.py`). For more details, see [Defining and Exposing Tasks](defininig_and_exposing_tasks.md).

## Complete Example

Here's a complete example that puts it all together:

**`tasks.py`**:

```python
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()

@fluxqueue.task()
def greet(name: str):
    print(f"Hello, {name}!")

@fluxqueue.task(queue="notifications", max_retries=3)
async def send_notification(user_id: int, message: str):
    # Send notification logic
    print(f"Notifying user {user_id}: {message}")
```

**`main.py`**:

```python
from tasks import greet, send_notification

# Enqueue tasks
greet("Alice")
greet("Bob")

# In an async context
await send_notification(123, "Welcome!")
```

**Run the worker**:

```bash
fluxqueue start --tasks-module-path tasks
```

In another terminal, run `main.py` to enqueue tasks. The worker will pick them up and execute them.
