//! Audio recording pipeline and monitoring utilities.

mod capture;
mod monitor;
mod writer;

use std::path::{Path, PathBuf};

use cpal::Stream;
use cpal::traits::StreamTrait;

use super::input::{AudioInputConfig, AudioInputError, ResolvedInput};
use monitor::{MonitorSenderSlot, set_monitor_sender};
use writer::RecorderWriter;

/// Summary returned when a recording completes.
pub struct RecordingOutcome {
    /// Path to the recorded WAV file on disk.
    pub path: PathBuf,
    /// Input configuration used for the capture.
    pub resolved: ResolvedInput,
    /// Number of audio frames written.
    pub frames: u64,
    /// Duration of the recording in seconds.
    pub duration_seconds: f32,
}

/// Active audio recorder that streams samples to a WAV file.
pub struct AudioRecorder {
    stream: Option<Stream>,
    writer: Option<RecorderWriter>,
    resolved: ResolvedInput,
    path: PathBuf,
    monitor_sender: MonitorSenderSlot,
    active: bool,
}

impl AudioRecorder {
    /// Start recording to the provided output path using the given input config.
    pub fn start(config: &AudioInputConfig, path: PathBuf) -> Result<Self, AudioInputError> {
        let runtime = capture::start_recording_runtime(config, &path)?;
        runtime
            .stream
            .play()
            .map_err(|source| AudioInputError::StartStream { source })?;
        Ok(Self {
            stream: Some(runtime.stream),
            writer: Some(runtime.writer),
            resolved: runtime.resolved,
            path,
            monitor_sender: runtime.monitor_sender,
            active: true,
        })
    }

    /// Stop recording and return summary metadata.
    pub fn stop(&mut self) -> Result<RecordingOutcome, AudioInputError> {
        if !self.active {
            return Err(AudioInputError::RecordingFailed {
                detail: "Recorder already stopped".into(),
            });
        }
        self.active = false;
        drop(self.stream.take());
        let mut writer = self
            .writer
            .take()
            .ok_or_else(|| AudioInputError::RecordingFailed {
                detail: "Recorder writer unavailable".into(),
            })?;
        writer.stop();
        let stats = writer.join()?;
        let duration_seconds = if stats.frames == 0 {
            0.0
        } else {
            stats.frames as f32 / self.resolved.sample_rate.max(1) as f32
        };
        Ok(RecordingOutcome {
            path: self.path.clone(),
            resolved: self.resolved.clone(),
            frames: stats.frames,
            duration_seconds,
        })
    }

    /// Return true while the recorder is actively capturing audio.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Return the resolved input configuration used for recording.
    pub fn resolved(&self) -> &ResolvedInput {
        &self.resolved
    }

    /// Return the output path for the recording.
    pub fn output_path(&self) -> &Path {
        &self.path
    }

    /// Attach a monitor that receives live audio samples.
    pub fn attach_monitor(&self, monitor: &monitor::InputMonitor) {
        self.set_monitor_sender(Some(monitor.sender()));
    }

    /// Detach any active input monitor.
    pub fn detach_monitor(&self) {
        self.set_monitor_sender(None);
    }

    fn set_monitor_sender(&self, sender: Option<std::sync::mpsc::Sender<monitor::MonitorCommand>>) {
        set_monitor_sender(&self.monitor_sender, sender);
    }
}

pub use monitor::InputMonitor;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn recorder_is_active_clears_after_stop() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("recording.wav");
        let (sender, receiver) = std::sync::mpsc::channel();
        let writer = RecorderWriter::spawn(path.clone(), 48_000, 2, receiver, sender).unwrap();
        let resolved = ResolvedInput {
            host_id: "test".to_string(),
            device_name: "test device".to_string(),
            sample_rate: 48_000,
            buffer_size_frames: Some(128),
            stream_channel_count: 2,
            recorded_channel_count: 2,
            selected_channels: vec![1, 2],
            used_fallback: false,
        };
        let mut recorder = AudioRecorder {
            stream: None,
            writer: Some(writer),
            resolved,
            path,
            monitor_sender: monitor::new_monitor_sender_slot(),
            active: true,
        };

        assert!(recorder.is_active());
        let outcome = recorder.stop().unwrap();
        assert!(!recorder.is_active());
        assert_eq!(outcome.frames, 0);
    }
}
