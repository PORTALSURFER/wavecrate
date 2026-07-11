//! Background WAV-writer worker for recording sessions.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use ringbuf::{HeapRb, traits::*};

use crate::input::AudioInputError;

use super::health::RecordingHealthState;

const WRITER_BUFFER_SECONDS: usize = 2;
const WRITER_DRAIN_SAMPLES: usize = 4_096;
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(1);

/// Writer-thread result summary returned when recording stops cleanly.
#[derive(Clone, Copy)]
pub(super) struct RecordingStats {
    pub(super) frames: u64,
}

/// Lock-free producer owned exclusively by the capture callback.
pub(super) struct RecorderCapture {
    producer: ringbuf::HeapProd<f32>,
    health: Arc<RecordingHealthState>,
}

impl RecorderCapture {
    pub(super) fn submit(&mut self, samples: &[f32]) {
        if self.producer.vacant_len() < samples.len() {
            self.health
                .writer_dropped_samples
                .fetch_add(samples.len() as u64, Ordering::Relaxed);
            self.health
                .writer_overrun_events
                .fetch_add(1, Ordering::Relaxed);
            return;
        }
        let pushed = self.producer.push_slice(samples);
        debug_assert_eq!(pushed, samples.len());
    }
}

/// Joinable WAV writer worker bound to one active recording session.
pub(super) struct RecorderWriter {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<Result<RecordingStats, AudioInputError>>>,
}

impl RecorderWriter {
    pub(super) fn spawn(
        path: PathBuf,
        sample_rate: u32,
        channels: u16,
        health: Arc<RecordingHealthState>,
    ) -> Result<(Self, RecorderCapture), AudioInputError> {
        let writer = WavSampleWriter::new(&path, sample_rate, channels)?;
        let capacity = (sample_rate as usize)
            .saturating_mul(channels.max(1) as usize)
            .saturating_mul(WRITER_BUFFER_SECONDS)
            .max(1);
        let ring = HeapRb::<f32>::new(capacity);
        let (producer, consumer) = ring.split();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_health = Arc::clone(&health);
        let join = thread::spawn(move || {
            let result = writer_loop(writer, consumer, &worker_stop);
            record_writer_result(&result, &worker_health);
            result
        });
        Ok((
            Self {
                stop,
                join: Some(join),
            },
            RecorderCapture { producer, health },
        ))
    }

    /// Ask the writer to drain its bounded ring and finalize the WAV file.
    pub(super) fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
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

fn record_writer_result(
    result: &Result<RecordingStats, AudioInputError>,
    health: &RecordingHealthState,
) {
    if result.is_err() {
        health.writer_failed.store(true, Ordering::Release);
    }
}

fn writer_loop(
    mut writer: WavSampleWriter,
    mut consumer: ringbuf::HeapCons<f32>,
    stop: &AtomicBool,
) -> Result<RecordingStats, AudioInputError> {
    let mut samples = vec![0.0; WRITER_DRAIN_SAMPLES];
    loop {
        let popped = consumer.pop_slice(&mut samples);
        if popped > 0 {
            writer.write_samples(&samples[..popped])?;
            continue;
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        thread::sleep(IDLE_POLL_INTERVAL);
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

    #[test]
    fn recorder_stop_drains_in_flight_callback_samples() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("recording.wav");
        let health = Arc::new(RecordingHealthState::default());
        let (mut writer, mut capture) =
            RecorderWriter::spawn(path.clone(), 48_000, 2, health).unwrap();

        capture.submit(&[0.0, 0.25]);
        capture.submit(&[-0.25, 0.5]);
        writer.stop();

        let stats = writer.join().unwrap();
        assert_eq!(stats.frames, 2);

        let reader = hound::WavReader::open(&path).unwrap();
        let samples = reader
            .into_samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(samples, vec![0.0, 0.25, -0.25, 0.5]);
    }

    #[test]
    fn stalled_writer_consumer_has_bounded_nonblocking_submission() {
        let ring = HeapRb::<f32>::new(4);
        let (producer, _consumer) = ring.split();
        let health = Arc::new(RecordingHealthState::default());
        let mut capture = RecorderCapture {
            producer,
            health: Arc::clone(&health),
        };

        capture.submit(&[0.0, 0.1, 0.2, 0.3, 0.4, 0.5]);

        let snapshot = health.snapshot();
        assert_eq!(snapshot.writer_dropped_samples, 6);
        assert_eq!(snapshot.writer_overrun_events, 1);
    }

    #[test]
    fn writer_failure_is_visible_in_health_snapshot() {
        let health = RecordingHealthState::default();
        let result = Err(AudioInputError::RecordingFailed {
            detail: "synthetic writer failure".into(),
        });

        record_writer_result(&result, &health);

        assert!(health.snapshot().writer_failed);
    }
}
