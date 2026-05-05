use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use crate::app::controller::AppController;
use crate::app::controller::playback::audio_cache::{CacheKey, FileMetadata};
use crate::app::controller::playback::audio_loader::AudioTransientResult;
use crate::app::controller::state::audio::AudioLoadIntent;
use crate::app_dirs::ConfigBaseGuard;
use crate::sample_sources::{SampleSource, SourceId};
use crate::selection::SelectionRange;
use crate::waveform::WaveformRenderer;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

fn controller_with_audio_file(file_name: &str) -> (AppController, SampleSource, PathBuf) {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let relative_path = PathBuf::from(file_name);
    write_test_wav(&source.root.join(&relative_path), &[0.0, 0.5, -0.5, 0.25]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        file_name,
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    (controller, source, relative_path)
}

fn load_selection_waveform(controller: &mut AppController, source: &SampleSource, path: &Path) {
    controller
        .load_waveform_for_selection(source, path)
        .expect("waveform load");
}

fn transient_result(
    source_id: SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
    cache_token: u64,
    transients: Arc<[f32]>,
    stretched: bool,
) -> AudioTransientResult {
    AudioTransientResult {
        request_id: 1,
        source_id,
        relative_path: relative_path.to_path_buf(),
        metadata,
        cache_token,
        transients,
        stretched,
    }
}

mod cache_reuse;
mod transient_updates;
