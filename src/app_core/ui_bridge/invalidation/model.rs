use crate::app::controller::state::runtime::{DerivedNodeId, DirtyReason};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InvalidationSource {
    Waveform,
    Browser,
    Map,
    Transport,
    Status,
}

impl InvalidationSource {
    pub(in crate::app_core::ui_bridge) fn legacy(self) -> DerivedNodeId {
        match self {
            Self::Waveform => DerivedNodeId::WaveformState,
            Self::Browser => DerivedNodeId::BrowserState,
            Self::Map => DerivedNodeId::MapState,
            Self::Transport => DerivedNodeId::TransportState,
            Self::Status => DerivedNodeId::StatusState,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InvalidationReason {
    WaveformOverlayAction,
    WaveformViewAction,
    BrowserAction,
    MapAction,
    TransportAction,
    StatusAction,
    BroadInvalidation,
}

impl InvalidationReason {
    pub(in crate::app_core::ui_bridge) fn from_legacy(reason: DirtyReason) -> Self {
        match reason {
            DirtyReason::WaveformOverlayAction => Self::WaveformOverlayAction,
            DirtyReason::WaveformViewAction => Self::WaveformViewAction,
            DirtyReason::BrowserAction => Self::BrowserAction,
            DirtyReason::MapAction => Self::MapAction,
            DirtyReason::TransportAction => Self::TransportAction,
            DirtyReason::StatusAction => Self::StatusAction,
            DirtyReason::BroadInvalidation => Self::BroadInvalidation,
        }
    }

    pub(in crate::app_core::ui_bridge) fn legacy(self) -> DirtyReason {
        match self {
            Self::WaveformOverlayAction => DirtyReason::WaveformOverlayAction,
            Self::WaveformViewAction => DirtyReason::WaveformViewAction,
            Self::BrowserAction => DirtyReason::BrowserAction,
            Self::MapAction => DirtyReason::MapAction,
            Self::TransportAction => DirtyReason::TransportAction,
            Self::StatusAction => DirtyReason::StatusAction,
            Self::BroadInvalidation => DirtyReason::BroadInvalidation,
        }
    }
}
