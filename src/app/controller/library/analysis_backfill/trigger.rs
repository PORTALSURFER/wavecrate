use super::*;

impl AppController {
    pub(super) fn spawn_analysis_trigger(&mut self, trigger: AnalysisTrigger) {
        let source_id = trigger.source_id().clone();
        if self
            .runtime
            .source_lane
            .pending_remap
            .as_ref()
            .is_some_and(|pending| pending.source.id == source_id)
        {
            tracing::debug!(
                source_id = %source_id,
                "Skipping analysis enqueue while source remap is pending"
            );
            return;
        }
        let enqueue_guard = self.runtime.analysis.begin_source_enqueue(source_id);
        let tx = self.runtime.jobs.message_sender();
        std::thread::spawn(move || {
            let _enqueue_guard = enqueue_guard;
            match trigger {
                AnalysisTrigger::ChangedSamples {
                    source,
                    changed_samples,
                    announce,
                } => enqueue_changed_samples(tx, source, changed_samples, announce),
                AnalysisTrigger::UserRequestedReanalysis { source, action } => {
                    enqueue_manual_reanalysis(tx, source, action);
                }
            }
        });
    }
}

fn enqueue_changed_samples(
    tx: super::super::jobs::JobMessageSender,
    source: SampleSource,
    changed_samples: Vec<ChangedSampleInput>,
    announce: bool,
) {
    let changed_samples: Vec<_> = changed_samples
        .iter()
        .map(ChangedSampleInput::to_changed_sample)
        .collect();
    let result = analysis_jobs::enqueue_jobs_for_source(&source, &changed_samples);
    send_changed_sample_enqueue_result(tx, result, announce);
}

fn enqueue_manual_reanalysis(
    tx: super::super::jobs::JobMessageSender,
    source: SampleSource,
    action: ManualReanalysisAction,
) {
    match action {
        ManualReanalysisAction::SelectedSource => {
            enqueue_selected_source_reanalysis(tx, source);
        }
        ManualReanalysisAction::SelectedRows {
            changed_samples,
            sample_ids,
        } => {
            enqueue_selected_row_reanalysis(tx, source, changed_samples, sample_ids);
        }
        ManualReanalysisAction::SimilarityPrepBootstrap {
            force_full_analysis,
        } => {
            enqueue_similarity_bootstrap_reanalysis(tx, source, force_full_analysis);
        }
    }
}

fn enqueue_selected_source_reanalysis(
    tx: super::super::jobs::JobMessageSender,
    source: SampleSource,
) {
    let result = analysis_jobs::enqueue_jobs_for_source_backfill_full(&source);
    send_changed_sample_enqueue_result(tx.clone(), result, true);

    let result = analysis_jobs::enqueue_jobs_for_embedding_backfill(&source);
    send_embedding_enqueue_result(tx, result, true);
}

fn enqueue_selected_row_reanalysis(
    tx: super::super::jobs::JobMessageSender,
    source: SampleSource,
    changed_samples: Vec<ChangedSampleInput>,
    sample_ids: Vec<String>,
) {
    if !changed_samples.is_empty() {
        let changed_samples: Vec<_> = changed_samples
            .iter()
            .map(ChangedSampleInput::to_changed_sample)
            .collect();
        let result = analysis_jobs::enqueue_jobs_for_source(&source, &changed_samples);
        send_changed_sample_enqueue_result(tx.clone(), result, true);
    }
    let result = analysis_jobs::enqueue_jobs_for_embedding_samples(&source, &sample_ids);
    send_embedding_enqueue_result(tx, result, true);
}

fn enqueue_similarity_bootstrap_reanalysis(
    tx: super::super::jobs::JobMessageSender,
    source: SampleSource,
    force_full_analysis: bool,
) {
    let analysis_result = if force_full_analysis {
        analysis_jobs::enqueue_jobs_for_source_backfill_full(&source)
    } else {
        analysis_jobs::enqueue_jobs_for_source_backfill(&source)
    };
    send_changed_sample_enqueue_result(tx.clone(), analysis_result, true);

    let embed_result = analysis_jobs::enqueue_jobs_for_embedding_backfill(&source);
    send_embedding_enqueue_result(tx, embed_result, true);
}

fn send_changed_sample_enqueue_result(
    tx: super::super::jobs::JobMessageSender,
    result: Result<(usize, analysis_jobs::AnalysisProgress), String>,
    announce: bool,
) {
    match result {
        Ok((inserted, progress)) => {
            let _ = tx.send(super::super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::EnqueueFinished {
                    inserted,
                    progress,
                    announce,
                },
            ));
        }
        Err(err) => {
            let _ = tx.send(super::super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::EnqueueFailed(err),
            ));
        }
    }
}

fn send_embedding_enqueue_result(
    tx: super::super::jobs::JobMessageSender,
    result: Result<(usize, analysis_jobs::AnalysisProgress), String>,
    announce: bool,
) {
    match result {
        Ok((inserted, progress)) => {
            let _ = tx.send(super::super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::EmbeddingBackfillEnqueueFinished {
                    inserted,
                    progress,
                    announce,
                },
            ));
        }
        Err(err) => {
            let _ = tx.send(super::super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::EmbeddingBackfillEnqueueFailed(err),
            ));
        }
    }
}
