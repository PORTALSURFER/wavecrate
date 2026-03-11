use super::*;
#[cfg(any(target_os = "windows", test))]
use std::time::{Duration, Instant};

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
            return false;
        };
        now.duration_since(armed_at) >= Self::EXTERNAL_DRAG_ARM_WINDOW
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn maybe_launch_external_drag(&mut self, pointer_outside: bool, pointer_left: bool) {
        if self.ui.drag.external_started {
            return;
        }
        if !self.should_launch_external_drag(pointer_outside, pointer_left, Instant::now()) {
            return;
        }
        self.ui.drag.external_started = true;
        let payload = self.ui.drag.payload.clone();
        let status = match payload {
            Some(DragPayload::Sample {
                source_id,
                relative_path,
            }) => {
                let absolute = self.sample_absolute_path(&source_id, &relative_path);
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
                self.start_external_drag(&absolutes)
                    .map(|_| format!("Drag {} samples to an external target", samples.len()))
            }
            Some(DragPayload::Selection { bounds, .. }) => self
                .export_selection_for_drag(bounds)
                .and_then(|(absolute, label)| {
                    self.start_external_drag(&[absolute])?;
                    Ok(label)
                }),
            Some(DragPayload::Folder { .. }) => return,
            Some(DragPayload::DropTargetReorder { .. }) => return,
            None => return,
        };
        match status {
            Ok(message) => {
                self.reset_drag();
                self.set_status(message, StatusTone::Info);
            }
            Err(err) => {
                self.reset_drag();
                self.set_status(err, StatusTone::Error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
