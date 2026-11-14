# Persistence, recovery and durability boundaries

This document distinguishes successful API operations, process-crash recovery behavior and power-loss durability. They are not interchangeable guarantees.

## Opening a store

`Engine::open(path)` creates the directory if needed, discovers published run files, sorts them by generation, and loads and validates each run before returning an engine.

A malformed published run causes `open` to return an error. TinyLSM does not silently skip a corrupt run because doing so could make older data appear current or hide data loss.

## Temporary run files

Run writes use names of the form `.run-{generation}.tmp`. On open, TinyLSM removes matching stale temporary files that are regular files. Unrelated files and names that do not match this exact temporary naming pattern are left alone.

On Unix, if temporary files were removed, the containing directory is synchronized after cleanup. This records the directory-entry changes as part of recovery cleanup.

## Flush failure behavior

A flush writes to a temporary file and publishes it by rename. The memtable is cleared only after the run-writing operation returns success. Therefore an ordinary reported failure leaves the logical data in the memtable for the caller to retry or inspect.

The temporary file is removed on a best-effort basis when the write operation fails. A failed cleanup is not currently reported as a second error.

## Compaction interruption

Compaction publishes the merged run before deleting old runs. If a process stops after publication and before cleanup finishes, a later open may discover both the replacement and old inputs. The replacement has a greater generation and contains the merged state, so it takes precedence during reads. A subsequent compaction or normal operation can continue from the discovered layout.

This behavior is covered by a recovery integration test that reconstructs the state immediately after publication and before old-run cleanup.

## What is synchronized today

The run writer flushes its buffered writer and calls `sync_all` on the temporary run file before renaming it to the final filename. On Unix, temporary-run recovery cleanup synchronizes the directory after removing files.

The normal publication path does not synchronize the parent directory after the rename. A successful `flush` or `compact` confirms that the run file was synced and published, but does not guarantee that the directory entry survives sudden power loss.

Changes held only in the memtable are not persisted until a flush publishes a run. The run directory is the catalog used during recovery; each published run is validated before it is loaded.
