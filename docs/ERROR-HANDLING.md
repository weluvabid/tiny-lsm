# Errors and failure behavior

The public operations that interact with persistent state return `Result<_, TinyLsmError>`. The error type distinguishes filesystem failures, malformed persisted data, invalid operation state and exhausted generation identifiers.

## Error variants

| Variant | Meaning |
|---|---|
| `Io` | An operating-system I/O operation failed; includes the operation, optional path and source error |
| `CorruptData` | A persisted filename or run file violates the supported format; includes an optional path and reason |
| `InvalidState` | An operation requires a disk-backed engine but the engine was created with `Engine::new()` |
| `IdentifierExhausted` | A generation cannot be incremented without overflowing `u64` |

`TinyLsmError` implements `Display` and `std::error::Error`. For I/O failures, `source()` exposes the underlying `io::Error`, preserving the causal error chain.

## Expected non-error outcomes

Some outcomes are represented as ordinary return values:

- `get` returns `None` for a missing key or a newest tombstone.
- `delete` returns `false` when no live value was visible before the tombstone was written.
- `flush` returns `false` when the memtable is empty.
- `compact` returns `false` when fewer than two runs exist.

These are not failures and should not be conflated with I/O or corruption errors.

## Failure principles

The implementation favors explicit failure over silently accepting unknown persisted state. In particular, malformed run files fail store opening; unsupported versions and unknown record tags fail parsing; and a failed flush does not clear the memtable.

Cleanup has narrower semantics: removal of a temporary file after a failed write is best effort, and compaction can report an error after publishing the replacement if old-run deletion fails. Callers should preserve and inspect the returned error rather than assuming every error means no filesystem state changed.

## Caller guidance

- Propagate `TinyLsmError` where the caller cannot make a safe recovery decision.
- Include the operation and store path in application-level diagnostics when handling an error.
- Do not automatically delete files that failed validation; preserve them for diagnosis.
- After a compaction cleanup error, reopen or inspect the directory before attempting manual cleanup. The replacement may already have been published.
- Do not retry an operation blindly if its failure may have occurred after publication; first establish the current on-disk state.
