# Read and write paths

## Read path

`Engine::get` follows the precedence order directly:

1. Look in the memtable.
2. If a record exists there, return its value or return `None` for a tombstone. Do not consult older runs.
3. Otherwise, inspect persisted runs newest to oldest.
4. Binary-search each run by key. The first matching record determines the result.
5. Return `None` if no layer contains the key.

The first match wins because layers are ordered by recency. Continuing to older layers after finding a tombstone would resurrect deleted data.

For `R` runs, lookup can perform up to `R` binary searches. The cost is approximately `O(log M + sum(log n_i))` in the worst case where all layers are searched, ignoring key comparison and I/O because the current implementation keeps run entries in memory after loading.

## Write path

`put` and `delete` mutate only the memtable. They do not write to disk immediately. This is why a newly created in-memory engine can accept writes without a directory.

`flush` behaves as follows:

1. Return `Ok(false)` if the memtable is empty.
2. Reject the operation if the engine has no backing directory.
3. Allocate the next generation.
4. Take a sorted snapshot of memtable entries.
5. Write and publish a new run.
6. Add the new run to the active run list.
7. Clear the memtable and return `Ok(true)`.

The memtable is not cleared before successful run publication. This protects the current in-memory state from ordinary write and publication failures.

## `len` and `is_empty`

`len()` collects candidate keys from the memtable and all run entries, deduplicates them, and resolves each key through the normal read path. Tombstoned keys are excluded. `is_empty()` delegates to `len()`.

These are correctness-oriented convenience operations. They scan the known key set and are not constant-time counters.

## `inspect`

`inspect()` returns an owned snapshot of memtable records and runs. Records within each layer are ordered by key; runs are ordered oldest to newest. It is intended for understanding and testing the storage layout, not as a zero-copy or low-cost metrics endpoint.
