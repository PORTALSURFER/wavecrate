use super::super::test_support::{dummy_controller, write_test_wav};
use super::super::*;
use crate::app::controller::library::analysis_jobs;
use crate::selection::SelectionRange;
use rusqlite::params;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

mod autoplay;
mod enable_loop;
mod selection_drag;
mod stretch;

fn setup_looping_controller(selection: SelectionRange) -> Option<AppController> {
    let mut player = crate::audio::AudioPlayer::playing_for_tests()?;
    let dir = tempdir().ok()?;
    let wav_path = dir.path().join("loop_drag_test.wav");
    let long_samples = vec![0.1_f32; 240];
    write_test_wav(&wav_path, &long_samples);
    let bytes: std::sync::Arc<[u8]> = std::fs::read(&wav_path).ok()?.into();
    let duration = 30.0;
    player.set_audio(bytes.clone(), duration);

    let (mut controller, source) = dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("loop_drag_test.wav"),
        bytes,
        duration_seconds: duration,
        sample_rate: 8,
    });
    controller.audio.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player)));
    controller.selection_state.range.set_range(Some(selection));
    controller.apply_selection(Some(selection));
    controller.ui.waveform.loop_enabled = true;
    let _ = controller.play_audio(true, None);
    if !controller.is_playing() {
        return None;
    }
    Some(controller)
}

fn insert_sample_bpm(source: &SampleSource, relative_path: &Path, bpm: f64) {
    let wav_path = source.root.join(relative_path);
    let metadata = std::fs::metadata(&wav_path).expect("wav metadata");
    let modified_ns = metadata
        .modified()
        .expect("modified time")
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("system time")
        .as_nanos() as i64;
    let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), relative_path);
    let conn = analysis_jobs::open_source_db(&source.root).expect("source db");
    conn.execute(
        "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version, bpm)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, ?5)
         ON CONFLICT(sample_id) DO UPDATE SET bpm = excluded.bpm",
        params![sample_id, "test", metadata.len() as i64, modified_ns, bpm],
    )
    .expect("insert sample bpm");
}
