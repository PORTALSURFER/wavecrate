//! Deterministic controller fixtures used by GUI test scenarios and tools.

use super::*;
use crate::app::state::{SampleBrowserActionPrompt, UpdateStatus, WaveformView};
use hound::{SampleFormat, WavSpec, WavWriter};
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
        "waveform" => build_waveform_fixture(renderer),
        "options" => build_options_fixture(renderer),
        "prompt" => build_prompt_fixture(renderer),
        "update" => build_update_fixture(renderer),
        other => Err(format!("unsupported GUI fixture '{other}'")),
    }
}

fn build_browser_fixture(renderer: WaveformRenderer) -> Result<GuiFixtureControllerBundle, String> {
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
    let source = SampleSource::new(source_root);
    let entries = vec![
        write_fixture_entry(&source.root, "kick_one.wav", &[0.1, 0.4, 0.2, 0.0])?,
        write_fixture_entry(&source.root, "snare_two.wav", &[0.0, 0.8, -0.6, 0.1])?,
        write_fixture_entry(&source.root, "hat_three.wav", &[0.2, -0.2, 0.2, -0.2])?,
    ];
    seed_source_fixture(&mut controller, &source, entries)?;
    controller.select_wav_by_path(Path::new("kick_one.wav"));
    controller.focus_browser_list();
    Ok(GuiFixtureControllerBundle {
        controller,
        sandbox_guards: vec![sandbox],
    })
}

fn build_waveform_fixture(renderer: WaveformRenderer) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_browser_fixture(renderer)?;
    let source = bundle
        .controller
        .current_source()
        .ok_or_else(|| String::from("waveform fixture missing current source"))?;
    bundle
        .controller
        .load_waveform_for_selection(&source, Path::new("kick_one.wav"))?;
    bundle.controller.ui.waveform.cursor = Some(0.35);
    bundle.controller.ui.waveform.selection = Some(SelectionRange::new(0.2, 0.6));
    bundle.controller.ui.waveform.view = WaveformView {
        start: 0.2,
        end: 0.6,
    };
    bundle.controller.focus_waveform();
    Ok(bundle)
}

fn build_options_fixture(renderer: WaveformRenderer) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_browser_fixture(renderer)?;
    bundle.controller.open_options_panel();
    Ok(bundle)
}

fn build_prompt_fixture(renderer: WaveformRenderer) -> Result<GuiFixtureControllerBundle, String> {
    let mut bundle = build_browser_fixture(renderer)?;
    bundle.controller.ui.browser.pending_action = Some(SampleBrowserActionPrompt::Rename {
        target: PathBuf::from("kick_one.wav"),
        name: String::from("kick_one"),
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
    bundle.controller.ui.update.available_published_at =
        Some(String::from("2026-03-11T12:00:00Z"));
    bundle.controller.ui.update.last_error = None;
    Ok(bundle)
}

fn seed_source_fixture(
    controller: &mut AppController,
    source: &SampleSource,
    entries: Vec<WavEntry>,
) -> Result<(), String> {
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller
        .cache_db(source)
        .map_err(|err| format!("cache GUI fixture source DB: {err}"))?;
    write_entries_to_source_db(controller, source, &entries)?;
    controller.wav_entries.clear();
    controller.wav_entries.total = entries.len();
    controller.wav_entries.insert_page(0, entries);
    controller.rebuild_wav_lookup();
    controller.refresh_sources_ui();
    controller.rebuild_browser_lists();
    Ok(())
}

fn write_entries_to_source_db(
    controller: &mut AppController,
    source: &SampleSource,
    entries: &[WavEntry],
) -> Result<(), String> {
    let db = controller
        .database_for(source)
        .map_err(|err| format!("open GUI fixture source DB: {err}"))?;
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("create GUI fixture source write batch: {err}"))?;
    for entry in entries {
        let hash = entry.content_hash.as_deref().unwrap_or("gui-fixture");
        batch
            .upsert_file_with_hash_and_tag(
                &entry.relative_path,
                entry.file_size,
                entry.modified_ns,
                hash,
                entry.tag,
                entry.missing,
            )
            .map_err(|err| {
                format!(
                    "seed GUI fixture DB row {}: {err}",
                    entry.relative_path.display()
                )
            })?;
    }
    batch
        .commit()
        .map_err(|err| format!("commit GUI fixture source DB batch: {err}"))
}

fn write_fixture_entry(root: &Path, name: &str, samples: &[f32]) -> Result<WavEntry, String> {
    let path = root.join(name);
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(&path, spec)
        .map_err(|err| format!("create fixture wav {}: {err}", path.display()))?;
    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|err| format!("write fixture wav {}: {err}", path.display()))?;
    }
    writer
        .finalize()
        .map_err(|err| format!("finalize fixture wav {}: {err}", path.display()))?;
    let metadata = fs::metadata(&path)
        .map_err(|err| format!("read fixture wav metadata {}: {err}", path.display()))?;
    let modified = metadata
        .modified()
        .map_err(|err| format!("read fixture wav modified time {}: {err}", path.display()))?;
    let modified_ns = modified
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|err| format!("fixture wav modified time before epoch {}: {err}", path.display()))?
        .as_nanos()
        .min(i64::MAX as u128) as i64;
    Ok(WavEntry {
        relative_path: PathBuf::from(name),
        file_size: metadata.len(),
        modified_ns,
        content_hash: Some(format!("fixture-{name}")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    })
}
