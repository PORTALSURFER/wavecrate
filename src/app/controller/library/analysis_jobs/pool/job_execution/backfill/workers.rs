//! Worker-thread execution for embedding backfill decode and embedding computation.

use crate::app::controller::library::analysis_jobs::pool::job_execution::errors::ErrorCollector;
use crate::app::controller::library::analysis_jobs::pool::job_execution::support::now_epoch_seconds;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc::Receiver, mpsc::channel};

use super::model::{EmbeddingComputation, EmbeddingResult, EmbeddingWork};

pub(super) fn run_embedding_workers(
    work: Vec<EmbeddingWork>,
    analysis_sample_rate: u32,
) -> (Vec<EmbeddingComputation>, Vec<String>) {
    if work.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let worker_count = worker_count_for(&work);
    let queue = Arc::new(Mutex::new(VecDeque::from(work)));
    let (tx, rx) = channel();

    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let queue = Arc::clone(&queue);
            let tx = tx.clone();
            scope.spawn(move || run_worker_loop(queue, tx, analysis_sample_rate));
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

fn worker_count_for(work: &[EmbeddingWork]) -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(work.len())
        .max(1)
}

fn run_worker_loop(
    queue: Arc<Mutex<VecDeque<EmbeddingWork>>>,
    tx: std::sync::mpsc::Sender<Result<EmbeddingComputation, String>>,
    analysis_sample_rate: u32,
) {
    let batch_max = crate::analysis::similarity::SIMILARITY_BATCH_MAX;
    loop {
        let batch = next_batch(&queue, batch_max);
        if batch.is_empty() {
            break;
        }
        for work in batch {
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
    let decoded = crate::analysis::audio::decode_for_analysis_with_rate(
        &work.absolute_path,
        analysis_sample_rate,
    )
    .map_err(|err| format!("Decode failed for {path}: {err}"))?;
    let features = crate::analysis::compute_feature_vector_v1_for_mono_samples(
        &decoded.mono,
        decoded.sample_rate_used,
    )
    .map_err(|err| format!("Feature extraction failed for {path}: {err}"))?;
    let embedding = crate::analysis::similarity::embedding_from_features(&features)
        .map_err(|err| format!("Embedding build failed for {path}: {err}"))?;
    Ok(EmbeddingComputation {
        content_hash: work.content_hash,
        sample_ids: work.sample_ids,
        embedding,
        created_at: now_epoch_seconds(),
    })
}
