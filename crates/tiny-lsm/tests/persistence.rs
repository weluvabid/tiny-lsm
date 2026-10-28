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
        let path = std::env::temp_dir().join(format!("tiny-lsm-test-{}-{id}", std::process::id()));
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
fn flush_publishes_sorted_run_and_reopens() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");

    engine.put("z", "last");
    engine.put("a", "first");
    engine.put("m", "");
    assert!(engine.flush().expect("flush"));
    assert!(!engine.flush().expect("empty flush"));
    assert!(directory.path().join("run-1.tlsm").is_file());
    assert!(!engine.is_empty());
    drop(engine);

    let engine = Engine::open(directory.path()).expect("reopen store");
    assert_eq!(
        engine.get("a").map(|value| value.as_bytes()),
        Some(&b"first"[..])
    );
    assert_eq!(
        engine.get("m").map(|value| value.as_bytes()),
        Some(&b""[..])
    );
    assert_eq!(
        engine.get("z").map(|value| value.as_bytes()),
        Some(&b"last"[..])
    );
    assert_eq!(engine.len(), 3);
}

#[test]
fn later_runs_and_tombstones_survive_reopen() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");

    engine.put("key", "old");
    engine.put("deleted", "old");
    engine.flush().expect("first flush");

    engine.put("key", "new");
    assert!(engine.delete("deleted"));
    engine.flush().expect("second flush");
    drop(engine);

    let engine = Engine::open(directory.path()).expect("reopen store");
    assert_eq!(
        engine.get("key").map(|value| value.as_bytes()),
        Some(&b"new"[..])
    );
    assert_eq!(engine.get("deleted"), None);
    assert_eq!(engine.len(), 1);
}

#[test]
fn failed_flush_keeps_memtable_contents() {
    let mut engine = Engine::new();
    engine.put("key", "value");

    assert!(engine.flush().is_err());
    assert_eq!(
        engine.get("key").map(|value| value.as_bytes()),
        Some(&b"value"[..])
    );
    assert_eq!(engine.len(), 1);
}

#[test]
fn malformed_run_is_rejected() {
    let directory = TestDir::new();
    fs::write(directory.path().join("run-1.tlsm"), b"not a run").expect("write invalid run");

    assert!(Engine::open(directory.path()).is_err());
}

#[test]
fn flush_persists_tombstones_and_empty_values_distinctly() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");

    engine.put("empty", "");
    engine.put("removed", "value");
    engine.flush().expect("first flush");
    assert!(engine.delete("removed"));
    engine.flush().expect("second flush");
    drop(engine);

    let engine = Engine::open(directory.path()).expect("reopen store");
    assert_eq!(
        engine.get("empty").map(|value| value.as_bytes()),
        Some(&b""[..])
    );
    assert_eq!(engine.get("removed"), None);
    assert_eq!(engine.len(), 1);
}
