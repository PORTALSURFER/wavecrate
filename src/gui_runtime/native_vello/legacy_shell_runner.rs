use super::*;

pub(in crate::gui_runtime::native_vello) struct NativeVelloRunner<B: NativeAppBridge> {
    pub(super) options: NativeRunOptions,
    pub(super) bridge: B,
    pub(super) repaint_event_pending: Arc<AtomicBool>,
    /// Enable bridge-driven static segment rebuild gating.
    pub(super) incremental_frame_pipeline: bool,
    pub(super) model: Arc<AppModel>,
    pub(super) window_id: Option<WindowId>,
    pub(super) window: Option<Arc<Window>>,
    pub(super) render_ctx: Option<RenderContext>,
    pub(super) render_surface: Option<RenderSurface<'static>>,
    pub(super) renderer: Option<Renderer>,
    pub(super) redraw_requested: bool,
    /// Retained static scene primitives (layout and stable browser).
    pub(super) frame_cache: NativeViewFrame,
    /// Retained per-segment static frame fragments.
    pub(super) static_segment_frame_cache: StaticFrameSegments,
    /// Retained immutable static segment nodes for diff-based rebuild planning.
    pub(super) static_segment_graph: StaticSegmentStateGraph,
    /// Retained per-segment static encoded scenes.
    pub(super) static_segment_scene_cache: StaticSegmentSceneCache,
    /// Retained hover/editor overlay primitives.
    pub(super) hover_overlay_frame_cache: NativeViewFrame,
    /// Retained focus-emphasis overlay primitives.
    pub(super) focus_overlay_frame_cache: NativeViewFrame,
    /// Retained modal/popover overlay primitives.
    pub(super) modal_overlay_frame_cache: NativeViewFrame,
    /// Retained waveform-motion overlay primitives (cursor/playhead/hover marker).
    pub(super) waveform_motion_overlay_frame_cache: NativeViewFrame,
    /// Retained chrome-motion overlay primitives (toolbar/tabs/status/lamp pulse).
    pub(super) chrome_motion_overlay_frame_cache: NativeViewFrame,
    /// Full scene sent to Vello after combining static + overlay scenes.
    pub(super) scene: Scene,
    /// Cached encoded static scene.
    pub(super) static_scene: Scene,
    /// Cached encoded hover/editor overlay scene.
    pub(super) hover_overlay_scene: Scene,
    /// Cached encoded focus-emphasis overlay scene.
    pub(super) focus_overlay_scene: Scene,
    /// Cached encoded modal/popover overlay scene.
    pub(super) modal_overlay_scene: Scene,
    /// Cached encoded composite for hover/editor and focus-emphasis overlays.
    pub(super) state_overlay_scene: Scene,
    /// Cached encoded waveform-motion overlay scene.
    pub(super) waveform_motion_overlay_scene: Scene,
    /// Cached encoded chrome-motion overlay scene.
    pub(super) chrome_motion_overlay_scene: Scene,
    /// Cached encoded composite for waveform/chrome motion overlays.
    pub(super) motion_overlay_scene: Scene,
    /// Retained blobs for repeated image draw payload uploads.
    pub(super) image_upload_blob_cache: HashMap<ImageUploadBlobCacheKey, Blob<u8>>,
    /// Recency queue for bounded retained image-upload blob eviction.
    pub(super) image_upload_blob_cache_order: VecDeque<ImageUploadBlobCacheKey>,
    /// Last hover-overlay fingerprint used for cache-skip checks.
    pub(super) hover_overlay_fingerprint: Option<HoverOverlayCacheFingerprint>,
    /// Last focus-overlay fingerprint used for cache-skip checks.
    pub(super) focus_overlay_fingerprint: Option<FocusOverlayCacheFingerprint>,
    /// Last modal-overlay fingerprint used for cache-skip checks.
    pub(super) modal_overlay_fingerprint: Option<ModalOverlayCacheFingerprint>,
    /// Last waveform-motion fingerprint used for cache-skip checks.
    pub(super) waveform_motion_overlay_fingerprint: Option<WaveformMotionOverlayCacheFingerprint>,
    /// Last chrome-motion fingerprint used for cache-skip checks.
    pub(super) chrome_motion_overlay_fingerprint: Option<ChromeMotionOverlayCacheFingerprint>,
    /// Cached latest motion-only model for lightweight overlay rebuilds.
    pub(super) motion_model: Option<NativeMotionModel>,
    /// Whether the active bridge supports `project_motion_model`.
    pub(super) motion_model_supported: bool,
    /// Latest bridge-provided static segment revision snapshot.
    pub(super) segment_revisions: SegmentRevisions,
    /// Whether the bridge reports non-zero static segment revisions.
    pub(super) segment_revisions_supported: bool,
    /// Whether we already forced one rebuild for zero-revision bridge fallbacks.
    pub(super) missing_segment_revision_fallback_applied: bool,
    pub(super) text_renderer: NativeTextRenderer,
    pub(super) style_cache: Option<StyleTokens>,
    pub(super) frame_state: NativeVelloFrameState,
    pub(super) layout_runtime: ShellLayoutRuntime,
    pub(super) shell_layout: Option<Arc<ShellLayout>>,
    pub(super) shell_state: NativeShellState,
    pub(super) clear_color: Rgba8,
    pub(super) cursor_icon: CursorIcon,
    pub(super) last_cursor: Option<Point>,
    pub(super) pending_cursor: Option<Point>,
    /// Pending first keypress for a multi-step hotkey chord.
    pub(super) pending_hotkey_chord: Option<KeyPress>,
    /// Latest queued top-bar volume update in normalized milli space.
    pub(super) pending_volume_milli: Option<u16>,
    /// Active waveform drag mode while primary pointer is held on waveform.
    pub(super) waveform_drag_mode: Option<WaveformPointerDragMode>,
    /// Whether the next waveform view-based interaction must refresh local bounds.
    pub(super) waveform_view_refresh_pending: bool,
    /// Exact press snapshot used for plain waveform click-to-seek release handling.
    pub(super) waveform_click_seek_press: Option<WaveformClickSeekPress>,
    /// Deferred browser-list row press captured until click-vs-drag resolution.
    pub(super) pending_browser_row_press: Option<PendingBrowserRowPress>,
    /// Active browser browser-item drag state for primary pointer movement.
    pub(super) content_item_drag: Option<ContentItemDragState>,
    /// Whether a waveform-selection export drag is currently active.
    pub(super) selection_drag_active: bool,
    /// Last waveform drag action emitted for pointer-move dedupe.
    pub(super) last_emitted_waveform_drag_action: Option<UiAction>,
    /// Whether spatial-browser focus drag is active for primary pointer movement.
    pub(super) spatial_focus_drag_active: bool,
    /// Last spatial browser id emitted during active focus drag.
    pub(super) last_emitted_spatial_drag_content_id: Option<String>,
    /// Active folder-scrollbar thumb drag state for primary pointer movement.
    pub(super) folder_scrollbar_drag: Option<FolderScrollbarDragState>,
    /// Active browser-list scrollbar thumb drag state for primary pointer movement.
    pub(super) browser_scrollbar_drag: Option<ContentListScrollbarDragState>,
    /// Last emitted browser-list viewport start during an active scrollbar drag.
    pub(super) last_emitted_browser_list_view_start: Option<usize>,
    /// Active waveform-scrollbar thumb drag state for primary pointer movement.
    pub(super) waveform_scrollbar_drag: Option<WaveformScrollbarDragState>,
    /// Active middle-button waveform pan drag state.
    pub(super) waveform_pan_drag: Option<WaveformPanDragState>,
    /// Last emitted waveform viewport center during active drag gestures.
    pub(super) last_emitted_waveform_view_center: Option<u32>,
    pub(super) volume_drag_active: bool,
    pub(super) last_emitted_volume_milli: Option<u16>,
    pub(super) modifiers: ModifiersState,
    pub(super) text_input_target: TextInputTarget,
    pub(super) text_input_buffer: Option<String>,
    pub(super) text_editor_state: Option<SingleLineTextEditorState>,
    pub(super) active_text_field_visual_cache: Option<ActiveTextFieldVisualCacheEntry>,
    pub(super) text_input_drag_active: bool,
    pub(super) waveform_bpm_input_buffer: Option<String>,
    pub(super) clipboard: Option<arboard::Clipboard>,
    pub(super) clipboard_fallback_text: String,
    pub(super) last_redraw: Instant,
    pub(super) resumed_count: u32,
    pub(super) window_event_count: u32,
    pub(super) redraw_count: u32,
    /// Whether at least one frame has been presented to the native surface.
    pub(super) first_frame_presented: bool,
    /// Whether the window has been revealed after startup frame sequencing.
    pub(super) startup_window_visible: bool,
    /// Whether the first startup full-model pull is deferred until first present.
    pub(super) startup_model_pull_pending: bool,
    /// Whether deferred startup full-model refresh is pending completion.
    pub(super) startup_deferred_model_refresh_pending: bool,
    /// Deadline used to prevent startup reveal from stalling indefinitely.
    pub(super) startup_reveal_deadline: Option<Instant>,
    /// Startup first-paint timing profile.
    pub(super) startup_timing: StartupTimingProfile,
    pub(super) target_frame_interval: Duration,
    pub(super) focus_animation_interval: Duration,
    pub(super) idle_status_refresh_interval: Duration,
    pub(super) next_idle_status_refresh: Instant,
    pub(super) cursor_activity_redraw_interval: Duration,
    pub(super) cursor_activity_redraw_until: Option<Instant>,
    pub(super) model_refresh_count: u32,
    pub(super) profiler: NativeVelloProfiler,
}
