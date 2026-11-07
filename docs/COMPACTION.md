# Compaction

Compaction merges two or more immutable sorted runs into one replacement run. It preserves the newest record for every key, including tombstones.

## Why merge sorted inputs?

Every run is already sorted by key. Concatenating the inputs and sorting the entire result would discard useful ordering work. TinyLSM instead performs a k-way merge with a `BinaryHeap`.

## Merge algorithm

For each input run, the algorithm inserts its first record into a min-oriented heap (implemented with `Reverse`). Heap entries contain the key, run index and record index.

The merge loop:

1. Pops the smallest key currently available.
2. Advances that run's cursor and pushes its next record, if any.
3. Collects any other heap entries with the same key.
4. Chooses the matching record from the greatest run index. Runs are stored oldest to newest, so the greatest index is the newest generation.
5. Emits exactly one record for that key.

Because each input run is strictly sorted and contains at most one record per key, each run contributes at most one current heap entry at a time.

The merge takes `O(N log K)` heap work for `N` total input records and `K` runs, plus output allocation and serialization. The current implementation materializes the merged output as a `Vec` before writing the replacement run, so peak memory is proportional to the merged record set and its payloads.

## Tombstone preservation

Compaction does not discard tombstones. A tombstone may be the only record preventing an older value from becoming visible. The current engine has no lower-level retention analysis or external snapshot mechanism that would prove a tombstone safe to remove. Preserving it is the conservative behavior.

## Publication and cleanup

The replacement run is written and published before old run files are removed. The engine then switches its in-memory run list to the replacement and attempts to remove the old files.

If removal fails, the operation returns an I/O error and restores the still-existing old runs to the in-memory list alongside the replacement, ordered by generation. Keeping both old and replacement runs is logically safe: the replacement has the newest generation and contains the merged logical state, so it wins on reads.

If the process stops after replacement publication but before all old files are removed, startup discovery loads all published runs. Generation ordering means the replacement run wins over its older inputs. This is a simple recovery strategy for the current design, not a general manifest-based atomic compaction protocol.

## When compaction does nothing

`compact()` returns `Ok(false)` when fewer than two runs exist. Otherwise it returns `Ok(true)` after successful replacement and cleanup. It is explicitly invoked by the caller; there is no automatic compaction policy.
