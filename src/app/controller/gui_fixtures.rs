//! Deterministic controller fixtures used by GUI test scenarios and tools.

mod entries;
mod map;
mod seeding;

use super::*;
use crate::app::state::{SampleBrowserActionPrompt, UpdateStatus, WaveformView};
use entries::{browser_fixture_entries, dense_waveform_fixture_samples, write_fixture_entry};
use seeding::{seed_browser_fixture_tags, seed_source_fixture};
use std::{fs, path::Path};
use tempfile::TempDir;

/// One controller fixture and the temporary resources it must keep alive.
pub(crate) struct GuiFixtureControllerBundle {
    /// Seeded controller ready for GUI scenario execution.
    pub controller: AppController,
    /// Temporary directories that back seeded files and databases.
    pub sandbox_guards: Vec<TempDir>,
}

/// Build a deterministic controller fixture for the requested GUI test tag.
///
/// These fixtures avoid persisted user configuration so contract tests, CLI
/// tooling, and desktop smoke runs all start from the same known state.
pub(crate) fn build_named_gui_fixture_controller(
    renderer: WaveformRenderer,
    fixture_tag: &str,
) -> Result<GuiFixtureControllerBundle, String> {
    match fixture_tag {
        "browser" => build_browser_fixture(renderer),
        "sources" => build_sources_fixture(renderer),
        "transport" => build_transport_fixture(renderer),
        "map" => map::build_map_fixture(renderer),
        "waveform" => build_waveform_fixture(renderer),
        "waveform_dense" => build_waveform_dense_fixture(renderer),
        "waveform_mixed" => build_waveform_mixed_fixture(renderer),
        "options" => build_options_fixture(renderer),
        "prompt" => build_prompt_fixture(renderer),
        "update" => build_update_fixture(renderer),
        other => Err(format!("unsupported GUI fixture '{other}'")),
    }
}

fn build_browser_fixture(renderer: WaveformRenderer) -> Result<GuiFixtureControllerBundle, String> {
    build_browser_fixture_with_source_id(renderer, None)
}

fn build_sources_fixture(renderer: WaveformRenderer) -> Result<GuiFixtureControllerBundle, String> {
    let mut controller = AppController::new(renderer, None);
    controller.settings.controls.advance_after_rating = false;
    let sandbox =
        tempfile::tempdir().map_err(|err| format!("create sources fixture tempdir: {err}"))?;
    let source_root = sandbox.path().join("sources-source");
    fs::create_dir_all(source_root.join("drums").join("kicks")).map_err(|err| {
        format!(
            "create sources fixture nested folder {}: {err}",
            source_root.join("drums").join("kicks").display()
        )
    })?;
    fs::create_dir_all(source_root.join("drums").join("snares")).map_err(|err| {
        format!(
            "create sources fixture nested folder {}: {err}",
            source_root.join("drums").join("snares").display()
        )
    })?;
    fs::create_dir_all(source_root.join("loops")).map_err(|err| {
        format!(
            "create sources fixture nested folder {}: {err}",
            source_root.join("loops").display()
        )
    })?;
    let source = SampleSource::new(source_root);
    let entries = vec![
        write_fixture_entry(
            &source.root,
            "drums/kicks/tight.wav",
            &[0.15, -0.20, 0.10, -0.05],
            crate::sample_sources::Rating::NEUTRAL,
        )?,
        write_fixture_entry(
            &source.root,
            "drums/snares/snap.wav",
            &[0.10, -0.08, 0.14, -0.12],
            crate::sample_sources::Rating::KEEP_1,
        )?,
        write_fixture_entry(
            &source.root,
            "loops/house_loop.wav",
            &[0.22, -0.16, 0.18, -0.14],
            crate::sample_sources::Rating::TRASH_1,
        )?,
    ];
    seed_source_fixture(&mut controller, &source, entries)?;
    controller.refresh_folder_browser();
    controller.focus_folder_row(1);
    Ok(GuiFixtureControllerBundle {
        controller,
        sandbox_guards: vec![sandbox],
    })
}

fn build_browser_fixture_with_source_id(
    renderer: WaveformRenderer,
    source_id: Option<SourceId>,
) -> Result<GuiFixtureControllerBundle, String> {
    let mut controller = AppController::new(renderer, None);
    controller.settings.controls.advance_after_rating = false;
    let sandbox =
        tempfile::tempdir().map_err(|err| format!("create browser fixture tempdir: {err}"))?;
    let source_root = sandbox.path().join("browser-source");
    fs::create_dir_all(&source_root).map_err(|err| {
        format!(
            "create browser fixture source root {}: {err}",
            source_root.display()
        )
    })?;
    let source = match source_id {
        Some(source_id) => SampleSource::new_with_id(source_id, source_root),
        None => SampleSource::new(source_root),
    };
    let entries = browser_fixture_entries(&source.root)?;
    seed_source_fixture(&mut controller, &source, entries)?;
    seed_browser_fixture_tags(&mut controller, &source)?;
    controller.select_wav_by_path(Path::new("kick_one.wav"));
    controller.focus_browser_list();
    Ok(GuiFixtureControllerBundle {
        controller,
        sandbox_guards: vec![sandbox],
    })
}

fn build_transport_fixture(
    renderer: WaveformRenderer,
) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_waveform_fixture(renderer)?;
    bundle.controller.apply_volume(0.42);
    Ok(bundle)
}

fn build_waveform_fixture(
    renderer: WaveformRenderer,
) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_browser_fixture(renderer)?;
    let source = bundle
        .controller
        .current_source()
        .ok_or_else(|| String::from("waveform fixture missing current source"))?;
    bundle
        .controller
        .load_waveform_for_selection(&source, Path::new("kick_one.wav"))?;
    bundle.controller.ui.waveform.cursor = Some(0.38);
    bundle.controller.ui.waveform.selection = Some(SelectionRange::new(0.35, 0.45));
    bundle.controller.ui.waveform.view = WaveformView {
        start: 0.25,
        end: 0.75,
    };
    bundle.controller.focus_waveform();
    Ok(bundle)
}

fn build_waveform_dense_fixture(
    renderer: WaveformRenderer,
) -> Result<GuiFixtureControllerBundle, String> {
    let mut controller = AppController::new(renderer, None);
    controller.settings.controls.advance_after_rating = false;
    let sandbox =
        tempfile::tempdir().map_err(|err| format!("create waveform fixture tempdir: {err}"))?;
    let source_root = sandbox.path().join("waveform-source");
    fs::create_dir_all(&source_root).map_err(|err| {
        format!(
            "create waveform fixture source root {}: {err}",
            source_root.display()
        )
    })?;
    let source = SampleSource::new(source_root);
    let entries = vec![write_fixture_entry(
        &source.root,
        "visible_waveform.wav",
        &dense_waveform_fixture_samples(),
        crate::sample_sources::Rating::NEUTRAL,
    )?];
    seed_source_fixture(&mut controller, &source, entries)?;
    controller.select_wav_by_path(Path::new("visible_waveform.wav"));
    controller.load_waveform_for_selection(&source, Path::new("visible_waveform.wav"))?;
    controller.ui.waveform.cursor = Some(0.38);
    controller.ui.waveform.selection = Some(SelectionRange::new(0.35, 0.45));
    controller.ui.waveform.view = WaveformView {
        start: 0.0,
        end: 1.0,
    };
    controller.focus_waveform();
    Ok(GuiFixtureControllerBundle {
        controller,
        sandbox_guards: vec![sandbox],
    })
}

fn build_waveform_mixed_fixture(
    renderer: WaveformRenderer,
) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_waveform_fixture(renderer)?;
    bundle.controller.ui.waveform.edit_selection = Some(SelectionRange::new(0.55, 0.65));
    bundle
        .controller
        .selection_state
        .edit_range
        .set_range(bundle.controller.ui.waveform.edit_selection);
    Ok(bundle)
}

fn build_options_fixture(renderer: WaveformRenderer) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_browser_fixture(renderer)?;
    bundle.controller.open_options_panel();
    Ok(bundle)
}

fn build_prompt_fixture(renderer: WaveformRenderer) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_browser_fixture(renderer)?;
    bundle.controller.ui.browser.pending_action =
        Some(SampleBrowserActionPrompt::MoveToFolderConflict {
            source_id: crate::sample_sources::SourceId::from_string("fixture-source"),
            source_relative: PathBuf::from("kick_one.wav"),
            target_folder: PathBuf::from("dest"),
            name: String::from("kick_one"),
            input_error: None,
        });
    bundle.controller.focus_browser_list();
    Ok(bundle)
}

fn build_update_fixture(renderer: WaveformRenderer) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_browser_fixture(renderer)?;
    bundle.controller.ui.update.status = UpdateStatus::UpdateAvailable;
    bundle.controller.ui.update.available_tag = Some(String::from("v20.1.0"));
    bundle.controller.ui.update.available_url =
        Some(String::from("https://example.invalid/releases/v20.1.0"));
    bundle.controller.ui.update.available_published_at = Some(String::from("2026-03-11T12:00:00Z"));
    bundle.controller.ui.update.last_error = None;
    Ok(bundle)
}
