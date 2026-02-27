from collections.abc import Callable, Coroutine
from typing import Any, Concatenate, ParamSpec, cast, overload

from ._core import FluxQueueCore
from ._task import _task_decorator
from .context import C, _with_context

P = ParamSpec("P")


class FluxQueue:
    """
    High-level client for enqueueing Python callables as background tasks.

    It uses the Rust-backed core to push tasks into Redis and is intended to be
    the main entry point used from your application code.

    In most cases you create a single instance per application or service and
    reuse it. The `redis_url` parameter controls which Redis instance is used.
    """

    def __init__(self, redis_url: str | None = "redis://127.0.0.1:6379"):
        self._core = FluxQueueCore(redis_url=redis_url)

    def task(
        self,
        *,
        name: str | None = None,
        queue: str = "default",
        max_retries: int = 3,
    ):
        """
        Mark a function as a FluxQueue task.

        When you apply to a function that function is being marked as a task function.
        Running it will enqueue the task and then the worker will execute it.

        Parameters
        ----------
        `name`:
            Optional explicit task name. If not set, a name is derived from the
            function name.

        `queue`:
            Name of the queue to push tasks to. Defaults to `"default"`.

        `max_retries`:
            Maximum number of retries the worker will attempt for this task
            before treating it as dead.

        Example
        ---

        ```py
        @fluxqueue.task()
        async def send_email_task(name: str, username: str, email: str):
            async with get_email_client() as client:
                await send_email(
                    email_client=client,
                    to_email=email,
                    subject="Welcome to FluxQueue",
                    config=email_config,
                )
        ```

        Enqueueing the task
        ---

        ```py
        await send_email_task(name, username, email)
        ```
        """

        @overload
        def decorator(func: Callable[P, None]) -> Callable[P, None]: ...

        @overload
        def decorator(
            func: Callable[P, Coroutine[Any, Any, None]],
        ) -> Callable[P, Coroutine[Any, Any, None]]: ...

        def decorator(
            func: Callable[P, None | Coroutine[Any, Any, None]],
        ) -> Callable[P, None | Coroutine[Any, Any, None]]:
            return _task_decorator(
                cast(Any, func),
                name=name,
                queue=queue,
                max_retries=max_retries,
                core=self._core,
            )

        return decorator

    def task_with_context(
        self,
        *,
        name: str | None = None,
        queue: str = "default",
        max_retries: int = 3,
    ):
        """
        Mark a function as a FluxQueue task with context.

        This decorator works like the `task` decorator but adds support for the `Context` class.
        The function must accept a context as its first argument, with `Context` (or a subclass of `Context`) as the type hint.
        When decorated, the context argument is automatically injected by the worker and is no longer
        part of the function's public signature - users calling the function do not need to provide it.

        Parameters
        ----------
        `name`:
            Optional explicit task name. If not set, a name is derived from the
            function name.

        `queue`:
            Name of the queue to push tasks to. Defaults to `"default"`.

        `max_retries`:
            Maximum number of retries the worker will attempt for this task
            before treating it as dead.

        Example
        ---

        ```py
        class DbContext(Context):
            def __init__(self) -> None:
                super().__init__()

            def _get_local_session(self) -> async_sessionmaker[AsyncSession]:
                if not self.thread_storage.get("session"):
                    engine = create_async_engine(SQLALCHEMY_DATABASE_URL)
                    self.thread_storage["session"] = async_sessionmaker(
                        bind=engine, expire_on_commit=False
                    )

                return self.thread_storage["session"]

            @asynccontextmanager
            async def session_context(self):
                local_session = self._get_local_session()
                async with local_session() as session:
                    try:
                        yield session
                        await session.commit()
                    except Exception:
                        await session.rollback()
                        raise

        @fluxqueue.task_with_context()
        async def create_user_task(ctx: DbContext, email: str, username: str):
            async with ctx.session_context() as db_session:
                user = User(
                    email=email,
                    username=username
                )
                db_session.add(user)

        await create_user_task
        ```
        """

        @overload
        def decorator(func: Callable[Concatenate[C, P], None]) -> Callable[P, None]: ...

        @overload
        def decorator(
            func: Callable[Concatenate[C, P], Coroutine[Any, Any, None]],
        ) -> Callable[P, Coroutine[Any, Any, None]]: ...

        def decorator(
            func: Callable[Concatenate[C, P], None | Coroutine[Any, Any, None]],
        ) -> Callable[P, None | Coroutine[Any, Any, None]]:
            return _with_context(
                cast(Any, func),
                name=name,
                queue=queue,
                max_retries=max_retries,
                core=self._core,
            )

        return decorator
