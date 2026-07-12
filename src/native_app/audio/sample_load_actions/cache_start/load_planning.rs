use super::*;

impl NativeAppState {
    pub(in crate::native_app) fn maybe_start_preview_audition_warm(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let phase_started_at = Instant::now();
        let display_mode = self.ui.chrome.sample_browser_display;
        if self.preview_audition_warm_should_yield() {
            self.background.preview_audition_warm_task.cancel();
            self.waveform.cache.cancel_preview_audition_warm_schedule();
            let reason = self.preview_audition_warm_yield_reason();
            record_preview_audition_warm_plan(display_mode, "yield", Some(reason), None, None);
            record_preview_audition_warm_phase_profile(
                display_mode,
                "yield",
                Some(reason),
                PreviewAuditionWarmPhaseSummary::default(),
                phase_started_at.elapsed(),
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
            );
            return;
        }
        if self
            .background
            .preview_audition_warm_task
            .active()
            .is_some()
        {
            record_preview_audition_warm_phase_profile(
                display_mode,
                "active",
                None,
                PreviewAuditionWarmPhaseSummary::default(),
                phase_started_at.elapsed(),
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
            );
            return;
        }
        let plan_started_at = Instant::now();
        let plan = self.preview_audition_warm_candidates();
        let plan_elapsed = plan_started_at.elapsed();
        if plan.paths.is_empty() {
            record_preview_audition_warm_plan(
                display_mode,
                "empty",
                None,
                Some(&plan),
                Some(plan_elapsed),
            );
            record_preview_audition_warm_phase_profile(
                display_mode,
                "empty",
                None,
                PreviewAuditionWarmPhaseSummary::from_plan(&plan),
                phase_started_at.elapsed(),
                plan_elapsed,
                Duration::ZERO,
                Duration::ZERO,
            );
            return;
        }
        record_preview_audition_warm_plan(
            display_mode,
            "scheduled",
            None,
            Some(&plan),
            Some(plan_elapsed),
        );
        let summary = PreviewAuditionWarmPhaseSummary::from_plan(&plan);
        let reservation_started_at = Instant::now();
        let paths = plan.paths;
        self.waveform
            .cache
            .mark_preview_audition_warm_scheduled(&paths);
        if let Some(signature) = plan.starmap_signature {
            self.waveform
                .cache
                .reserve_starmap_preview_warm_batch(signature, paths.len());
        }
        if let Some(signature) = plan.list_signature {
            self.waveform
                .cache
                .reserve_list_preview_warm_batch(signature, paths.len());
        }
        let reservation_elapsed = reservation_started_at.elapsed();
        let started_at = Instant::now();
        let task_schedule_started_at = Instant::now();
        context
            .business()
            .background(PREVIEW_AUDITION_WARM_TASK_NAME)
            .latest(&mut self.background.preview_audition_warm_task)
            .run(
                move |worker_context| {
                    let scheduled_paths = paths.clone();
                    let mut attempted_paths = Vec::new();
                    let mut failed_paths = Vec::new();
                    let mut clips = Vec::new();
                    let mut waveform_previews = Vec::new();
                    let mut errors = 0;
                    for path in paths {
                        if worker_context.is_cancelled() {
                            break;
                        }
                        attempted_paths.push(path.clone());
                        match decode_wav_preview_clip(
                            PathBuf::from(path.as_str()),
                            PREVIEW_AUDITION_DURATION,
                        ) {
                            Ok(clip) => {
                                if let Ok(preview) = instant_waveform_head_preview_from_clip(
                                    clip.clone(),
                                    &|_| {},
                                    &|| worker_context.is_cancelled(),
                                ) {
                                    waveform_previews.push(preview);
                                }
                                clips.push(clip);
                            }
                            Err(_) => {
                                errors += 1;
                                failed_paths.push(path);
                            }
                        }
                    }
                    PreviewAuditionWarmResult {
                        scheduled_paths,
                        attempted_paths,
                        failed_paths,
                        clips,
                        waveform_previews,
                        errors,
                    }
                },
                move |completion| GuiMessage::PreviewAuditionWarmFinished {
                    completion,
                    started_at,
                },
            );
        record_preview_audition_warm_phase_profile(
            display_mode,
            "scheduled",
            None,
            summary,
            phase_started_at.elapsed(),
            plan_elapsed,
            reservation_elapsed,
            task_schedule_started_at.elapsed(),
        );
    }

    pub(in crate::native_app) fn cancel_preview_audition_warm_for_playback(&mut self) {
        self.background.preview_audition_warm_task.cancel();
        self.waveform.cache.cancel_preview_audition_warm_schedule();
    }

    fn preview_audition_warm_should_yield(&self) -> bool {
        self.ui.chrome.starmap_audition_drag.is_some()
            || self.sample_cache_warm_should_pause_active()
            || self.playback_visual_activity_active()
    }

    fn preview_audition_warm_yield_reason(&self) -> &'static str {
        if self.ui.chrome.starmap_audition_drag.is_some() {
            "starmap_drag"
        } else if self.sample_cache_warm_should_pause_active() {
            "sample_load_or_normalization"
        } else if self.playback_visual_activity_active() {
            "playback_active"
        } else {
            "unknown"
        }
    }

    fn preview_audition_warm_candidates(&mut self) -> PreviewAuditionWarmPlan {
        match self.ui.chrome.sample_browser_display {
            SampleBrowserDisplayMode::Map => self.preview_audition_warm_starmap_candidates(),
            SampleBrowserDisplayMode::List => self.preview_audition_warm_list_candidates(),
        }
    }

    pub(super) fn preview_audition_warm_starmap_candidates(&mut self) -> PreviewAuditionWarmPlan {
        let selected = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let center_x = self.ui.chrome.starmap_viewport.center_x;
        let center_y = self.ui.chrome.starmap_viewport.center_y;
        let zoom = self.ui.chrome.starmap_viewport.zoom.max(f32::EPSILON);
        let signature = starmap_preview_warm_view_signature(
            self.library.folder_browser.selected_source_id(),
            self.library.folder_browser.cached_starmap_projection_len(),
            center_x,
            center_y,
            zoom,
        );
        let mut remaining_budget = self
            .waveform
            .cache
            .remaining_starmap_preview_warm_budget(signature, PREVIEW_AUDITION_STARMAP_VIEW_BUDGET);
        if remaining_budget == 0 {
            return PreviewAuditionWarmPlan {
                paths: Vec::new(),
                starmap_signature: Some(signature),
                inspected_count: 0,
                candidate_count: 0,
                eligible_count: 0,
                starmap_remaining_budget: Some(0),
                ..PreviewAuditionWarmPlan::default()
            };
        }
        let Some(candidates) = self
            .library
            .folder_browser
            .cached_starmap_preview_warm_candidates(
                center_x,
                center_y,
                zoom,
                PREVIEW_AUDITION_STARMAP_VIEWPORT_PAD,
                selected.as_deref(),
                PREVIEW_AUDITION_STARMAP_NEIGHBORHOOD,
            )
        else {
            return PreviewAuditionWarmPlan::default();
        };
        let candidate_count = candidates.indices.len();
        let eligible_paths = candidates
            .indices
            .iter()
            .map(|&index| candidates.items[index].file_id.clone())
            .filter(|path| {
                self.waveform
                    .cache
                    .preview_audition_warm_needed(Path::new(path))
            })
            .take(PREVIEW_AUDITION_STARMAP_NEIGHBORHOOD)
            .collect::<Vec<_>>();
        let eligible_count = eligible_paths.len();
        if eligible_count == 0 && remaining_budget > 0 {
            self.waveform
                .cache
                .reserve_starmap_preview_warm_budget(signature, remaining_budget);
            remaining_budget = 0;
        }
        let paths = eligible_paths
            .into_iter()
            .take(PREVIEW_AUDITION_WARM_BATCH.min(remaining_budget))
            .collect();
        PreviewAuditionWarmPlan {
            paths,
            starmap_signature: Some(signature),
            inspected_count: candidates.inspected_count,
            candidate_count,
            eligible_count,
            starmap_cell_count: candidates.cell_count,
            starmap_visited_cell_count: candidates.visited_cell_count,
            starmap_remaining_budget: Some(remaining_budget),
            ..PreviewAuditionWarmPlan::default()
        }
    }

    pub(super) fn preview_audition_warm_list_candidates(&mut self) -> PreviewAuditionWarmPlan {
        let ordered_paths: Vec<String> = {
            let Some(visible_paths) = self
                .library
                .folder_browser
                .prepared_visible_sample_file_ids_matching_tags(
                    &self.metadata.tags_by_file,
                    PREVIEW_AUDITION_LIST_VIEW_BUDGET,
                )
            else {
                return PreviewAuditionWarmPlan::default();
            };
            Self::preview_audition_list_warm_ordered_paths(
                &visible_paths,
                self.library.folder_browser.selected_file_id(),
                PREVIEW_AUDITION_LIST_VIEW_BUDGET,
            )
        };
        let signature = list_preview_warm_view_signature(
            self.library.folder_browser.selected_source_id(),
            &ordered_paths,
        );
        let mut remaining_budget = self
            .waveform
            .cache
            .remaining_list_preview_warm_budget(signature, PREVIEW_AUDITION_LIST_VIEW_BUDGET);
        if remaining_budget == 0 {
            return PreviewAuditionWarmPlan {
                paths: Vec::new(),
                list_signature: Some(signature),
                inspected_count: 0,
                candidate_count: 0,
                eligible_count: 0,
                list_remaining_budget: Some(0),
                ..PreviewAuditionWarmPlan::default()
            };
        }
        let inspected_count = ordered_paths.len();
        let candidate_paths = ordered_paths
            .into_iter()
            .filter(|path| preview_audition_can_decode(path))
            .collect::<Vec<_>>();
        let candidate_count = candidate_paths.len();
        let eligible_paths = candidate_paths
            .into_iter()
            .filter(|path| {
                self.waveform
                    .cache
                    .preview_audition_warm_needed(Path::new(path))
            })
            .collect::<Vec<_>>();
        let eligible_count = eligible_paths.len();
        if eligible_count == 0 && remaining_budget > 0 {
            self.waveform
                .cache
                .reserve_list_preview_warm_budget(signature, remaining_budget);
            remaining_budget = 0;
        }
        let paths = eligible_paths
            .into_iter()
            .take(PREVIEW_AUDITION_WARM_BATCH.min(remaining_budget))
            .collect();
        PreviewAuditionWarmPlan {
            paths,
            list_signature: Some(signature),
            inspected_count,
            candidate_count,
            eligible_count,
            list_remaining_budget: Some(remaining_budget),
            ..PreviewAuditionWarmPlan::default()
        }
    }

    pub(super) fn preview_audition_list_warm_ordered_paths(
        rows: &[String],
        selected_file_id: Option<&str>,
        limit: usize,
    ) -> Vec<String> {
        if limit == 0 {
            return Vec::new();
        }
        let selected_index =
            selected_file_id.and_then(|selected| rows.iter().position(|row| row == selected));
        let Some(selected_index) = selected_index else {
            return rows.iter().take(limit).cloned().collect();
        };
        let mut ordered = Vec::with_capacity(limit.min(rows.len()));
        for offset in 0..rows.len() {
            if offset == 0 {
                if let Some(row) = rows.get(selected_index) {
                    ordered.push(row.clone());
                }
            } else {
                if let Some(row) = selected_index
                    .checked_add(offset)
                    .and_then(|index| rows.get(index))
                {
                    ordered.push(row.clone());
                }
                if ordered.len() >= limit {
                    break;
                }
                if let Some(row) = selected_index
                    .checked_sub(offset)
                    .and_then(|index| rows.get(index))
                {
                    ordered.push(row.clone());
                }
            }
            if ordered.len() >= limit {
                break;
            }
        }
        ordered
    }
}
