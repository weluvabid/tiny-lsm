//! Foundation types and behavioral contracts for TinyLSM.
//!
//! Storage, persistence, and engine operations are intentionally not
//! implemented in Phase 0.

mod error;
mod key;
mod record;
mod run;

pub use error::TinyLsmError;
pub use key::{Key, Value};
pub use record::RecordValue;
pub use run::{Generation, RunId};

#[cfg(test)]
mod tests {
    use super::{Generation, Key, RecordValue, RunId, Value};

    #[test]
    fn keys_use_lexicographic_byte_order() {
        let mut keys = [
            Key::from(&b"b"[..]),
            Key::from(&b"aa"[..]),
            Key::from(&b"a"[..]),
        ];
        keys.sort();

        assert_eq!(
            keys,
            [
                Key::from(&b"a"[..]),
                Key::from(&b"aa"[..]),
                Key::from(&b"b"[..])
            ]
        );
    }

    #[test]
    fn empty_value_is_not_a_tombstone() {
        let empty_value = RecordValue::Value(Value::empty());
        let tombstone = RecordValue::Tombstone;

        assert_ne!(empty_value, tombstone);
    }

    #[test]
    fn run_identity_and_generation_are_orderable() {
        assert!(Generation::new(2) > Generation::new(1));
        assert_ne!(RunId::new(7), RunId::new(8));
    }
}
