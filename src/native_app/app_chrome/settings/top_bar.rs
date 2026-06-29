use radiant::prelude as ui;

mod projection;
#[cfg(test)]
mod tests;

use crate::native_app::app::{GuiMessage, NativeAppState, SettingsMessage};
use crate::native_app::ui::ids as widget_ids;

use self::projection::{
    AudioEnginePillProjection, GeneralSettingsButtonProjection, HelpTooltipsButtonProjection,
    NormalizedAuditionButtonProjection, ReleaseUpdateButtonProjection, SettingsControlsProjection,
    TopControlBarProjection, VolumeSliderProjection,
};

pub(in crate::native_app) const VOLUME_SLIDER_ID: u64 = widget_ids::VOLUME_SLIDER_ID;
pub(in crate::native_app) const NORMALIZED_AUDITION_BUTTON_ID: u64 =
    widget_ids::NORMALIZED_AUDITION_BUTTON_ID;
const VOLUME_SLIDER_SIZE: ControlSize = ControlSize {
    width: 92.0,
    height: 14.0,
};
const NORMALIZED_AUDITION_BUTTON_SIZE: ControlSize = ControlSize {
    width: 18.0,
    height: 18.0,
};
pub(in crate::native_app) const HELP_TOOLTIPS_BUTTON_ID: u64 = widget_ids::HELP_TOOLTIPS_BUTTON_ID;
const HELP_TOOLTIPS_BUTTON_SIZE: ControlSize = ControlSize {
    width: 18.0,
    height: 22.0,
};
pub(in crate::native_app) const AUDIO_ENGINE_PILL_ID: u64 = widget_ids::AUDIO_ENGINE_PILL_ID;
const AUDIO_ENGINE_PILL_SIZE: ControlSize = ControlSize {
    width: 54.0,
    height: 18.0,
};
pub(in crate::native_app) const GENERAL_SETTINGS_BUTTON_ID: u64 =
    widget_ids::GENERAL_SETTINGS_BUTTON_ID;
const GENERAL_SETTINGS_BUTTON_SIZE: ControlSize = ControlSize {
    width: 28.0,
    height: 24.0,
};
pub(in crate::native_app) const RELEASE_UPDATE_BUTTON_ID: u64 =
    widget_ids::RELEASE_UPDATE_BUTTON_ID;
const RELEASE_UPDATE_BUTTON_SIZE: ControlSize = ControlSize {
    width: 24.0,
    height: 24.0,
};
const SETTINGS_ICON_TINTS: ui::SvgIconTintPalette = ui::SvgIconTintPalette::new(
    ui::Rgba8::new(238, 238, 238, 255),
    ui::Rgba8::new(255, 160, 82, 255),
    ui::Rgba8::new(145, 145, 145, 255),
);

struct ControlSize {
    width: f32,
    height: f32,
}

pub(in crate::native_app) fn top_control_bar(state: &NativeAppState) -> ui::View<GuiMessage> {
    let projection = TopControlBarProjection::from_app_state(state);
    ui::toolbar_from_parts(
        ui::ToolbarParts::new([
            volume_slider(projection.volume.value)
                .tooltip_if(projection.help_tooltips_enabled, projection.volume.tooltip),
            normalized_audition_button(projection.normalized_audition).tooltip_if(
                projection.help_tooltips_enabled,
                projection.normalized_audition.tooltip,
            ),
        ])
        .trailing(settings_controls(projection.settings_controls))
        .spacing(6.0)
        .padding_x(12.0)
        .padding_y(4.0)
        .height(30.0),
    )
}

fn settings_controls(model: SettingsControlsProjection) -> ui::View<GuiMessage> {
    let audio_engine_tooltip = model.audio_engine.tooltip;
    let mut controls = vec![
        audio_engine_pill(model.audio_engine)
            .tooltip_if(model.help_tooltips_enabled, audio_engine_tooltip),
        general_settings_button(model.general_settings)
            .tooltip_if(model.help_tooltips_enabled, model.general_settings.tooltip),
    ];
    if model.release_update.visible {
        controls.push(
            release_update_button(model.release_update)
                .tooltip_if(model.help_tooltips_enabled, model.release_update.tooltip),
        );
    }
    controls.push(help_tooltips_button(model.help_tooltips));
    ui::row(controls).spacing(4.0).height(24.0)
}

fn help_tooltips_button(projection: HelpTooltipsButtonProjection) -> ui::View<GuiMessage> {
    let button = ui::icon_button(help_tooltips_icon(projection.active))
        .bare()
        .active(projection.active)
        .message(GuiMessage::Settings(SettingsMessage::ToggleHelpTooltips))
        .id(HELP_TOOLTIPS_BUTTON_ID)
        .size(
            HELP_TOOLTIPS_BUTTON_SIZE.width,
            HELP_TOOLTIPS_BUTTON_SIZE.height,
        );
    button.tooltip_if(projection.active, projection.active_tooltip)
}

fn audio_engine_pill(projection: AudioEnginePillProjection) -> ui::View<GuiMessage> {
    ui::badge(projection.label)
        .style(projection.style)
        .active(projection.active)
        .message(GuiMessage::Settings(SettingsMessage::ToggleAudioSettings))
        .id(AUDIO_ENGINE_PILL_ID)
        .size(AUDIO_ENGINE_PILL_SIZE.width, AUDIO_ENGINE_PILL_SIZE.height)
}

fn general_settings_button(projection: GeneralSettingsButtonProjection) -> ui::View<GuiMessage> {
    ui::icon_button(settings_gear_icon(projection.active))
        .active(projection.active)
        .message(GuiMessage::Settings(SettingsMessage::OpenGeneralSettings))
        .id(GENERAL_SETTINGS_BUTTON_ID)
        .size(
            GENERAL_SETTINGS_BUTTON_SIZE.width,
            GENERAL_SETTINGS_BUTTON_SIZE.height,
        )
}

fn release_update_button(projection: ReleaseUpdateButtonProjection) -> ui::View<GuiMessage> {
    ui::icon_button(release_update_icon(projection.active))
        .active(projection.active)
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .message(GuiMessage::OpenReleaseDownloadPage)
        .id(RELEASE_UPDATE_BUTTON_ID)
        .size(
            RELEASE_UPDATE_BUTTON_SIZE.width,
            RELEASE_UPDATE_BUTTON_SIZE.height,
        )
}

pub(in crate::native_app) fn volume_slider(volume: f32) -> ui::View<GuiMessage> {
    volume_slider_from_projection(VolumeSliderProjection::new(volume))
}

fn volume_slider_from_projection(projection: VolumeSliderProjection) -> ui::View<GuiMessage> {
    ui::slider(projection.value)
        .compact()
        .paint_focus(false)
        .message(|volume| GuiMessage::Settings(SettingsMessage::SetVolume(volume)))
        .id(VOLUME_SLIDER_ID)
        .size(VOLUME_SLIDER_SIZE.width, VOLUME_SLIDER_SIZE.height)
}

fn normalized_audition_button(
    projection: NormalizedAuditionButtonProjection,
) -> ui::View<GuiMessage> {
    ui::icon_button(normalized_audition_icon(projection.active))
        .bare()
        .active(projection.active)
        .message(GuiMessage::Settings(
            SettingsMessage::SetNormalizedAuditionEnabled(!projection.active),
        ))
        .id(NORMALIZED_AUDITION_BUTTON_ID)
        .size(
            NORMALIZED_AUDITION_BUTTON_SIZE.width,
            NORMALIZED_AUDITION_BUTTON_SIZE.height,
        )
}

fn settings_gear_icon(active: bool) -> ui::SvgIcon {
    SETTINGS_GEAR_ICON.icon_for_state(SETTINGS_ICON_TINTS, true, active)
}

fn normalized_audition_icon(active: bool) -> ui::SvgIcon {
    NORMALIZED_AUDITION_ICON.icon_for_state(SETTINGS_ICON_TINTS, true, active)
}

fn help_tooltips_icon(active: bool) -> ui::SvgIcon {
    HELP_TOOLTIPS_ICON.icon_for_state(SETTINGS_ICON_TINTS, true, active)
}

fn release_update_icon(active: bool) -> ui::SvgIcon {
    RELEASE_UPDATE_ICON.icon_for_state(SETTINGS_ICON_TINTS, true, active)
}

static HELP_TOOLTIPS_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M6.3 5.6c.1-1.2.9-1.9 2.1-1.9 1.3 0 2.1.8 2.1 1.9 0 .8-.4 1.3-1.2 1.8-.6.4-.8.8-.8 1.5v.3H7.3v-.5c0-.9.4-1.5 1.1-1.9.6-.4.8-.7.8-1.1 0-.5-.4-.8-1-.8s-.9.3-1 1z" fill="currentColor"/>
  <circle cx="7.9" cy="11.7" r=".7" fill="currentColor"/>
</svg>"#,
);

static NORMALIZED_AUDITION_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M2 10.2h1.4l1-3.8 1.4 6.1 1.5-9 1.5 7.2 1.1-3.2.9 2.7H14v1.4h-4.2l-.8-2.2-1.5 4.2L6.2 7 4.9 13H3.3l-1-2.8z"/>
  <path d="M11.8 2.2h1.3v2.1h2.1v1.3h-2.1v2.1h-1.3V5.6H9.7V4.3h2.1z"/>
</svg>"#,
);

const SETTINGS_GEAR_ICON_SVG: &str = r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path fill-rule="evenodd" d="M6.9 1.5h2.2l.35 1.45c.31.09.61.22.9.37l1.25-.78 1.56 1.56-.78 1.25c.15.29.28.59.37.9l1.45.35v2.2l-1.45.35c-.09.31-.22.61-.37.9l.78 1.25-1.56 1.56-1.25-.78c-.29.15-.59.28-.9.37L9.1 14.2H6.9l-.35-1.45c-.31-.09-.61-.22-.9-.37l-1.25.78-1.56-1.56.78-1.25c-.15-.29-.28-.59-.37-.9L1.8 9.1V6.9l1.45-.35c.09-.31.22-.61.37-.9L2.84 4.4 4.4 2.84l1.25.78c.29-.15.59-.28.9-.37L6.9 1.5zM8 5.6a2.4 2.4 0 1 0 0 4.8 2.4 2.4 0 0 0 0-4.8z"/>
</svg>"#;

static SETTINGS_GEAR_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(SETTINGS_GEAR_ICON_SVG);

static RELEASE_UPDATE_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M7.25 2h1.5v6.2l2.1-2.1 1.05 1.05L8 11.05 4.1 7.15 5.15 6.1l2.1 2.1z"/>
  <path d="M3 12.3h10V14H3z"/>
  <circle cx="12.2" cy="3.8" r="2.1"/>
</svg>"#,
);
