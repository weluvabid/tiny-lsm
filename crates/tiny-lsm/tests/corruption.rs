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
            std::env::temp_dir().join(format!("tiny-lsm-corrupt-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write_run(&self, bytes: &[u8]) {
        fs::write(self.path().join("run-1.tlsm"), bytes).expect("write run fixture");
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn valid_empty_record_run() -> Vec<u8> {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");
    engine.put(Vec::<u8>::new(), Vec::<u8>::new());
    engine.flush().expect("flush fixture");
    fs::read(directory.path().join("run-1.tlsm")).expect("read fixture")
}

#[test]
fn every_truncated_prefix_of_a_valid_run_is_rejected() {
    let valid = valid_empty_record_run();
    let directory = TestDir::new();

    for length in 0..valid.len() {
        directory.write_run(&valid[..length]);
        assert!(
            Engine::open(directory.path()).is_err(),
            "accepted truncated run with {length} bytes"
        );
    }

    directory.write_run(&valid);
    assert!(Engine::open(directory.path()).is_ok());
}

#[test]
fn trailing_bytes_and_unknown_record_tags_are_rejected() {
    let valid = valid_empty_record_run();
    let directory = TestDir::new();

    let mut trailing = valid.clone();
    trailing.push(0x7f);
    directory.write_run(&trailing);
    assert!(Engine::open(directory.path()).is_err());

    let mut unknown_tag = valid;
    unknown_tag[26] = 2;
    directory.write_run(&unknown_tag);
    assert!(Engine::open(directory.path()).is_err());
}

#[test]
fn excessive_record_count_is_rejected_before_record_allocation() {
    let mut bytes = valid_empty_record_run();
    bytes[10..18].copy_from_slice(&(10_000_001_u64).to_le_bytes());

    let directory = TestDir::new();
    directory.write_run(&bytes);
    assert!(Engine::open(directory.path()).is_err());
}
