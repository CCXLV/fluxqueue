# FluxQueue Worker
 
The FluxQueue worker is a Rust binary which internally runs a `tokio` multi-threaded runtime. The `--concurrency` argument it takes actually determines the number of `tokio` tasks it is going to spawn. That's why even with a single worker and multiple `concurrency` it performs well with high concurrency and the memory usage is so low.
 
## Executor Loop
 
Executor loops are the `tokio tasks` that the runtime spawns. They are responsible of running the tasks.
