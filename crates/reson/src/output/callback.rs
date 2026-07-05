use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

/// Commands sent to the audio callback for non-blocking control.
pub(crate) enum StreamCommand {
    Append {
        generation: u64,
        source: Box<dyn crate::Source + Send>,
        volume: f32,
    },
    Clear {
        generation: u64,
    },
}

/// Callback-owned mixing state that avoids blocking the audio thread.
pub(super) struct CallbackState {
    pub(super) sources: Vec<(Box<dyn crate::Source + Send>, f32)>,
    command_receiver: Receiver<StreamCommand>,
    error_sender: Sender<String>,
    volume_bits: Arc<AtomicU32>,
    active_sources: Arc<AtomicUsize>,
    clear_pending: Arc<AtomicBool>,
    command_generation: Arc<AtomicU64>,
    current_generation: u64,
}

impl CallbackState {
    pub(super) fn new(
        command_receiver: Receiver<StreamCommand>,
        error_sender: Sender<String>,
        volume_bits: Arc<AtomicU32>,
        active_sources: Arc<AtomicUsize>,
        clear_pending: Arc<AtomicBool>,
        command_generation: Arc<AtomicU64>,
    ) -> Self {
        let current_generation = command_generation.load(Ordering::Acquire);
        Self {
            sources: Vec::new(),
            command_receiver,
            error_sender,
            volume_bits,
            active_sources,
            clear_pending,
            command_generation,
            current_generation,
        }
    }

    pub(super) fn error_sender(&self) -> Sender<String> {
        self.error_sender.clone()
    }

    fn apply_commands(&mut self) {
        const MAX_COMMANDS_PER_CALLBACK: usize = 64;
        if self.clear_pending.swap(false, Ordering::AcqRel) {
            let generation = self.command_generation.load(Ordering::Acquire);
            self.apply_clear_generation(generation);
        }
        for _ in 0..MAX_COMMANDS_PER_CALLBACK {
            match self.command_receiver.try_recv() {
                Ok(StreamCommand::Append {
                    generation,
                    source,
                    volume,
                }) => {
                    if generation < self.current_generation {
                        continue;
                    }
                    if generation > self.current_generation {
                        self.apply_clear_generation(generation);
                    }
                    self.sources.push((source, sanitize_gain(volume)));
                }
                Ok(StreamCommand::Clear { generation }) => {
                    if generation >= self.current_generation {
                        self.apply_clear_generation(generation);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    fn apply_clear_generation(&mut self, generation: u64) {
        self.sources.clear();
        self.current_generation = generation;
    }
}

pub(super) fn process_audio_callback(state: &mut CallbackState, data: &mut [f32]) {
    state.apply_commands();
    let volume = load_volume(&state.volume_bits);

    for sample in data.iter_mut() {
        *sample = 0.0;
    }

    let mut last_error = None;
    state.sources.retain_mut(|(source, source_volume)| {
        let mut finished = false;
        let combined_volume = volume * *source_volume;
        for sample_out in data.iter_mut() {
            if let Some(sample_in) = source.next() {
                *sample_out += sample_in * combined_volume;
            } else {
                finished = true;
                break;
            }
        }
        if finished && let Some(err) = source.last_error() {
            last_error = Some(err);
        }
        !finished
    });

    state
        .active_sources
        .store(state.sources.len(), Ordering::Relaxed);

    if let Some(err) = last_error
        && state.error_sender.send(err).is_err()
    {
        // Receiver dropped; nothing left to report.
    }
}

pub(super) fn sanitize_gain(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn load_volume(bits: &AtomicU32) -> f32 {
    f32::from_bits(bits.load(Ordering::Relaxed))
}

pub(super) fn store_volume(bits: &AtomicU32, volume: f32) {
    bits.store(volume.to_bits(), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize};
    use std::sync::mpsc;
    use std::time::Duration;

    struct ConstantSource {
        value: f32,
        remaining: usize,
    }

    impl Iterator for ConstantSource {
        type Item = f32;

        fn next(&mut self) -> Option<Self::Item> {
            if self.remaining == 0 {
                return None;
            }
            self.remaining -= 1;
            Some(self.value)
        }
    }

    impl crate::Source for ConstantSource {
        fn current_frame_len(&self) -> Option<usize> {
            Some(self.remaining)
        }

        fn channels(&self) -> u16 {
            1
        }

        fn sample_rate(&self) -> u32 {
            44_100
        }

        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn newer_append_generation_replaces_older_queued_sources() {
        let (command_sender, command_receiver) = mpsc::channel();
        let (error_sender, _error_receiver) = mpsc::channel();
        let active_sources = Arc::new(AtomicUsize::new(0));
        let command_generation = Arc::new(AtomicU64::new(1));
        let mut state = CallbackState::new(
            command_receiver,
            error_sender,
            Arc::new(AtomicU32::new(1.0f32.to_bits())),
            active_sources.clone(),
            Arc::new(AtomicBool::new(false)),
            command_generation,
        );

        command_sender
            .send(StreamCommand::Append {
                generation: 1,
                source: Box::new(ConstantSource {
                    value: 0.25,
                    remaining: 32,
                }),
                volume: 1.0,
            })
            .unwrap();
        command_sender
            .send(StreamCommand::Append {
                generation: 2,
                source: Box::new(ConstantSource {
                    value: 0.75,
                    remaining: 32,
                }),
                volume: 1.0,
            })
            .unwrap();

        let mut data = [0.0; 8];
        process_audio_callback(&mut state, &mut data);

        assert_eq!(state.current_generation, 2);
        assert_eq!(state.sources.len(), 1);
        assert_eq!(active_sources.load(Ordering::Relaxed), 1);
        assert!(
            data.iter()
                .all(|sample| (*sample - 0.75).abs() < f32::EPSILON)
        );
    }
}
