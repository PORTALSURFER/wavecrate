use super::*;

impl BrowserController<'_> {
    pub(in crate::app::controller::library::browser_controller::actions) fn rename_browser_sample_action(
        &mut self,
        row: usize,
        new_name: &str,
    ) -> Result<(), String> {
        let ctx = self.resolve_browser_sample(row)?;
        if self.warn_if_any_browser_context_busy(std::slice::from_ref(&ctx), "renaming") {
            return Ok(());
        }
        let result = self.try_rename_browser_sample(row, new_name);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    pub(crate) fn auto_rename_browser_sample_paths_action(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<(), String> {
        #[cfg(test)]
        let dispatch_started_at = std::time::Instant::now();
        if paths.is_empty() {
            return Ok(());
        }
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        self.preload_bpm_values_for_paths(paths);
        let requests = self.prepare_auto_rename_requests(&source, paths)?;
        if requests.is_empty() {
            return Ok(());
        }
        let intent_key = auto_rename_intent_key(&source.id, &requests);
        if self.runtime.jobs.file_ops_in_progress() {
            let pending = PendingBrowserAutoRenameIntent {
                key: intent_key.clone(),
                source_id: source.id.clone(),
                paths: paths.to_vec(),
            };
            return match self
                .runtime
                .source_lane
                .mutations
                .handle_busy_browser_auto_rename_intent(intent_key, pending)
            {
                BrowserRenameBusyDecision::Collapsed => {
                    self.set_file_op_status("Auto rename already in progress...", StatusTone::Busy);
                    Ok(())
                }
                BrowserRenameBusyDecision::Queued => {
                    self.set_file_op_status(
                        "Auto rename queued after current rename...",
                        StatusTone::Busy,
                    );
                    Ok(())
                }
                BrowserRenameBusyDecision::UnrelatedFileOp => {
                    Err("File operation already in progress".to_string())
                }
            };
        }
        let requested_paths = requests
            .iter()
            .map(|request| request.old_relative.clone())
            .collect::<Vec<_>>();
        self.runtime
            .source_lane
            .mutations
            .begin_browser_rename_intent(intent_key);
        self.begin_pending_file_mutation(&source.id, requested_paths.clone());
        #[cfg(test)]
        crate::app::controller::batch_latency::record(
            crate::app::controller::batch_latency::BatchLatencySample::new(
                crate::app::controller::batch_latency::BatchLatencyPhase::AutoRenameDispatch,
                requested_paths.len(),
                dispatch_started_at.elapsed(),
            ),
        );
        if cfg!(test) {
            let result = run_sample_auto_rename_job(
                source.clone(),
                requests,
                Arc::new(AtomicBool::new(false)),
            );
            self.apply_file_op_result(FileOpResult::SampleAutoRename(result));
            return Ok(());
        }
        self.set_file_op_status(
            format!("Auto renaming {} sample(s)...", requested_paths.len()),
            StatusTone::Busy,
        );
        let pending_source_id = source.id.clone();
        if let Err(err) = self.runtime.jobs.begin_one_shot_file_op(move |cancel| {
            FileOpResult::SampleAutoRename(run_sample_auto_rename_job(source, requests, cancel))
        }) {
            self.runtime
                .source_lane
                .mutations
                .clear_browser_rename_intent();
            self.finish_pending_file_mutation(&pending_source_id, requested_paths);
            return Err(err);
        }
        Ok(())
    }
}

fn auto_rename_intent_key(
    source_id: &crate::sample_sources::SourceId,
    requests: &[SampleAutoRenameRequest],
) -> BrowserRenameIntentKey {
    BrowserRenameIntentKey::new(
        source_id.clone(),
        requests
            .iter()
            .map(|request| (request.old_relative.clone(), request.new_relative.clone()))
            .collect(),
    )
}
