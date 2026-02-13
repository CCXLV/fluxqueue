# Client Library

In this section, we are going to talk about the client library. For installation, please visit the [Installation Tutorial](../tutorial/installation.md).

## FluxQueue class

When you import the `FluxQueue` class from the library and initialize it:

```py
from fluxqueue import FluxQueue

fluxqueue = FluxQueue(redis_url="redis://localhost:6379")
```

You might think that this establishes a TCP connection to the Redis server, but it does not. Internally, this only saves the `redis_url` you pass to the class, which is then used by the `enqueue` methods on the Rust side of the library.

## `task` decorator

This decorator is used to mark a function as a task function, which lets you enqueue the task by just running that function:

```py
@fluxqueue.task()
def send_email_task(email: str):
    send_email(email)

send_email_task()
```

When the function is executed, it internally calls a Rust method that establishes a Redis connection and pushes the task into the queue. This entire process is handled on the Rust side of the library.

On the Python side, the only responsibility is dispatching based on the function type. If the function is asynchronous, the async Redis connection method is used, and the resulting Rust Future is converted into a Python awaitable using `pyo3_async_runtimes::tokio::future_into_py`, allowing it to be awaited in Python. If the function is synchronous, a standard (blocking) Redis connection is used instead.
