//! Deterministic controller fixtures used by GUI test scenarios and tools.

use super::*;
use crate::app::state::{SampleBrowserActionPrompt, UpdateStatus, WaveformView};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{fs, path::Path};
use tempfile::TempDir;

const BROWSER_FIXTURE_ENTRY_COUNT: usize = 40;

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
        "waveform_mixed" => build_waveform_mixed_fixture(renderer),
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
    let entries = browser_fixture_entries(&source.root)?;
    seed_source_fixture(&mut controller, &source, entries)?;
    controller.select_wav_by_path(Path::new("kick_one.wav"));
    controller.focus_browser_list();
    Ok(GuiFixtureControllerBundle {
        controller,
        sandbox_guards: vec![sandbox],
    })
}

fn browser_fixture_entries(root: &Path) -> Result<Vec<WavEntry>, String> {
    let mut entries = Vec::with_capacity(BROWSER_FIXTURE_ENTRY_COUNT);
    for index in 0..BROWSER_FIXTURE_ENTRY_COUNT {
        let (name, tag) = browser_fixture_entry_spec(index);
        let samples = browser_fixture_samples(index);
        entries.push(write_fixture_entry(root, &name, &samples, tag)?);
    }
    Ok(entries)
}

fn browser_fixture_entry_spec(index: usize) -> (String, crate::sample_sources::Rating) {
    let fixed = match index {
        0 => Some(("kick_one.wav", crate::sample_sources::Rating::NEUTRAL)),
        1 => Some(("snare_two.wav", crate::sample_sources::Rating::KEEP_3)),
        2 => Some(("hat_three.wav", crate::sample_sources::Rating::TRASH_1)),
        3 => Some(("loop_four.wav", crate::sample_sources::Rating::KEEP_1)),
        4 => Some(("fx_five.wav", crate::sample_sources::Rating::TRASH_3)),
        _ => None,
    };
    if let Some((name, tag)) = fixed {
        return (String::from(name), tag);
    }
    let prefix = match index % 8 {
        0 => "kick",
        1 => "snare",
        2 => "hat",
        3 => "loop",
        4 => "fx",
        5 => "bass",
        6 => "perc",
        _ => "stab",
    };
    let name = format!("{prefix}_{index:02}.wav");
    let tag = match index % 5 {
        0 => crate::sample_sources::Rating::new(-2),
        1 => crate::sample_sources::Rating::NEUTRAL,
        2 => crate::sample_sources::Rating::KEEP_1,
        3 => crate::sample_sources::Rating::new(2),
        _ => crate::sample_sources::Rating::TRASH_1,
    };
    (name, tag)
}

fn browser_fixture_samples(index: usize) -> Vec<f32> {
    let seed = index as f32 + 1.0;
    vec![
        0.0125 * seed,
        0.18 + ((index % 5) as f32 * 0.05),
        -0.10 - ((index % 4) as f32 * 0.04),
        0.06 * ((index % 3) as f32 + 1.0),
        -0.03 * ((index % 6) as f32 + 1.0),
        0.015 * ((index % 7) as f32 + 1.0),
    ]
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
    bundle.controller.ui.update.available_published_at = Some(String::from("2026-03-11T12:00:00Z"));
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

fn write_fixture_entry(
    root: &Path,
    name: &str,
    samples: &[f32],
    tag: crate::sample_sources::Rating,
) -> Result<WavEntry, String> {
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
        .map_err(|err| {
            format!(
                "fixture wav modified time before epoch {}: {err}",
                path.display()
            )
        })?
        .as_nanos()
        .min(i64::MAX as u128) as i64;
    Ok(WavEntry {
        relative_path: PathBuf::from(name),
        file_size: metadata.len(),
        modified_ns,
        content_hash: Some(format!("fixture-{name}")),
        tag,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    })
}
