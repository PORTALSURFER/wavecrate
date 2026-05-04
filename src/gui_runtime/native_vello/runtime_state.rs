use super::SingleLineTextEditorState;
use super::{
    ShellLayoutDirtyKind, ShellLayoutTreeKind, TextFieldVisualState,
    dirty_segments_for_layout_subtree,
};
use crate::compat_app_contract::DirtySegments;
use crate::gui::panel::SplitPaneSlot;
use crate::gui::types::Point;
use std::mem;
#[cfg(target_os = "windows")]
use tracing::{debug, info};

/// Active browser-list scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ContentListScrollbarDragState {
    pub(super) thumb_pointer_offset_y: f32,
}

/// Active folder-scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FolderScrollbarDragState {
    pub(super) pane: SplitPaneSlot,
    pub(super) thumb_pointer_offset_y: f32,
}

/// Active waveform-scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformScrollbarDragState {
    pub(super) thumb_pointer_offset_x: f32,
    pub(super) thumb_pointer_ratio_x: f32,
}

/// Active middle-button waveform pan drag state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformPanDragState {
    pub(super) origin_x: f32,
    pub(super) view_start_micros: u32,
    pub(super) view_end_micros: u32,
    pub(super) view_start_nanos: u32,
    pub(super) view_end_nanos: u32,
}

/// Exact waveform press state used to preserve click-to-seek precision on release.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformClickSeekPress {
    pub(super) press_x: f32,
    pub(super) position_micros: u32,
    pub(super) position_nanos: u32,
    pub(super) clear_selection_on_release: bool,
}

/// Deferred browser-list row press used to preserve click behavior until release.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PendingBrowserRowPress {
    pub(super) action: crate::compat_app_contract::UiAction,
    pub(super) visible_row: usize,
    pub(super) press_point: Point,
}

/// Active browser-item drag session while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ContentItemDragState {
    pub(super) visible_row: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum TextInputTarget {
    #[default]
    None,
    BrowserSearch,
    BrowserPillEditor,
    FolderSearch,
    FolderCreate,
    PromptInput,
    WaveformBpm,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ActiveTextFieldVisualCacheEntry {
    pub(super) target: TextInputTarget,
    pub(super) text: String,
    pub(super) editor: SingleLineTextEditorState,
    pub(super) font_size_bits: u32,
    pub(super) available_width_bits: u32,
    pub(super) visual: TextFieldVisualState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeInvalidationScope {
    OverlayStateOnly,
    OverlayMotionOnly,
    ModelAndOverlays,
    StaticAndOverlays,
    LayoutAndAll,
    #[cfg_attr(not(test), allow(dead_code))]
    LayoutSubtreeAndAll(RuntimeLayoutSubtreeInvalidation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RuntimeLayoutSubtreeInvalidation {
    pub(super) tree_kind: ShellLayoutTreeKind,
    pub(super) node_id: u64,
    pub(super) dirty_kind: ShellLayoutDirtyKind,
}

impl RuntimeLayoutSubtreeInvalidation {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn new(
        tree_kind: ShellLayoutTreeKind,
        node_id: u64,
        dirty_kind: ShellLayoutDirtyKind,
    ) -> Self {
        Self {
            tree_kind,
            node_id,
            dirty_kind,
        }
    }

    pub(super) fn dirty_segments(self) -> DirtySegments {
        dirty_segments_for_layout_subtree(self.tree_kind, self.node_id)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeLayoutInvalidation {
    pub(super) full_rebuild: bool,
    pub(super) dirty_segments: DirtySegments,
    pub(super) subtrees: Vec<RuntimeLayoutSubtreeInvalidation>,
}

impl RuntimeLayoutInvalidation {
    pub(super) fn is_pending(&self) -> bool {
        self.full_rebuild || !self.subtrees.is_empty()
    }

    pub(super) fn mark_full(&mut self) {
        self.full_rebuild = true;
        self.subtrees.clear();
        self.dirty_segments.insert(
            DirtySegments::STATUS_BAR
                | DirtySegments::BROWSER_FRAME
                | DirtySegments::BROWSER_ROWS_WINDOW
                | DirtySegments::MAP_PANEL
                | DirtySegments::WAVEFORM_OVERLAY
                | DirtySegments::GLOBAL_STATIC,
        );
    }

    pub(super) fn mark_subtree(&mut self, invalidation: RuntimeLayoutSubtreeInvalidation) {
        if self.full_rebuild {
            return;
        }
        self.dirty_segments
            .insert(invalidation.dirty_segments().bits());
        self.subtrees.push(invalidation);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ActivePointerSession {
    Volume,
    FolderScrollbar,
    ContentListScrollbar,
    WaveformScrollbar,
    WaveformPan,
    WaveformDrag,
    ContentItemDrag,
    SelectionDrag,
    SpatialFocusDrag,
    TextInputDrag,
    Hover,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct NativeVelloFrameState {
    pub(crate) layout_invalidation: RuntimeLayoutInvalidation,
    pub(crate) scene_dirty: bool,
    pub(crate) state_overlay_dirty: bool,
    pub(crate) motion_overlay_dirty: bool,
    pub(crate) model_dirty: bool,
}

impl NativeVelloFrameState {
    pub(super) fn mark_layout_dirty(&mut self) {
        self.layout_invalidation.mark_full();
        self.scene_dirty = true;
    }

    pub(super) fn mark_layout_subtree_dirty(
        &mut self,
        invalidation: RuntimeLayoutSubtreeInvalidation,
    ) {
        self.layout_invalidation.mark_subtree(invalidation);
        self.scene_dirty = true;
    }

    pub(super) fn mark_state_overlay_dirty(&mut self) {
        self.state_overlay_dirty = true;
    }
    pub(super) fn mark_motion_overlay_dirty(&mut self) {
        self.motion_overlay_dirty = true;
    }
    pub(super) fn take_layout_invalidation(&mut self) -> RuntimeLayoutInvalidation {
        mem::take(&mut self.layout_invalidation)
    }

    pub(super) fn mark_model_dirty(&mut self) {
        self.model_dirty = true;
        self.scene_dirty = true;
    }

    pub(super) fn mark_model_overlay_dirty(&mut self) {
        self.model_dirty = true;
        self.state_overlay_dirty = true;
        self.motion_overlay_dirty = true;
    }

    pub(super) fn take_scene(&mut self) -> bool {
        let dirty = self.scene_dirty;
        self.scene_dirty = false;
        dirty
    }

    pub(super) fn take_state_overlay(&mut self) -> bool {
        let dirty = self.state_overlay_dirty;
        self.state_overlay_dirty = false;
        dirty
    }

    pub(super) fn take_motion_overlay(&mut self) -> bool {
        let dirty = self.motion_overlay_dirty;
        self.motion_overlay_dirty = false;
        dirty
    }

    pub(super) fn take_model(&mut self) -> bool {
        let dirty = self.model_dirty;
        self.model_dirty = false;
        dirty
    }

    pub(super) fn has_pending_rebuild(&self) -> bool {
        self.layout_invalidation.is_pending()
            || self.scene_dirty
            || self.state_overlay_dirty
            || self.motion_overlay_dirty
            || self.model_dirty
    }
}

impl<Bridge> super::NativeVelloRunner<Bridge>
where
    Bridge: crate::compat_app_contract::NativeAppBridge,
{
    pub(super) fn has_external_drag_candidate(&self) -> bool {
        self.content_item_drag.is_some() || self.selection_drag_active
    }

    pub(super) fn maybe_launch_external_drag_session(
        &mut self,
        pointer_outside: bool,
        pointer_left: bool,
    ) -> bool {
        #[cfg(target_os = "windows")]
        {
            if self.has_external_drag_candidate() {
                let consumed = self
                    .bridge
                    .maybe_launch_external_drag(pointer_outside, pointer_left);
                debug!(
                    pointer_outside,
                    pointer_left,
                    consumed,
                    content_item_drag = self.content_item_drag.is_some(),
                    selection_drag_active = self.selection_drag_active,
                    "radiant external drag: host bridge launch poll"
                );
                return consumed;
            }
        }
        let _ = pointer_outside;
        let _ = pointer_left;
        false
    }

    #[cfg(target_os = "windows")]
    pub(super) fn poll_external_drag_window_state(&self) -> Option<(bool, bool)> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::Foundation::{POINT, RECT};
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};

        if !self.has_external_drag_candidate() {
            return None;
        }
        let window = self.window.as_ref()?;
        let handle = window.window_handle().ok()?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return None;
        };
        let hwnd = handle.hwnd.get();
        let mut cursor = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut cursor) } == 0 {
            return None;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if unsafe { GetWindowRect(hwnd as *mut _, &mut rect) } == 0 {
            return None;
        }
        let inside = cursor.x >= rect.left
            && cursor.x < rect.right
            && cursor.y >= rect.top
            && cursor.y < rect.bottom;
        debug!(
            cursor_x = cursor.x,
            cursor_y = cursor.y,
            window_left = rect.left,
            window_top = rect.top,
            window_right = rect.right,
            window_bottom = rect.bottom,
            pointer_outside = !inside,
            pointer_left = !inside,
            content_item_drag = self.content_item_drag.is_some(),
            selection_drag_active = self.selection_drag_active,
            "radiant external drag: polled native window bounds"
        );
        Some((!inside, !inside))
    }

    pub(super) fn active_pointer_session(&self) -> ActivePointerSession {
        if self.volume_drag_active {
            ActivePointerSession::Volume
        } else if self.folder_scrollbar_drag.is_some() {
            ActivePointerSession::FolderScrollbar
        } else if self.browser_scrollbar_drag.is_some() {
            ActivePointerSession::ContentListScrollbar
        } else if self.waveform_scrollbar_drag.is_some() {
            ActivePointerSession::WaveformScrollbar
        } else if self.waveform_pan_drag.is_some() {
            ActivePointerSession::WaveformPan
        } else if self.waveform_drag_mode.is_some() {
            ActivePointerSession::WaveformDrag
        } else if self.content_item_drag.is_some() {
            ActivePointerSession::ContentItemDrag
        } else if self.selection_drag_active {
            ActivePointerSession::SelectionDrag
        } else if self.spatial_focus_drag_active {
            ActivePointerSession::SpatialFocusDrag
        } else if self.text_input_drag_active {
            ActivePointerSession::TextInputDrag
        } else {
            ActivePointerSession::Hover
        }
    }

    pub(super) fn begin_pointer_press_cycle(&mut self) {
        self.pending_volume_milli = None;
        self.volume_drag_active = false;
        self.last_emitted_volume_milli = None;
        self.pending_browser_row_press = None;
        self.clear_pointer_drag_session();
    }

    pub(super) fn clear_pointer_release_state(&mut self) {
        self.text_input_drag_active = false;
        self.folder_scrollbar_drag = None;
        self.browser_scrollbar_drag = None;
        self.waveform_scrollbar_drag = None;
        self.waveform_pan_drag = None;
        self.last_emitted_browser_list_view_start = None;
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn clear_pointer_drag_session(&mut self) {
        #[cfg(target_os = "windows")]
        if self.has_external_drag_candidate() {
            info!(
                content_item_drag = self.content_item_drag.is_some(),
                selection_drag_active = self.selection_drag_active,
                "radiant external drag: clearing runtime drag session"
            );
        }
        self.waveform_drag_mode = None;
        self.waveform_click_seek_press = None;
        self.content_item_drag = None;
        self.selection_drag_active = false;
        self.last_emitted_waveform_drag_action = None;
        self.spatial_focus_drag_active = false;
        self.last_emitted_spatial_drag_content_id = None;
        self.folder_scrollbar_drag = None;
        self.browser_scrollbar_drag = None;
        self.last_emitted_browser_list_view_start = None;
        self.waveform_scrollbar_drag = None;
        self.waveform_pan_drag = None;
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn begin_folder_scrollbar_drag(
        &mut self,
        pane: SplitPaneSlot,
        thumb_pointer_offset_y: f32,
    ) {
        self.folder_scrollbar_drag = Some(FolderScrollbarDragState {
            pane,
            thumb_pointer_offset_y,
        });
    }

    pub(super) fn begin_browser_scrollbar_drag(&mut self, thumb_pointer_offset_y: f32) {
        self.browser_scrollbar_drag = Some(ContentListScrollbarDragState {
            thumb_pointer_offset_y,
        });
        self.last_emitted_browser_list_view_start = None;
    }

    pub(super) fn begin_waveform_scrollbar_drag(
        &mut self,
        thumb_pointer_offset_x: f32,
        thumb_pointer_ratio_x: f32,
    ) {
        self.waveform_scrollbar_drag = Some(WaveformScrollbarDragState {
            thumb_pointer_offset_x,
            thumb_pointer_ratio_x,
        });
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn begin_waveform_pan_drag(&mut self, origin_x: f32) {
        self.refresh_waveform_view_if_needed();
        self.waveform_pan_drag = Some(WaveformPanDragState {
            origin_x,
            view_start_micros: self.model.waveform.view_start_micros,
            view_end_micros: self.model.waveform.view_end_micros,
            view_start_nanos: self.model.waveform.view_start_nanos,
            view_end_nanos: self.model.waveform.view_end_nanos,
        });
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn begin_spatial_focus_drag(&mut self, content_id: Option<String>) {
        self.spatial_focus_drag_active = true;
        self.last_emitted_spatial_drag_content_id = content_id;
    }

    pub(super) fn begin_content_item_drag(&mut self, visible_row: usize) {
        self.content_item_drag = Some(ContentItemDragState { visible_row });
        #[cfg(target_os = "windows")]
        info!(
            visible_row,
            "radiant external drag: browser-item drag session started"
        );
    }

    pub(super) fn begin_waveform_pointer_interaction(
        &mut self,
        action: &crate::compat_app_contract::UiAction,
        click_seek_press: Option<WaveformClickSeekPress>,
    ) {
        self.waveform_drag_mode =
            super::input::waveform_drag_mode_for_action(action).or_else(|| {
                click_seek_press.and_then(|press| {
                    matches!(
                        action,
                        crate::compat_app_contract::UiAction::ClearWaveformSelection
                            | crate::compat_app_contract::UiAction::ClearWaveformEditSelection
                            | crate::compat_app_contract::UiAction::ClearWaveformSelections
                    )
                    .then_some(
                        super::input::WaveformPointerDragMode::Selection {
                            anchor_micros: press.position_nanos,
                            boundary_lock: None,
                        },
                    )
                })
            });
        self.waveform_click_seek_press = click_seek_press;
        if self.waveform_drag_mode.is_some() {
            self.shell_state.clear_waveform_hover_feedback();
        }
        self.selection_drag_active = matches!(
            action,
            crate::compat_app_contract::UiAction::StartWaveformSelectionDrag { .. }
        );
        #[cfg(target_os = "windows")]
        if self.selection_drag_active {
            info!("radiant external drag: waveform selection drag session started");
        }
    }
}
