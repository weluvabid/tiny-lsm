# The LSM-tree model

A log-structured merge-tree moves write work away from in-place mutation of a large sorted disk structure. New records are first accumulated in memory. The in-memory state is later written as an immutable sorted component, and multiple components are periodically merged.

TinyLSM implements a deliberately small version of this model: one mutable memtable and a collection of immutable sorted runs.

## Why sorted layers?

The memtable is a `BTreeMap`, so it maintains key order while accepting updates. When flushed, iteration produces records in key order. The resulting run is also sorted, allowing binary search during reads and a heap-based merge during compaction.

The trade-off is that reads may need to consult several layers. A key absent from the newest layer may still exist in an older one. TinyLSM resolves this by checking newest state first.

## Versions and precedence

TinyLSM does not store an explicit sequence number in each record. Instead, recency is represented by the layer containing the record:

1. The current memtable wins over all persisted runs.
2. A run with a larger generation wins over a run with a smaller generation.
3. Within a layer there is one record per key.

This is sufficient for the current single-writer, synchronous model. It is not a substitute for sequence numbers or concurrency control in a multi-writer database.

## Tombstones

A deletion must override an older value. Removing a key only from the memtable would allow an older run to reveal it again. TinyLSM therefore stores a tombstone in the memtable and persists it like any other record.

A read that encounters the newest tombstone returns `None` immediately. Compaction keeps the newest tombstone for a key so that older values remain suppressed after the input runs are replaced.

## Flush and compaction are different

- **Flush** converts the current mutable memtable into a new immutable run. It does not merge older runs.
- **Compaction** merges existing immutable runs into a replacement run, resolving duplicate keys by generation.
