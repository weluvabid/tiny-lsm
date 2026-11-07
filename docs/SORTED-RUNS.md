# Sorted runs and the on-disk format

A sorted run is an immutable, lexicographically ordered sequence of `(Key, RecordValue)` records stored in one file. The current filename is `run-{generation}.tlsm`.

## File layout

The current format is little-endian and versioned:

| Field | Encoding | Meaning |
|---|---|---|
| Magic | 8 bytes | ASCII bytes `TINYLSM1` |
| Format version | `u16` | Currently `1` |
| Record count | `u64` | Number of records that follow |
| Record key length | `u64` | Number of key bytes |
| Record tag | `u8` | `0` for tombstone, `1` for value |
| Value length | `u64` | Present only for tag `1` |
| Key bytes | variable | Opaque key payload |
| Value bytes | variable | Present only for tag `1` |

For a tombstone, no value-length field or value payload is written. For a value record, a zero-length payload is valid and represents an empty value.

The parser enforces the current maximum record count and field length limits. These are defensive format limits, not recommended application-level sizes.

## Writing a run

`SortedRun::write` creates a generation-specific temporary file using `create_new`, writes the header and sorted records through a buffered writer, flushes the buffer, calls `sync_all` on the file, and renames the temporary file to its final run filename.

The engine only appends the resulting in-memory run to its active run list and clears the memtable after this operation succeeds. If writing or publication fails, the memtable remains available for retry. A temporary file created by the failed attempt is removed on a best-effort basis; cleanup failure is not currently surfaced separately.

## Loading and validation

Loading rejects:

- Incorrect magic or unsupported format versions.
- Record counts above the configured format limit.
- Lengths above the configured field limit or values that do not fit the platform's `usize`.
- Unknown record tags.
- Truncated headers, lengths, keys or values.
- Keys that are not strictly increasing.
- Any trailing bytes after the declared final record.

These checks prevent malformed files from being treated as valid runs. The format currently has no checksum, block index or independent metadata manifest.

## Generation names

Run generations are numeric and monotonically allocated relative to the highest currently loaded generation. Discovery accepts canonical names such as `run-12.tlsm`; malformed or non-canonical names matching the run filename pattern are treated as corruption rather than silently skipped.

The format is currently version 1. A version field makes future evolution possible, but compatibility and migration behavior are not yet a stable public guarantee.
