use crate::native_app::app::{GuiMessage, NativeAppState, default_gui_shortcuts};
use crate::native_app::app_chrome::layout;
use crate::native_app::app_chrome::library_browser::sample_browser_view;
use crate::native_app::ui::ids::SAMPLE_BROWSER_MAP_ID;
use radiant::{
    prelude as ui,
    runtime::{PaintPrimitive, TransientOverlayContext},
};

const APP_TRANSIENT_OVERLAY_KEY: u64 = 0x6170_705f_6f76_726c;
const APP_FRAME_CLOCK_FPS: u32 = 60;

pub(in crate::native_app) fn view(state: &NativeAppState) -> ui::View<GuiMessage> {
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
        .paint(paint_app_transient_overlay)
}

fn paint_app_transient_overlay(
    state: &mut NativeAppState,
    context: TransientOverlayContext<'_>,
    primitives: &mut Vec<PaintPrimitive>,
) {
    state.paint_waveform_transient_overlay(context, primitives);
    paint_starmap_active_audition_overlay(state, context, primitives);
}

fn paint_starmap_active_audition_overlay(
    state: &mut NativeAppState,
    context: TransientOverlayContext<'_>,
    primitives: &mut Vec<PaintPrimitive>,
) {
    let Some(active_file_id) = state.active_starmap_audition_file_id() else {
        return;
    };
    let Some(bounds) = context
        .plan
        .first_widget_rect_by_priority([SAMPLE_BROWSER_MAP_ID])
    else {
        return;
    };
    let Some(items) = state.library.folder_browser.cached_starmap_projection() else {
        return;
    };
    sample_browser_view::paint_active_starmap_audition_overlay(
        primitives,
        bounds,
        &items,
        state.ui.chrome.starmap_viewport,
        active_file_id,
    );
}
