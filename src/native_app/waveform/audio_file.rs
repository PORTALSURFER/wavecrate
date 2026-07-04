mod cache_facade;
mod construction;
mod diagnostics;
mod downmix;
mod extraction;
#[cfg(test)]
mod extraction_tests;
mod file_io;
mod loader;
mod model;
mod preview;
mod progress;
mod signal_summary;
mod visual_bands;
mod wav_decode;
mod wav_format;
mod wav_summary;
mod wav_summary_builder;
mod wav_summary_hound;
mod waveform_cache;

#[cfg(test)]
pub(in crate::native_app) use cache_facade::cached_waveform_file_playback_ready_exists;
pub(in crate::native_app) use cache_facade::{
    cached_waveform_file_audition_ready_exists, cached_waveform_file_exists,
    flush_background_waveform_cache_stores_for_shutdown, invalidate_persisted_waveform_cache_path,
    invalidate_persisted_waveform_cache_paths, load_cached_waveform_file_for_playback,
    load_cached_waveform_playback_descriptor_sidecar,
    mark_cached_waveform_file_source_warm_attempted, remap_persisted_waveform_cache_after_move,
};
#[cfg(test)]
pub(in crate::native_app) use cache_facade::{
    store_cached_waveform_file_for_tests, store_summary_only_cached_waveform_file_for_tests,
};
#[cfg(test)]
pub(super) use construction::synthetic_waveform_file;
#[cfg(test)]
pub(super) use construction::waveform_file_from_mono_samples;
pub(super) use construction::{
    content_revision_for_audio_bytes, empty_waveform_file, gain_preview_for_range_with_gain,
    gain_preview_for_selection, waveform_file_from_mono_samples_with_progress_and_cancel,
};
#[cfg(test)]
pub(in crate::native_app) use construction::{
    test_decoded_waveform_file_from_mono_samples, test_file_backed_waveform_file_from_mono_samples,
    test_waveform_file_from_mono_samples,
};
#[cfg(test)]
pub(super) use downmix::downmix_to_mono;
pub(super) use downmix::downmix_to_mono_with_progress_and_cancel;
pub(super) use extraction::{
    InterleavedF32FileExtractionSource, extract_interleaved_f32_file_range_to_folder,
    extract_interleaved_f32_range_to_folder, extract_wav_file_range_to_folder,
    extract_wav_range_to_folder,
};
#[cfg(test)]
pub(super) use loader::load_waveform_file;
#[cfg(test)]
pub(super) use loader::load_waveform_file_with_progress_cancel_and_playback_ready;
pub(in crate::native_app) use loader::{
    file_backed_wav_playback_descriptor, should_use_file_backed_wav_decode,
    should_use_file_backed_wav_decode_for_entry,
};
pub(super) use loader::{
    is_wav_path, load_waveform_file_for_foreground_audition,
    load_waveform_file_for_instant_audition_display,
    load_waveform_file_for_looped_foreground_audition, load_waveform_file_with_progress_and_cancel,
};
pub(in crate::native_app) use model::{
    PersistedPlaybackCacheFile, PersistedPlaybackDescriptor, WaveformFile, WaveformPlaybackReady,
};
pub(in crate::native_app) use preview::{PreviewAuditionClip, decode_wav_preview_clip};
pub(super) use progress::{cooperate_with_ui, report_phase_progress_throttled};
#[cfg(test)]
pub(super) use visual_bands::split_frequency_bands;
pub(in crate::native_app::waveform) use wav_decode::read_wav_playback_samples;
