use super::super::{model, workers};
use super::support::make_work;
use std::collections::VecDeque;
use std::sync::mpsc::channel;

#[test]
fn drain_batch_caps_at_limit() {
    let mut queue = VecDeque::new();
    queue.push_back(make_work("a"));
    queue.push_back(make_work("b"));
    queue.push_back(make_work("c"));

    let batch = workers::drain_batch(&mut queue, 2);
    assert_eq!(batch.len(), 2);
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.front().unwrap().sample_ids[0], "c");
}

#[test]
fn collect_results_limits_error_list() {
    let (tx, rx) = channel();
    tx.send(Err("err-1".to_string())).unwrap();
    tx.send(Ok(model::EmbeddingComputation {
        content_hash: "hash-a".to_string(),
        sample_ids: vec!["a".to_string()],
        embedding: vec![0.0_f32; 2],
        aspect_descriptors: model::AspectDescriptorData {
            vec_blob: vec![0; wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM * 4],
            valid_mask: 0,
        },
        created_at: 1,
    }))
    .unwrap();
    tx.send(Err("err-2".to_string())).unwrap();
    tx.send(Err("err-3".to_string())).unwrap();
    tx.send(Err("err-4".to_string())).unwrap();
    drop(tx);

    let (results, errors) = workers::collect_results(rx);
    assert_eq!(results.len(), 1);
    assert_eq!(errors.len(), 3);
    assert_eq!(errors[0], "err-1");
    assert_eq!(errors[2], "err-3");
}
