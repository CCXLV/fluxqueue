from dataclasses import dataclass


@dataclass(slots=True)
class TaskMetadata:
    """
    Read-only metadata for a FluxQueue task.
    """

    task_id: str
    """Unique identifier for the current task execution."""
    retry_count: int
    """Number of times this task has been retried."""
    max_retries: int
    """Maximum number of retry attempts allowed before failure."""
    enqueued_at: int
    """ISO 8601 timestamp of when the task was originally enqueued."""
