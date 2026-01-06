# FastQueue

**A blazingly fast, lightweight task queue for Python, powered by Rust.**

*Work in progress...*

## TODOs

- [ ] Make the `enqueue` function async.
- [ ] Refactor the `cli` and release it as a PyPI package to make it easier to use.
- [ ] Add a program entry point to `fastqueue-worker`.
- [ ] Add task processing functionality to the `fastqueue-worker` crate; use `pyo3-async-runtimes` for running async functions instead of the method used in the main crate.
- [ ] Move TaskRegistry to `fastqueue-worker` crate from the main crate and handle it from there.
