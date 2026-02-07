# FluxQueue in FastAPI project

This example shows how to use FluxQueue in a FastAPI project. In particular, it demonstrates an endpoint for sending emails asynchronously using `aiosmtplib`.

You might wonder why we are declaring `fluxqueue` object globally.

```py
from fluxqueue import FluxQueue

fluxqueue = FluxQueue()
```

This is safe because initializing the class does not establish a TCP connection to Redis. A connection is only created when a task function is actually executed.
