# TinyLSM

TinyLSM is a small, inspectable log-structured merge-tree (LSM-tree) key-value store written in Rust. It is an educational storage engine for understanding ordered in-memory state, immutable sorted runs, persistence, recovery and compaction.

It stores opaque byte-string keys and values. Empty keys and empty values are valid and are distinct from missing keys.

## What it implements

- An ordered in-memory memtable backed by `BTreeMap`.
- Immutable, lexicographically sorted disk runs.
- Newest-first reads across the memtable and persisted runs.
- Tombstones that preserve deletion semantics across flushes and compaction.
- K-way compaction of sorted runs.
- Versioned run files with structural validation during recovery.
- Deterministic storage introspection through `Engine::inspect`.
- Typed errors, integration tests, adversarial recovery tests and a local benchmark.

## Quick start

```rust,no_run
use tiny_lsm::Engine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = Engine::open("./data")?;

    store.put("user:42", "Ada");
    assert_eq!(store.get("user:42").map(|value| value.as_bytes()), Some(&b"Ada"[..]));

    // Publish the current memtable as an immutable run.
    store.flush()?;

    // A deletion is represented by a tombstone and can be persisted too.
    store.delete("user:42");
    store.flush()?;
    assert_eq!(store.get("user:42"), None);

    // Merge multiple runs while preserving the newest record per key.
    store.compact()?;
    Ok(())
}
```

`Engine::new()` creates an in-memory engine. It supports `put`, `get`, `delete`, `len` and `is_empty`, but `flush` and `compact` require an engine opened with `Engine::open(path)`.

## Design overview

Writes update the memtable. A flush serializes its ordered snapshot into a new immutable run and only clears the memtable after the run has been successfully published. Reads consult the memtable first, then inspect runs from newest to oldest. Compaction merges the sorted inputs and selects the newest generation for duplicate keys.

```text
                 writes
                    |
                    v
             +--------------+
             |   Memtable   |  BTreeMap<Key, RecordValue>
             +--------------+
                    |
                  flush
                    v
       +-------------------------+
       | Immutable sorted runs   |
       | run-1, run-2, ...       |
       +-------------------------+
                    |
                 compact
                    v
       +-------------------------+
       | New merged sorted run   |
       +-------------------------+
```

This is a compact implementation. It has a synchronous single-writer API and does not implement a write-ahead log, concurrent access, background maintenance, Bloom filters, block cache, transactions or a network protocol.

## Engineering notes

The documentation is organized around the actual implementation and its trade-offs:

- [Architecture and invariants](docs/ARCHITECTURE.md)
- [LSM-tree model](docs/LSM-TREE.md)
- [Memtable and record semantics](docs/MEMTABLE.md)
- [Sorted-run file format](docs/SORTED-RUNS.md)
- [Read and write paths](docs/READ-WRITE-PATHS.md)
- [Compaction](docs/COMPACTION.md)
- [Persistence and recovery](docs/RECOVERY.md)
- [Errors and failure behavior](docs/ERROR-HANDLING.md)
- [Testing strategy](docs/TESTING.md)
- [Benchmark methodology](docs/BENCHMARKS.md)

## Benchmark recording

The recording below shows the benchmark harness running locally. See the [benchmark notes](docs/BENCHMARKS.md) for workload definitions, configuration and interpretation.

![TinyLSM benchmark run](benchmarks/tiny-lsm-benchmark.gif)

## Development

Requires a stable Rust toolchain with `rustfmt` and `clippy` components.

```sh
./scripts/check.sh
./scripts/bench.sh
```

The check script runs formatting validation, Clippy with warnings denied, and the workspace test suite. The benchmark reports local median and p95 nanoseconds per item for selected workloads. It is a repeatable local baseline, not a cross-machine performance claim. Set `BENCH_ITEMS` and `BENCH_SAMPLES` to adjust workload size and sample count.
