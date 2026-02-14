from .conftest import TestEnvFixture


def test_correct_connection(test_env: TestEnvFixture):
    @test_env.fluxqueue.task()
    def task():
        print("Correct Task")

    assert task() is None
