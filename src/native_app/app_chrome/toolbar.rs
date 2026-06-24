mod icons;
mod identity;
mod projection;

use radiant::{prelude as ui, widgets::ButtonMessage};

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::toolbar::MainToolbarViewModel;

pub(in crate::native_app) use icons::{ToolbarIcon, toolbar_icon_color, toolbar_icon_glyph};
pub(in crate::native_app) use projection::{
    ToolbarControlProjection, ToolbarIconButtonProjection, ToolbarProjection,
};

pub(in crate::native_app) const TOOLBAR_FOCUS_LOADED_ID: u64 = identity::TOOLBAR_FOCUS_LOADED_ID;
pub(in crate::native_app) const TOOLBAR_APPLY_EDIT_MARK_EDITS_ID: u64 =
    identity::TOOLBAR_APPLY_EDIT_MARK_EDITS_ID;
pub(in crate::native_app) const TOOLBAR_SIMILAR_SECTIONS_ID: u64 =
    identity::TOOLBAR_SIMILAR_SECTIONS_ID;
pub(in crate::native_app) const TOOLBAR_STOP_ID: u64 = identity::TOOLBAR_STOP_ID;
pub(in crate::native_app) const TOOLBAR_RANDOM_ID: u64 = identity::TOOLBAR_RANDOM_ID;
pub(in crate::native_app) const TOOLBAR_ZERO_CROSSING_SNAP_ID: u64 =
    identity::TOOLBAR_ZERO_CROSSING_SNAP_ID;

pub(in crate::native_app) fn main_toolbar(model: MainToolbarViewModel) -> ui::View<GuiMessage> {
    let projection = ToolbarProjection::from_model(model);
    let controls = projection
        .controls
        .iter()
        .map(|control| toolbar_control(*control, projection.help_tooltips_enabled))
        .collect::<Vec<_>>();

    ui::toolbar_from_parts(ui::ToolbarParts::new(controls).alignment(ui::ToolbarAlignment::End))
}

fn toolbar_control(
    control: ToolbarControlProjection,
    help_tooltips_enabled: bool,
) -> ui::View<GuiMessage> {
    match control {
        ToolbarControlProjection::Icon(button) => toolbar_icon_button_from_projection(button)
            .tooltip_if(help_tooltips_enabled, button.tooltip),
        ToolbarControlProjection::BeatGuideCount { count, key } => {
            beat_guide_count_label(count, key)
        }
        ToolbarControlProjection::ApplyEditMarkEdits { id, tooltip } => {
            apply_edit_mark_edits_button(id).tooltip_if(help_tooltips_enabled, tooltip)
        }
    }
}

fn beat_guide_count_label(count: u8, key: &'static str) -> ui::View<GuiMessage> {
    ui::text_line(count.to_string(), 24.0)
        .align_text(ui::TextAlign::Center)
        .key(key)
        .width(20.0)
        .height(24.0)
}

fn apply_edit_mark_edits_button(id: u64) -> ui::View<GuiMessage> {
    ui::button("Apply")
        .style(ui::WidgetStyle::strong(ui::WidgetTone::Accent))
        .message(GuiMessage::RequestApplyEditSelectionEffects)
        .id(id)
        .size(58.0, 24.0)
}

#[cfg(test)]
pub(in crate::native_app) fn toolbar_icon_button(
    id: u64,
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::View<GuiMessage> {
    toolbar_icon_button_from_projection(ToolbarIconButtonProjection {
        id,
        icon,
        enabled,
        icon_enabled: enabled,
        active,
        tooltip: "",
    })
}

fn toolbar_icon_button_from_projection(
    button: ToolbarIconButtonProjection,
) -> ui::View<GuiMessage> {
    ui::icon_button(toolbar_icon_glyph(
        button.icon,
        button.icon_enabled,
        button.active,
    ))
    .enabled(button.enabled)
    .active(button.active)
    .mapped(move |message| toolbar_button_message(button.icon, message))
    .id(button.id)
    .size(28.0, 24.0)
}

fn toolbar_button_message(icon: ToolbarIcon, message: ButtonMessage) -> GuiMessage {
    match icon {
        ToolbarIcon::FocusLoaded => GuiMessage::FocusLoadedFile,
        ToolbarIcon::Loop => GuiMessage::ToggleLoopPlayback,
        ToolbarIcon::Random
            if message
                .activation_modifiers()
                .is_some_and(|modifiers| modifiers.command) =>
        {
            GuiMessage::ToggleStickyRandomSampleRangePlayback
        }
        ToolbarIcon::Random
            if message
                .activation_modifiers()
                .is_some_and(|modifiers| modifiers.shift) =>
        {
            GuiMessage::PlayRandomListedSampleRange
        }
        ToolbarIcon::Random => GuiMessage::PlayRandomSampleRange,
        ToolbarIcon::SimilarSections => GuiMessage::ToggleSimilarSections,
        ToolbarIcon::ZeroCrossingSnap => GuiMessage::ToggleZeroCrossingSnap,
        ToolbarIcon::BeatGuides => GuiMessage::ToggleBeatGuides,
        ToolbarIcon::Metronome => GuiMessage::ToggleMetronome,
        ToolbarIcon::BeatGuideMinus => GuiMessage::AdjustBeatGuideCount(-1),
        ToolbarIcon::BeatGuidePlus => GuiMessage::AdjustBeatGuideCount(1),
        ToolbarIcon::Play => GuiMessage::PlaySelectedSample,
        ToolbarIcon::Stop => GuiMessage::StopPlayback,
    }
}
