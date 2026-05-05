use super::*;
use crate::app::state::UiPoint;
use crate::app::state::{DragSample, DragSource};

mod drop_resolution;
mod external_drag;
mod payload_finish;

use drop_resolution::resolve_drop_target;

pub(crate) trait DragDropActions {
    fn start_sample_drag(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        label: String,
        pos: UiPoint,
    );
    fn start_samples_drag(&mut self, samples: Vec<DragSample>, label: String, pos: UiPoint);
    fn start_folder_drag(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        label: String,
        pos: UiPoint,
    );
    fn start_selection_drag_payload(
        &mut self,
        bounds: SelectionRange,
        pos: UiPoint,
        keep_source_focused: bool,
    );
    /// Begin dragging a drop target row to reorder the sidebar list.
    fn start_drop_target_drag(&mut self, path: PathBuf, label: String, pos: UiPoint);
    fn update_active_drag(
        &mut self,
        pos: UiPoint,
        source: DragSource,
        target: DragTarget,
        shift_down: bool,
        alt_down: bool,
    );
    fn refresh_drag_position(&mut self, pos: UiPoint, shift_down: bool, alt_down: bool);
    fn finish_active_drag(&mut self);
}

impl DragDropActions for DragDropController<'_> {
    fn start_sample_drag(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        label: String,
        pos: UiPoint,
    ) {
        self.begin_drag(
            DragPayload::Sample {
                source_id,
                relative_path,
            },
            label,
            pos,
        );
    }

    fn start_samples_drag(&mut self, samples: Vec<DragSample>, label: String, pos: UiPoint) {
        self.begin_drag(DragPayload::Samples { samples }, label, pos);
    }

    fn start_folder_drag(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        label: String,
        pos: UiPoint,
    ) {
        self.begin_drag(
            DragPayload::Folder {
                source_id,
                relative_path,
            },
            label,
            pos,
        );
    }

    fn start_selection_drag_payload(
        &mut self,
        bounds: SelectionRange,
        pos: UiPoint,
        keep_source_focused: bool,
    ) {
        if bounds.width() < MIN_SELECTION_WIDTH {
            return;
        }
        let Some(audio) = self.sample_view.wav.loaded_audio.clone() else {
            self.set_status(
                "Load a sample before dragging a selection",
                StatusTone::Warning,
            );
            return;
        };
        let payload = DragPayload::Selection {
            source_id: audio.source_id.clone(),
            relative_path: audio.relative_path.clone(),
            bounds,
            keep_source_focused,
        };
        let label = self.selection_drag_label(&audio, bounds);
        self.begin_drag(payload, label, pos);
    }

    fn start_drop_target_drag(&mut self, path: PathBuf, label: String, pos: UiPoint) {
        self.begin_drag(DragPayload::DropTargetReorder { path }, label, pos);
    }

    fn update_active_drag(
        &mut self,
        pos: UiPoint,
        source: DragSource,
        target: DragTarget,
        shift_down: bool,
        alt_down: bool,
    ) {
        if self.ui.drag.payload.is_none() || self.ui.drag.pointer_left_window {
            return;
        }
        debug!(
            "update_active_drag: pos={:?} source={:?} target={:?}",
            pos, source, target
        );
        self.ui.drag.position = Some(pos);
        self.ui.drag.copy_on_drop = alt_down;
        if self.ui.drag.origin_source.is_none() {
            self.ui.drag.origin_source = Some(source);
        }
        self.ui.drag.set_target(source, target);
        if let Some(DragPayload::Selection {
            keep_source_focused,
            ..
        }) = self.ui.drag.payload.as_mut()
        {
            let _ = shift_down;
            *keep_source_focused = true;
        }
    }

    fn refresh_drag_position(&mut self, pos: UiPoint, shift_down: bool, alt_down: bool) {
        if self.ui.drag.payload.is_some() {
            if self.ui.drag.pointer_left_window {
                return;
            }
            self.ui.drag.position = Some(pos);
            self.ui.drag.copy_on_drop = alt_down;
            if let Some(DragPayload::Selection {
                keep_source_focused,
                ..
            }) = self.ui.drag.payload.as_mut()
            {
                let _ = shift_down;
                *keep_source_focused = true;
            }
        }
    }

    fn finish_active_drag(&mut self) {
        let origin_source = self.ui.drag.origin_source;
        let payload = match self.ui.drag.payload.take() {
            Some(payload) => payload,
            None => {
                self.reset_drag();
                return;
            }
        };

        let active_target = self.ui.drag.active_target.clone();
        let copy_requested = self.ui.drag.copy_on_drop;
        let resolved_target = resolve_drop_target(self, &active_target);

        info!(
            "Finish drag payload={:?} active_target={:?} last_folder_target={:?}",
            payload, active_target, self.ui.drag.last_folder_target
        );
        debug!(
            "Drag origin_source={:?} active_target={:?} payload={:?}",
            origin_source, active_target, payload
        );

        let is_sample_payload = matches!(
            payload,
            DragPayload::Sample { .. } | DragPayload::Samples { .. }
        );
        let is_folder_payload = matches!(payload, DragPayload::Folder { .. });
        if is_sample_payload
            && resolved_target.over_folder_panel
            && resolved_target.folder_target.is_none()
        {
            self.reset_drag();
            self.set_status("Drop onto a folder to move the sample", StatusTone::Warning);
            return;
        }
        if is_folder_payload
            && resolved_target.over_folder_panel
            && resolved_target.folder_target.is_none()
        {
            self.reset_drag();
            self.set_status("Drop onto a folder to move it", StatusTone::Warning);
            return;
        }

        self.reset_drag();
        self.finish_drag_payload(
            payload,
            active_target,
            resolved_target,
            copy_requested,
            origin_source,
        );
    }
}
