use super::*;

fn browser_test_sample_entry(
    name: &str,
    tag: crate::sample_sources::Rating,
) -> crate::sample_sources::WavEntry {
    crate::sample_sources::WavEntry {
        relative_path: PathBuf::from(name),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }
}

fn browser_test_write_wav(path: &std::path::Path, samples: &[f32]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 8,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav fixture");
    for sample in samples {
        writer.write_sample(*sample).expect("write wav sample");
    }
    writer.finalize().expect("finalize wav fixture");
}

fn browser_visible_paths(controller: &mut AppController) -> Vec<PathBuf> {
    (0..controller.visible_browser_len())
        .filter_map(|row| controller.browser_path_for_visible(row))
        .collect()
}

mod browser_row_actions;
mod config;
mod folder_actions;
mod source_row_actions;
