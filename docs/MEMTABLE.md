# Memtable and record semantics

## Representation

The memtable is a `BTreeMap<Key, RecordValue>`. It provides deterministic lexicographic ordering and one current record per key.

`Key` and `Value` are opaque byte strings. Convenience conversions accept byte vectors, byte slices, strings and string slices. The store does not interpret UTF-8, delimiters or application-level schemas.

## Put and overwrite

`put(key, value)` inserts or replaces the record for that key in the memtable. Repeated writes to the same key within the current memtable collapse to the latest value. Earlier values in that same layer are not retained as versions.

## Delete and tombstones

`delete(key)` first checks the visible logical state to determine its boolean return value, then writes a tombstone into the memtable. The tombstone is retained even when the key is not currently visible; this ensures it suppresses any older value in a persisted run.

The return value answers whether a live value was visible before the deletion. It does not indicate whether a tombstone was written: a tombstone is written either way.

## Empty values are not deletions

An empty `Value` is a valid present value. `get` returns `Some(&Value)` for an empty value and `None` for a missing key or a key whose newest record is a tombstone.

This distinction is preserved in the on-disk format through separate record tags.

## Snapshot behavior

`Engine::inspect()` clones the current memtable records into a deterministic snapshot. The snapshot is a point-in-time representation of the layout at the time of the call; later writes do not mutate an already returned snapshot.

## Cost characteristics

A `BTreeMap` gives ordered insertion and lookup in logarithmic time in the number of memtable keys. Snapshotting clones keys and record payloads, so its memory and time cost scale with the memtable contents and the size of stored data.
