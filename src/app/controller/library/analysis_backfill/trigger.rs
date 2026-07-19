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
        let tx = self.runtime.jobs.message_sender();
        std::thread::spawn(move || {
            let AnalysisTrigger::ChangedSamples {
                source,
                changed_samples,
                announce,
            } = trigger;
            reconcile_changed_samples(tx, source, changed_samples, announce);
        });
    }
}

fn reconcile_changed_samples(
    tx: super::super::jobs::JobMessageSender,
    source: SampleSource,
    changed_samples: Vec<ChangedSampleInput>,
    announce: bool,
) {
    let paths = changed_samples
        .into_iter()
        .map(|sample| sample.relative_path)
        .collect::<Vec<_>>();
    let result = source
        .open_db()
        .map_err(|error| error.to_string())
        .and_then(|database| {
            crate::sample_sources::scanner::sync_paths(&database, &paths)
                .map_err(|error| error.to_string())
        });
    match result {
        Ok(stats) => {
            let _ = tx.send(super::super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::ReadinessReconciliationFinished {
                    source_id: source.id,
                    changed: stats
                        .committed_delta
                        .created
                        .len()
                        .saturating_add(stats.committed_delta.changed.len()),
                    announce,
                },
            ));
        }
        Err(err) => {
            let _ = tx.send(super::super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::ReadinessReconciliationFailed(err),
            ));
        }
    }
}
