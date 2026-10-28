use crate::{Generation, Key, RecordValue, RunId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordInfo {
    pub key: Key,
    pub value: RecordValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunInfo {
    pub id: RunId,
    pub generation: Generation,
    pub records: Vec<RecordInfo>,
    pub tombstone_count: usize,
}

impl RunInfo {
    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records.len()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StoreSnapshot {
    pub memtable_records: Vec<RecordInfo>,
    pub runs: Vec<RunInfo>,
}

impl StoreSnapshot {
    #[must_use]
    pub fn memtable_tombstone_count(&self) -> usize {
        self.memtable_records
            .iter()
            .filter(|record| matches!(&record.value, RecordValue::Tombstone))
            .count()
    }
}
