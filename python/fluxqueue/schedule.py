from collections.abc import Callable
from datetime import timedelta
from typing import Any, ParamSpec, Protocol, TypeVar, cast, runtime_checkable

P = ParamSpec("P")
R = TypeVar("R")
R_co = TypeVar("R_co", covariant=True)


def cron(
    expression: str | None = None,
    *,
    minute: int | str = "*",
    hour: int | str = "*",
    day_of_month: int | str = "*",
    month: int | str = "*",
    day_of_week: int | str = "*",
) -> Callable[[Callable[P, R]], Callable[P, R]]:
    final_expression: str | None = None

    if expression:
        final_expression = expression
    else:
        final_expression = f"{minute} {hour} {day_of_month} {month} {day_of_week}"

    def decorator(func: Callable[P, R]) -> Callable[P, R]:
        cast(Any, func).cron_expression = final_expression

        return func

    return decorator


@runtime_checkable
class ScheduledTask(Protocol[P, R_co]):
    def __call__(self, *args: P.args, **kwargs: P.kwargs) -> R_co: ...
    def defer(self, delay: timedelta, **kwargs: Any) -> None: ...
    def cron(
        self,
        expression: str | None = None,
        *,
        minute: int | str = "*",
        hour: int | str = "*",
        day_of_month: int | str = "*",
        month: int | str = "*",
        day_of_week: int | str = "*",
    ) -> "ScheduledTask[P, R_co]": ...
