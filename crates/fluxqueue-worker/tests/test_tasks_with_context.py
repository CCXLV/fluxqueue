from fluxqueue import Context

from .main import fluxqueue


@fluxqueue.task_with_context()
def sync_func_with_context(ctx: Context):
    print(ctx.metadata)
