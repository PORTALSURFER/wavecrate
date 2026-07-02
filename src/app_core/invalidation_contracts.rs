//! App-core invalidation contracts for retained UI projection.
//!
//! The legacy controller dirty graph remains the backend during migration, but
//! app-core frame preparation and bridge policy should speak in app-core-owned
//! invalidation names. Conversion to the retained dirty graph is isolated here.

use crate::app::controller::state::runtime::{
    DerivedNodeId as LegacyDerivedNodeId, DirtyReason as LegacyDirtyReason,
};

/// Stable app-core node identifiers for incremental projection invalidation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum InvalidationNode {
    /// Source state for waveform position/selection/view edits.
    WaveformState,
    /// Source state for browser filters/search/focus/selection.
    BrowserState,
    /// Source state for map tab/query/pan/zoom selection.
    MapState,
    /// Source state for transport/volume/playback toggles.
    TransportState,
    /// Source state for status/progress/update messaging.
    StatusState,
    /// Derived waveform render input stage.
    WaveformRenderInputs,
    /// Derived waveform image signature stage.
    WaveformImageSignature,
    /// Derived browser visible row list stage.
    BrowserVisibleRows,
    /// Derived browser projection model stage.
    BrowserProjectionModel,
    /// Derived map projection input stage.
    MapProjectionInputs,
    /// Derived map projection model stage.
    MapProjectionModel,
    /// Derived status projection stage.
    StatusProjectionModel,
    /// Projection cache key invalidation stage.
    NativeAppProjectionKey,
}

impl InvalidationNode {
    /// Convert from the retained controller dirty graph node.
    pub(in crate::app_core) fn from_legacy(node: LegacyDerivedNodeId) -> Self {
        match node {
            LegacyDerivedNodeId::WaveformState => Self::WaveformState,
            LegacyDerivedNodeId::BrowserState => Self::BrowserState,
            LegacyDerivedNodeId::MapState => Self::MapState,
            LegacyDerivedNodeId::TransportState => Self::TransportState,
            LegacyDerivedNodeId::StatusState => Self::StatusState,
            LegacyDerivedNodeId::WaveformRenderInputs => Self::WaveformRenderInputs,
            LegacyDerivedNodeId::WaveformImageSignature => Self::WaveformImageSignature,
            LegacyDerivedNodeId::BrowserVisibleRows => Self::BrowserVisibleRows,
            LegacyDerivedNodeId::BrowserProjectionModel => Self::BrowserProjectionModel,
            LegacyDerivedNodeId::MapProjectionInputs => Self::MapProjectionInputs,
            LegacyDerivedNodeId::MapProjectionModel => Self::MapProjectionModel,
            LegacyDerivedNodeId::StatusProjectionModel => Self::StatusProjectionModel,
            LegacyDerivedNodeId::NativeAppProjectionKey => Self::NativeAppProjectionKey,
        }
    }

    /// Convert to the retained controller dirty graph node.
    pub(in crate::app_core) fn legacy(self) -> LegacyDerivedNodeId {
        match self {
            Self::WaveformState => LegacyDerivedNodeId::WaveformState,
            Self::BrowserState => LegacyDerivedNodeId::BrowserState,
            Self::MapState => LegacyDerivedNodeId::MapState,
            Self::TransportState => LegacyDerivedNodeId::TransportState,
            Self::StatusState => LegacyDerivedNodeId::StatusState,
            Self::WaveformRenderInputs => LegacyDerivedNodeId::WaveformRenderInputs,
            Self::WaveformImageSignature => LegacyDerivedNodeId::WaveformImageSignature,
            Self::BrowserVisibleRows => LegacyDerivedNodeId::BrowserVisibleRows,
            Self::BrowserProjectionModel => LegacyDerivedNodeId::BrowserProjectionModel,
            Self::MapProjectionInputs => LegacyDerivedNodeId::MapProjectionInputs,
            Self::MapProjectionModel => LegacyDerivedNodeId::MapProjectionModel,
            Self::StatusProjectionModel => LegacyDerivedNodeId::StatusProjectionModel,
            Self::NativeAppProjectionKey => LegacyDerivedNodeId::NativeAppProjectionKey,
        }
    }
}

/// Source-state buckets used by app-core invalidation policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InvalidationSource {
    Waveform,
    Browser,
    Map,
    Transport,
    Status,
}

impl InvalidationSource {
    /// Return the full invalidation node represented by this source bucket.
    pub(crate) fn node(self) -> InvalidationNode {
        match self {
            Self::Waveform => InvalidationNode::WaveformState,
            Self::Browser => InvalidationNode::BrowserState,
            Self::Map => InvalidationNode::MapState,
            Self::Transport => InvalidationNode::TransportState,
            Self::Status => InvalidationNode::StatusState,
        }
    }

    /// Convert to the retained controller dirty graph source node.
    pub(in crate::app_core) fn legacy(self) -> LegacyDerivedNodeId {
        self.node().legacy()
    }
}

/// Reason categories used when marking app-core invalidation sources dirty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InvalidationReason {
    /// Dirty due to waveform overlay mutations (cursor/selection/seek only).
    WaveformOverlayAction,
    /// Dirty due to waveform view mutations that can change rendered pixels.
    WaveformViewAction,
    /// Dirty due to browser interaction/action flow.
    BrowserAction,
    /// Dirty due to map interaction/action flow.
    MapAction,
    /// Dirty due to transport interaction/action flow.
    TransportAction,
    /// Dirty due to status/progress/prompt/update action flow.
    StatusAction,
    /// Dirty due to conservative broad invalidation.
    BroadInvalidation,
}

impl InvalidationReason {
    /// Convert from a retained controller dirty graph reason.
    pub(in crate::app_core) fn from_legacy(reason: LegacyDirtyReason) -> Self {
        match reason {
            LegacyDirtyReason::WaveformOverlayAction => Self::WaveformOverlayAction,
            LegacyDirtyReason::WaveformViewAction => Self::WaveformViewAction,
            LegacyDirtyReason::BrowserAction => Self::BrowserAction,
            LegacyDirtyReason::MapAction => Self::MapAction,
            LegacyDirtyReason::TransportAction => Self::TransportAction,
            LegacyDirtyReason::StatusAction => Self::StatusAction,
            LegacyDirtyReason::BroadInvalidation => Self::BroadInvalidation,
        }
    }

    /// Convert to the retained controller dirty graph reason.
    pub(in crate::app_core) fn legacy(self) -> LegacyDirtyReason {
        match self {
            Self::WaveformOverlayAction => LegacyDirtyReason::WaveformOverlayAction,
            Self::WaveformViewAction => LegacyDirtyReason::WaveformViewAction,
            Self::BrowserAction => LegacyDirtyReason::BrowserAction,
            Self::MapAction => LegacyDirtyReason::MapAction,
            Self::TransportAction => LegacyDirtyReason::TransportAction,
            Self::StatusAction => LegacyDirtyReason::StatusAction,
            Self::BroadInvalidation => LegacyDirtyReason::BroadInvalidation,
        }
    }
}
