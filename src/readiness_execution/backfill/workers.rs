//! Worker-thread execution for embedding backfill decode and embedding computation.

use super::super::errors::ErrorCollector;
use super::super::support::now_epoch_seconds;
use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::Receiver,
    mpsc::channel,
};

use super::model::{AspectDescriptorData, EmbeddingComputation, EmbeddingResult, EmbeddingWork};

pub(super) fn run_embedding_workers(
    work: Vec<EmbeddingWork>,
    analysis_sample_rate: u32,
    cancel: Option<&AtomicBool>,
    worker_limit: Option<usize>,
) -> (Vec<EmbeddingComputation>, Vec<String>) {
    if work.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let worker_count = worker_count_for(&work, worker_limit);
    let queue = Arc::new(Mutex::new(VecDeque::from(work)));
    let (tx, rx) = channel();

    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let queue = Arc::clone(&queue);
            let tx = tx.clone();
            scope.spawn(move || run_worker_loop(queue, tx, analysis_sample_rate, cancel));
        }
        drop(tx);
    });

    collect_results(rx)
}

pub(super) fn expand_computations(computed: Vec<EmbeddingComputation>) -> Vec<EmbeddingResult> {
    let mut results = Vec::new();
    for item in computed {
        for sample_id in item.sample_ids {
            results.push(EmbeddingResult {
                sample_id,
                content_hash: item.content_hash.clone(),
                embedding: item.embedding.clone(),
                aspect_descriptors: item.aspect_descriptors.clone(),
                created_at: item.created_at,
            });
        }
    }
    results
}

pub(super) fn drain_batch(
    queue: &mut VecDeque<EmbeddingWork>,
    batch_max: usize,
) -> Vec<EmbeddingWork> {
    let mut batch = Vec::with_capacity(batch_max);
    for _ in 0..batch_max {
        let Some(work) = queue.pop_front() else {
            break;
        };
        batch.push(work);
    }
    batch
}

pub(super) fn collect_results(
    rx: Receiver<Result<EmbeddingComputation, String>>,
) -> (Vec<EmbeddingComputation>, Vec<String>) {
    let mut results = Vec::new();
    let mut errors = ErrorCollector::new(3);
    while let Ok(result) = rx.recv() {
        match result {
            Ok(result) => results.push(result),
            Err(err) => errors.push(err),
        }
    }
    (results, errors.into_vec())
}

fn worker_count_for(work: &[EmbeddingWork], worker_limit: Option<usize>) -> usize {
    let available = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    bounded_worker_count(available, work.len(), worker_limit)
}

fn bounded_worker_count(available: usize, work_items: usize, worker_limit: Option<usize>) -> usize {
    worker_limit.unwrap_or(available).max(1).min(work_items)
}

#[cfg(test)]
pub(super) fn bounded_worker_count_for_test(work_items: usize, worker_limit: usize) -> usize {
    bounded_worker_count(usize::MAX, work_items, Some(worker_limit))
}

fn run_worker_loop(
    queue: Arc<Mutex<VecDeque<EmbeddingWork>>>,
    tx: std::sync::mpsc::Sender<Result<EmbeddingComputation, String>>,
    analysis_sample_rate: u32,
    cancel: Option<&AtomicBool>,
) {
    let batch_max = wavecrate_analysis::similarity::SIMILARITY_BATCH_MAX;
    loop {
        if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
            break;
        }
        let batch = next_batch(&queue, batch_max);
        if batch.is_empty() {
            break;
        }
        for work in batch {
            if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
                return;
            }
            let result = compute_embedding(work, analysis_sample_rate);
            let _ = tx.send(result);
        }
    }
}

fn next_batch(queue: &Arc<Mutex<VecDeque<EmbeddingWork>>>, batch_max: usize) -> Vec<EmbeddingWork> {
    let mut guard = match queue.lock() {
        Ok(guard) => guard,
        Err(_) => return Vec::new(),
    };
    drain_batch(&mut guard, batch_max)
}

fn compute_embedding(
    work: EmbeddingWork,
    analysis_sample_rate: u32,
) -> Result<EmbeddingComputation, String> {
    let path = work.absolute_path.display().to_string();
    let decoded = wavecrate_analysis::decode_for_analysis_with_rate(
        &work.absolute_path,
        analysis_sample_rate,
    )
    .map_err(|err| format!("Decode failed for {path}: {err}"))?;
    let features = wavecrate_analysis::compute_feature_vector_v1_for_mono_samples(
        &decoded.mono,
        decoded.sample_rate_used,
    )
    .map_err(|err| format!("Feature extraction failed for {path}: {err}"))?;
    let embedding = wavecrate_analysis::similarity::embedding_from_features(&features)
        .map_err(|err| format!("Embedding build failed for {path}: {err}"))?;
    let aspect_descriptors =
        wavecrate_analysis::aspects::aspect_descriptors_from_features_v1(&features)
            .map_err(|err| format!("Aspect descriptor build failed for {path}: {err}"))?;
    Ok(EmbeddingComputation {
        content_hash: work.content_hash,
        sample_ids: work.sample_ids,
        embedding,
        aspect_descriptors: AspectDescriptorData {
            vec_blob: wavecrate_analysis::vector::encode_f32_le_blob(aspect_descriptors.packed()),
            valid_mask: aspect_descriptors.valid_mask(),
        },
        created_at: now_epoch_seconds(),
    })
}
