use std::{cmp::Reverse, collections::BinaryHeap};

use crate::{Key, RecordValue, run::SortedRun};

pub(crate) fn merge_runs(runs: &[SortedRun]) -> Vec<(Key, RecordValue)> {
    let mut heap = BinaryHeap::new();

    for (run_index, run) in runs.iter().enumerate() {
        if let Some((key, _)) = run.entries.first() {
            heap.push(Reverse((key.clone(), run_index, 0_usize)));
        }
    }

    let mut merged = Vec::new();

    while let Some(Reverse((key, run_index, record_index))) = heap.pop() {
        let mut newest = (run_index, record_index);
        push_next(&mut heap, runs, run_index, record_index);

        while heap
            .peek()
            .is_some_and(|Reverse((next_key, _, _))| next_key == &key)
        {
            let Reverse((_, next_run, next_record)) = heap.pop().expect("peeked entry exists");
            if next_run > newest.0 {
                newest = (next_run, next_record);
            }
            push_next(&mut heap, runs, next_run, next_record);
        }

        merged.push((key, runs[newest.0].entries[newest.1].1.clone()));
    }

    merged
}

fn push_next(
    heap: &mut BinaryHeap<Reverse<(Key, usize, usize)>>,
    runs: &[SortedRun],
    run_index: usize,
    record_index: usize,
) {
    let next_index = record_index + 1;
    if let Some((key, _)) = runs[run_index].entries.get(next_index) {
        heap.push(Reverse((key.clone(), run_index, next_index)));
    }
}
