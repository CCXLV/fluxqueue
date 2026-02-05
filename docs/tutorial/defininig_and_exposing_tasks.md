# Defining and Exposing Tasks

The worker is a standalone Rust binary that needs to discover your task functions at runtime. Since it runs separately from your Python application, it imports your tasks module and inspects it to find functions decorated with `@fluxqueue.task()`. Properly organizing and exposing your tasks helps the worker discover them quickly, which is crucial for performance. This guide shows you the recommended way to structure your tasks.

## Recommended Structure

Use a dedicated module for declaring task functions, then expose them through `__init__.py` with `__all__` to avoid linter errors.

Here's the recommended structure:

```
myapp/
  tasks/
    __init__.py
    client.py
    email.py
    notifications.py
    processing.py
```

## Step-by-Step Guide

### 1. Create a Client Module

Create `tasks/client.py` to define your FluxQueue instance once:

```python
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()
```

### 2. Create Task Files

Create dedicated files for your tasks. For example, `tasks/email.py`:

```python
from .client import fluxqueue

@fluxqueue.task(name="send-welcome-email")
def send_welcome_email(user_id: int, email: str):
    # Send welcome email logic
    print(f"Sending welcome email to {email}")

@fluxqueue.task(name="send-password-reset", queue="high-priority")
def send_password_reset(user_id: int):
    # Password reset logic
    print(f"Sending password reset to user {user_id}")
```

### 3. Expose Tasks in `__init__.py`

Create `tasks/__init__.py` to import and expose all task functions:

```python
from .email import send_welcome_email, send_password_reset
from .notifications import send_notification
from .processing import process_data

__all__ = [
    "send_welcome_email",
    "send_password_reset",
    "send_notification",
    "process_data",
]
```

This pattern:

- Keeps your tasks organized in separate files
- Makes it clear which functions are tasks
- Avoids linter errors by explicitly listing exported functions
- Makes it easy to see all available tasks in one place

### 4. Run the Worker

When starting the worker, point it to your tasks module:

```bash
fluxqueue start --tasks-module-path myapp.tasks --queue default
```

The worker will import `myapp.tasks` and discover all task functions that are exposed in the module.

## How It Works

The worker uses Python's `inspect` module to find functions in your module that have:

- A `task_name` attribute (set by the `@fluxqueue.task()` decorator)
- A `queue` attribute matching the queue you specified (defaults to `default`)

Functions listed in `__all__` are explicitly exported, which helps the worker discover them and keeps your linter happy.

## Complete Example

Here's a complete example with multiple task files:

**`tasks/client.py`**:

```python
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()
```

**`tasks/email.py`**:

```python
from .client import fluxqueue

@fluxqueue.task()
def send_email(to: str, subject: str, body: str):
    print(f"Email to {to}: {subject}")

@fluxqueue.task(queue="urgent")
def send_urgent_email(to: str, message: str):
    print(f"URGENT to {to}: {message}")
```

**`tasks/notifications.py`**:

```python
from .client import fluxqueue

@fluxqueue.task()
async def send_notification(user_id: int, message: str):
    print(f"Notification to user {user_id}: {message}")
```

**`tasks/__init__.py`**:

```python
from .email import send_email, send_urgent_email
from .notifications import send_notification

__all__ = [
    "send_email",
    "send_urgent_email",
    "send_notification",
]
```

**Run workers for different queues**:

In your first terminal:

```bash
fluxqueue start --tasks-module-path myapp.tasks --queue default
```

In a second terminal:

```bash
fluxqueue start --tasks-module-path myapp.tasks --queue urgent
```

## Alternative: Single File

If you prefer a simpler setup, you can also use a single file:

**`tasks.py`**:

```python
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()

@fluxqueue.task()
def task_one(data: str):
    pass

@fluxqueue.task()
def task_two(data: int):
    pass
```

Then run:

```bash
fluxqueue start --tasks-module-path tasks --queue default
```

<!-- prettier-ignore-start -->
!!! tip
    Use a dedicated module for your tasks and expose them through `__init__.py` with `__all__`. This pattern improves worker startup performance by reducing module inspection overhead and keeps your codebase organized.
<!-- prettier-ignore-end -->
