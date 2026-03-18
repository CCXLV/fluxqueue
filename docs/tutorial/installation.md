# Installation

FluxQueue has two parts: the Python client library for enqueueing tasks, and the Rust worker that executes them. Both use Rust under the hood for it's low memory usage. The worker needs to be installed separately on your system. Currently Linux is supported, with Windows and macOS support coming soon. The Python version range will be expanded in future releases as well.

## Prerequisites

- Python 3.11, 3.12, 3.13 or 3,14
- Redis server running and accessible
- Linux (Windows and macOS support coming soon)

## Install the Client Library

Install FluxQueue from PyPI:

```bash
pip install fluxqueue[cli]
```

The `[cli]` extra includes the `fluxqueue-cli` tool needed for worker management.

## Install the Worker

The worker is a standalone Rust binary that executes your tasks. The easiest way to install it is using `fluxqueue-cli`, which you already installed above.

Run the installation command:

```bash
fluxqueue worker install
```

<!-- prettier-ignore -->
!!! note
    Installing the worker that way will require `sudo` permissions. To avoid that you can install the worker at desired destination with the following command:
    ```bash
    fluxqueue worker install --path .fluxqueue/
    ```

    Then you can add a `worker_path` in your `pyproject.toml` to run the correct worker with `fluxqueue start` command:
    ```toml
    [tool.fluxqueue_cli]
    worker_path = ".fluxqueue/fluxqueue_worker"
    ```

This downloads and installs the `fluxqueue-worker` binary on your system. Once installed, you can run the worker in two ways:

- Use the `fluxqueue-worker` binary directly
- Use `fluxqueue start` command
- Or just run the downloaded binary

Both commands run the same worker binary, so choose whichever fits your workflow better.
