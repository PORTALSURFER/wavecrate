#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum AudioSettingsDropdown {
    Backend,
    Output,
    SampleRate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) enum AppSettingsTab {
    General,
    #[default]
    AudioEngine,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) enum GlobalStorageUsageState {
    #[default]
    NotLoaded,
    Loading,
    Ready(wavecrate::app_dirs::GlobalStorageUsage),
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SampleNameViewMode {
    DiskFilename,
    MetadataLabel,
}

impl SampleNameViewMode {
    pub(in crate::native_app) fn toggled(self) -> Self {
        match self {
            Self::DiskFilename => Self::MetadataLabel,
            Self::MetadataLabel => Self::DiskFilename,
        }
    }
}
