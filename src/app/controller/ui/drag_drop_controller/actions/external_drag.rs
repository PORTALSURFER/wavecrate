use super::*;
#[cfg(any(target_os = "windows", test))]
use std::time::{Duration, Instant};
#[cfg(target_os = "windows")]
use tracing::{debug, info, warn};

impl DragDropController<'_> {
    #[cfg(any(target_os = "windows", test))]
    pub(crate) const EXTERNAL_DRAG_ARM_WINDOW: Duration = Duration::from_millis(250);

    #[cfg(any(target_os = "windows", test))]
    pub(crate) fn should_launch_external_drag(
        &mut self,
        pointer_outside: bool,
        pointer_left: bool,
        now: Instant,
    ) -> bool {
        let should_consider = matches!(
            self.ui.drag.payload,
            Some(
                DragPayload::Sample { .. }
                    | DragPayload::Samples { .. }
                    | DragPayload::Selection { .. }
            )
        );
        if !should_consider {
            self.ui.drag.external_arm_at = None;
            return false;
        }
        if self.ui.drag.payload.is_none() {
            self.ui.drag.external_arm_at = None;
            return false;
        }
        if !(pointer_outside || pointer_left) {
            self.ui.drag.external_arm_at = None;
            return false;
        }
        let Some(armed_at) = self.ui.drag.external_arm_at else {
            self.ui.drag.external_arm_at = Some(now);
            #[cfg(target_os = "windows")]
            debug!(
                pointer_outside,
                pointer_left,
                payload = ?self.ui.drag.payload.as_ref().map(drag_payload_kind),
                arm_window_ms = Self::EXTERNAL_DRAG_ARM_WINDOW.as_millis(),
                "drag controller: armed external drag dwell window"
            );
            return false;
        };
        let ready = now.duration_since(armed_at) >= Self::EXTERNAL_DRAG_ARM_WINDOW;
        #[cfg(target_os = "windows")]
        if ready {
            info!(
                pointer_outside,
                pointer_left,
                armed_for_ms = now.duration_since(armed_at).as_millis(),
                payload = ?self.ui.drag.payload.as_ref().map(drag_payload_kind),
                "drag controller: external drag dwell satisfied"
            );
        }
        ready
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn maybe_launch_external_drag(
        &mut self,
        pointer_outside: bool,
        pointer_left: bool,
    ) -> bool {
        if self.ui.drag.external_started {
            debug!("drag controller: skipping external drag poll because drag already started");
            return false;
        }
        if !self.should_launch_external_drag(pointer_outside, pointer_left, Instant::now()) {
            return false;
        }
        let payload = self.ui.drag.payload.clone();
        info!(
            pointer_outside,
            pointer_left,
            payload = ?payload.as_ref().map(drag_payload_kind),
            "drag controller: attempting external drag launch"
        );
        let status = match payload {
            Some(DragPayload::Sample {
                source_id,
                relative_path,
            }) => {
                let absolute = self.sample_absolute_path(&source_id, &relative_path);
                info!(
                    path = %absolute.display(),
                    "drag controller: resolved single-sample external drag path"
                );
                self.start_external_drag(&[absolute])
                    .map(|_| format!("Drag {} to an external target", relative_path.display()))
            }
            Some(DragPayload::Samples { samples }) => {
                let absolutes: Vec<PathBuf> = samples
                    .iter()
                    .map(|sample| {
                        self.sample_absolute_path(&sample.source_id, &sample.relative_path)
                    })
                    .collect();
                info!(
                    path_count = absolutes.len(),
                    first_path = %absolutes
                        .first()
                        .map(|path| path.display().to_string())
                        .unwrap_or_default(),
                    "drag controller: resolved multi-sample external drag paths"
                );
                self.start_external_drag(&absolutes)
                    .map(|_| format!("Drag {} samples to an external target", samples.len()))
            }
            Some(DragPayload::Selection { bounds, .. }) => {
                let snapshot = match self.capture_selection_export_snapshot(bounds, None) {
                    Ok(snapshot) => snapshot,
                    Err(err) => {
                        warn!(error = %err, "drag controller: failed to capture external selection drag snapshot");
                        self.reset_drag();
                        self.set_status(err, StatusTone::Error);
                        return true;
                    }
                };
                let request_id = self.runtime.jobs.next_selection_export_request_id();
                self.ui.drag.external_started = true;
                self.ui.drag.pending_external_selection_request_id = Some(request_id);
                info!(
                    request_id,
                    "drag controller: queued selection export backing external drag"
                );
                self.runtime.jobs.begin_selection_export(
                    crate::app::controller::jobs::SelectionExportJob::Clip {
                        request_id,
                        snapshot,
                        destination:
                            crate::app::controller::jobs::SelectionClipDestination::ExternalDrag,
                    },
                );
                self.set_status("Preparing clip for external drag...", StatusTone::Busy);
                return true;
            }
            Some(DragPayload::Folder { .. }) => return false,
            Some(DragPayload::DropTargetReorder { .. }) => return false,
            None => return false,
        };
        self.ui.drag.external_started = true;
        match status {
            Ok(message) => {
                info!(message, "drag controller: external drag launch succeeded");
                self.reset_drag();
                self.set_status(message, StatusTone::Info);
                true
            }
            Err(err) => {
                self.ui.drag.external_started = false;
                self.ui.drag.external_arm_at = None;
                warn!(error = %err, "drag controller: external drag launch failed");
                self.set_status(err, StatusTone::Error);
                false
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn drag_payload_kind(payload: &DragPayload) -> &'static str {
    match payload {
        DragPayload::Sample { .. } => "sample",
        DragPayload::Samples { .. } => "samples",
        DragPayload::Selection { .. } => "selection",
        DragPayload::Folder { .. } => "folder",
        DragPayload::DropTargetReorder { .. } => "drop_target_reorder",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_sources::{Rating, SampleSource, WavEntry};
    use tempfile::tempdir;

    #[test]
    fn external_drag_arms_and_resets_when_pointer_returns() {
        let renderer = WaveformRenderer::new(12, 12);
        let mut controller = AppController::new(renderer, None);
        let mut drag = DragDropController::new(&mut controller);
        drag.ui.drag.payload = Some(DragPayload::Sample {
            source_id: SourceId::new(),
            relative_path: PathBuf::from("one.wav"),
        });

        let start = Instant::now();
        assert!(!drag.should_launch_external_drag(true, false, start));
        assert!(drag.ui.drag.external_arm_at.is_some());

        assert!(!drag.should_launch_external_drag(false, false, start));
        assert!(drag.ui.drag.external_arm_at.is_none());
    }

    #[test]
    fn external_drag_requires_outside_dwell_time() {
        let renderer = WaveformRenderer::new(12, 12);
        let mut controller = AppController::new(renderer, None);
        let mut drag = DragDropController::new(&mut controller);
        drag.ui.drag.payload = Some(DragPayload::Sample {
            source_id: SourceId::new(),
            relative_path: PathBuf::from("one.wav"),
        });

        let start = Instant::now();
        assert!(!drag.should_launch_external_drag(true, false, start));
        assert!(!drag.should_launch_external_drag(
            true,
            false,
            start + DragDropController::EXTERNAL_DRAG_ARM_WINDOW - Duration::from_millis(1)
        ));
        assert!(drag.should_launch_external_drag(
            true,
            false,
            start + DragDropController::EXTERNAL_DRAG_ARM_WINDOW
        ));
    }

    #[test]
    fn external_drag_arms_on_pointer_gone_then_launches_after_dwell_time() {
        let renderer = WaveformRenderer::new(12, 12);
        let mut controller = AppController::new(renderer, None);
        let mut drag = DragDropController::new(&mut controller);
        drag.ui.drag.payload = Some(DragPayload::Sample {
            source_id: SourceId::new(),
            relative_path: PathBuf::from("one.wav"),
        });

        let start = Instant::now();
        assert!(!drag.should_launch_external_drag(false, true, start));
        assert!(drag.ui.drag.external_arm_at.is_some());

        assert!(drag.should_launch_external_drag(
            true,
            false,
            start + DragDropController::EXTERNAL_DRAG_ARM_WINDOW
        ));
    }

    #[test]
    fn stale_external_selection_export_completion_does_not_reset_active_drag() {
        let temp = tempdir().unwrap();
        let renderer = WaveformRenderer::new(12, 12);
        let mut controller = AppController::new(renderer, None);
        let source = SampleSource::new(temp.path().join("source"));
        std::fs::create_dir_all(&source.root).unwrap();
        controller.library.sources.push(source.clone());
        let export_relative = PathBuf::from("one_selection_001.wav");
        let export_absolute = source.root.join(&export_relative);
        std::fs::write(&export_absolute, b"wav").unwrap();
        let backup =
            crate::app::controller::undo::OverwriteBackup::capture_before(&export_absolute)
                .unwrap();
        backup.capture_after(&export_absolute).unwrap();
        controller
            .cache_db(&source)
            .unwrap()
            .upsert_file(&export_relative, 3, 1)
            .unwrap();
        controller.ui.drag.payload = Some(DragPayload::Selection {
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            bounds: SelectionRange::new(0.2, 0.8),
            keep_source_focused: true,
        });
        controller.ui.drag.external_started = true;
        controller.ui.drag.pending_external_selection_request_id = Some(42);

        controller.apply_background_job_message_for_tests(
            crate::app::controller::jobs::JobMessage::SelectionExport(
                crate::app::controller::jobs::SelectionExportMessage::Finished(
                    crate::app::controller::jobs::SelectionExportResult::Clip {
                    request_id: 41,
                    result: Ok(crate::app::controller::jobs::SelectionClipExportSuccess {
                        request_id: 41,
                        source_id: source.id.clone(),
                        source_root: source.root.clone(),
                        entry: WavEntry {
                            relative_path: export_relative,
                            file_size: 3,
                            modified_ns: 1,
                            content_hash: None,
                            tag: Rating::NEUTRAL,
                            looped: false,
                            locked: false,
                            missing: false,
                            last_played_at: None,
                        },
                        absolute_path: export_absolute,
                        backup,
                        destination:
                            crate::app::controller::jobs::SelectionClipDestination::ExternalDrag,
                        timings: Default::default(),
                    }),
                }),
            ),
        );

        assert_eq!(
            controller.ui.drag.pending_external_selection_request_id,
            Some(42)
        );
        assert!(controller.ui.drag.payload.is_some());
    }
}
