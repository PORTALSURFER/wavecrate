#[cfg(test)]
use super::super::format::CachedPlaybackCacheFile;
use super::super::prune::PruneWaveformCacheOutcome;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_app::waveform::audio_file::waveform_cache) enum StoreWriteOutcome {
    Completed(StoreWriteReport),
    StaleInput(StoreWriteReport),
    SerializeFailed(StoreWriteReport),
    TempWriteFailed(StoreWriteReport),
    RenameFailed(StoreWriteReport),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::native_app::waveform::audio_file::waveform_cache) struct StoreWriteReport {
    pub(super) initial_marker: MarkerUpdateOutcome,
    pub(super) sidecar: PlaybackSidecarOutcome,
    pub(super) stale_sidecar_cleanup: Option<FileCleanupOutcome>,
    pub(super) ready_marker: Option<MarkerUpdateOutcome>,
    pub(super) prune: Option<PruneWaveformCacheOutcome>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::native_app::waveform::audio_file::waveform_cache) enum MarkerUpdateOutcome {
    Written,
    Removed,
    #[default]
    AlreadyMissing,
    WriteFailed,
    RemoveFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_app::waveform::audio_file::waveform_cache) enum FileCleanupOutcome {
    Removed,
    AlreadyMissing,
    Failed,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::native_app::waveform::audio_file::waveform_cache) enum PlaybackSidecarOutcome {
    #[cfg(test)]
    Stored(CachedPlaybackCacheFile),
    #[cfg(test)]
    TooLarge,
    #[cfg(test)]
    SampleBytesOverflow,
    #[cfg(test)]
    CreateTempFailed,
    #[cfg(test)]
    WriteTempFailed,
    #[cfg(test)]
    FlushTempFailed,
    #[cfg(test)]
    RenameFailed,
    #[default]
    NoPlaybackPayload,
}

impl PlaybackSidecarOutcome {
    #[cfg(test)]
    pub(super) fn cache_file(&self) -> Option<CachedPlaybackCacheFile> {
        match self {
            Self::Stored(cache_file) => Some(cache_file.clone()),
            Self::TooLarge
            | Self::SampleBytesOverflow
            | Self::CreateTempFailed
            | Self::WriteTempFailed
            | Self::FlushTempFailed
            | Self::RenameFailed
            | Self::NoPlaybackPayload => None,
        }
    }
}

impl StoreWriteOutcome {
    pub(in crate::native_app::waveform::audio_file::waveform_cache) fn report(
        &self,
    ) -> &StoreWriteReport {
        match self {
            Self::Completed(report)
            | Self::StaleInput(report)
            | Self::SerializeFailed(report)
            | Self::TempWriteFailed(report)
            | Self::RenameFailed(report) => report,
        }
    }

    pub(in crate::native_app::waveform::audio_file::waveform_cache) fn kind(&self) -> &'static str {
        match self {
            Self::Completed(_) => "completed",
            Self::StaleInput(_) => "stale_input",
            Self::SerializeFailed(_) => "serialize_failed",
            Self::TempWriteFailed(_) => "temp_write_failed",
            Self::RenameFailed(_) => "rename_failed",
        }
    }
}

impl StoreWriteReport {
    pub(in crate::native_app::waveform::audio_file::waveform_cache) fn has_failures(&self) -> bool {
        self.initial_marker == MarkerUpdateOutcome::WriteFailed
            || self.initial_marker == MarkerUpdateOutcome::RemoveFailed
            || playback_sidecar_has_failures(&self.sidecar)
            || self.stale_sidecar_cleanup == Some(FileCleanupOutcome::Failed)
            || matches!(
                self.ready_marker,
                Some(MarkerUpdateOutcome::WriteFailed | MarkerUpdateOutcome::RemoveFailed)
            )
            || self.prune.is_some_and(|prune| prune.has_failures())
    }
}

#[cfg(not(test))]
fn playback_sidecar_has_failures(_outcome: &PlaybackSidecarOutcome) -> bool {
    false
}

#[cfg(test)]
fn playback_sidecar_has_failures(outcome: &PlaybackSidecarOutcome) -> bool {
    matches!(
        outcome,
        PlaybackSidecarOutcome::CreateTempFailed
            | PlaybackSidecarOutcome::WriteTempFailed
            | PlaybackSidecarOutcome::FlushTempFailed
            | PlaybackSidecarOutcome::RenameFailed
            | PlaybackSidecarOutcome::SampleBytesOverflow
    )
}

impl PruneWaveformCacheOutcome {
    fn has_failures(&self) -> bool {
        self.read_dir_failed
            || self.stale_temp_remove_failed > 0
            || self.orphan_sidecar_remove_failed > 0
            || self.orphan_marker_remove_failed > 0
            || self.cache_remove_failed > 0
            || self.companion_remove_failed > 0
    }
}
