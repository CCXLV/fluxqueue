from fluxqueue import Context

from .conftest import TestEnvFixture


def test_scheduled_task_type_hints(test_env: TestEnvFixture):
    @test_env.fluxqueue.task()
    def task():
        print("Correct Task")

    assert hasattr(task, "defer")
    assert hasattr(task, "cron")

    @test_env.fluxqueue.task_with_context()
    def task_with_ctx(ctx: Context):
        pass

    assert hasattr(task_with_ctx, "defer")
    assert hasattr(task_with_ctx, "cron")
