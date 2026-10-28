use crate::Value;

/// The logical payload stored for a key.
///
/// A tombstone is a versioned deletion marker, not an absent record. During
/// reads it terminates lookup as `NotFound`; it must not reveal older values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecordValue {
    /// A present value, including an empty byte sequence.
    Value(Value),
    /// A deletion marker that suppresses older versions of the key.
    Tombstone,
}
