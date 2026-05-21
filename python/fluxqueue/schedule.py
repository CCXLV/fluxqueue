from collections.abc import Callable
from typing import Any, ParamSpec, TypeVar, cast

P = ParamSpec("P")
R = TypeVar("R")


class CronSchedule:
    def __init__(
        self,
        minute: int | str = "*",
        hour: int | str = "*",
        day_of_month: int | str = "*",
        month: int | str = "*",
        day_of_week: int | str = "*",
    ) -> None:
        self.minute = minute
        self.hour = hour
        self.day_of_month = day_of_month
        self.month = month
        self.day_of_week = day_of_week


def cron(
    cron_schedule: CronSchedule | None = None,
    *,
    expression: str | None = None,
    minute: int | str = "*",
    hour: int | str = "*",
    day_of_month: int | str = "*",
    month: int | str = "*",
    day_of_week: int | str = "*",
) -> Callable[[Callable[P, R]], Callable[P, R]]:
    final_expression: str | None = None

    if not cron_schedule:
        if expression:
            final_expression = expression
        else:
            final_expression = f"{minute} {hour} {day_of_month} {month} {day_of_week}"
    else:
        final_expression = f"{cron_schedule.minute} {cron_schedule.hour} {cron_schedule.day_of_month} {cron_schedule.month} {cron_schedule.day_of_week}"

    def decorator(func: Callable[P, R]) -> Callable[P, R]:
        cast(Any, func).cron_expression = final_expression

        return func

    return decorator
