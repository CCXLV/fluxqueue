# Installation

FluxQueue has two parts: the Python client library for enqueueing tasks, and the Rust worker that executes them. Both use Rust under the hood for speed. The worker needs to be installed separately on your system. Currently Linux is supported, with Windows and macOS support coming soon. The Python version range will be expanded in future releases as well.

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

This downloads and installs the `fluxqueue-worker` binary on your system. Once installed, you can run the worker in two ways:

- Use the `fluxqueue-worker` binary directly
- Use `fluxqueue start` command

Both commands run the same worker binary, so choose whichever fits your workflow better.
