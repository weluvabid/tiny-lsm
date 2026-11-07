# Testing strategy

TinyLSM's tests validate logical behavior and persisted-state behavior separately. The suite is designed to catch regressions in ordering, version precedence, tombstones, serialization, compaction and recovery.

Run the complete verification script from the repository root:

```sh
./scripts/check.sh
```

It runs:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

## Test layers

### Logical engine tests

`tests/engine.rs` checks put/get/overwrite/delete behavior, empty keys and values, live-key counts, and a deterministic operation sequence against a `BTreeMap` reference model.

### Persistence tests

`tests/persistence.rs` checks flush publication, reopening, multiple generations, tombstones, empty values and preservation of memtable contents when flush fails.

### Compaction tests

`tests/compaction.rs` checks logical equivalence before and after compaction, generation and record metadata, tombstone retention, reopening the compacted store and failure behavior when temporary-file creation is blocked.

### Introspection tests

`tests/introspection.rs` checks deterministic ordering, stable snapshots and the separation between memtable records and persisted runs.

### Corruption tests

`tests/corruption.rs` checks rejection of every truncated prefix of a valid run, trailing bytes, unknown tags and excessive record counts.

### Recovery tests

`tests/recovery.rs` checks stale temporary-file cleanup, preservation of unrelated files, recovery after replacement publication but interrupted old-run cleanup, and rejection of non-canonical run filenames.

### Adversarial model test

`tests/adversarial.rs` runs a deterministic randomized sequence of writes, deletes, reads, flushes, compactions and reopen operations against a reference map. It checks visible state throughout the sequence and after a final reopen.
