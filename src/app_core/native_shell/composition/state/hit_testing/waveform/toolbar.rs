use super::*;

pub(in crate::gui::native_shell::state) fn waveform_toolbar_hit_test_cache_key(
    layout: &ShellLayout,
    model: &NativeMotionModel,
    bpm_editor_active: bool,
    bpm_editor_display: Option<&str>,
) -> WaveformToolbarHitTestCacheKey {
    let presentation = model.waveform_presentation();
    let raster_preview = model.waveform_image_preview();
    WaveformToolbarHitTestCacheKey {
        waveform_header_min_x: f32_to_bits(layout.waveform_header.min.x),
        waveform_header_min_y: f32_to_bits(layout.waveform_header.min.y),
        waveform_header_max_x: f32_to_bits(layout.waveform_header.max.x),
        waveform_header_max_y: f32_to_bits(layout.waveform_header.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_flags: waveform_toolbar_model_flags(model),
        tempo_label_signature: text_signature(presentation.primary_label.as_deref()),
        loaded_label_signature: text_signature(raster_preview.loaded_label.as_deref()),
        waveform_loading: raster_preview.loading,
        bpm_editor_active,
        bpm_editor_display_signature: text_signature(bpm_editor_display),
        waveform_slice_count: model.waveform_slices.len().min(u32::MAX as usize) as u32,
    }
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_model_flags(
    model: &NativeMotionModel,
) -> u16 {
    let chrome = model.signal_chrome();
    let tools = model.signal_tools();
    let presentation = model.waveform_presentation();
    let mut bits = 0u16;
    if chrome.channel_view == crate::gui::visualization::ChannelViewMode::Stereo {
        bits |= 1 << 0;
    }
    if tools.audition_enabled {
        bits |= 1 << 1;
    }
    if tools.primary_snap_enabled {
        bits |= 1 << 2;
    }
    if tools.relative_grid_enabled {
        bits |= 1 << 3;
    }
    if tools.secondary_snap_enabled {
        bits |= 1 << 4;
    }
    if tools.markers_visible {
        bits |= 1 << 5;
    }
    if tools.review_mode_enabled {
        bits |= 1 << 6;
    }
    if presentation.repeat_enabled {
        bits |= 1 << 7;
    }
    if tools.lock_enabled {
        bits |= 1 << 8;
    }
    if model.transport_running {
        bits |= 1 << 9;
    }
    if chrome.reference_anchor_available {
        bits |= 1 << 10;
    }
    bits
}

pub(in crate::gui::native_shell::state) fn text_signature(value: Option<&str>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_hover_hint(
    label: &str,
) -> Option<WaveformToolbarHoverHint> {
    match label {
        "Channel" => Some(WaveformToolbarHoverHint::ChannelView),
        "Norm" => Some(WaveformToolbarHoverHint::NormalizedAudition),
        "BPM Value" => Some(WaveformToolbarHoverHint::BpmValue),
        "BPM Snap" => Some(WaveformToolbarHoverHint::BpmSnap),
        "Rel Grid" => Some(WaveformToolbarHoverHint::RelativeBpmGrid),
        "Tr Snap" => Some(WaveformToolbarHoverHint::TransientSnap),
        "Show Tr" => Some(WaveformToolbarHoverHint::ShowTransients),
        "Slice" => Some(WaveformToolbarHoverHint::SliceMode),
        "Silence Split" => Some(WaveformToolbarHoverHint::SilenceSplit),
        "Exact Dedupe" => Some(WaveformToolbarHoverHint::ExactDedupe),
        "Clean Dups" => Some(WaveformToolbarHoverHint::CleanDuplicates),
        "Loop" => Some(WaveformToolbarHoverHint::Loop),
        "Compare" => Some(WaveformToolbarHoverHint::Compare),
        "Stop" => Some(WaveformToolbarHoverHint::Stop),
        "Play" => Some(WaveformToolbarHoverHint::Play),
        "Rec" => Some(WaveformToolbarHoverHint::Record),
        _ => None,
    }
}
