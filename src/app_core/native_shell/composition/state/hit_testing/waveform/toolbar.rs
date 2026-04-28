use super::*;

pub(in crate::gui::native_shell::state) fn waveform_toolbar_hit_test_cache_key(
    layout: &ShellLayout,
    model: &NativeMotionModel,
    bpm_editor_active: bool,
    bpm_editor_display: Option<&str>,
) -> WaveformToolbarHitTestCacheKey {
    WaveformToolbarHitTestCacheKey {
        waveform_header_min_x: f32_to_bits(layout.waveform_header.min.x),
        waveform_header_min_y: f32_to_bits(layout.waveform_header.min.y),
        waveform_header_max_x: f32_to_bits(layout.waveform_header.max.x),
        waveform_header_max_y: f32_to_bits(layout.waveform_header.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_flags: waveform_toolbar_model_flags(model),
        tempo_label_signature: waveform_tempo_label_signature(model),
        loaded_label_signature: text_signature(model.waveform_loaded_label.as_deref()),
        waveform_loading: model.waveform_loading,
        bpm_editor_active,
        bpm_editor_display_signature: text_signature(bpm_editor_display),
        waveform_slice_count: model.waveform_slices.len().min(u32::MAX as usize) as u32,
    }
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_model_flags(
    model: &NativeMotionModel,
) -> u16 {
    let mut bits = 0u16;
    if model.waveform_channel_view == crate::app::WaveformChannelViewModel::Stereo {
        bits |= 1 << 0;
    }
    if model.waveform_normalized_audition_enabled {
        bits |= 1 << 1;
    }
    if model.waveform_bpm_snap_enabled {
        bits |= 1 << 2;
    }
    if model.waveform_relative_bpm_grid_enabled {
        bits |= 1 << 3;
    }
    if model.waveform_transient_snap_enabled {
        bits |= 1 << 4;
    }
    if model.waveform_transient_markers_enabled {
        bits |= 1 << 5;
    }
    if model.waveform_slice_mode_enabled {
        bits |= 1 << 6;
    }
    if model.waveform_loop_enabled {
        bits |= 1 << 7;
    }
    if model.waveform_loop_lock_enabled {
        bits |= 1 << 8;
    }
    if model.transport_running {
        bits |= 1 << 9;
    }
    if model.waveform_compare_anchor_available {
        bits |= 1 << 10;
    }
    bits
}

pub(in crate::gui::native_shell::state) fn waveform_tempo_label_signature(
    model: &NativeMotionModel,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.waveform_tempo_label.hash(&mut hasher);
    hasher.finish()
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
