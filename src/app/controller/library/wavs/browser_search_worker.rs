use crate::app::controller::jobs::{SearchJob, SearchResult};
use crate::app::state::{SampleBrowserSort, TriageFlagFilter, VisibleRows};
use crate::hotpath_telemetry;
use crate::sample_sources::Rating;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use tracing::warn;

pub(super) use super::search_scoring::{
    ScoreCandidateResult, promote_exact_query_score_cache_entry,
    reusable_prefix_query_score_cache_entry, score_query_candidates, store_query_score_cache_entry,
};

/// Source/revision-scoped search worker cache types.
mod cache;
/// Search execution pipeline and filter/sort helper routines.
mod pipeline;
/// Search queue lifecycle and hot-path telemetry utilities.
mod queue;
/// Shared hot-path telemetry counters for queue and worker stages.
mod telemetry;

pub(crate) use self::queue::{SearchJobSender, SearchWorkerHandle, spawn_search_worker};
#[cfg(test)]
use self::{cache::*, pipeline::*, queue::*};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_sources::SourceId;
    use crate::sample_sources::WavEntry;
    use std::collections::BTreeSet;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn test_compact_search_entry() {
        let entries = vec![
            WavEntry {
                relative_path: std::path::PathBuf::from("kits/drums/kick.wav"),
                file_size: 100,
                modified_ns: 1,
                content_hash: None,
                tag: Rating::NEUTRAL,
                looped: false,
                missing: false,
                last_played_at: None,
            },
            WavEntry {
                relative_path: std::path::PathBuf::from("kits/drums/snare.wav"),
                file_size: 110,
                modified_ns: 2,
                content_hash: None,
                tag: Rating::NEUTRAL,
                looped: false,
                missing: false,
                last_played_at: None,
            },
        ];

        let compacts: Vec<CompactSearchEntry> = entries
            .into_iter()
            .map(|e| {
                let relative_path = e.relative_path.to_string_lossy().to_string();
                let display_label = crate::app::view_model::sample_display_label(&e.relative_path);
                CompactSearchEntry {
                    display_label: display_label.into_boxed_str(),
                    relative_path: relative_path.into_boxed_str(),
                    tag: e.tag,
                    last_played_at: e.last_played_at,
                }
            })
            .collect();

        assert_eq!(compacts.len(), 2);
        assert_eq!(compacts[0].display_label.as_ref(), "kick");
        assert_eq!(compacts[1].display_label.as_ref(), "snare");
        assert_eq!(compacts[0].relative_path.as_ref(), "kits/drums/kick.wav");
    }

    #[test]
    fn latest_search_job_replaces_pending() {
        let queue = Arc::new(SearchJobQueue::new());
        let sender = SearchJobSender {
            queue: Arc::clone(&queue),
        };

        let first = make_search_job("first", "one");
        let second = make_search_job("second", "two");

        sender.send(first);
        sender.send(second);

        let pending = queue.try_take().expect("expected pending search job");
        assert_eq!(pending.job.query, "second");
        assert!(queue.try_take().is_none());
    }

    #[test]
    fn newest_send_invalidates_inflight_generation() {
        let queue = Arc::new(SearchJobQueue::new());
        queue.send(make_search_job("first", "root"));
        let inflight = queue
            .take_blocking()
            .expect("expected first queued search job");
        assert!(queue.is_generation_current(inflight.generation));

        queue.send(make_search_job("second", "root"));
        assert!(!queue.is_generation_current(inflight.generation));
    }

    #[test]
    fn search_queue_recovers_from_poison() {
        let queue = Arc::new(SearchJobQueue::new());
        let queue_for_panic = Arc::clone(&queue);
        let _ = std::panic::catch_unwind(move || {
            let _guard = queue_for_panic.state.lock().expect("queue lock failed");
            panic!("poison search job queue");
        });

        let (tx, rx) = mpsc::channel();
        let queue_for_worker = Arc::clone(&queue);
        let handle = thread::spawn(move || {
            let job = queue_for_worker
                .take_blocking()
                .expect("expected job after recovery");
            tx.send(job.job.query).expect("send result");
        });

        queue.send(make_search_job("recovered", "root"));

        let received = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("search job never received");
        assert_eq!(received, "recovered");
        handle.join().expect("worker thread panicked");
    }

    #[test]
    fn search_queue_shutdown_unblocks() {
        let queue = Arc::new(SearchJobQueue::new());
        let (tx, rx) = mpsc::channel();
        let queue_for_worker = Arc::clone(&queue);
        let handle = thread::spawn(move || {
            let result = queue_for_worker.take_blocking();
            tx.send(result.is_none()).expect("send shutdown");
        });
        queue.shutdown();
        let shutdown = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("shutdown result");
        assert!(shutdown);
        handle.join().expect("worker thread panicked");
    }

    #[test]
    /// Revision-keyed triage cache should reuse cached partitions without changing outputs.
    fn triage_partitions_are_cached_by_revision() {
        let mut cache = SearchWorkerCache {
            entries: Some(vec![
                compact_entry("a.wav", Rating::TRASH_3),
                compact_entry("b.wav", Rating::NEUTRAL),
                compact_entry("c.wav", Rating::KEEP_3),
            ]),
            ..SearchWorkerCache::default()
        };
        let queue = SearchJobQueue::new();
        queue.send(make_search_job("query", "root"));
        let generation = queue
            .take_blocking()
            .expect("expected queued search job generation")
            .generation;
        let first = triage_partitions_for_revision(&mut cache, "source", 7, &queue, generation)
            .expect("expected triage partitions");
        let second = triage_partitions_for_revision(&mut cache, "source", 7, &queue, generation)
            .expect("expected triage partitions");

        assert_eq!(first.0.as_ref(), &[0]);
        assert_eq!(first.1.as_ref(), &[1]);
        assert_eq!(first.2.as_ref(), &[2]);
        assert!(Arc::ptr_eq(&first.0, &second.0));
        assert!(Arc::ptr_eq(&first.1, &second.1));
        assert!(Arc::ptr_eq(&first.2, &second.2));
        assert!(cache.triage_cache.is_some());
    }

    #[test]
    /// Worker folder acceptance cache must invalidate when root-mode semantics change.
    fn folder_filter_hash_changes_with_root_mode() {
        let mut all = make_search_job("q", "root");
        all.folder_selection = Some(BTreeSet::from([PathBuf::from("")]));
        all.root_mode = crate::app::state::RootFolderFilterMode::AllDescendants;

        let mut root_only = make_search_job("q", "root");
        root_only.folder_selection = Some(BTreeSet::from([PathBuf::from("")]));
        root_only.root_mode = crate::app::state::RootFolderFilterMode::RootOnly;

        assert_ne!(
            folder_filter_hash_for_job(&all),
            folder_filter_hash_for_job(&root_only)
        );
    }

    /// Build a compact worker entry for unit tests.
    fn compact_entry(relative_path: &str, tag: Rating) -> CompactSearchEntry {
        CompactSearchEntry {
            display_label: relative_path.into(),
            relative_path: relative_path.into(),
            tag,
            last_played_at: None,
        }
    }

    fn make_search_job(query: &str, root: &str) -> SearchJob {
        SearchJob {
            request_id: 1,
            source_id: SourceId::new(),
            source_root: std::path::PathBuf::from(root),
            query: query.to_string(),
            filter: TriageFlagFilter::All,
            rating_filter: BTreeSet::new(),
            sort: SampleBrowserSort::ListOrder,
            similar_query: None,
            folder_selection: None,
            folder_negated: None,
            root_mode: crate::app::state::RootFolderFilterMode::AllDescendants,
        }
    }
}
