import pytest

from fluxqueue import FluxQueue

from .main import fluxqueue


def test_invalid_connection():
    invalid_fluxqueue = FluxQueue(redis_url="redis://localhost:123")

    @invalid_fluxqueue.task()
    def invalid_task():
        print("Invalid Task")

    with pytest.raises(RuntimeError):
        invalid_task()


def test_correct_connection():
    @fluxqueue.task()
    def task():
        print("Correct Task")

    assert task() is None
