use crate::app_core::app_api::state::BrowserDuplicateCleanupState;

/// Resolve one inline browser metadata label for the sample lane.
///
/// Rating text is intentionally omitted because the row already renders keep/trash
/// state via the right-edge indicator rectangles.
pub(in crate::app_core::native_shell::browser_projection) fn browser_bucket_label(
    bpm_value: Option<f32>,
    looped: bool,
    long_sample_mark: bool,
) -> String {
    let mut tags = Vec::new();
    if let Some(bpm) = bpm_value {
        tags.push(format_bpm_badge_label(bpm));
    }
    if looped {
        tags.push(String::from("LOOP"));
    }
    if long_sample_mark {
        tags.push(String::from("LONG"));
    }
    tags.join(" · ")
}

/// Append transient duplicate-cleanup badges to one browser-row metadata label.
pub(super) fn browser_duplicate_cleanup_bucket_label(
    duplicate_cleanup: Option<&BrowserDuplicateCleanupState>,
    absolute_index: usize,
    base_label: &str,
) -> String {
    let Some(cleanup) = duplicate_cleanup else {
        return base_label.to_owned();
    };
    let mut tags = Vec::new();
    if !base_label.is_empty() {
        tags.push(base_label.to_owned());
    }
    if cleanup.is_anchor(absolute_index) {
        tags.push(String::from("ANCHOR"));
    } else if cleanup.is_kept(absolute_index) {
        tags.push(String::from("KEEP"));
    }
    tags.join(" · ")
}

/// Format one BPM metadata label for inline browser-row display.
fn format_bpm_badge_label(bpm: f32) -> String {
    if !bpm.is_finite() || bpm <= 0.0 {
        return String::new();
    }
    let rounded = bpm.round();
    if (bpm - rounded).abs() < 0.05 {
        format!("{rounded:.0} BPM")
    } else {
        format!("{bpm:.1} BPM")
    }
}
