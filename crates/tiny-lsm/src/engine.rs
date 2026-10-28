use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    Key, RecordValue, TinyLsmError, Value,
    memtable::Memtable,
    run::{SortedRun, discover, next_generation},
};

#[derive(Debug, Default)]
pub struct Engine {
    directory: Option<PathBuf>,
    memtable: Memtable,
    runs: Vec<SortedRun>,
}

impl Engine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, TinyLsmError> {
        let directory = path.as_ref().to_path_buf();
        fs::create_dir_all(&directory).map_err(|source| TinyLsmError::Io {
            operation: "creating store directory",
            path: Some(directory.clone()),
            source,
        })?;
        let runs = discover(&directory)?;

        Ok(Self {
            directory: Some(directory),
            memtable: Memtable::default(),
            runs,
        })
    }

    pub fn put(&mut self, key: impl Into<Key>, value: impl Into<Value>) {
        self.memtable.put(key.into(), value.into());
    }

    #[must_use]
    pub fn get(&self, key: impl Into<Key>) -> Option<&Value> {
        let key = key.into();
        if let Some(record) = self.memtable.record(&key) {
            return match record {
                RecordValue::Value(value) => Some(value),
                RecordValue::Tombstone => None,
            };
        }

        for run in self.runs.iter().rev() {
            if let Ok(index) = run
                .entries
                .binary_search_by(|(candidate, _)| candidate.cmp(&key))
            {
                return match &run.entries[index].1 {
                    RecordValue::Value(value) => Some(value),
                    RecordValue::Tombstone => None,
                };
            }
        }
        None
    }

    pub fn delete(&mut self, key: impl Into<Key>) -> bool {
        let key = key.into();
        let existed = self.get(key.clone()).is_some();
        self.memtable.delete(key);
        existed
    }

    pub fn flush(&mut self) -> Result<bool, TinyLsmError> {
        if self.memtable.is_empty() {
            return Ok(false);
        }

        let directory = self.directory.as_deref().ok_or_else(|| {
            TinyLsmError::InvalidState("flush requires an engine opened from a directory".into())
        })?;
        let generation = next_generation(&self.runs)?;
        let entries = self.memtable.snapshot_entries();
        let run = SortedRun::write(directory, generation, &entries)?;

        self.runs.push(run);
        self.memtable.clear();
        Ok(true)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        let keys: BTreeSet<&Key> = self
            .memtable
            .keys()
            .chain(
                self.runs
                    .iter()
                    .flat_map(|run| run.entries.iter().map(|(key, _)| key)),
            )
            .collect();

        keys.into_iter()
            .filter(|key| self.get(Key::from(key.as_bytes())).is_some())
            .count()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
