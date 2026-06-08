use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

const PLAYBACK_CURSOR_OVERLAY_KEY: u64 = 0x706c_6179_6375_7273;
const PLAYBACK_CURSOR_OVERLAY_FPS: u32 = 60;

pub(in crate::native_app) fn native_app_presentation()
-> ui::Presentation<NativeAppState, GuiMessage> {
    ui::presentation()
        .frame_clock(
            ui::FrameClock::message(GuiMessage::Frame)
                .when(|state: &mut NativeAppState| state.frame_message_animation_active())
                .repaint_scope(
                    |state: &mut NativeAppState| state.frame_repaint_scope_before_update(),
                    |state, scope| state.frame_can_use_paint_only(scope),
                ),
        )
        .transient_overlay(
            ui::TransientOverlay::new(PLAYBACK_CURSOR_OVERLAY_KEY)
                .paint_only()
                .when(|state: &mut NativeAppState| state.waveform.is_playing())
                .fps(PLAYBACK_CURSOR_OVERLAY_FPS)
                .paint(NativeAppState::paint_playback_overlay),
        )
}

pub(in crate::native_app) fn apply_message_with_presentation_repaint(
    state: &mut NativeAppState,
    message: GuiMessage,
    context: &mut ui::UpdateContext<GuiMessage>,
) {
    let frame_message = matches!(message, GuiMessage::Frame);
    state.apply_message(message, context);
    if !frame_message {
        context.request_repaint();
    }
}
