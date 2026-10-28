# TinyLSM

A tiny, inspectable LSM-tree key-value store written in Rust.

TinyLSM demonstrates mutable memtables, immutable disk-backed sorted runs,
layered reads, tombstones, and compaction through explicit storage contracts
and invariant-driven tests.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
