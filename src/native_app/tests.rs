use super::test_support::{
    sample_browser::DEFAULT_FOLDER_WIDTH,
    state::{NativeAppState, WaveformInteraction},
};
use super::waveform::{WaveformSelectionEdge, WaveformSelectionKind};
use radiant::{
    gui::types::{Point, Rect, Vector2},
    prelude::{self as ui, IntoView},
    runtime::{
        Command, DeclarativeOwnedCommandRuntimeBridge, Event, PaintTextInput, RuntimeBridge,
        SurfaceRuntime, TransientOverlayContext, UiSurface,
    },
    widgets::{DragHandleMessage, PointerButton, PointerModifiers, WidgetInput, WidgetKey},
};
use std::{fs, path::PathBuf, time::Duration};

mod audio_settings_controls;
mod audio_settings_dropdowns;
mod browser_context_menu;
mod cache_cleanup;
mod collections;
mod config_sources;
mod file_operations;
mod folder_layout;
mod metadata_tag_tests;
mod native_file_drop;
mod sample_browser;
mod shortcuts_context;
mod startup;
mod status_bar;
mod toolbar_playback;
mod transactions;
mod waveform_playback;
mod window_chrome;

fn first_visible_asset_file_path(
    browser: &super::test_support::state::FolderBrowserState,
) -> String {
    browser
        .selected_audio_files()
        .first()
        .unwrap_or_else(|| panic!("expected at least one visible audio sample"))
        .id
        .clone()
}

fn gui_state_for_span_tests() -> NativeAppState {
    super::test_support::state::NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .with_sample_status("")
        .build()
}

type NativeRuntimeForTests = SurfaceRuntime<
    DeclarativeOwnedCommandRuntimeBridge<
        NativeAppState,
        super::test_support::state::GuiMessage,
        fn(&mut NativeAppState) -> UiSurface<super::test_support::state::GuiMessage>,
        fn(
            &mut NativeAppState,
            super::test_support::state::GuiMessage,
        ) -> Command<super::test_support::state::GuiMessage>,
    >,
    super::test_support::state::GuiMessage,
>;

fn native_runtime_for_tests(state: NativeAppState, viewport: Vector2) -> NativeRuntimeForTests {
    let mut runtime = radiant::runtime::SurfaceRuntime::new(
        radiant::runtime::declarative_owned_command_runtime_bridge(
            state,
            project_gui_surface_for_tests
                as fn(&mut NativeAppState) -> UiSurface<super::test_support::state::GuiMessage>,
            reduce_gui_message_for_tests
                as fn(
                    &mut NativeAppState,
                    super::test_support::state::GuiMessage,
                ) -> Command<super::test_support::state::GuiMessage>,
        ),
        viewport,
    );
    apply_strict_update_diagnostics(&mut runtime);
    runtime
}

fn apply_strict_update_diagnostics<Bridge, Message>(runtime: &mut SurfaceRuntime<Bridge, Message>)
where
    Bridge: RuntimeBridge<Message>,
{
    runtime.set_update_handler_diagnostics_policy(ui::UiUpdateHandlerDiagnosticsPolicy::panic_at(
        Duration::from_millis(250),
    ));
}

fn project_gui_surface_for_tests(
    state: &mut NativeAppState,
) -> UiSurface<super::test_support::state::GuiMessage> {
    super::test_support::state::view(state).into_surface()
}

fn reduce_gui_message_for_tests(
    state: &mut NativeAppState,
    message: super::test_support::state::GuiMessage,
) -> Command<super::test_support::state::GuiMessage> {
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(message, &mut context);
    context.into_command()
}

fn metadata_message(
    message: super::test_support::state::MetadataMessage,
) -> super::test_support::state::GuiMessage {
    super::test_support::state::GuiMessage::Metadata(message)
}

fn focus_metadata_tag_input() -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::FocusMetadataTagInput)
}

fn metadata_tag_input(
    message: radiant::widgets::TextInputMessage,
) -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::MetadataTagInput(message))
}

fn cancel_metadata_tag_entry() -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::CancelMetadataTagEntry)
}

fn move_metadata_tag_completion(delta: i32) -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::MoveMetadataTagCompletion(delta))
}

fn hover_metadata_tag_completion(value: String) -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::HoverMetadataTagCompletion(value))
}

fn select_metadata_tag_completion(value: String) -> super::test_support::state::GuiMessage {
    metadata_message(
        super::test_support::state::MetadataMessage::SelectMetadataTagCompletion(value),
    )
}

fn toggle_metadata_tag_library() -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::ToggleMetadataTagLibrary)
}

fn toggle_metadata_tag_category(category_id: String) -> super::test_support::state::GuiMessage {
    metadata_message(
        super::test_support::state::MetadataMessage::ToggleMetadataTagCategory(category_id),
    )
}

fn select_metadata_tag(tag: String) -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::SelectMetadataTag(tag))
}

fn toggle_metadata_tag(tag: String) -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::ToggleMetadataTag(tag))
}

fn delete_selected_metadata_tag() -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::DeleteSelectedMetadataTag)
}

fn toggle_sample_name_view_mode() -> super::test_support::state::GuiMessage {
    metadata_message(super::test_support::state::MetadataMessage::ToggleSampleNameViewMode)
}

fn native_app_state_with_temp_sample(name: &str) -> (NativeAppState, tempfile::TempDir, String) {
    let mut state = gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join(name);
    fs::write(&sample_path, []).expect("sample file");
    state.library.folder_browser =
        super::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let selected_file = sample_path.display().to_string();
    state
        .library
        .folder_browser
        .select_file(selected_file.clone());
    (state, source_root, selected_file)
}

fn submit_folder_browser_rename_for_tests(state: &mut NativeAppState, value: impl Into<String>) {
    let result = state
        .library
        .folder_browser
        .apply_rename_input(radiant::widgets::TextInputMessage::Submitted {
            value: value.into(),
        })
        .expect("rename result");
    match result {
        super::sample_library::folder_browser::commands::RenameInputResult::Status(result) => {
            state.ui.status.sample = result.status;
        }
        super::sample_library::folder_browser::commands::RenameInputResult::Commit(request) => {
            let completion =
                super::sample_library::folder_browser::commands::execute_rename_commit_request(
                    request,
                );
            state.finish_folder_browser_rename(completion);
        }
    }
}

fn run_command_for_tests(
    state: &mut NativeAppState,
    command: Command<super::test_support::state::GuiMessage>,
) {
    match command {
        Command::None
        | Command::RequestRepaint
        | Command::RequestPaintOnly
        | Command::SetDpiScale(_)
        | Command::SetWindowLogicalSize(_)
        | Command::Focus(_)
        | Command::ScrollTo { .. }
        | Command::ScrollIntoView { .. }
        | Command::ScrollFixedRowIntoView { .. }
        | Command::BeginExternalDrag { .. }
        | Command::BeginDrag { .. }
        | Command::EndDrag
        | Command::PlatformRequest { .. }
        | Command::EndExternalDrag
        | Command::After { .. }
        | Command::PerformStream { .. }
        | Command::Exit => {}
        Command::Message(message) => {
            state.apply_message(message, &mut ui::UiUpdateContext::default());
        }
        Command::Batch(commands) => {
            for command in commands {
                run_command_for_tests(state, command);
            }
        }
        Command::Perform { work, .. } => {
            state.apply_message(work(), &mut ui::UiUpdateContext::default());
        }
    }
}

fn start_deferred_sample_load_for_tests(
    state: &mut NativeAppState,
    path: String,
    autoplay: bool,
    context: &mut ui::UiUpdateContext<super::test_support::state::GuiMessage>,
) {
    let Some(ticket) = state.background.deferred_sample_load_task.active() else {
        assert!(
            state.active_sample_load_task().is_some(),
            "sample load queued"
        );
        return;
    };
    state.apply_message(
        super::test_support::state::GuiMessage::DeferredSampleLoad {
            ticket,
            path,
            autoplay,
            check_cache: false,
            scheduled_at: std::time::Instant::now(),
        },
        context,
    );
}

fn temp_gui_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn write_test_wav_i16(path: &std::path::Path, samples: &[i16]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    for sample in samples {
        writer.write_sample(*sample).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}

fn read_test_wav_f32(path: &std::path::Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("open wav");
    reader
        .samples::<f32>()
        .collect::<Result<Vec<_>, _>>()
        .expect("read samples")
}

fn frame_has_clip_height(frame: &ui::SurfaceFrame, expected: f32) -> bool {
    frame
        .paint_plan
        .clip_starts()
        .any(|clip| (clip.rect.height() - expected).abs() < 0.01)
}

fn text_input_widget_id(frame: &ui::SurfaceFrame) -> Option<u64> {
    metadata_tag_text_input(frame).map(|input| input.widget_id)
}

fn metadata_tag_text_input(frame: &ui::SurfaceFrame) -> Option<&PaintTextInput> {
    frame.paint_plan.text_inputs().find(|input| {
        input.widget_id == crate::native_app::test_support::metadata_sidebar::METADATA_TAG_INPUT_ID
    })
}
