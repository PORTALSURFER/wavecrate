use super::*;

#[test]
fn audio_engine_pill_activates_settings_toggle() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.settings.ui.audio_settings_open = true;
    let surface = crate::native_app::test_support::settings::top_control_bar(&state).into_surface();
    let pill = surface
        .find_widget(crate::native_app::test_support::settings::AUDIO_ENGINE_PILL_ID)
        .and_then(|widget| {
            widget
                .widget_object()
                .as_any()
                .downcast_ref::<BadgeWidget>()
        })
        .expect("audio pill should use a Radiant badge");

    assert!(pill.common.is_active());
    assert_eq!(
        surface.dispatch_widget_output(
            crate::native_app::test_support::settings::AUDIO_ENGINE_PILL_ID,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate),
        ),
        Some(
            crate::native_app::test_support::state::GuiMessage::Settings(
                crate::native_app::app::SettingsMessage::ToggleAudioSettings
            )
        )
    );
}

#[test]
fn general_settings_button_opens_general_tab() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.settings.ui.audio_settings_open = true;
    state.ui.settings.ui.app_settings_tab =
        crate::native_app::test_support::state::AppSettingsTab::General;
    let surface = crate::native_app::test_support::settings::top_control_bar(&state).into_surface();
    let button = surface
        .find_widget(crate::native_app::test_support::settings::GENERAL_SETTINGS_BUTTON_ID)
        .and_then(|widget| {
            widget
                .widget_object()
                .as_any()
                .downcast_ref::<IconButtonWidget>()
        })
        .expect("general settings button should use a Radiant icon button");

    assert!(button.common.is_active());
    assert_eq!(
        surface.dispatch_widget_output(
            crate::native_app::test_support::settings::GENERAL_SETTINGS_BUTTON_ID,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(
            crate::native_app::test_support::state::GuiMessage::Settings(
                crate::native_app::app::SettingsMessage::OpenGeneralSettings
            )
        )
    );
}

#[test]
fn help_tooltips_button_toggles_help_mode() {
    let state = NativeAppState::load_default().expect("default state loads");
    let surface = crate::native_app::test_support::settings::top_control_bar(&state).into_surface();
    let button = surface
        .find_widget(crate::native_app::test_support::settings::HELP_TOOLTIPS_BUTTON_ID)
        .and_then(|widget| {
            widget
                .widget_object()
                .as_any()
                .downcast_ref::<IconButtonWidget>()
        })
        .expect("help tooltips button should use a Radiant icon button");

    assert!(!button.common.is_active());
    assert_eq!(
        surface.dispatch_widget_output(
            crate::native_app::test_support::settings::HELP_TOOLTIPS_BUTTON_ID,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(
            crate::native_app::test_support::state::GuiMessage::Settings(
                crate::native_app::app::SettingsMessage::ToggleHelpTooltips
            )
        )
    );
}

#[test]
fn settings_top_bar_actions_open_expected_tabs() {
    let mut state = gui_state_for_span_tests();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::OpenGeneralSettings,
        ),
        &mut context,
    );
    assert!(state.ui.settings.ui.audio_settings_open);
    assert_eq!(
        state.ui.settings.ui.app_settings_tab,
        crate::native_app::test_support::state::AppSettingsTab::General
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioSettings,
        ),
        &mut context,
    );
    assert!(state.ui.settings.ui.audio_settings_open);
    assert_eq!(
        state.ui.settings.ui.app_settings_tab,
        crate::native_app::test_support::state::AppSettingsTab::AudioEngine
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioSettings,
        ),
        &mut context,
    );
    assert!(!state.ui.settings.ui.audio_settings_open);
}

#[test]
fn settings_help_tooltips_toggle_updates_chrome_state() {
    let mut state = gui_state_for_span_tests();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleHelpTooltips,
        ),
        &mut context,
    );
    assert!(state.ui.chrome.help_tooltips_enabled);

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleHelpTooltips,
        ),
        &mut context,
    );
    assert!(!state.ui.chrome.help_tooltips_enabled);
}
