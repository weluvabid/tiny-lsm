use std::collections::BTreeMap;

use crate::{Key, RecordValue, Value};

#[derive(Debug, Default)]
pub(crate) struct Memtable {
    entries: BTreeMap<Key, RecordValue>,
}

impl Memtable {
    pub(crate) fn put(&mut self, key: Key, value: Value) {
        self.entries.insert(key, RecordValue::Value(value));
    }

    pub(crate) fn get(&self, key: &Key) -> Option<&Value> {
        match self.entries.get(key) {
            Some(RecordValue::Value(value)) => Some(value),
            Some(RecordValue::Tombstone) | None => None,
        }
    }

    pub(crate) fn delete(&mut self, key: Key) {
        self.entries.insert(key, RecordValue::Tombstone);
    }

    pub(crate) fn live_len(&self) -> usize {
        self.entries
            .values()
            .filter(|record| matches!(record, RecordValue::Value(_)))
            .count()
    }
}
