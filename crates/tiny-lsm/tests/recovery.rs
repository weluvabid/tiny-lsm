use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use tiny_lsm::Engine;

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("tiny-lsm-recovery-{}-{id}", std::process::id()));
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
fn open_removes_stale_temporary_runs_without_touching_published_runs() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");
    engine.put("key", "value");
    engine.flush().expect("flush run");
    drop(engine);

    let stale = directory.path().join(".run-2.tmp");
    fs::write(&stale, b"incomplete run").expect("write stale temporary run");
    fs::write(
        directory.path().join(".run-not-a-generation.tmp"),
        b"leave alone",
    )
    .expect("write unrelated file");

    let engine = Engine::open(directory.path()).expect("recover store");
    assert!(!stale.exists());
    assert!(directory.path().join("run-1.tlsm").is_file());
    assert_eq!(
        engine.get("key").map(|value| value.as_bytes()),
        Some(&b"value"[..])
    );
    assert_eq!(
        fs::read(directory.path().join(".run-not-a-generation.tmp")).unwrap(),
        b"leave alone"
    );
}

#[test]
fn reopen_recovers_if_compaction_published_but_old_run_cleanup_was_interrupted() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");
    engine.put("key", "old");
    engine.put("deleted", "old");
    engine.flush().expect("first flush");
    engine.put("key", "new");
    engine.delete("deleted");
    engine.flush().expect("second flush");
    let old_run_one = fs::read(directory.path().join("run-1.tlsm")).expect("read old run one");
    let old_run_two = fs::read(directory.path().join("run-2.tlsm")).expect("read old run two");
    engine.compact().expect("compact");
    drop(engine);

    // Restore pre-cleanup files to model a crash immediately after publication.
    fs::write(directory.path().join("run-1.tlsm"), old_run_one).expect("restore old run one");
    fs::write(directory.path().join("run-2.tlsm"), old_run_two).expect("restore old run two");

    let mut engine = Engine::open(directory.path()).expect("recover published compaction");
    assert_eq!(
        engine.get("key").map(|value| value.as_bytes()),
        Some(&b"new"[..])
    );
    assert_eq!(engine.get("deleted"), None);
    assert_eq!(engine.inspect().runs.len(), 3);

    engine.put("next", "value");
    engine.flush().expect("flush after recovery");
    assert!(directory.path().join("run-4.tlsm").is_file());
    drop(engine);

    let engine = Engine::open(directory.path()).expect("reopen after recovery flush");
    assert_eq!(
        engine.get("key").map(|value| value.as_bytes()),
        Some(&b"new"[..])
    );
    assert_eq!(engine.get("deleted"), None);
    assert_eq!(
        engine.get("next").map(|value| value.as_bytes()),
        Some(&b"value"[..])
    );
}

#[test]
fn open_rejects_non_canonical_run_filenames() {
    let directory = TestDir::new();
    fs::write(directory.path().join("run-01.tlsm"), b"not loaded").expect("write invalid run");

    assert!(Engine::open(directory.path()).is_err());
}
