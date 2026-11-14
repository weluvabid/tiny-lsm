# Benchmarks

TinyLSM includes a local benchmark example at `crates/tiny-lsm/examples/benchmark.rs`. Run it with:

```sh
./scripts/bench.sh
```

The script runs the example in release mode. The workload size and sample count can be adjusted with environment variables:

```sh
BENCH_ITEMS=50000 BENCH_SAMPLES=25 ./scripts/bench.sh
```

The implementation clamps the item count to at least the number of configured layers and the sample count to at least one.

## Workloads

| Workload | What is timed |
|---|---|
| `memtable_put` | Inserting generated keys and values into a fresh in-memory engine |
| `memtable_get` | Looking up generated keys in a populated in-memory engine |
| `flush` | Flushing a populated memtable to a newly opened temporary directory |
| `layered_get_8_runs` | Looking up keys in a store distributed across eight non-overlapping runs |
| `compaction_8_runs` | Compacting eight overlapping runs, including tombstones in the newest layer |
| `reopen_8_runs` | Opening a store with eight persisted runs |

Each workload is sampled repeatedly. The output reports median and p95 nanoseconds per item and a derived items-per-second figure based on the median.

## Interpreting results

These measurements provide a repeatable local baseline for observing changes in TinyLSM.

Results depend on CPU, memory, filesystem, storage device, operating system, build toolchain, cache state, item count and sample count. Flush, compaction and reopen include filesystem activity; in-memory workloads do not.

For useful comparisons, run the same workload on the same machine and toolchain, repeat measurements, and account for variance.
