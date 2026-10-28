use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use tiny_lsm::{Engine, RecordValue};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("tiny-lsm-inspect-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn inspect_reports_ordered_memtable_and_runs() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");

    engine.put("z", "last");
    engine.put("a", "first");
    engine.put("gone", "old");
    assert!(engine.flush().expect("first flush"));

    engine.put("z", "new");
    assert!(engine.delete("gone"));
    engine.put("m", "middle");

    let snapshot = engine.inspect();
    assert_eq!(snapshot.runs.len(), 1);
    assert_eq!(snapshot.runs[0].generation.get(), 1);
    assert_eq!(snapshot.runs[0].id.get(), 1);
    assert_eq!(snapshot.runs[0].record_count(), 3);
    assert_eq!(snapshot.runs[0].tombstone_count, 0);
    assert_eq!(
        snapshot.runs[0]
            .records
            .iter()
            .map(|record| record.key.as_bytes())
            .collect::<Vec<_>>(),
        vec![&b"a"[..], &b"gone"[..], &b"z"[..]]
    );

    assert_eq!(snapshot.memtable_records.len(), 3);
    assert_eq!(snapshot.memtable_tombstone_count(), 1);
    assert_eq!(snapshot.memtable_records[0].key.as_bytes(), b"gone");
    assert!(matches!(
        snapshot.memtable_records[0].value,
        RecordValue::Tombstone
    ));
    assert_eq!(snapshot.memtable_records[1].key.as_bytes(), b"m");
    assert_eq!(snapshot.memtable_records[2].key.as_bytes(), b"z");
}

#[test]
fn inspect_is_stable_and_reopened_runs_are_ordered_by_generation() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");

    engine.put("key", "one");
    engine.flush().expect("first flush");
    engine.put("key", "two");
    engine.put("other", "value");
    engine.flush().expect("second flush");
    drop(engine);

    let engine = Engine::open(directory.path()).expect("reopen store");
    let first = engine.inspect();
    let second = engine.inspect();
    assert_eq!(first, second);
    assert_eq!(
        first
            .runs
            .iter()
            .map(|run| run.generation.get())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(first.runs[0].record_count(), 1);
    assert_eq!(first.runs[1].record_count(), 2);
    assert_eq!(
        engine.get("key").map(|value| value.as_bytes()),
        Some(&b"two"[..])
    );
}

#[test]
fn memtable_snapshot_does_not_mutate_engine_state() {
    let mut engine = Engine::new();
    engine.put("b", "2");
    engine.put("a", "1");

    let snapshot = engine.inspect();
    assert_eq!(snapshot.memtable_records.len(), 2);
    assert_eq!(snapshot.memtable_records[0].key.as_bytes(), b"a");

    engine.put("a", "updated");
    assert_eq!(
        snapshot.memtable_records[0].value,
        RecordValue::Value("1".into())
    );
    assert_eq!(
        engine.get("a").map(|value| value.as_bytes()),
        Some(&b"updated"[..])
    );
}
