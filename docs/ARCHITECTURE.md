# Architecture and invariants

TinyLSM is a synchronous, single-writer LSM-style key-value store. Its implementation is split into small modules with explicit responsibilities:

| Module | Responsibility |
|---|---|
| `engine` | Public operations and coordination of the memtable and runs |
| `memtable` | Ordered mutable in-memory records |
| `record` | Distinguishes values from tombstones |
| `key` | Opaque byte-string keys and values |
| `run` | Run serialization, loading, discovery and generation allocation |
| `compaction` | K-way merge of immutable sorted runs |
| `inspection` | Stable snapshots of the storage layout |
| `error` | Public error boundary |

## State model

An `Engine` contains:

- An optional directory. `None` means the engine was created with `Engine::new()` and is memory-only.
- One mutable `Memtable`, backed by `BTreeMap<Key, RecordValue>`.
- Zero or more immutable `SortedRun`s ordered by generation, oldest first.

The memtable and every run contain at most one record per key. A record is either a value (including an empty byte sequence) or a tombstone.

## Core invariants

1. **Key ordering:** keys are ordered lexicographically by their bytes. The memtable and every persisted run use this ordering.
2. **Layer precedence:** the memtable is newer than every run; among runs, a greater generation is newer.
3. **Deletion is data:** a tombstone is a record that suppresses older values. It is not equivalent to the absence of a record in a layer.
4. **Run immutability:** once published, a run is treated as immutable by the engine.
5. **Publish before discard:** flush does not clear the memtable until the new run has been written and published successfully.
6. **Validate before use:** discovered runs are parsed and structurally validated before the engine is returned from `open`.
7. **Unknown persisted structures fail:** malformed filenames, invalid headers, unknown tags, invalid ordering, truncation and trailing bytes are reported as errors rather than silently ignored.

## Ownership and concurrency

The public API takes `&mut self` for mutations and does not expose shared concurrent mutation. There is no internal locking or background worker. Treat a store directory as owned by one active writer. Multiple processes or independently opened `Engine` instances writing to the same directory are not coordinated and are outside the supported model.

## Complexity at a glance

Let `M` be the number of records in the memtable, `R` the number of runs, `N` the total number of records across runs, and `K` the number of runs participating in compaction.

- Memtable insertion and lookup are `O(log M)` due to `BTreeMap`.
- A run lookup is `O(log n_i)` for that run's `n_i` records; a lookup can inspect up to `R` runs.
- Flush writes `M` records in sorted order, plus serialization and filesystem costs.
- Compaction merges sorted inputs in `O(N log K)` heap operations, plus output serialization and file I/O.
- `len()` builds a set of candidate keys and resolves each against the layered state; it is a convenience scan, not a constant-time metadata lookup.

These are algorithmic bounds. Filesystem latency, allocation, key/value sizes and cache behavior affect observed timings.

## Omissions

There is no WAL, transaction protocol, concurrent reader/writer coordination, background compaction, leveled/tiered policy, Bloom filter, block cache, checksummed block format, manifest, or remote API. These omissions keep the implementation focused and should be considered before using it outside an educational or experimental context.
