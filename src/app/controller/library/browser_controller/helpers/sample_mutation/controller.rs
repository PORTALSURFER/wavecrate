use super::*;

impl BrowserController<'_> {
    /// Move the resolved browser sample into the configured trash folder.
    pub(crate) fn try_delete_browser_sample_ctx(
        &mut self,
        ctx: &TriageSampleContext,
    ) -> Result<(), String> {
        if self.controller.warn_if_retained_delete_path_busy(
            &ctx.source.id,
            &ctx.entry.relative_path,
            "deleting",
        ) {
            return Ok(());
        }
        let moved = self.move_samples_to_configured_trash_detailed(
            vec![(ctx.source.clone(), ctx.entry.clone())],
            None,
        );
        if moved.moved_count() > 0 {
            return Ok(());
        }
        let err = moved
            .errors
            .last()
            .cloned()
            .unwrap_or_else(|| self.ui.status.text.clone());
        Err(err)
    }

    /// Rename the browser row at `row` while preserving playback resume details.
    pub(crate) fn try_rename_browser_sample(
        &mut self,
        row: usize,
        new_name: &str,
    ) -> Result<(), String> {
        let ctx = self.resolve_browser_sample(row)?;
        if self.controller.warn_if_retained_delete_path_busy(
            &ctx.source.id,
            &ctx.entry.relative_path,
            "renaming",
        ) {
            return Ok(());
        }
        let tag = self.sample_tag_for(&ctx.source, &ctx.entry.relative_path)?;
        let full_name = self.name_with_preserved_extension(&ctx.entry.relative_path, new_name)?;
        let new_relative = self.validate_new_sample_name_in_parent(
            &ctx.entry.relative_path,
            &ctx.source.root,
            &full_name,
        )?;
        let intent_key = BrowserRenameIntentKey::new(
            ctx.source.id.clone(),
            vec![(ctx.entry.relative_path.clone(), new_relative.clone())],
        );
        if self.runtime.jobs.file_ops_in_progress() {
            if self
                .runtime
                .source_lane
                .mutations
                .browser_rename_intent_is_active(&intent_key)
            {
                self.set_file_op_status("Rename already in progress...", StatusTone::Busy);
                return Ok(());
            }
            return Err("File operation already in progress".to_string());
        }
        self.dispatch_browser_sample_rename(ctx, new_relative, tag, intent_key)
    }

    fn dispatch_browser_sample_rename(
        &mut self,
        ctx: TriageSampleContext,
        new_relative: PathBuf,
        tag: crate::sample_sources::Rating,
        intent_key: BrowserRenameIntentKey,
    ) -> Result<(), String> {
        let request = BrowserSampleRenameRequest::capture(self, &ctx, new_relative, tag);
        if cfg!(test) {
            self.run_browser_sample_rename_inline(ctx, request, intent_key);
            return Ok(());
        }
        self.start_browser_sample_rename_job(ctx, request, intent_key)
    }

    fn run_browser_sample_rename_inline(
        &mut self,
        ctx: TriageSampleContext,
        request: BrowserSampleRenameRequest,
        intent_key: BrowserRenameIntentKey,
    ) {
        self.runtime
            .source_lane
            .mutations
            .begin_browser_rename_intent(intent_key);
        self.begin_pending_file_mutation(&ctx.source.id, [ctx.entry.relative_path.clone()]);
        let result = run_sample_rename_job(ctx, request, Arc::new(AtomicBool::new(false)));
        self.apply_file_op_result(FileOpResult::SampleRename(result));
    }

    fn start_browser_sample_rename_job(
        &mut self,
        ctx: TriageSampleContext,
        request: BrowserSampleRenameRequest,
        intent_key: BrowserRenameIntentKey,
    ) -> Result<(), String> {
        self.runtime
            .source_lane
            .mutations
            .begin_browser_rename_intent(intent_key);
        self.begin_pending_file_mutation(&ctx.source.id, [ctx.entry.relative_path.clone()]);
        self.set_file_op_status(
            format!("Renaming {}...", ctx.entry.relative_path.display()),
            StatusTone::Busy,
        );
        let pending_source_id = ctx.source.id.clone();
        let pending_path = ctx.entry.relative_path.clone();
        if let Err(err) = self.runtime.jobs.begin_one_shot_file_op(move |cancel| {
            FileOpResult::SampleRename(run_sample_rename_job(ctx, request, cancel))
        }) {
            self.runtime
                .source_lane
                .mutations
                .clear_browser_rename_intent();
            self.finish_pending_file_mutation(&pending_source_id, [pending_path]);
            return Err(err);
        }
        Ok(())
    }
}

pub(super) struct BrowserSampleRenameRequest {
    new_relative: PathBuf,
    tag: crate::sample_sources::Rating,
    fallback_sound_type: Option<crate::sample_sources::SampleSoundType>,
    resume_playback: bool,
    resume_looped: bool,
    resume_start_override: Option<f64>,
}

impl BrowserSampleRenameRequest {
    fn capture(
        browser: &BrowserController<'_>,
        ctx: &TriageSampleContext,
        new_relative: PathBuf,
        tag: crate::sample_sources::Rating,
    ) -> Self {
        let playhead_position = browser.ui.waveform.playhead.position;
        Self {
            new_relative,
            tag,
            fallback_sound_type: ctx.entry.sound_type,
            resume_playback: browser.is_playing() && is_currently_loaded(browser, ctx),
            resume_looped: browser.ui.waveform.loop_enabled,
            resume_start_override: playhead_position
                .is_finite()
                .then(|| f64::from(playhead_position.clamp(0.0, 1.0))),
        }
    }
}

pub(super) fn run_sample_rename_job(
    ctx: TriageSampleContext,
    request: BrowserSampleRenameRequest,
    cancel: Arc<AtomicBool>,
) -> SampleRenameResult {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return SampleRenameResult {
            source_id: ctx.source.id,
            old_relative: ctx.entry.relative_path,
            new_relative: request.new_relative,
            entry: None,
            resume_playback: request.resume_playback,
            resume_looped: request.resume_looped,
            resume_start_override: request.resume_start_override,
            result: Err(String::from("Rename cancelled")),
        };
    }
    let old_relative = ctx.entry.relative_path.clone();
    let result = perform_sample_rename(
        &ctx.source,
        &ctx.absolute_path,
        &old_relative,
        &request.new_relative,
        request.tag,
        RenameLoopedMetadata::DbOrFallback(ctx.entry.looped),
        ctx.entry.locked,
        ctx.entry.last_played_at,
        request.fallback_sound_type,
        ctx.entry.user_tag.clone(),
        None,
    );
    SampleRenameResult {
        source_id: ctx.source.id,
        old_relative,
        new_relative: request.new_relative,
        entry: result.as_ref().ok().cloned(),
        resume_playback: request.resume_playback,
        resume_looped: request.resume_looped,
        resume_start_override: request.resume_start_override,
        result: result.map(|_| ()),
    }
}

fn is_currently_loaded(browser: &BrowserController<'_>, ctx: &TriageSampleContext) -> bool {
    browser
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .is_some_and(|audio| {
            audio.source_id == ctx.source.id && audio.relative_path == ctx.entry.relative_path
        })
}
