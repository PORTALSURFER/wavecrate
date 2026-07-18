use super::*;

impl AppController {
    pub(super) fn spawn_analysis_trigger(&mut self, trigger: AnalysisTrigger) {
        let source_id = trigger.source_id().clone();
        let live_remap_owns_source = self
            .runtime
            .source_lane
            .pending_remap
            .as_ref()
            .is_some_and(|pending| !pending.canceled && pending.source.id == source_id);
        if live_remap_owns_source {
            self.cancel_pending_source_remap_for_mutation(&source_id);
        }
        let enqueue_guard = self.runtime.analysis.begin_source_enqueue(source_id);
        let tx = self.runtime.jobs.message_sender();
        std::thread::spawn(move || {
            let _enqueue_guard = enqueue_guard;
            let AnalysisTrigger::ChangedSamples {
                source,
                changed_samples,
                announce,
            } = trigger;
            enqueue_changed_samples(tx, source, changed_samples, announce);
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
