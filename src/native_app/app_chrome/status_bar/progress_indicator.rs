use crate::native_app::app::GuiMessage;
#[cfg(test)]
use crate::native_app::app_chrome::view_models::status_bar::WorkerProgressViewModel;
use crate::native_app::ui::ids::WORKER_PROGRESS_ROOT_ID;
use radiant::prelude as ui;
use radiant::runtime::{PaintPrimitive, push_fill_rect};
use radiant::widgets::{
    ActivationInputPolicy, FocusBehavior, PaintBounds, Widget, WidgetCommon, WidgetInput,
    WidgetOutput, WidgetSizing, handle_activation_input,
};

use super::projection::{WorkerProgressBarContentProjection, WorkerProgressBarProjection};

pub(super) const WORKER_PROGRESS_INDICATOR_SIZE: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkerProgressIndicatorMessage {
    Activate,
}

#[derive(Clone, Debug, PartialEq)]
struct WorkerProgressIndicatorWidget {
    common: WidgetCommon,
}

impl WorkerProgressIndicatorWidget {
    fn new() -> Self {
        let mut common = WidgetCommon::new(
            WORKER_PROGRESS_ROOT_ID,
            WidgetSizing::fixed(ui::Vector2::new(
                WORKER_PROGRESS_INDICATOR_SIZE,
                WORKER_PROGRESS_INDICATOR_SIZE,
            )),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }

    fn handle_activation(
        &mut self,
        bounds: ui::Rect,
        input: &WidgetInput,
    ) -> Option<WorkerProgressIndicatorMessage> {
        handle_activation_input(
            &mut self.common.state,
            bounds,
            input,
            ActivationInputPolicy::pointer_only(),
        )
        .activated()
        .then_some(WorkerProgressIndicatorMessage::Activate)
    }
}

impl Widget for WorkerProgressIndicatorWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: ui::Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.handle_activation(bounds, &input)
            .map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state = previous.common.state;
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn needs_state_synchronization(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: ui::Rect,
        _layout: &ui::LayoutOutput,
        _theme: &ui::ThemeTokens,
    ) {
        // The transparent rect gives the transient painter a stable anchor.
        // The activity mark itself is painted only by the animated overlay so
        // a stale retained dot can never masquerade as a working pulse.
        push_fill_rect(
            primitives,
            self.common.id,
            bounds,
            ui::Rgba8::new(0, 0, 0, 0),
        );
    }
}

/// Project every active worker state onto one compact, clickable pulse anchor.
///
/// Determinate progress remains available in the job-details projection. The
/// status bar deliberately uses one stable activity affordance for every worker
/// so changing worker types cannot make the chrome jump between track layouts.
pub(super) fn worker_progress_indicator_from_projection(
    projection: WorkerProgressBarProjection,
) -> ui::View<GuiMessage> {
    if matches!(
        projection.content,
        WorkerProgressBarContentProjection::Hidden
    ) {
        return ui::empty()
            .width(0.0)
            .height(WORKER_PROGRESS_INDICATOR_SIZE);
    }

    ui::custom_widget_mapped(
        WorkerProgressIndicatorWidget::new(),
        |_message: WorkerProgressIndicatorMessage| GuiMessage::ToggleJobDetails,
    )
    .id(WORKER_PROGRESS_ROOT_ID)
    .width(WORKER_PROGRESS_INDICATOR_SIZE)
    .height(WORKER_PROGRESS_INDICATOR_SIZE)
}

#[cfg(test)]
pub(super) fn worker_progress_indicator(
    progress: Option<WorkerProgressViewModel>,
    progress_tick: f32,
) -> ui::View<GuiMessage> {
    worker_progress_indicator_from_projection(WorkerProgressBarProjection::from_progress(
        progress,
        progress_tick,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indicator_activates_on_primary_click_release() {
        let bounds = ui::Rect::from_size(
            WORKER_PROGRESS_INDICATOR_SIZE,
            WORKER_PROGRESS_INDICATOR_SIZE,
        );
        let center = bounds.center();
        let mut indicator = WorkerProgressIndicatorWidget::new();

        assert_eq!(
            indicator.handle_activation(bounds, &WidgetInput::primary_press(center)),
            None
        );
        assert_eq!(
            indicator.handle_activation(bounds, &WidgetInput::primary_release(center)),
            Some(WorkerProgressIndicatorMessage::Activate)
        );
    }
}
