//! Background WAV-writer worker for recording sessions.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::audio::input::AudioInputError;

/// Writer-thread result summary returned when recording stops cleanly.
#[derive(Clone, Copy)]
pub(super) struct RecordingStats {
    pub(super) frames: u64,
}

/// Commands sent from the capture callback into the WAV writer worker.
pub(super) enum RecorderCommand {
    Samples(Vec<f32>),
    Stop,
}

/// Joinable WAV writer worker bound to one active recording session.
pub(super) struct RecorderWriter {
    sender: Sender<RecorderCommand>,
    join: Option<JoinHandle<Result<RecordingStats, AudioInputError>>>,
}

impl RecorderWriter {
    pub(super) fn spawn(
        path: PathBuf,
        sample_rate: u32,
        channels: u16,
        receiver: Receiver<RecorderCommand>,
        sender: Sender<RecorderCommand>,
    ) -> Result<Self, AudioInputError> {
        let writer = WavSampleWriter::new(&path, sample_rate, channels)?;
        let join = thread::spawn(move || writer_loop(writer, receiver));
        Ok(Self {
            sender,
            join: Some(join),
        })
    }

    pub(super) fn stop(&self) -> Result<(), AudioInputError> {
        self.sender
            .send(RecorderCommand::Stop)
            .map_err(|err| AudioInputError::RecordingFailed {
                detail: format!("Failed to stop recorder: {err}"),
            })
    }

    pub(super) fn join(&mut self) -> Result<RecordingStats, AudioInputError> {
        let handle = self
            .join
            .take()
            .ok_or_else(|| AudioInputError::RecordingFailed {
                detail: "Recorder writer already joined".into(),
            })?;
        handle
            .join()
            .map_err(|_| AudioInputError::RecordingFailed {
                detail: "Recorder writer thread panicked".into(),
            })?
    }
}

fn writer_loop(
    mut writer: WavSampleWriter,
    receiver: Receiver<RecorderCommand>,
) -> Result<RecordingStats, AudioInputError> {
    while let Ok(command) = receiver.recv() {
        match command {
            RecorderCommand::Samples(samples) => writer.write_samples(&samples)?,
            RecorderCommand::Stop => break,
        }
    }
    writer.finalize()
}

struct WavSampleWriter {
    writer: hound::WavWriter<BufWriter<File>>,
    channels: u16,
    written_samples: u64,
}

impl WavSampleWriter {
    fn new(path: &Path, sample_rate: u32, channels: u16) -> Result<Self, AudioInputError> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let file = File::create(path).map_err(|err| AudioInputError::RecordingFailed {
            detail: format!("Failed to create wav file: {err}"),
        })?;
        let writer = hound::WavWriter::new(BufWriter::new(file), spec).map_err(|err| {
            AudioInputError::RecordingFailed {
                detail: format!("Failed to create wav writer: {err}"),
            }
        })?;
        Ok(Self {
            writer,
            channels,
            written_samples: 0,
        })
    }

    fn write_samples(&mut self, samples: &[f32]) -> Result<(), AudioInputError> {
        for &sample in samples {
            self.writer
                .write_sample(sample)
                .map_err(|err| AudioInputError::RecordingFailed {
                    detail: format!("Failed to write wav sample: {err}"),
                })?;
            self.written_samples += 1;
        }
        Ok(())
    }

    fn finalize(self) -> Result<RecordingStats, AudioInputError> {
        self.writer
            .finalize()
            .map_err(|err| AudioInputError::RecordingFailed {
                detail: format!("Failed to finalize wav writer: {err}"),
            })?;
        let channels = self.channels.max(1) as u64;
        Ok(RecordingStats {
            frames: self.written_samples / channels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn wav_writer_outputs_float_wav() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("recording.wav");
        let mut writer = WavSampleWriter::new(&path, 48_000, 2).unwrap();
        writer.write_samples(&[0.0, 0.5, -0.5, 1.0]).unwrap();
        let stats = writer.finalize().unwrap();
        assert_eq!(stats.frames, 2);

        let mut reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.sample_format, hound::SampleFormat::Float);
        assert_eq!(reader.samples::<f32>().count(), 4);
    }
}
