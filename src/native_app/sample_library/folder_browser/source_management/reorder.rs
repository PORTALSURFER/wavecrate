use radiant::widgets::DragHandleMessage;

use super::super::{FolderBrowserState, SOURCE_ROW_STEP};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app::sample_library::folder_browser) struct SourceReorderDrag {
    source_id: String,
    start_y: f32,
    source_index: usize,
    target_index: usize,
}

impl SourceReorderDrag {
    fn new(source_id: String, start_y: f32, target_index: usize) -> Self {
        Self {
            source_id,
            start_y,
            source_index: target_index,
            target_index,
        }
    }
}

impl FolderBrowserState {
    pub(in crate::native_app) fn apply_source_reorder_drag(
        &mut self,
        source_id: String,
        message: DragHandleMessage,
    ) -> bool {
        match message {
            DragHandleMessage::Started { position } => {
                let Some(source_index) = self.source_index(&source_id) else {
                    return false;
                };
                if self.source.sources.len() < 2 {
                    return false;
                }
                self.source.reorder_drag =
                    Some(SourceReorderDrag::new(source_id, position.y, source_index));
                false
            }
            DragHandleMessage::Moved { position } => {
                self.update_source_reorder_target(&source_id, position.y);
                false
            }
            DragHandleMessage::Ended { position } => {
                self.update_source_reorder_target(&source_id, position.y);
                self.commit_source_reorder(&source_id)
            }
            DragHandleMessage::Cancelled { .. } => {
                self.source.reorder_drag = None;
                false
            }
            DragHandleMessage::DoubleActivate { .. } => false,
        }
    }

    pub(in crate::native_app) fn source_reorder_drag_active(&self) -> bool {
        self.source.reorder_drag.is_some()
    }

    pub(in crate::native_app) fn source_reorder_drag_source_id(&self) -> Option<&str> {
        self.source
            .reorder_drag
            .as_ref()
            .map(|drag| drag.source_id.as_str())
    }

    #[cfg(test)]
    pub(in crate::native_app) fn source_reorder_target_source_id(&self) -> Option<&str> {
        let drag = self.source.reorder_drag.as_ref()?;
        self.source
            .sources
            .get(drag.target_index)
            .map(|source| source.id.as_str())
    }

    pub(in crate::native_app) fn source_reorder_drop_marker_after(
        &self,
        source_id: &str,
    ) -> Option<bool> {
        let drag = self.source.reorder_drag.as_ref()?;
        if drag.target_index == drag.source_index
            || self
                .source
                .sources
                .get(drag.target_index)
                .is_none_or(|source| source.id != source_id)
        {
            return None;
        }
        Some(drag.target_index > drag.source_index)
    }

    pub(in crate::native_app) fn clear_source_reorder_drag(&mut self) {
        self.source.reorder_drag = None;
    }

    fn source_index(&self, source_id: &str) -> Option<usize> {
        self.source
            .sources
            .iter()
            .position(|source| source.id == source_id)
    }

    fn update_source_reorder_target(&mut self, source_id: &str, pointer_y: f32) {
        let Some(drag) = self.source.reorder_drag.as_mut() else {
            return;
        };
        if drag.source_id != source_id || !pointer_y.is_finite() {
            return;
        }
        let Some(source_index) = self
            .source
            .sources
            .iter()
            .position(|source| source.id == source_id)
        else {
            self.source.reorder_drag = None;
            return;
        };
        let row_delta = ((pointer_y - drag.start_y) / SOURCE_ROW_STEP).round() as isize;
        let last_index = self.source.sources.len().saturating_sub(1) as isize;
        drag.target_index = (source_index as isize + row_delta).clamp(0, last_index) as usize;
    }

    fn commit_source_reorder(&mut self, source_id: &str) -> bool {
        let Some(drag) = self.source.reorder_drag.take() else {
            return false;
        };
        if drag.source_id != source_id {
            return false;
        }
        let Some(source_index) = self.source_index(source_id) else {
            return false;
        };
        if source_index == drag.target_index {
            return false;
        }
        let source = self.source.sources.remove(source_index);
        let target_index = drag.target_index.min(self.source.sources.len());
        self.source.sources.insert(target_index, source);
        true
    }
}
