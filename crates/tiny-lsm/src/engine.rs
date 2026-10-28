use crate::memtable::Memtable;
use crate::{Key, Value};

/// In-memory key-value engine. Disk persistence is added in a later phase.
#[derive(Debug, Default)]
pub struct Engine {
    memtable: Memtable,
}

impl Engine {
    /// Creates an empty engine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces a key's value.
    pub fn put(&mut self, key: impl Into<Key>, value: impl Into<Value>) {
        self.memtable.put(key.into(), value.into());
    }

    /// Returns the current value for a key, if present.
    #[must_use]
    pub fn get(&self, key: impl Into<Key>) -> Option<&Value> {
        self.memtable.get(&key.into())
    }

    /// Deletes a key and returns whether it had a visible value.
    ///
    /// A tombstone is recorded even when the key is already absent.
    pub fn delete(&mut self, key: impl Into<Key>) -> bool {
        let key = key.into();
        let existed = self.memtable.get(&key).is_some();
        self.memtable.delete(key);
        existed
    }

    /// Returns the number of live keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.memtable.live_len()
    }

    /// Returns whether the engine contains any live keys.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
