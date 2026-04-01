use super::drag_effects::SelectionDropDestination;
use super::drop_resolution::ResolvedDropTarget;
use super::*;

impl DragDropController<'_> {
    pub(super) fn finish_drag_payload(
        &mut self,
        payload: DragPayload,
        active_target: DragTarget,
        resolved_target: ResolvedDropTarget,
        copy_requested: bool,
        origin_source: Option<DragSource>,
    ) {
        match payload {
            DragPayload::Sample {
                source_id,
                relative_path,
            } => self.finish_sample_payload(
                source_id,
                relative_path,
                active_target,
                resolved_target,
                copy_requested,
                origin_source,
            ),
            DragPayload::Samples { samples } => {
                self.finish_samples_payload(samples, resolved_target, copy_requested)
            }
            DragPayload::Folder {
                source_id,
                relative_path,
            } => self.finish_folder_payload(source_id, relative_path, resolved_target),
            DragPayload::Selection {
                source_id,
                relative_path,
                bounds,
                keep_source_focused,
            } => self.finish_selection_payload(
                source_id,
                relative_path,
                bounds,
                keep_source_focused,
                resolved_target,
            ),
            DragPayload::DropTargetReorder { path } => {
                self.finish_drop_target_reorder(path, active_target)
            }
        }
    }

    fn finish_sample_payload(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        active_target: DragTarget,
        resolved_target: ResolvedDropTarget,
        copy_requested: bool,
        origin_source: Option<DragSource>,
    ) {
        if origin_source == Some(DragSource::Waveform)
            && matches!(active_target, DragTarget::BrowserTriage(_))
        {
            self.handle_waveform_sample_drop_to_browser(source_id, relative_path);
        } else if let Some(target_source) = resolved_target.folder_source_target.clone() {
            self.handle_samples_transfer_to_source_folder(
                &[DragSample {
                    source_id,
                    relative_path,
                }],
                target_source,
                resolved_target.folder_target.unwrap_or_default(),
                copy_requested,
            );
        } else if let Some(target) = resolved_target.source_target {
            self.handle_samples_transfer_to_source_folder(
                &[DragSample {
                    source_id,
                    relative_path,
                }],
                target,
                PathBuf::new(),
                copy_requested,
            );
        } else if let Some(target_path) = resolved_target.drop_target_path {
            self.handle_sample_drop_to_drop_target(
                source_id,
                relative_path,
                target_path,
                copy_requested,
            );
        } else if let Some(folder) = resolved_target.folder_target {
            self.handle_sample_drop_to_folder(source_id, relative_path, &folder);
        } else if resolved_target.triage_target.is_some() {
            self.handle_sample_drop(source_id, relative_path, resolved_target.triage_target);
        } else {
            self.set_status(
                "Drop onto a triage column or folder to move the sample",
                StatusTone::Warning,
            );
        }
    }

    fn finish_samples_payload(
        &mut self,
        samples: Vec<DragSample>,
        resolved_target: ResolvedDropTarget,
        copy_requested: bool,
    ) {
        if let Some(target) = resolved_target.folder_source_target.or(resolved_target.source_target)
        {
            self.handle_samples_transfer_to_source_folder(
                &samples,
                target,
                resolved_target.folder_target.unwrap_or_default(),
                copy_requested,
            );
        } else if let Some(target_path) = resolved_target.drop_target_path {
            self.handle_samples_drop_to_drop_target(&samples, target_path, copy_requested);
        } else if let Some(folder) = resolved_target.folder_target {
            self.handle_samples_drop_to_folder(&samples, &folder);
        } else if resolved_target.triage_target.is_some() {
            self.handle_samples_drop(&samples, resolved_target.triage_target);
        } else {
            self.set_status(
                "Drop onto a triage column or folder to move samples",
                StatusTone::Warning,
            );
        }
    }

    fn finish_folder_payload(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        resolved_target: ResolvedDropTarget,
    ) {
        if let Some(folder) = resolved_target.folder_target {
            self.handle_folder_drop_to_folder(source_id, relative_path, &folder);
        } else if resolved_target.drop_targets_panel {
            self.handle_folder_drop_to_drop_targets(source_id, relative_path);
        } else if resolved_target.drop_target_path.is_some() {
            self.set_status(
                "Drop targets accept samples, not folders",
                StatusTone::Warning,
            );
        } else {
            self.set_status("Drop onto a folder to move it", StatusTone::Warning);
        }
    }

    fn finish_selection_payload(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        bounds: SelectionRange,
        keep_source_focused: bool,
        resolved_target: ResolvedDropTarget,
    ) {
        if resolved_target.source_target.is_some() {
            self.set_status(
                "Drop samples onto a source to move them",
                StatusTone::Warning,
            );
            return;
        }
        if resolved_target.drop_target_path.is_some() {
            self.set_status(
                "Drop targets accept samples, not selections",
                StatusTone::Warning,
            );
            return;
        }
        if !resolved_target.browser_list_target
            && resolved_target.triage_target.is_none()
            && resolved_target.folder_source_target.is_none()
            && resolved_target.folder_target.is_none()
        {
            return;
        }
        self.handle_selection_drop(
            source_id,
            relative_path,
            bounds,
            SelectionDropDestination {
                browser_list_target: resolved_target.browser_list_target,
                triage_target: resolved_target.triage_target,
                target_source_id: resolved_target.folder_source_target.or(resolved_target.source_target),
                folder_target: resolved_target.folder_target,
            },
            keep_source_focused,
        );
    }

    fn finish_drop_target_reorder(&mut self, path: PathBuf, active_target: DragTarget) {
        let target_path = match active_target {
            DragTarget::DropTarget { path } => Some(path),
            DragTarget::DropTargetsPanel => None,
            _ => return,
        };
        self.reorder_drop_targets(&path, target_path.as_deref());
    }
}
