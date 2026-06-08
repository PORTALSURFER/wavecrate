use crate::native_app::app::{GuiMessage, NativeAppState, default_gui_shortcuts};
use crate::native_app::app_chrome::layout;
use radiant::prelude as ui;

const PLAYBACK_CURSOR_OVERLAY_KEY: u64 = 0x706c_6179_6375_7273;
const PLAYBACK_CURSOR_OVERLAY_FPS: u32 = 60;

pub(in crate::native_app) fn view(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    scene(state).into_view().fill()
}

fn scene(state: &mut NativeAppState) -> ui::Scene<GuiMessage> {
    ui::scene(layout::shell(state))
        .shortcuts(default_gui_shortcuts(state))
        .frame_clock(frame_clock())
        .overlay(playback_cursor_overlay())
}

fn frame_clock() -> ui::FrameClock<NativeAppState, GuiMessage> {
    ui::FrameClock::message(GuiMessage::Frame)
        .when(|state: &mut NativeAppState| state.frame_message_animation_active())
        .repaint_scope(
            |state: &mut NativeAppState| state.frame_repaint_scope_before_update(),
            |state, scope| state.frame_can_use_paint_only(scope),
        )
}

fn playback_cursor_overlay() -> ui::TransientOverlay<NativeAppState> {
    ui::TransientOverlay::new(PLAYBACK_CURSOR_OVERLAY_KEY)
        .paint_only()
        .when(|state: &mut NativeAppState| state.waveform.is_playing())
        .fps(PLAYBACK_CURSOR_OVERLAY_FPS)
        .paint(NativeAppState::paint_playback_overlay)
}
