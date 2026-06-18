mod cache_facade;
mod construction;
mod diagnostics;
mod downmix;
mod extraction;
mod file_io;
mod loader;
mod model;
mod progress;
mod signal_summary;
mod visual_bands;
mod wav_decode;
mod wav_format;
mod wav_summary;
mod wav_summary_builder;
mod wav_summary_hound;
mod waveform_cache;

pub(in crate::native_app) use cache_facade::{
    cached_waveform_file_exists, cached_waveform_file_playback_ready_exists,
    flush_background_waveform_cache_stores_for_shutdown, load_cached_waveform_file_for_playback,
};
#[cfg(test)]
pub(in crate::native_app) use cache_facade::{
    store_cached_waveform_file_for_tests, store_summary_only_cached_waveform_file_for_tests,
};
#[cfg(test)]
pub(super) use construction::synthetic_waveform_file;
#[cfg(test)]
pub(in crate::native_app) use construction::test_waveform_file_from_mono_samples;
#[cfg(test)]
pub(super) use construction::waveform_file_from_mono_samples;
pub(super) use construction::{
    content_revision_for_audio_bytes, empty_waveform_file, gain_preview_for_selection,
    waveform_file_from_mono_samples_with_progress_and_cancel,
};
#[cfg(test)]
pub(super) use downmix::downmix_to_mono;
pub(super) use downmix::downmix_to_mono_with_progress_and_cancel;
pub(super) use extraction::{extract_wav_range_to_folder, extract_wav_range_to_sibling};
#[cfg(test)]
pub(super) use loader::load_waveform_file;
#[cfg(test)]
pub(super) use loader::load_waveform_file_with_progress_cancel_and_playback_ready;
pub(super) use loader::{
    is_wav_path, load_waveform_file_for_foreground_audition,
    load_waveform_file_with_progress_and_cancel, should_use_file_backed_wav_decode,
};
pub(in crate::native_app) use model::{
    PersistedPlaybackCacheFile, WaveformFile, WaveformPlaybackReady,
};
pub(super) use progress::{cooperate_with_ui, report_phase_progress_throttled};
#[cfg(test)]
pub(super) use visual_bands::split_frequency_bands;
