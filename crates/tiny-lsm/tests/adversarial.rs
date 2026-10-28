use std::{
    collections::BTreeMap,
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
        let path = std::env::temp_dir().join(format!("tiny-lsm-model-{}-{id}", std::process::id()));
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

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn bytes(&mut self, max_len: usize) -> Vec<u8> {
        let len = (self.next() as usize) % (max_len + 1);
        (0..len).map(|_| self.next() as u8).collect()
    }
}

fn assert_matches(engine: &Engine, model: &BTreeMap<Vec<u8>, Vec<u8>>) {
    assert_eq!(engine.len(), model.len());
    assert_eq!(engine.is_empty(), model.is_empty());

    for (key, expected) in model {
        assert_eq!(
            engine.get(key.as_slice()).map(|value| value.as_bytes()),
            Some(expected.as_slice()),
            "value mismatch for key {key:?}"
        );
    }

    for key in [Vec::new(), vec![0], vec![0xff], b"missing".to_vec()] {
        assert_eq!(
            engine
                .get(key.as_slice())
                .map(|value| value.as_bytes().to_vec()),
            model.get(&key).cloned(),
            "lookup mismatch for key {key:?}"
        );
    }
}

#[test]
fn randomized_operations_match_reference_model_across_flush_compact_and_reopen() {
    let directory = TestDir::new();
    let mut engine = Engine::open(directory.path()).expect("open store");
    let mut model = BTreeMap::<Vec<u8>, Vec<u8>>::new();
    let mut rng = Rng(0x8d26_4f31_2a77_91c5);

    for step in 0..1_200 {
        let key = if step % 11 == 0 {
            Vec::new()
        } else {
            rng.bytes(8)
        };

        match rng.next() % 6 {
            0..=2 => {
                let value = rng.bytes(16);
                engine.put(key.clone(), value.clone());
                model.insert(key, value);
            }
            3 => {
                assert_eq!(engine.delete(key.clone()), model.remove(&key).is_some());
            }
            4 => {
                assert_eq!(
                    engine
                        .get(key.as_slice())
                        .map(|value| value.as_bytes().to_vec()),
                    model.get(&key).cloned()
                );
            }
            _ => {
                engine.put(key.clone(), Vec::<u8>::new());
                model.insert(key, Vec::new());
            }
        }

        if step % 19 == 0 {
            engine.flush().expect("flush");
        }
        if step % 73 == 0 {
            engine.flush().expect("flush before compaction");
            engine.compact().expect("compact");
        }
        if step % 101 == 0 {
            engine.flush().expect("flush before reopen");
            assert_matches(&engine, &model);
            drop(engine);
            engine = Engine::open(directory.path()).expect("reopen store");
        }

        assert_matches(&engine, &model);
    }

    engine.flush().expect("final flush");
    engine.compact().expect("final compaction");
    assert_matches(&engine, &model);
    drop(engine);

    let engine = Engine::open(directory.path()).expect("final reopen");
    assert_matches(&engine, &model);
}
