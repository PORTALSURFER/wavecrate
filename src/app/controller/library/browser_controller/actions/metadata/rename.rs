use super::*;

impl BrowserController<'_> {
    fn first_pending_auto_rename_metadata_path(
        &self,
        source_id: &SourceId,
        paths: &[PathBuf],
    ) -> Option<PathBuf> {
        paths.iter().find_map(|path| {
            self.metadata_mutation_pending_for(source_id, path)
                .then(|| path.clone())
        })
    }

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
        if paths.is_empty() {
            return Ok(());
        }
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        if self.runtime.jobs.file_ops_in_progress() {
            return Err("File operation already in progress".to_string());
        }
        if let Some(path) = self.first_pending_auto_rename_metadata_path(&source.id, paths) {
            return Err(format!(
                "Metadata update still in progress for {}; wait for it to finish before auto rename",
                path.display()
            ));
        }
        self.preload_bpm_values_for_paths(paths);
        let requests = self.prepare_auto_rename_requests(&source, paths)?;
        let requested_paths = requests
            .iter()
            .map(|request| request.old_relative.clone())
            .collect::<Vec<_>>();
        self.begin_pending_file_mutation(&source.id, requested_paths.clone());
        if cfg!(test) {
            let result = run_sample_auto_rename_job(
                source.clone(),
                requests,
                Arc::new(AtomicBool::new(false)),
            );
            self.apply_file_op_result(FileOpResult::SampleAutoRename(result));
            return Ok(());
        }
        self.set_status(
            format!("Auto renaming {} sample(s)...", requested_paths.len()),
            StatusTone::Busy,
        );
        let pending_source_id = source.id.clone();
        if let Err(err) = self.runtime.jobs.begin_one_shot_file_op(move |cancel| {
            FileOpResult::SampleAutoRename(run_sample_auto_rename_job(source, requests, cancel))
        }) {
            self.finish_pending_file_mutation(&pending_source_id, requested_paths);
            return Err(err);
        }
        Ok(())
    }
}
