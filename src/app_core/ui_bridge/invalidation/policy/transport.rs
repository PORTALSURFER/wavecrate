use super::{InvalidationReason, InvalidationSource};
use crate::app_core::actions::{NativeOptionsAction, NativeUiAction};

pub(super) fn can_skip_projection_cache_invalidation(action: &NativeUiAction) -> bool {
    matches!(action, |NativeUiAction::Options(
        NativeOptionsAction::SetVolume { .. },
    )| NativeUiAction::Options(
        NativeOptionsAction::CommitVolumeSetting
    ))
}

pub(super) fn classify_dirty_source(
    action: &NativeUiAction,
) -> Option<(InvalidationSource, InvalidationReason)> {
    match action {
        NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayCompareAnchor,
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayFromStart,
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayFromCurrentPlayhead,
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayFromWaveformCursor,
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayWaveformAtPrecise { .. },
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::ToggleTransport,
        )
        | NativeUiAction::Options(
            crate::app_core::actions::NativeOptionsAction::ToggleLoopPlayback,
        )
        | NativeUiAction::Options(NativeOptionsAction::SetVolume { .. })
        | NativeUiAction::Options(NativeOptionsAction::CommitVolumeSetting) => Some((
            InvalidationSource::Transport,
            InvalidationReason::TransportAction,
        )),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetCompareAnchorFromFocusedBrowserSample,
        ) => Some((
            InvalidationSource::Transport,
            InvalidationReason::TransportAction,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::actions::{NativeBrowserAction, NativeTransportAction};

    #[test]
    fn volume_actions_skip_broad_projection_cache_invalidation() {
        let action = NativeUiAction::Options(NativeOptionsAction::SetVolume { value_milli: 500 });

        assert!(can_skip_projection_cache_invalidation(&action));
        assert_eq!(
            classify_dirty_source(&action),
            Some((
                InvalidationSource::Transport,
                InvalidationReason::TransportAction
            ))
        );
    }

    #[test]
    fn transport_and_compare_anchor_actions_dirty_transport_source() {
        let actions = [
            NativeUiAction::Transport(NativeTransportAction::ToggleTransport),
            NativeUiAction::Browser(NativeBrowserAction::SetCompareAnchorFromFocusedBrowserSample),
        ];

        for action in actions {
            assert_eq!(
                classify_dirty_source(&action),
                Some((
                    InvalidationSource::Transport,
                    InvalidationReason::TransportAction
                ))
            );
        }
    }
}
