use super::*;

pub(super) struct WaveformSlideTarget {
    pub(super) source: SampleSource,
    pub(super) relative_path: PathBuf,
    pub(super) absolute_path: PathBuf,
}

impl AppController {
    pub(super) fn waveform_slide_target(&self) -> Result<WaveformSlideTarget, String> {
        let audio = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .ok_or_else(|| "Load a sample to edit it".to_string())?;
        let source = self
            .library
            .sources
            .iter()
            .find(|s| s.id == audio.source_id)
            .cloned()
            .ok_or_else(|| "Source not available for loaded sample".to_string())?;
        let relative_path = audio.relative_path.clone();
        let absolute_path = source.root.join(&relative_path);
        Ok(WaveformSlideTarget {
            source,
            relative_path,
            absolute_path,
        })
    }
}
