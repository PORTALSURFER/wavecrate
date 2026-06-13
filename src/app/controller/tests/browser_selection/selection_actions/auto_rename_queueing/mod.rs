use super::*;

use crate::app::controller::jobs::SampleAutoRenameProgress;
use crate::app::controller::state::runtime::{
    AutoRenameBatchRowSnapshot, AutoRenameBatchRowState, BrowserRenameIntentKey,
};
use crate::app_core::ui_projection::project_waveform_model;

/// Seed a loaded waveform with a reusable UI projection signature.
fn seed_stable_waveform_projection(
    controller: &mut crate::app::controller::AppController,
    source: &crate::sample_sources::SampleSource,
    relative_path: &Path,
) {
    controller
        .load_waveform_for_selection(source, relative_path)
        .expect("waveform should load");
    controller.ui.waveform.image = Some(crate::waveform::WaveformImage {
        size: [2, 1],
        pixels: vec![
            crate::waveform::WaveformRgba::from_rgba_unmultiplied(10, 20, 30, 255),
            crate::waveform::WaveformRgba::from_rgba_unmultiplied(40, 50, 60, 255),
        ],
    });
    controller.ui.waveform.waveform_image_signature = Some(77);
    controller.set_waveform_render_meta_for_tests(Some(
        crate::app::controller::WaveformRenderMeta {
            view_start: controller.ui.waveform.view.start,
            view_end: controller.ui.waveform.view.end,
            size: controller.sample_view.waveform.size,
            samples_len: 2,
            texture_width: 2,
            channel_view: crate::waveform::WaveformChannelView::Mono,
            channels: 1,
            edit_fade: None,
            transient_visual_token: None,
        },
    ));
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.375;
}

/// Build one successful auto-rename result row for controller application tests.
fn auto_rename_success(
    old_relative: &str,
    new_relative: &str,
) -> crate::app::controller::jobs::SampleAutoRenameSuccess {
    crate::app::controller::jobs::SampleAutoRenameSuccess {
        old_relative: PathBuf::from(old_relative),
        new_relative: PathBuf::from(new_relative),
        entry: sample_entry(new_relative, crate::sample_sources::Rating::NEUTRAL),
        resume_playback: false,
        resume_looped: false,
        resume_start_override: None,
    }
}

/// Apply a completed auto-rename batch directly through the file-op result path.
fn apply_auto_rename_successes(
    controller: &mut crate::app::controller::AppController,
    source_id: crate::sample_sources::SourceId,
    requested_paths: Vec<PathBuf>,
    renamed: Vec<crate::app::controller::jobs::SampleAutoRenameSuccess>,
) {
    controller.apply_file_op_result(
        crate::app::controller::jobs::FileOpResult::SampleAutoRename(
            crate::app::controller::jobs::SampleAutoRenameResult {
                source_id,
                requested_paths,
                renamed,
                skipped: Vec::new(),
                errors: Vec::new(),
            },
        ),
    );
}

mod batch_lifecycle;
mod request_lifecycle;
mod selection_scope;
mod waveform_remap;

fn assert_auto_rename_row(
    row: &AutoRenameBatchRowSnapshot,
    requested: &str,
    current: &str,
    state: AutoRenameBatchRowState,
) {
    assert_eq!(row.requested_path, PathBuf::from(requested));
    assert_eq!(row.current_path, PathBuf::from(current));
    assert_eq!(row.state, state);
}
