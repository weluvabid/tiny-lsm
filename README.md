# TinyLSM

A small, inspectable LSM-tree key-value store in Rust. It stores opaque byte keys and values, including empty byte strings.

## Model

- Writes and tombstones accumulate in a sorted memtable.
- `flush` publishes an immutable, sorted run to disk.
- Reads check the memtable, then runs newest to oldest.
- `compact` merges runs while preserving the newest value or tombstone for each key.
- `inspect` exposes a deterministic snapshot of the current layers.

Disk-backed stores are opened with `Engine::open(path)`. `Engine::new()` is in-memory only; flushing or compacting it returns an error.

## Scope

Single-writer, synchronous API. No WAL, concurrent writers, background compaction, Bloom filters, block cache, or network server. A directory scan discovers published runs on open; abandoned temporary run files are removed during recovery. This is an educational storage engine, not a replacement for a production database.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
