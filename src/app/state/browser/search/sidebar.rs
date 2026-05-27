use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// File-format facets available to the sidebar filter model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BrowserFormatFacet {
    /// Supported WAV-family rows.
    Wav,
}

/// Bit-depth facets available to the sidebar filter model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BrowserBitDepthFacet {
    /// Bit-depth metadata is not currently indexed for the row.
    Unavailable,
}

/// Channel-count facets available to the sidebar filter model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BrowserChannelFacet {
    /// Mono rows when channel metadata becomes available.
    Mono,
    /// Stereo rows when channel metadata becomes available.
    Stereo,
    /// Multi-channel rows when channel metadata becomes available.
    Multi,
    /// Channel metadata is not currently indexed for the row.
    Unavailable,
}

/// BPM facets available to the sidebar filter model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BrowserBpmFacet {
    /// Rows without persisted BPM metadata.
    Unknown,
    /// Persisted BPM below 90.
    Slow,
    /// Persisted BPM from 90 up to 130.
    Mid,
    /// Persisted BPM at or above 130.
    Fast,
}

impl BrowserBpmFacet {
    /// Classify one optional BPM value into the sidebar BPM facet.
    pub fn from_bpm(bpm: Option<f32>) -> Self {
        let Some(bpm) = bpm.filter(|value| value.is_finite() && *value > 0.0) else {
            return Self::Unknown;
        };
        if bpm < 90.0 {
            Self::Slow
        } else if bpm < 130.0 {
            Self::Mid
        } else {
            Self::Fast
        }
    }
}

/// Musical-key facets available to the sidebar filter model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BrowserKeyFacet {
    /// Key metadata is unknown because no stable key analyzer exists yet.
    Unknown,
}

/// Sidebar filter facet identifier used by actions and automation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BrowserSidebarFilterFacet {
    /// File format facet.
    Format,
    /// Bit-depth facet.
    BitDepth,
    /// Channel count facet.
    Channels,
    /// BPM bucket facet.
    Bpm,
    /// Musical key facet.
    Key,
}

/// Sidebar filter option payload used by UI actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BrowserSidebarFilterOption {
    /// WAV format option.
    Format(BrowserFormatFacet),
    /// Bit-depth option.
    BitDepth(BrowserBitDepthFacet),
    /// Channel-count option.
    Channels(BrowserChannelFacet),
    /// BPM option.
    Bpm(BrowserBpmFacet),
    /// Musical-key option.
    Key(BrowserKeyFacet),
}

impl BrowserSidebarFilterOption {
    /// Return the facet owned by this option.
    pub fn facet(self) -> BrowserSidebarFilterFacet {
        match self {
            Self::Format(_) => BrowserSidebarFilterFacet::Format,
            Self::BitDepth(_) => BrowserSidebarFilterFacet::BitDepth,
            Self::Channels(_) => BrowserSidebarFilterFacet::Channels,
            Self::Bpm(_) => BrowserSidebarFilterFacet::Bpm,
            Self::Key(_) => BrowserSidebarFilterFacet::Key,
        }
    }
}

/// Browser sidebar filter state shared by projection and visible-row filtering.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BrowserSidebarFilterState {
    /// Selected file-format facets.
    pub formats: BTreeSet<BrowserFormatFacet>,
    /// Selected bit-depth facets.
    pub bit_depths: BTreeSet<BrowserBitDepthFacet>,
    /// Selected channel-count facets.
    pub channels: BTreeSet<BrowserChannelFacet>,
    /// Selected BPM facets.
    pub bpms: BTreeSet<BrowserBpmFacet>,
    /// Selected key facets.
    pub keys: BTreeSet<BrowserKeyFacet>,
}

impl BrowserSidebarFilterState {
    /// Return whether any sidebar facet is actively filtering rows.
    pub fn is_empty(&self) -> bool {
        self.formats.is_empty()
            && self.bit_depths.is_empty()
            && self.channels.is_empty()
            && self.bpms.is_empty()
            && self.keys.is_empty()
    }

    /// Return whether the active facet state needs BPM metadata.
    pub fn needs_bpm_metadata(&self) -> bool {
        !self.bpms.is_empty()
    }

    /// Toggle one sidebar filter option.
    pub fn toggle(&mut self, option: BrowserSidebarFilterOption, additive: bool) -> bool {
        match option {
            BrowserSidebarFilterOption::Format(value) => {
                toggle_filter_value(&mut self.formats, value, additive)
            }
            BrowserSidebarFilterOption::BitDepth(value) => {
                toggle_filter_value(&mut self.bit_depths, value, additive)
            }
            BrowserSidebarFilterOption::Channels(value) => {
                toggle_filter_value(&mut self.channels, value, additive)
            }
            BrowserSidebarFilterOption::Bpm(value) => {
                toggle_filter_value(&mut self.bpms, value, additive)
            }
            BrowserSidebarFilterOption::Key(value) => {
                toggle_filter_value(&mut self.keys, value, additive)
            }
        }
    }

    /// Clear every option under one sidebar facet.
    pub fn clear_facet(&mut self, facet: BrowserSidebarFilterFacet) -> bool {
        match facet {
            BrowserSidebarFilterFacet::Format => clear_filter_values(&mut self.formats),
            BrowserSidebarFilterFacet::BitDepth => clear_filter_values(&mut self.bit_depths),
            BrowserSidebarFilterFacet::Channels => clear_filter_values(&mut self.channels),
            BrowserSidebarFilterFacet::Bpm => clear_filter_values(&mut self.bpms),
            BrowserSidebarFilterFacet::Key => clear_filter_values(&mut self.keys),
        }
    }

    /// Return whether one row is accepted by all active sidebar facets.
    pub fn accepts_path_and_bpm(&self, relative_path: &Path, bpm: Option<f32>) -> bool {
        let format_ok = self.formats.is_empty()
            || self.formats.iter().any(|facet| match facet {
                BrowserFormatFacet::Wav => relative_path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("wav")),
            });
        let bit_depth_ok = self.bit_depths.is_empty()
            || self.bit_depths.contains(&BrowserBitDepthFacet::Unavailable);
        let channels_ok =
            self.channels.is_empty() || self.channels.contains(&BrowserChannelFacet::Unavailable);
        let bpm_ok = self.bpms.is_empty() || self.bpms.contains(&BrowserBpmFacet::from_bpm(bpm));
        let key_ok = self.keys.is_empty() || self.keys.contains(&BrowserKeyFacet::Unknown);
        format_ok && bit_depth_ok && channels_ok && bpm_ok && key_ok
    }
}

/// Toggle one value inside a set while supporting single-select and additive modes.
fn toggle_filter_value<T: Ord>(set: &mut BTreeSet<T>, value: T, additive: bool) -> bool {
    if additive {
        if set.remove(&value) {
            true
        } else {
            set.insert(value)
        }
    } else if set.len() == 1 && set.contains(&value) {
        set.clear();
        true
    } else {
        set.clear();
        set.insert(value)
    }
}

/// Clear one filter set and report whether it changed.
fn clear_filter_values<T>(set: &mut BTreeSet<T>) -> bool {
    if set.is_empty() {
        false
    } else {
        set.clear();
        true
    }
}
