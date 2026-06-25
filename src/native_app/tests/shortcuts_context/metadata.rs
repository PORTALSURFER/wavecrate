use super::super::{
    cancel_metadata_tag_entry, delete_selected_metadata_tag, focus_metadata_tag_input,
    native_app_state_with_temp_sample, toggle_metadata_tag,
};
use crate::native_app::test_support::{
    state::{NativeAppState, default_gui_shortcuts},
    waveform::MetadataTagInputMode,
};
use radiant::prelude as ui;

#[test]
fn metadata_tag_category_escape_shortcut_cancels_tag_entry() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.metadata.tag_input_mode = MetadataTagInputMode::Category {
        pending_tag: String::from("deep-kick"),
    };

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(resolution.action, Some(cancel_metadata_tag_entry()));
    assert!(resolution.handled);
}

#[test]
fn delete_shortcut_removes_selected_metadata_tag_before_deleting_files() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.metadata.selected_tag = Some(String::from("bass"));

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Delete));

    assert_eq!(resolution.action, Some(delete_selected_metadata_tag()));
    assert!(resolution.handled);
}

#[test]
fn backquote_shortcut_routes_to_metadata_tag_input_focus() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Backquote));

    assert_eq!(resolution.action, Some(focus_metadata_tag_input()));
    assert!(resolution.handled);
}

#[test]
fn number_shortcuts_route_to_playback_type_tags() {
    let state = NativeAppState::load_default().expect("default state loads");

    let one_shot = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Num9));
    let looped = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Num0));

    assert_eq!(
        one_shot.action,
        Some(toggle_metadata_tag(String::from("one-shot")))
    );
    assert!(one_shot.handled);
    assert_eq!(
        looped.action,
        Some(toggle_metadata_tag(String::from("loop")))
    );
    assert!(looped.handled);
}

#[test]
fn playback_type_shortcuts_use_metadata_tag_replacement() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("playback-type-shortcut.wav");

    let one_shot = default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Num9))
        .action
        .expect("9 shortcut action");
    state.apply_message(one_shot, &mut ui::UiUpdateContext::default());
    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("one-shot")])
    );

    let looped = default_gui_shortcuts(&state)
        .resolve(ui::KeyPress::new(ui::KeyCode::Num0))
        .action
        .expect("0 shortcut action");
    state.apply_message(looped, &mut ui::UiUpdateContext::default());
    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("loop")])
    );
}
