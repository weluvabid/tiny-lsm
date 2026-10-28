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

    pub(crate) fn record(&self, key: &Key) -> Option<&RecordValue> {
        self.entries.get(key)
    }

    pub(crate) fn delete(&mut self, key: Key) {
        self.entries.insert(key, RecordValue::Tombstone);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn snapshot_entries(&self) -> Vec<(Key, RecordValue)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &Key> {
        self.entries.keys()
    }
}
