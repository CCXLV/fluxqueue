from fluxqueue import Context

from .main import fluxqueue


@fluxqueue.task_with_context()
def sync_func_with_context(ctx: Context):
    print("sync_func_with_context metadata: ", ctx.metadata)


@fluxqueue.task_with_context()
async def async_func_with_context(ctx: Context):
    print("async_func_with_context metadata: ", ctx.metadata)


class TestContext(Context):
    def get_test_connection(self):
        return "conn"


@fluxqueue.task_with_context()
def test_custom_context(ctx: TestContext):
    print("test_custom_context metadata: ", ctx.metadata)
