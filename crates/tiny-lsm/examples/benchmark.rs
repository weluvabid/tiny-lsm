use std::{
    error::Error,
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use tiny_lsm::Engine;

const DEFAULT_ITEMS: usize = 10_000;
const DEFAULT_SAMPLES: usize = 15;
const LAYERS: usize = 8;
static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tiny-lsm-bench-{}-{label}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create benchmark directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let items = std::env::var("BENCH_ITEMS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_ITEMS)
        .max(LAYERS);
    let samples = std::env::var("BENCH_SAMPLES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_SAMPLES)
        .max(1);
    let keys: Vec<String> = (0..items).map(|i| format!("key-{i:08}")).collect();
    let values: Vec<String> = (0..items).map(|i| format!("value-{i:08}")).collect();

    println!("TinyLSM benchmark | items={items} | samples={samples} | layers={LAYERS}");
    println!(
        "{:<24} {:>14} {:>14} {:>14}",
        "workload", "median ns/item", "p95 ns/item", "items/s"
    );

    measure("memtable_put", samples, items, || {
        let mut engine = Engine::new();
        let start = Instant::now();
        for (key, value) in keys.iter().zip(&values) {
            engine.put(key.as_str(), value.as_str());
        }
        let elapsed = start.elapsed();
        black_box(engine);
        elapsed
    });

    let mut memory_engine = Engine::new();
    for (key, value) in keys.iter().zip(&values) {
        memory_engine.put(key.as_str(), value.as_str());
    }
    measure("memtable_get", samples, items, || {
        let start = Instant::now();
        for key in &keys {
            black_box(memory_engine.get(key.as_str()));
        }
        start.elapsed()
    });

    measure("flush", samples, items, || {
        let directory = TempDir::new("flush");
        let mut engine = Engine::open(directory.path()).expect("open flush store");
        for (key, value) in keys.iter().zip(&values) {
            engine.put(key.as_str(), value.as_str());
        }
        let start = Instant::now();
        black_box(engine.flush().expect("flush run"));
        start.elapsed()
    });

    measure("layered_get_8_runs", samples, items, || {
        let directory = TempDir::new("layered-get");
        let engine = build_layered_store(directory.path(), &keys, &values);
        let start = Instant::now();
        for key in &keys {
            black_box(engine.get(key.as_str()));
        }
        start.elapsed()
    });

    measure("compaction_8_runs", samples, items, || {
        let directory = TempDir::new("compaction");
        let mut engine = build_overlapping_store(directory.path(), &keys);
        let start = Instant::now();
        black_box(engine.compact().expect("compact runs"));
        start.elapsed()
    });

    measure("reopen_8_runs", samples, items, || {
        let directory = TempDir::new("reopen");
        let engine = build_layered_store(directory.path(), &keys, &values);
        drop(engine);
        let start = Instant::now();
        let reopened = Engine::open(directory.path()).expect("reopen store");
        let elapsed = start.elapsed();
        black_box(reopened);
        elapsed
    });

    Ok(())
}

fn build_layered_store(directory: &Path, keys: &[String], values: &[String]) -> Engine {
    let mut engine = Engine::open(directory).expect("open layered store");
    let items = keys.len();
    for layer in 0..LAYERS {
        let start = layer * items / LAYERS;
        let end = (layer + 1) * items / LAYERS;
        for index in start..end {
            engine.put(keys[index].as_str(), values[index].as_str());
        }
        engine.flush().expect("flush benchmark layer");
    }
    engine
}

fn build_overlapping_store(directory: &Path, keys: &[String]) -> Engine {
    let mut engine = Engine::open(directory).expect("open overlapping store");
    for layer in 0..LAYERS {
        for (index, key) in keys.iter().enumerate() {
            let value = format!("value-{layer}-{index:08}");
            engine.put(key.as_str(), value);
        }
        if layer == LAYERS - 1 {
            for key in keys.iter().step_by(10) {
                black_box(engine.delete(key.as_str()));
            }
        }
        engine.flush().expect("flush overlapping layer");
    }
    engine
}

fn measure<F>(name: &str, samples: usize, items: usize, mut workload: F)
where
    F: FnMut() -> Duration,
{
    let mut ns_per_item: Vec<f64> = (0..samples)
        .map(|_| workload().as_nanos() as f64 / items as f64)
        .collect();
    ns_per_item.sort_by(f64::total_cmp);

    let median = ns_per_item[ns_per_item.len() / 2];
    let p95_index = (ns_per_item.len() * 95).div_ceil(100).saturating_sub(1);
    let p95 = ns_per_item[p95_index];
    let items_per_second = 1_000_000_000.0 / median.max(1.0);

    println!("{name:<24} {median:>14.1} {p95:>14.1} {items_per_second:>14.0}");
}
