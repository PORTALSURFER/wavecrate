use crate::native_app::app::{GuiMessage, NativeAppState, default_gui_shortcuts};
use crate::native_app::app_chrome::layout;
use crate::native_app::app_chrome::view_models::sample_browser::prepare_sample_browser_view;
use radiant::prelude as ui;

const APP_TRANSIENT_OVERLAY_KEY: u64 = 0x6170_705f_6f76_726c;
const APP_FRAME_CLOCK_FPS: u32 = 60;

pub(in crate::native_app) fn view(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    prepare_sample_browser_view(state);
    scene(state).into_view().fill()
}

fn scene(state: &NativeAppState) -> ui::Scene<GuiMessage> {
    ui::scene(layout::shell(state))
        .shortcuts(default_gui_shortcuts(state))
        .frame_clock(frame_clock())
        .overlay(app_transient_overlay())
}

fn frame_clock() -> ui::FrameClock<NativeAppState, GuiMessage> {
    ui::FrameClock::message(GuiMessage::Frame)
        .fps_with(|state: &mut NativeAppState| {
            (!state.playback_visual_activity_active()).then_some(APP_FRAME_CLOCK_FPS)
        })
        .repaint_scope(
            |state: &mut NativeAppState| state.frame_repaint_scope_before_update(),
            |state, scope| state.frame_can_use_paint_only(scope),
        )
}

fn app_transient_overlay() -> ui::TransientOverlay<NativeAppState> {
    ui::TransientOverlay::new(APP_TRANSIENT_OVERLAY_KEY)
        .paint_only()
        .when(|state: &mut NativeAppState| state.should_paint_app_transient_overlay())
        .paint(NativeAppState::paint_app_transient_overlay)
}
