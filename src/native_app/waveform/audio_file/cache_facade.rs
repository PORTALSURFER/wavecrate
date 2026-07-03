#[cfg(test)]
pub(in crate::native_app) use super::waveform_cache::cached_waveform_file_playback_ready_exists;
pub(in crate::native_app) use super::waveform_cache::{
    cached_waveform_file_audition_ready_exists, cached_waveform_file_exists,
    flush_background_waveform_cache_stores_for_shutdown, invalidate_persisted_waveform_cache_path,
    invalidate_persisted_waveform_cache_paths, load_cached_waveform_file_for_playback,
    load_cached_waveform_playback_descriptor, mark_cached_waveform_file_source_warm_attempted,
    remap_persisted_waveform_cache_after_move,
};

#[cfg(test)]
use super::WaveformFile;

#[cfg(test)]
pub(in crate::native_app) fn store_cached_waveform_file_for_tests(file: &WaveformFile) {
    super::waveform_cache::store_cached_waveform_file(file);
}

#[cfg(test)]
pub(in crate::native_app) fn store_summary_only_cached_waveform_file_for_tests(
    file: &WaveformFile,
) {
    let mut file = file.clone();
    file.playback_samples = None;
    file.playback_cache_file = None;
    super::waveform_cache::store_cached_waveform_file(&file);
}
