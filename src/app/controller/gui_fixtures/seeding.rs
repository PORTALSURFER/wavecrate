//! Source and database seeding for deterministic controller GUI fixtures.

use super::*;
use std::path::Path;

pub(super) fn seed_source_fixture(
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
    load_fixture_entries(controller, source, entries);
    Ok(())
}

pub(super) fn seed_browser_fixture_tags(
    controller: &mut AppController,
    source: &SampleSource,
) -> Result<(), String> {
    let db = controller
        .database_for(source)
        .map_err(|err| format!("open browser fixture source DB for tags: {err}"))?;
    for path in texture_tagged_browser_paths() {
        db.assign_tag_to_path(Path::new(path), "Texture")
            .map_err(|err| format!("seed browser fixture Texture tag for {path}: {err}"))?;
    }
    for path in deep_kick_tagged_browser_paths() {
        db.assign_tag_to_path(Path::new(path), "Deep Kick")
            .map_err(|err| format!("seed browser fixture Deep Kick tag for {path}: {err}"))?;
    }
    db.assign_tag_to_path(Path::new("fx_five.wav"), "Rare FX")
        .map_err(|err| format!("seed browser fixture Rare FX tag: {err}"))?;
    Ok(())
}

fn texture_tagged_browser_paths() -> [&'static str; 4] {
    [
        "kick_one.wav",
        "snare_two.wav",
        "hat_three.wav",
        "loop_four.wav",
    ]
}

fn deep_kick_tagged_browser_paths() -> [&'static str; 2] {
    ["kick_one.wav", "kick_08.wav"]
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

fn load_fixture_entries(
    controller: &mut AppController,
    source: &SampleSource,
    entries: Vec<WavEntry>,
) {
    controller.wav_entries.clear();
    controller.wav_entries.source_id = Some(source.id.clone());
    controller.wav_entries.total = entries.len();
    controller.wav_entries.insert_page(0, entries);
    controller.rebuild_wav_lookup();
    controller.refresh_sources_ui();
    controller.rebuild_browser_lists();
}
