use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use radiant::prelude as ui;
use wavecrate::sample_sources::{SourceDatabase, config::AudioWriteFormatConfig};
use wavecrate::selection::SelectionRange;

use crate::native_app::app::{
    GuiMessage, NativeAppState, PendingWaveformDestructiveEdit, WaveformDestructiveEditKind,
    sample_path_label,
};
use crate::native_app::transaction_history::TransactionContext;
use crate::native_app::waveform::WaveformState;
use crate::native_app::waveform::execute_waveform_extraction;
use crate::native_app::waveform_edit_effects::apply_edit_selection_effects;

#[derive(Clone, Debug)]
struct OverwriteBackup {
    dir: PathBuf,
    before: PathBuf,
    after: PathBuf,
}

impl OverwriteBackup {
    fn capture_before(target: &Path) -> Result<Self, String> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| format!("Clock error: {err}"))?
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wavecrate-native-edit-{stamp}"));
        fs::create_dir_all(&dir).map_err(|err| format!("Failed to create undo folder: {err}"))?;
        let before = dir.join("before.wav");
        let after = dir.join("after.wav");
        fs::copy(target, &before).map_err(|err| format!("Failed to snapshot audio file: {err}"))?;
        Ok(Self { dir, before, after })
    }

    fn capture_after(&self, target: &Path) -> Result<(), String> {
        fs::copy(target, &self.after)
            .map_err(|err| format!("Failed to snapshot edited audio file: {err}"))?;
        Ok(())
    }

    fn capture_extracted(&self, target: &Path) -> Result<PathBuf, String> {
        let extracted = self.dir.join("extracted.wav");
        fs::copy(target, &extracted)
            .map_err(|err| format!("Failed to snapshot extracted audio file: {err}"))?;
        Ok(extracted)
    }
}

impl Drop for OverwriteBackup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[derive(Clone, Debug)]
struct AppliedWaveformEdit {
    source_id: String,
    source_root: PathBuf,
    relative_path: PathBuf,
    absolute_path: PathBuf,
    backup: OverwriteBackup,
    extracted: Option<AppliedExtractedFile>,
}

#[derive(Clone, Debug)]
struct AppliedExtractedFile {
    path: PathBuf,
    relative_path: PathBuf,
    backup_path: PathBuf,
}

#[derive(Clone)]
struct EditableWav {
    samples: Vec<f32>,
    channels: usize,
    sample_rate: u32,
}

impl NativeAppState {
    pub(in crate::native_app) fn request_crop_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(WaveformDestructiveEditKind::CropSelection, context);
    }

    pub(in crate::native_app) fn request_trim_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(WaveformDestructiveEditKind::TrimSelection, context);
    }

    pub(in crate::native_app) fn request_extract_and_trim_waveform_selection(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::ExtractAndTrimSelection,
            context,
        );
    }

    pub(in crate::native_app) fn request_apply_edit_selection_effects(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.request_waveform_destructive_edit(
            WaveformDestructiveEditKind::ApplyEditSelectionEffects,
            context,
        );
    }

    fn request_waveform_destructive_edit(
        &mut self,
        kind: WaveformDestructiveEditKind,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let request = match self.pending_destructive_edit_request(kind) {
            Ok(request) => request,
            Err(error) => {
                self.ui.status.sample = error;
                return;
            }
        };

        if self.ui.settings.persisted.controls.destructive_yolo_mode {
            self.ui
                .browser_interaction
                .pending_waveform_destructive_edit = None;
            if let Err(error) = self.apply_destructive_edit_request(&request, context) {
                self.ui.status.sample = format!("{} failed: {error}", kind.action_label());
            }
            return;
        }

        self.ui
            .browser_interaction
            .pending_waveform_destructive_edit = Some(request);
    }

    pub(in crate::native_app) fn confirm_pending_waveform_destructive_edit(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(request) = self
            .ui
            .browser_interaction
            .pending_waveform_destructive_edit
            .take()
        else {
            return;
        };

        let result = self.apply_destructive_edit_request(&request, context);
        if let Err(error) = result {
            self.ui.status.sample = format!("Edit failed: {error}");
        }
    }

    pub(in crate::native_app) fn cancel_pending_waveform_destructive_edit(&mut self) {
        self.ui
            .browser_interaction
            .pending_waveform_destructive_edit = None;
    }

    fn pending_destructive_edit_request(
        &self,
        kind: WaveformDestructiveEditKind,
    ) -> Result<PendingWaveformDestructiveEdit, String> {
        let absolute_path = self.waveform.current.path();
        if !self.waveform.current.has_loaded_sample() || absolute_path.as_os_str().is_empty() {
            return Err(format!("Load a sample before {}", kind.gerund_label()));
        }
        let selection = self.destructive_edit_selection_for_kind(kind)?;
        let (source, relative_path) = self
            .library
            .folder_browser
            .sample_source_for_file_path(&absolute_path)
            .ok_or_else(|| String::from("Loaded sample is not inside a configured source"))?;
        validate_destructive_edit_target(&absolute_path)?;
        Ok(PendingWaveformDestructiveEdit {
            prompt: destructive_edit_prompt(kind, &self.ui.settings.persisted.audio_write_format),
            source,
            relative_path,
            absolute_path,
            selection,
        })
    }

    fn apply_destructive_edit_request(
        &mut self,
        request: &PendingWaveformDestructiveEdit,
        _context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> Result<(), String> {
        let before_selected_path = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let playback_was_active = self.waveform.current.is_playing();
        self.stop_audio_output_playback();
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.pending_playback_start = None;
        self.audio.pending_sample_playback = None;
        self.audio.pending_runtime_start = None;

        let extracted_path = self.extract_for_destructive_edit_request(request)?;
        let applied = match execute_destructive_edit_write(request, extracted_path.clone()) {
            Ok(applied) => applied,
            Err(error) => {
                if let Some(path) = extracted_path {
                    cleanup_failed_destructive_extraction(&request.source.root, &path);
                }
                return Err(error);
            }
        };
        self.apply_destructive_edit_visual_state(
            &applied,
            before_selected_path.as_deref(),
            request,
        )?;
        self.register_destructive_edit_transaction(request.prompt.edit, applied);

        let label = sample_path_label(&request.absolute_path.display().to_string());
        self.ui.status.sample = if playback_was_active {
            format!(
                "{} {label} and stopped playback",
                request.prompt.edit.past_tense_label()
            )
        } else {
            format!("{} {label}", request.prompt.edit.past_tense_label())
        };
        Ok(())
    }

    fn extract_for_destructive_edit_request(
        &mut self,
        request: &PendingWaveformDestructiveEdit,
    ) -> Result<Option<PathBuf>, String> {
        if request.prompt.edit != WaveformDestructiveEditKind::ExtractAndTrimSelection {
            return Ok(None);
        }
        let extraction = self
            .waveform
            .current
            .selection_extraction_request(None, request.selection)?;
        let completion = execute_waveform_extraction(extraction);
        let path = completion.result?;
        self.waveform
            .current
            .mark_extracted_play_selection(&completion.source_path, completion.selection);
        self.waveform.current.flash_play_selection();
        Ok(Some(path))
    }

    fn apply_destructive_edit_visual_state(
        &mut self,
        applied: &AppliedWaveformEdit,
        before_selected_path: Option<&str>,
        request: &PendingWaveformDestructiveEdit,
    ) -> Result<(), String> {
        self.evict_waveform_cache_path(&applied.absolute_path);
        self.library
            .folder_browser
            .refresh_filesystem_paths(&applied.source_id, &[applied.relative_path.clone()]);
        if let Some(extracted) = applied.extracted.as_ref() {
            self.library
                .folder_browser
                .refresh_file_path(&extracted.path);
        }
        if before_selected_path == Some(applied.absolute_path.to_string_lossy().as_ref()) {
            self.library
                .folder_browser
                .select_file(applied.absolute_path.display().to_string());
        }
        self.reload_waveform_path_now_if_loaded(&applied.absolute_path)?;
        if request.prompt.edit == WaveformDestructiveEditKind::ApplyEditSelectionEffects
            && self.waveform.current.path() == applied.absolute_path
        {
            self.waveform
                .current
                .set_edit_selection_range(request.selection.clear_fades().with_gain(1.0));
            self.waveform.current.flash_edit_selection();
        }
        Ok(())
    }

    fn destructive_edit_selection_for_kind(
        &self,
        kind: WaveformDestructiveEditKind,
    ) -> Result<SelectionRange, String> {
        if kind == WaveformDestructiveEditKind::ApplyEditSelectionEffects {
            let selection = self
                .waveform
                .current
                .edit_selection()
                .filter(|selection| selection.width() > 0.0)
                .ok_or_else(|| String::from("Set an edit selection before applying it"))?;
            if !selection.has_edit_effects() {
                return Err(String::from(
                    "Adjust an edit fade or gain before applying it",
                ));
            }
            return Ok(selection);
        }
        self.waveform
            .current
            .destructive_edit_selection()
            .ok_or_else(|| format!("Mark an edit or play range before {}", kind.gerund_label()))
    }

    fn register_destructive_edit_transaction(
        &mut self,
        kind: WaveformDestructiveEditKind,
        applied: AppliedWaveformEdit,
    ) {
        let undo_applied = applied.clone();
        let redo_applied = applied;
        self.begin_transaction(kind.transaction_label());
        self.register_transaction_action(
            kind.undo_label(),
            move |transaction| {
                transaction.restore_edited_waveform(&undo_applied.backup.before, &undo_applied)
            },
            move |transaction| {
                transaction.restore_edited_waveform(&redo_applied.backup.after, &redo_applied)
            },
        );
        self.commit_transaction();
    }

    fn reload_waveform_path_now_if_loaded(&mut self, absolute_path: &Path) -> Result<(), String> {
        if self.waveform.current.path() != absolute_path {
            return Ok(());
        }
        self.waveform.current = WaveformState::load_path_with_progress_and_cancel(
            absolute_path.to_path_buf(),
            |_| {},
            || false,
        )?;
        Ok(())
    }
}

impl TransactionContext<'_> {
    fn restore_edited_waveform(
        &mut self,
        backup_path: &Path,
        applied: &AppliedWaveformEdit,
    ) -> Result<(), String> {
        fs::copy(backup_path, &applied.absolute_path)
            .map_err(|err| format!("Failed to restore waveform file: {err}"))?;
        sync_source_entry(
            &applied.source_root,
            &applied.relative_path,
            &applied.absolute_path,
        )?;
        if let Some(extracted) = applied.extracted.as_ref() {
            if backup_path == applied.backup.before.as_path() {
                remove_extracted_file_for_undo(&applied.source_root, extracted)?;
            } else {
                restore_extracted_file_for_redo(&applied.source_root, extracted)?;
            }
        }
        self.state.evict_waveform_cache_path(&applied.absolute_path);
        let mut relative_paths = vec![applied.relative_path.clone()];
        if let Some(extracted) = applied.extracted.as_ref() {
            relative_paths.push(extracted.relative_path.clone());
        }
        self.state
            .library
            .folder_browser
            .refresh_filesystem_paths(&applied.source_id, &relative_paths);
        self.state
            .reload_waveform_path_now_if_loaded(&applied.absolute_path)?;
        Ok(())
    }
}

impl WaveformDestructiveEditKind {
    fn action_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Crop",
            Self::TrimSelection => "Trim",
            Self::ExtractAndTrimSelection => "Extract and trim",
            Self::ApplyEditSelectionEffects => "Apply edit mark edits",
        }
    }

    fn gerund_label(self) -> &'static str {
        match self {
            Self::CropSelection => "cropping",
            Self::TrimSelection => "trimming",
            Self::ExtractAndTrimSelection => "extracting and trimming",
            Self::ApplyEditSelectionEffects => "applying edit mark edits",
        }
    }

    fn past_tense_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Cropped",
            Self::TrimSelection => "Trimmed",
            Self::ExtractAndTrimSelection => "Extracted and trimmed",
            Self::ApplyEditSelectionEffects => "Applied edit mark edits to",
        }
    }

    fn transaction_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Crop waveform selection",
            Self::TrimSelection => "Trim waveform selection",
            Self::ExtractAndTrimSelection => "Extract and trim waveform selection",
            Self::ApplyEditSelectionEffects => "Apply edit mark edits",
        }
    }

    fn undo_label(self) -> &'static str {
        match self {
            Self::CropSelection => "Restore cropped audio",
            Self::TrimSelection => "Restore trimmed audio",
            Self::ExtractAndTrimSelection => "Restore extracted and trimmed audio",
            Self::ApplyEditSelectionEffects => "Restore edit mark edits",
        }
    }
}

fn destructive_edit_prompt(
    edit: WaveformDestructiveEditKind,
    write_format: &AudioWriteFormatConfig,
) -> crate::native_app::app::WaveformDestructiveEditPrompt {
    let message = match edit {
        WaveformDestructiveEditKind::CropSelection => {
            "This will keep only the selected region and remove audio outside it from the source file."
        }
        WaveformDestructiveEditKind::TrimSelection => {
            "This will remove the selected region and close the gap in the source file."
        }
        WaveformDestructiveEditKind::ExtractAndTrimSelection => {
            "This will extract the selected region into a new sibling file, then remove that region and close the gap in the source file."
        }
        WaveformDestructiveEditKind::ApplyEditSelectionEffects => {
            "This will overwrite the edit selection with the currently previewed fade and gain edits."
        }
    };
    crate::native_app::app::WaveformDestructiveEditPrompt {
        edit,
        title: destructive_edit_title(edit),
        message: format!(
            "{message} Wavecrate will rewrite the file using the current write format: {}.",
            write_format.summary_label()
        ),
    }
}

fn destructive_edit_title(edit: WaveformDestructiveEditKind) -> String {
    match edit {
        WaveformDestructiveEditKind::ApplyEditSelectionEffects => {
            String::from("Apply edit mark edits")
        }
        _ => format!("{} selection", edit.action_label()),
    }
}

fn validate_destructive_edit_target(path: &Path) -> Result<(), String> {
    let is_wav = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"));
    if !is_wav {
        return Err(String::from("This edit currently supports WAV files"));
    }
    let reader = hound::WavReader::open(path).map_err(|err| format!("Invalid WAV: {err}"))?;
    let channels = reader.spec().channels;
    if channels == 0 || channels > 2 {
        return Err(String::from(
            "This edit currently supports mono or stereo WAV files",
        ));
    }
    Ok(())
}

fn execute_destructive_edit_write(
    request: &PendingWaveformDestructiveEdit,
    extracted_path: Option<PathBuf>,
) -> Result<AppliedWaveformEdit, String> {
    let backup = OverwriteBackup::capture_before(&request.absolute_path)?;
    let extracted = extracted_path
        .map(|path| {
            let relative_path = source_relative_path(&request.source.root, &path)?;
            let backup_path = backup.capture_extracted(&path)?;
            sync_source_entry(&request.source.root, &relative_path, &path)?;
            Ok::<_, String>(AppliedExtractedFile {
                path,
                relative_path,
                backup_path,
            })
        })
        .transpose()?;
    let mut wav = load_editable_wav(&request.absolute_path)?;
    apply_destructive_edit_to_wav(&mut wav, request.prompt.edit, request.selection)?;
    if wav.samples.is_empty() {
        return Err(format!(
            "No audio data after {}",
            request.prompt.edit.gerund_label()
        ));
    }
    write_edited_wav(&request.absolute_path, &wav)?;
    sync_source_entry(
        &request.source.root,
        &request.relative_path,
        &request.absolute_path,
    )?;
    backup.capture_after(&request.absolute_path)?;
    Ok(AppliedWaveformEdit {
        source_id: request.source.id.as_str().to_string(),
        source_root: request.source.root.clone(),
        relative_path: request.relative_path.clone(),
        absolute_path: request.absolute_path.clone(),
        backup,
        extracted,
    })
}

fn apply_destructive_edit_to_wav(
    wav: &mut EditableWav,
    edit: WaveformDestructiveEditKind,
    selection: wavecrate::selection::SelectionRange,
) -> Result<(), String> {
    let total_frames = wav.samples.len() / wav.channels.max(1);
    let (start_frame, end_frame) = selection_frame_bounds(total_frames, selection);
    let start = start_frame * wav.channels;
    let end = end_frame * wav.channels;
    match edit {
        WaveformDestructiveEditKind::CropSelection => {
            wav.samples = wav.samples[start..end].to_vec();
        }
        WaveformDestructiveEditKind::TrimSelection
        | WaveformDestructiveEditKind::ExtractAndTrimSelection => {
            wav.samples.drain(start..end);
        }
        WaveformDestructiveEditKind::ApplyEditSelectionEffects => {
            apply_edit_selection_effects(
                &mut wav.samples,
                wav.channels,
                wav.sample_rate,
                selection,
                start_frame,
                end_frame,
            );
        }
    }
    if wav.samples.is_empty() {
        return Err(format!("No audio data after {}", edit.gerund_label()));
    }
    Ok(())
}

fn load_editable_wav(path: &Path) -> Result<EditableWav, String> {
    let mut reader = hound::WavReader::open(path).map_err(|err| format!("Invalid WAV: {err}"))?;
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("Failed to read WAV samples: {err}"))?,
        hound::SampleFormat::Int => {
            let scale = (1_i64 << spec.bits_per_sample.saturating_sub(1)).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| value as f32 / scale)
                        .map_err(|err| format!("Failed to read WAV samples: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    if samples.is_empty() {
        return Err(String::from("No audio data available"));
    }
    Ok(EditableWav {
        samples,
        channels,
        sample_rate: spec.sample_rate.max(1),
    })
}

fn write_edited_wav(path: &Path, wav: &EditableWav) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: wav.channels as u16,
        sample_rate: wav.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|err| format!("Failed to write wav: {err}"))?;
    for sample in &wav.samples {
        writer
            .write_sample(*sample)
            .map_err(|err| format!("Failed to write sample: {err}"))?;
    }
    writer
        .finalize()
        .map_err(|err| format!("Failed to finalize wav: {err}"))
}

fn selection_frame_bounds(
    total_frames: usize,
    bounds: wavecrate::selection::SelectionRange,
) -> (usize, usize) {
    let start_frame = ((bounds.start() * total_frames as f32).floor() as usize)
        .min(total_frames.saturating_sub(1));
    let mut end_frame = ((bounds.end() * total_frames as f32).ceil() as usize).min(total_frames);
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    (start_frame, end_frame)
}

fn sync_source_entry(
    source_root: &Path,
    relative_path: &Path,
    absolute_path: &Path,
) -> Result<(), String> {
    let db =
        SourceDatabase::open(source_root).map_err(|err| format!("Database unavailable: {err}"))?;
    let metadata =
        fs::metadata(absolute_path).map_err(|err| format!("Failed to stat file: {err}"))?;
    let file_size = metadata.len();
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Failed to read modified time: {err}"))?
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("Failed to normalize modified time: {err}"))?
        .as_nanos() as i64;
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("Failed to sync database entry: {err}"))?;
    batch
        .upsert_file_without_hash(relative_path, file_size, modified_ns)
        .map_err(|err| format!("Failed to sync database entry: {err}"))?;
    batch
        .commit()
        .map_err(|err| format!("Failed to sync database entry: {err}"))
}

fn source_relative_path(source_root: &Path, absolute_path: &Path) -> Result<PathBuf, String> {
    absolute_path
        .strip_prefix(source_root)
        .map(Path::to_path_buf)
        .map_err(|_| String::from("Edited sample is not inside the configured source"))
}

fn remove_extracted_file_for_undo(
    source_root: &Path,
    extracted: &AppliedExtractedFile,
) -> Result<(), String> {
    match fs::remove_file(&extracted.path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Failed to remove extracted audio file: {error}")),
    }
    mark_source_entry_missing(source_root, &extracted.relative_path, true)
}

fn cleanup_failed_destructive_extraction(source_root: &Path, extracted_path: &Path) {
    let _ = fs::remove_file(extracted_path);
    let Ok(relative_path) = source_relative_path(source_root, extracted_path) else {
        return;
    };
    let _ = mark_source_entry_missing(source_root, &relative_path, true);
}

fn restore_extracted_file_for_redo(
    source_root: &Path,
    extracted: &AppliedExtractedFile,
) -> Result<(), String> {
    fs::copy(&extracted.backup_path, &extracted.path)
        .map_err(|err| format!("Failed to restore extracted audio file: {err}"))?;
    sync_source_entry(source_root, &extracted.relative_path, &extracted.path)
}

fn mark_source_entry_missing(
    source_root: &Path,
    relative_path: &Path,
    missing: bool,
) -> Result<(), String> {
    SourceDatabase::open(source_root)
        .map_err(|err| format!("Database unavailable: {err}"))?
        .set_missing(relative_path, missing)
        .map_err(|err| format!("Failed to sync database entry: {err}"))
}
