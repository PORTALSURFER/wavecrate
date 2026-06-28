use std::path::{Path, PathBuf};
use std::time::Instant;

use radiant::prelude as ui;
use wavecrate::sample_sources::{HarvestDerivationOperation, SampleSource, SourceDatabase};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::context_menu_target as context_menu;
use crate::native_app::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextTargetKind,
};
use crate::native_app::sample_library::file_actions::sample_path_label;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct ContextSampleDoubleResult {
    pub(in crate::native_app) destination: PathBuf,
}

#[derive(Clone, Debug)]
struct DuplicateDoubleRequest {
    source_path: PathBuf,
    target_folder: PathBuf,
    target_source_root: PathBuf,
    target_database_root: PathBuf,
}

impl NativeAppState {
    pub(in crate::native_app) fn duplicate_context_sample_same(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        match self.duplicate_context_sample_same_path(&menu) {
            Ok(destination) => {
                let duplicated_id = destination.display().to_string();
                self.library
                    .folder_browser
                    .refresh_file_path_across_sources(&destination);
                self.library.folder_browser.select_file(duplicated_id);
                self.library
                    .folder_browser
                    .follow_selected_file_view_matching_tags(12, 6, 2, &self.metadata.tags_by_file);
                self.ui.status.sample = format!("Duplicated {}", sample_path_label(&destination));
                emit_gui_action(
                    "browser.context_menu.sample.duplicate_same",
                    Some("browser"),
                    Some(context_menu::target_label(&destination).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.context_menu.sample.duplicate_same",
                    Some("browser"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn duplicate_context_sample_same_path(
        &self,
        menu: &BrowserContextMenu,
    ) -> Result<PathBuf, String> {
        if menu.kind != BrowserContextTargetKind::Sample {
            return Err(String::from("Choose a sample to duplicate"));
        }
        if menu.sample_missing || !menu.path.is_file() {
            return Err(String::from("Sample file is missing"));
        }
        let Some((source_root, source_database_root)) =
            self.context_sample_source_roots(menu.source_id.as_deref(), &menu.path)
        else {
            return Err(String::from("Sample source is unavailable"));
        };
        let destination = duplicate_same_destination(&menu.path);
        duplicate_sample_file_with_metadata(
            &menu.path,
            &destination,
            &source_root,
            &source_database_root,
        )?;
        Ok(destination)
    }

    fn context_sample_source_roots(
        &self,
        source_id: Option<&str>,
        path: &Path,
    ) -> Option<(PathBuf, PathBuf)> {
        source_id
            .and_then(|source_id| self.library.folder_browser.source_roots(source_id))
            .or_else(|| self.library.folder_browser.source_roots_for_path(path))
    }

    pub(in crate::native_app) fn duplicate_context_sample_double(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        let source_path = menu.path.clone();
        match self.duplicate_context_sample_double_request(&menu) {
            Ok(request) => {
                self.ui.status.sample = format!("Duplicating {}", sample_path_label(&source_path));
                context
                    .business()
                    .background("gui-sample-duplicate-double")
                    .run(
                        move |_| execute_duplicate_double(request),
                        move |result| GuiMessage::ContextSampleDoubleFinished {
                            source_path,
                            started_at,
                            result,
                        },
                    );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.context_menu.sample.duplicate_double",
                    Some("browser"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn duplicate_context_sample_double_request(
        &self,
        menu: &BrowserContextMenu,
    ) -> Result<DuplicateDoubleRequest, String> {
        if menu.kind != BrowserContextTargetKind::Sample {
            return Err(String::from("Choose a sample to duplicate"));
        }
        if menu.sample_missing {
            return Err(String::from("Sample file is missing"));
        }
        let Some((origin_source, _)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(&menu.path)
        else {
            return Err(String::from("Sample source is unavailable"));
        };
        let (target_folder, target_source) =
            self.double_duplicate_target(&menu.path, &origin_source)?;
        if let Some(error) = self
            .library
            .folder_browser
            .folder_target_lock_error(&target_folder, "Duplicate")
        {
            return Err(error);
        }
        let target_database_root = target_source
            .database_root()
            .map_err(|err| format!("Resolve target metadata location failed: {err}"))?;
        Ok(DuplicateDoubleRequest {
            source_path: menu.path.clone(),
            target_folder,
            target_source_root: target_source.root,
            target_database_root,
        })
    }

    fn double_duplicate_target(
        &self,
        source_path: &Path,
        origin_source: &SampleSource,
    ) -> Result<(PathBuf, SampleSource), String> {
        if origin_source.is_protected() {
            let Some(primary_source) = self.library.folder_browser.primary_sample_source() else {
                return Err(String::from(
                    "Set a Primary source before duplicating from a protected source",
                ));
            };
            let target_folder = self
                .harvest_destination_for_protected_origin(source_path)?
                .unwrap_or_else(|| primary_source.primary_import_path());
            return Ok((target_folder, primary_source));
        }
        let target_folder = source_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| String::from("Sample file has no destination folder"))?;
        Ok((target_folder, origin_source.clone()))
    }

    pub(in crate::native_app) fn finish_context_sample_double(
        &mut self,
        source_path: PathBuf,
        started_at: Instant,
        result: Result<ContextSampleDoubleResult, String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match result {
            Ok(completion) => {
                self.library
                    .folder_browser
                    .refresh_file_path_across_sources(&completion.destination);
                self.library
                    .folder_browser
                    .focus_file_across_sources_matching_tags(
                        &completion.destination,
                        &self.metadata.tags_by_file,
                    );
                self.record_harvest_whole_file_derivation(
                    &source_path,
                    &completion.destination,
                    HarvestDerivationOperation::DuplicateDoubleCopy,
                );
                self.load_navigation_sample_validated(
                    completion.destination.to_string_lossy().to_string(),
                    context,
                    started_at,
                );
                self.ui.status.sample =
                    format!("Duplicated {}", sample_path_label(&completion.destination));
                emit_gui_action(
                    "browser.context_menu.sample.duplicate_double",
                    Some("browser"),
                    Some(context_menu::target_label(&completion.destination).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.context_menu.sample.duplicate_double",
                    Some("browser"),
                    Some(context_menu::target_label(&source_path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }
}

fn duplicate_same_destination(source: &Path) -> PathBuf {
    let parent = source.parent().unwrap_or_else(|| Path::new(""));
    let stem = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("sample"));
    let extension = source
        .extension()
        .map(|extension| extension.to_string_lossy().to_string());
    for count in 1.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem}_copy{count:03}.{extension}"),
            None => format!("{stem}_copy{count:03}"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded copy suffix search should find a destination")
}

fn duplicate_sample_file_with_metadata(
    source: &Path,
    destination: &Path,
    source_root: &Path,
    source_database_root: &Path,
) -> Result<(), String> {
    std::fs::copy(source, destination)
        .map(|_| ())
        .map_err(|err| format!("Duplicate failed: {err}"))?;
    if let Err(error) = wavecrate::sample_sources::persist_copied_file_metadata(
        source_root,
        source_database_root,
        source,
        destination,
    ) {
        let _ = std::fs::remove_file(destination);
        return Err(error);
    }
    Ok(())
}

fn execute_duplicate_double(
    request: DuplicateDoubleRequest,
) -> Result<ContextSampleDoubleResult, String> {
    wavecrate::sample_sources::harvest_file_ops::ensure_dir(
        &request.target_folder,
        "Create duplicate destination failed",
    )?;
    let destination = next_doubled_destination(&request.source_path, &request.target_folder)?;
    let result = write_doubled_wav(&request.source_path, &destination)
        .and_then(|()| register_fresh_duplicate(&request, &destination));
    if result.is_err() {
        let _ = std::fs::remove_file(&destination);
    }
    result.map(|()| ContextSampleDoubleResult { destination })
}

fn next_doubled_destination(source: &Path, target_folder: &Path) -> Result<PathBuf, String> {
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| String::from("Source sample has no file name"))?;
    for index in 0..10_000 {
        let suffix = if index == 0 {
            String::from("_doubled")
        } else {
            format!("_doubled_{index:03}")
        };
        let candidate = target_folder.join(format!("{stem}{suffix}.wav"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(String::from("Unable to find a duplicate file name"))
}

fn write_doubled_wav(source: &Path, destination: &Path) -> Result<(), String> {
    let first_reader = open_wav_reader(source)?;
    let spec = first_reader.spec();
    if first_reader.duration() == 0 {
        return Err(String::from("Source WAV contains no audio"));
    }
    let file = std::fs::File::create(destination)
        .map_err(|err| format!("Failed to create doubled sample: {err}"))?;
    let writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
    let mut writer = hound::WavWriter::new(writer, spec)
        .map_err(|err| format!("Failed to write doubled sample: {err}"))?;
    write_reader_samples(first_reader, spec, &mut writer)?;
    let second_reader = open_wav_reader(source)?;
    write_reader_samples(second_reader, spec, &mut writer)?;
    writer
        .finalize()
        .map_err(|err| format!("Failed to finalize doubled sample: {err}"))
}

fn open_wav_reader(source: &Path) -> Result<hound::WavReader<impl std::io::Read>, String> {
    let file = wavecrate::wav_sanitize::open_sanitized_wav(source)
        .map_err(|err| format!("Invalid WAV: {err}"))?;
    Ok(
        hound::WavReader::new(std::io::BufReader::with_capacity(1024 * 1024, file))
            .map_err(|err| format!("Invalid WAV: {err}"))?,
    )
}

fn write_reader_samples<W: std::io::Write + std::io::Seek, R: std::io::Read>(
    mut reader: hound::WavReader<R>,
    spec: hound::WavSpec,
    writer: &mut hound::WavWriter<W>,
) -> Result<(), String> {
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                writer
                    .write_sample(sample.map_err(|err| format!("Invalid WAV sample data: {err}"))?)
                    .map_err(|err| format!("Failed to write doubled sample: {err}"))?;
            }
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            for sample in reader.samples::<i16>() {
                writer
                    .write_sample(sample.map_err(|err| format!("Invalid WAV sample data: {err}"))?)
                    .map_err(|err| format!("Failed to write doubled sample: {err}"))?;
            }
        }
        hound::SampleFormat::Int => {
            for sample in reader.samples::<i32>() {
                writer
                    .write_sample(sample.map_err(|err| format!("Invalid WAV sample data: {err}"))?)
                    .map_err(|err| format!("Failed to write doubled sample: {err}"))?;
            }
        }
    }
    Ok(())
}

fn register_fresh_duplicate(
    request: &DuplicateDoubleRequest,
    destination: &Path,
) -> Result<(), String> {
    let relative_path = destination
        .strip_prefix(&request.target_source_root)
        .map_err(|_| String::from("Duplicate metadata update failed: target file mismatch"))?;
    let metadata = std::fs::metadata(destination)
        .map_err(|err| format!("Duplicate metadata update failed: {err}"))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Duplicate metadata update failed: {err}"))?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|_| String::from("Duplicate modified time is before epoch"))?
        .as_nanos() as i64;
    let db = SourceDatabase::open_for_user_metadata_write_with_database_root(
        &request.target_source_root,
        &request.target_database_root,
    )
    .map_err(|err| format!("Duplicate metadata update failed: {err}"))?;
    db.upsert_file(relative_path, metadata.len(), modified_ns)
        .map_err(|err| format!("Duplicate metadata update failed: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_i16_wav(path: &Path, samples: &[i16]) {
        let spec = hound::WavSpec {
            channels: 2,
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

    #[test]
    fn doubled_wav_repeats_audio_and_preserves_spec() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("loop.wav");
        let destination = temp.path().join("loop_doubled.wav");
        write_i16_wav(&source, &[1, -2, 3, -4, 5, -6]);

        write_doubled_wav(&source, &destination).expect("double wav");

        let mut reader = hound::WavReader::open(&destination).expect("open doubled wav");
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(reader.duration(), 6);
        let samples = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .expect("read samples");
        assert_eq!(samples, vec![1, -2, 3, -4, 5, -6, 1, -2, 3, -4, 5, -6]);
    }

    #[test]
    fn doubled_destination_numbers_collisions() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("kick.wav");
        std::fs::write(temp.path().join("kick_doubled.wav"), []).expect("collision");

        let destination = next_doubled_destination(&source, temp.path()).expect("destination");

        assert_eq!(destination, temp.path().join("kick_doubled_001.wav"));
    }
}
