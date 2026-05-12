from .conftest import TestEnvFixture


def test_scheduled_task_type_hints(test_env: TestEnvFixture):
    @test_env.fluxqueue.task()
    def task():
        print("Correct Task")

    assert hasattr(task, "defer")
    assert hasattr(task, "cron")
