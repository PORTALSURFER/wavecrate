use crate::app::controller::AppController;
use crate::app::controller::library::analysis_jobs;
use crate::app::state::MapSimilarityPrepStatus;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use crate::sample_sources::SampleSource;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SimilarityPrepFailureCounts {
    failed_count: usize,
    unsupported_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SimilarityPrepFacts {
    scan_completed_at: Option<i64>,
    prep_completed_at: Option<i64>,
    has_embeddings: bool,
    has_aspects: bool,
    has_layout: bool,
    failures: Option<SimilarityPrepFailureCounts>,
}

fn failure_counts(failures: &HashMap<PathBuf, String>) -> SimilarityPrepFailureCounts {
    let unsupported_count = failures
        .values()
        .filter(|message| message.to_ascii_lowercase().contains("unsupported"))
        .count();
    SimilarityPrepFailureCounts {
        failed_count: failures.len(),
        unsupported_count,
    }
}

fn resolve_similarity_prep_status(facts: SimilarityPrepFacts) -> MapSimilarityPrepStatus {
    let prep_up_to_date = facts.scan_completed_at.is_some()
        && facts.scan_completed_at == facts.prep_completed_at
        && facts.has_embeddings
        && facts.has_aspects
        && facts.has_layout;
    if prep_up_to_date {
        return MapSimilarityPrepStatus::UpToDate;
    }
    if let Some(failures) = facts.failures
        && failures.failed_count > 0
    {
        return MapSimilarityPrepStatus::Blocked {
            failed_count: failures.failed_count,
            unsupported_count: failures.unsupported_count,
        };
    }
    if facts.scan_completed_at.is_some() && facts.scan_completed_at != facts.prep_completed_at {
        return MapSimilarityPrepStatus::Outdated;
    }
    MapSimilarityPrepStatus::MissingArtifacts {
        missing_embeddings: !facts.has_embeddings,
        missing_aspects: !facts.has_aspects,
        missing_layout: !facts.has_layout,
    }
}

fn action_outcome(status: &MapSimilarityPrepStatus) -> &'static str {
    match status {
        MapSimilarityPrepStatus::UpToDate => "up_to_date",
        MapSimilarityPrepStatus::Outdated => "outdated",
        MapSimilarityPrepStatus::Blocked { .. } => "blocked_failed_rows",
        MapSimilarityPrepStatus::MissingArtifacts {
            missing_embeddings,
            missing_aspects,
            missing_layout,
        } => missing_artifacts_outcome(*missing_embeddings, *missing_aspects, *missing_layout),
    }
}

fn missing_artifacts_outcome(
    missing_embeddings: bool,
    missing_aspects: bool,
    missing_layout: bool,
) -> &'static str {
    match (missing_embeddings, missing_aspects, missing_layout) {
        (true, true, true) => "missing_embeddings_aspects_and_layout",
        (true, true, false) => "missing_embeddings_and_aspects",
        (true, false, true) => "missing_embeddings_and_layout",
        (true, false, false) => "missing_embeddings",
        (false, true, true) => "missing_aspects_and_layout",
        (false, true, false) => "missing_aspects",
        (false, false, true) => "missing_layout",
        (false, false, false) => "missing_artifacts",
    }
}

impl AppController {
    pub(crate) fn refresh_selected_source_similarity_prep_status(&mut self) {
        let Some(source) = self.current_source() else {
            if self.ui.map.similarity_prep_status.take().is_some() {
                self.mark_map_dataset_projection_revision_dirty();
                self.mark_map_query_projection_revision_dirty();
            }
            return;
        };
        let next = self.resolve_selected_source_similarity_prep_status(&source);
        if self.ui.map.similarity_prep_status.as_ref() == Some(&next) {
            return;
        }
        self.ui.map.similarity_prep_status = Some(next.clone());
        self.mark_map_dataset_projection_revision_dirty();
        self.mark_map_query_projection_revision_dirty();
        emit_action_debug_event(ActionDebugEvent {
            action: "similarity_prep.status_resolved",
            pane: Some("map"),
            source: Some(source.id.as_str()),
            outcome: action_outcome(&next),
            elapsed: Duration::ZERO,
            error: None,
        });
    }

    fn resolve_selected_source_similarity_prep_status(
        &self,
        source: &SampleSource,
    ) -> MapSimilarityPrepStatus {
        let failures = self
            .ui_cache
            .browser
            .analysis_failures
            .get(&source.id)
            .map(failure_counts)
            .or_else(|| {
                analysis_jobs::failed_samples_for_source(source)
                    .ok()
                    .map(|rows| failure_counts(&rows))
            });
        resolve_similarity_prep_status(SimilarityPrepFacts {
            scan_completed_at: super::db::read_source_scan_timestamp(source),
            prep_completed_at: super::db::read_source_prep_timestamp(source),
            has_embeddings: super::db::source_has_embeddings(source),
            has_aspects: super::db::source_has_aspect_descriptors(source),
            has_layout: super::db::source_has_layout(source, self.ui.map.umap_version.as_str()),
            failures,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_blocked_before_outdated_when_failures_exist() {
        let status = resolve_similarity_prep_status(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(10),
            has_embeddings: false,
            has_aspects: false,
            has_layout: false,
            failures: Some(SimilarityPrepFailureCounts {
                failed_count: 3,
                unsupported_count: 1,
            }),
        });

        assert_eq!(
            status,
            MapSimilarityPrepStatus::Blocked {
                failed_count: 3,
                unsupported_count: 1,
            }
        );
    }

    #[test]
    fn resolves_outdated_when_scan_is_newer_than_prep() {
        let status = resolve_similarity_prep_status(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(10),
            has_embeddings: true,
            has_aspects: true,
            has_layout: true,
            failures: None,
        });

        assert_eq!(status, MapSimilarityPrepStatus::Outdated);
    }

    #[test]
    fn resolves_missing_artifacts_when_embeddings_or_layout_are_absent() {
        let status = resolve_similarity_prep_status(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(20),
            has_embeddings: true,
            has_aspects: false,
            has_layout: false,
            failures: None,
        });

        assert_eq!(
            status,
            MapSimilarityPrepStatus::MissingArtifacts {
                missing_embeddings: false,
                missing_aspects: true,
                missing_layout: true,
            }
        );
    }

    #[test]
    fn resolves_up_to_date_when_timestamps_and_artifacts_match() {
        let status = resolve_similarity_prep_status(SimilarityPrepFacts {
            scan_completed_at: Some(20),
            prep_completed_at: Some(20),
            has_embeddings: true,
            has_aspects: true,
            has_layout: true,
            failures: Some(SimilarityPrepFailureCounts {
                failed_count: 0,
                unsupported_count: 0,
            }),
        });

        assert_eq!(status, MapSimilarityPrepStatus::UpToDate);
    }

    #[test]
    fn counts_unsupported_failures_separately() {
        let failures = HashMap::from([
            (
                PathBuf::from("a.wav"),
                String::from("Timed out while running"),
            ),
            (
                PathBuf::from("b.wav"),
                String::from("Unsupported codec for decode"),
            ),
        ]);

        assert_eq!(
            failure_counts(&failures),
            SimilarityPrepFailureCounts {
                failed_count: 2,
                unsupported_count: 1,
            }
        );
    }
}
