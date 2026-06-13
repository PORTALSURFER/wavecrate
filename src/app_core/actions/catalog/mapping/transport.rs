use super::shared::{GuiActionKind, Kind, NativeTransportAction};

pub(super) fn transport_action_kind(action: &NativeTransportAction) -> GuiActionKind {
    match action {
        NativeTransportAction::ToggleTransport => Kind::ToggleTransport,
        NativeTransportAction::PlayCompareAnchor => Kind::PlayCompareAnchor,
        NativeTransportAction::PlayFromStart => Kind::PlayFromStart,
        NativeTransportAction::PlayFromCurrentPlayhead => Kind::PlayFromCurrentPlayhead,
        NativeTransportAction::PlayFromWaveformCursor => Kind::PlayFromWaveformCursor,
        NativeTransportAction::PlayWaveformAtPrecise { .. } => Kind::PlayWaveformAtPrecise,
        NativeTransportAction::HandleEscape => Kind::HandleEscape,
    }
}
