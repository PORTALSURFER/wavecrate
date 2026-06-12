use super::helpers::{cleanup_written_export_after_registration_failure, crop_selection_samples};
use super::*;

impl AppController {
    pub(crate) fn export_selection_clip(
        &mut self,
        request: SelectionClipExportRequest<'_>,
    ) -> Result<WavEntry, String> {
        let audio = self.selection_audio(request.source_id, request.relative_path)?;
        let source = self
            .library
            .sources
            .iter()
            .find(|s| &s.id == request.source_id)
            .cloned()
            .ok_or_else(|| "Source not available".to_string())?;
        let target_rel = self.next_selection_path_in_dir(&source.root, &audio.relative_path);
        self.write_and_record_selection_clip(request, audio, source, target_rel)
    }

    pub(crate) fn export_selection_clip_in_folder(
        &mut self,
        request: SelectionClipExportRequest<'_>,
        folder: &Path,
    ) -> Result<WavEntry, String> {
        let audio = self.selection_audio(request.source_id, request.relative_path)?;
        let source = self
            .library
            .sources
            .iter()
            .find(|s| &s.id == request.source_id)
            .cloned()
            .ok_or_else(|| "Source not available".to_string())?;
        let name_hint = folder.join(
            audio
                .relative_path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("selection.wav")),
        );
        let target_rel = self.next_selection_path_in_dir(&source.root, &name_hint);
        self.write_and_record_selection_clip(request, audio, source, target_rel)
    }

    pub(crate) fn export_selection_clip_to_root(
        &mut self,
        request: SelectionClipExportRequest<'_>,
        clip_root: &Path,
        name_hint: &Path,
    ) -> Result<WavEntry, String> {
        let audio = self.selection_audio(request.source_id, request.relative_path)?;
        let target_rel = self.next_selection_path_in_dir(clip_root, name_hint);
        let target_abs = clip_root.join(&target_rel);
        let (mut samples, spec) = crop_selection_samples(&audio, request.bounds)?;
        self.apply_auto_edge_fades_to_selection_export(
            &mut samples,
            spec.sample_rate,
            spec.channels,
        );
        write_wav_with_spec(
            &target_abs,
            &samples,
            self.settings
                .audio_write_format
                .wav_spec_for_source(spec.channels, spec.sample_rate),
        )?;
        let source = SampleSource {
            id: SourceId::new(),
            root: clip_root.to_path_buf(),
        };
        // Clips saved outside sources are not inserted into browser or source DB.
        let (looped, bpm) = self.selection_export_metadata();
        self.record_selection_entry(SelectionEntryRecordRequest {
            source: &source,
            relative_path: target_rel,
            target_tag: request.target_tag,
            add_to_browser: false,
            register_in_source: false,
            looped,
            bpm,
        })
    }

    pub(crate) fn selection_audio(
        &self,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> Result<LoadedAudio, String> {
        let Some(audio) = self.sample_view.wav.loaded_audio.as_ref() else {
            return Err("Selection audio not available; load a sample first".into());
        };
        if &audio.source_id != source_id || audio.relative_path != relative_path {
            return Err("Selection no longer matches the loaded sample".into());
        }
        Ok(audio.clone())
    }

    fn write_and_record_selection_clip(
        &mut self,
        request: SelectionClipExportRequest<'_>,
        audio: LoadedAudio,
        source: SampleSource,
        target_rel: PathBuf,
    ) -> Result<WavEntry, String> {
        let target_abs = source.root.join(&target_rel);
        let (mut samples, spec) = crop_selection_samples(&audio, request.bounds)?;
        self.apply_auto_edge_fades_to_selection_export(
            &mut samples,
            spec.sample_rate,
            spec.channels,
        );
        write_wav_with_spec(
            &target_abs,
            &samples,
            self.settings
                .audio_write_format
                .wav_spec_for_source(spec.channels, spec.sample_rate),
        )?;
        let (looped, bpm) = self.selection_export_metadata();
        self.record_selection_entry(SelectionEntryRecordRequest {
            source: &source,
            relative_path: target_rel,
            target_tag: request.target_tag,
            add_to_browser: request.add_to_browser,
            register_in_source: request.register_in_source,
            looped,
            bpm,
        })
        .map_err(|err| cleanup_written_export_after_registration_failure(&target_abs, err))
    }
}
