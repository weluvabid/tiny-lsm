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
            std::env::temp_dir().join(format!("tiny-lsm-compact-{}-{id}", std::process::id()));
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
fn compaction_preserves_reads_and_reopens_as_one_run() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");

    engine.put("a", "old");
    engine.put("b", "kept");
    engine.put("deleted", "old");
    engine.flush().expect("first flush");

    engine.put("a", "new");
    engine.delete("deleted");
    engine.put("c", "");
    engine.flush().expect("second flush");

    engine.put("b", "newest");
    engine.flush().expect("third flush");

    let before = ["a", "b", "c", "deleted", "missing"]
        .map(|key| engine.get(key).map(|value| value.as_bytes().to_vec()));
    assert_eq!(engine.inspect().runs.len(), 3);

    assert!(engine.compact().expect("compact"));
    assert!(!engine.compact().expect("already compacted"));
    let after = ["a", "b", "c", "deleted", "missing"]
        .map(|key| engine.get(key).map(|value| value.as_bytes().to_vec()));

    assert_eq!(before, after);
    assert_eq!(engine.inspect().runs.len(), 1);
    let run = &engine.inspect().runs[0];
    assert_eq!(run.generation.get(), 4);
    assert_eq!(run.record_count(), 4);
    assert_eq!(run.tombstone_count, 1);
    assert!(matches!(
        run.records
            .iter()
            .find(|record| record.key.as_bytes() == b"deleted")
            .unwrap()
            .value,
        RecordValue::Tombstone
    ));

    drop(engine);
    let engine = Engine::open(directory.path()).expect("reopen store");
    assert_eq!(engine.inspect().runs.len(), 1);
    assert_eq!(
        engine.get("a").map(|value| value.as_bytes()),
        Some(&b"new"[..])
    );
    assert_eq!(
        engine.get("b").map(|value| value.as_bytes()),
        Some(&b"newest"[..])
    );
    assert_eq!(
        engine.get("c").map(|value| value.as_bytes()),
        Some(&b""[..])
    );
    assert_eq!(engine.get("deleted"), None);
    assert!(!directory.path().join("run-1.tlsm").exists());
    assert!(!directory.path().join("run-2.tlsm").exists());
    assert!(!directory.path().join("run-3.tlsm").exists());
    assert!(directory.path().join("run-4.tlsm").is_file());
}

#[test]
fn failed_compaction_preserves_existing_runs_and_temp_file() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");

    engine.put("key", "one");
    engine.flush().expect("first flush");
    engine.put("key", "two");
    engine.flush().expect("second flush");

    let temp_path = directory.path().join(".run-3.tmp");
    fs::write(&temp_path, b"owned by test").expect("create conflicting temp file");

    assert!(engine.compact().is_err());
    assert_eq!(
        engine.get("key").map(|value| value.as_bytes()),
        Some(&b"two"[..])
    );
    assert_eq!(engine.inspect().runs.len(), 2);
    assert!(directory.path().join("run-1.tlsm").is_file());
    assert!(directory.path().join("run-2.tlsm").is_file());
    assert_eq!(
        fs::read(&temp_path).expect("read temp file"),
        b"owned by test"
    );
}

#[test]
fn compacting_tombstone_keeps_older_value_deleted() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");

    engine.put("key", "old");
    engine.flush().expect("first flush");
    assert!(engine.delete("key"));
    engine.flush().expect("second flush");
    assert_eq!(engine.get("key"), None);

    engine.compact().expect("compact");
    assert_eq!(engine.get("key"), None);
    assert_eq!(engine.len(), 0);
}
