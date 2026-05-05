use super::*;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

const INITIAL_COMMAND_GENERATION: u64 = 1;

fn initial_command_generation() -> Arc<AtomicU64> {
    Arc::new(AtomicU64::new(INITIAL_COMMAND_GENERATION))
}

#[test]
fn default_config_has_no_preferences() {
    let cfg = AudioOutputConfig::default();
    assert!(cfg.host.is_none());
    assert!(cfg.device.is_none());
    assert!(cfg.sample_rate.is_none());
    assert!(cfg.buffer_size.is_none());
}

#[test]
fn sample_rate_filter_returns_common_values() {
    let rates = sample_rates_in_range(40_000, 50_000);
    assert_eq!(rates, vec![44_100, 48_000]);
}

#[test]
fn callback_propagates_error() {
    use crate::audio::Source;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize};
    use std::sync::mpsc;
    use std::time::Duration;

    struct MockSource {
        error: Option<String>,
    }

    impl Iterator for MockSource {
        type Item = f32;
        fn next(&mut self) -> Option<Self::Item> {
            None // Finish immediately
        }
    }

    impl Source for MockSource {
        fn current_frame_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            2
        }
        fn sample_rate(&self) -> u32 {
            44100
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
        fn last_error(&self) -> Option<String> {
            self.error.clone()
        }
    }

    let (command_sender, command_receiver) = mpsc::sync_channel(8);
    let (error_sender, error_receiver) = mpsc::channel();
    let volume_bits = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let active_sources = Arc::new(AtomicUsize::new(0));
    let clear_pending = Arc::new(AtomicBool::new(false));
    let command_generation = initial_command_generation();
    let mut state = CallbackState::new(
        command_receiver,
        error_sender,
        volume_bits,
        active_sources,
        clear_pending,
        command_generation,
    );
    command_sender
        .send(StreamCommand::Append {
            generation: INITIAL_COMMAND_GENERATION,
            source: Box::new(MockSource {
                error: Some("failure".into()),
            }),
            volume: 1.0,
        })
        .unwrap();

    let mut data = vec![0.0; 10];
    process_audio_callback(&mut state, &mut data);

    let err = error_receiver.try_recv().ok();
    assert_eq!(err, Some("failure".into()));
}

#[test]
fn callback_clears_sources_with_command() {
    use crate::audio::Source;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    struct ConstantSource;

    impl Iterator for ConstantSource {
        type Item = f32;
        fn next(&mut self) -> Option<Self::Item> {
            Some(1.0)
        }
    }

    impl Source for ConstantSource {
        fn current_frame_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            1
        }
        fn sample_rate(&self) -> u32 {
            44100
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    let (command_sender, command_receiver) = mpsc::sync_channel(8);
    let (error_sender, _error_receiver) = mpsc::channel();
    let volume_bits = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let active_sources = Arc::new(AtomicUsize::new(0));
    let clear_pending = Arc::new(AtomicBool::new(false));
    let command_generation = initial_command_generation();
    let mut state = CallbackState::new(
        command_receiver,
        error_sender,
        volume_bits,
        active_sources.clone(),
        clear_pending,
        command_generation,
    );
    command_sender
        .send(StreamCommand::Append {
            generation: INITIAL_COMMAND_GENERATION,
            source: Box::new(ConstantSource),
            volume: 1.0,
        })
        .unwrap();
    command_sender
        .send(StreamCommand::Clear {
            generation: INITIAL_COMMAND_GENERATION + 1,
        })
        .unwrap();

    let mut data = vec![1.0; 4];
    process_audio_callback(&mut state, &mut data);

    assert!(data.iter().all(|sample| *sample == 0.0));
    assert_eq!(active_sources.load(Ordering::Relaxed), 0);
}

#[test]
fn callback_stays_non_blocking_under_command_contention() {
    use crate::audio::Source;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize};
    use std::sync::mpsc;
    use std::sync::{Barrier, Mutex};
    use std::thread;
    use std::time::Duration;

    struct ConstantSource;

    impl Iterator for ConstantSource {
        type Item = f32;
        fn next(&mut self) -> Option<Self::Item> {
            Some(0.25)
        }
    }

    impl Source for ConstantSource {
        fn current_frame_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            1
        }
        fn sample_rate(&self) -> u32 {
            44100
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    let (command_sender, command_receiver) = mpsc::sync_channel(512);
    let (error_sender, _error_receiver) = mpsc::channel();
    let volume_bits = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let active_sources = Arc::new(AtomicUsize::new(0));
    let clear_pending = Arc::new(AtomicBool::new(false));
    let command_generation = initial_command_generation();
    let ui_lock = Arc::new(Mutex::new(()));
    let barrier = Arc::new(Barrier::new(2));
    let (done_sender, done_receiver) = mpsc::channel();

    let sender_lock = ui_lock.clone();
    let sender_barrier = barrier.clone();
    let sender_thread = thread::spawn(move || {
        sender_barrier.wait();
        let _guard = sender_lock.lock().unwrap();
        for _ in 0..256 {
            let _ = command_sender.send(StreamCommand::Append {
                generation: INITIAL_COMMAND_GENERATION,
                source: Box::new(ConstantSource),
                volume: 1.0,
            });
        }
    });

    let callback_thread = thread::spawn(move || {
        let mut state = CallbackState::new(
            command_receiver,
            error_sender,
            volume_bits,
            active_sources,
            clear_pending,
            command_generation,
        );
        let mut data = vec![0.0; 64];
        for _ in 0..256 {
            process_audio_callback(&mut state, &mut data);
        }
        let _ = done_sender.send(());
    });

    let guard = ui_lock.lock().unwrap();
    barrier.wait();

    done_receiver
        .recv_timeout(Duration::from_millis(200))
        .expect("callback should stay non-blocking under contention");

    drop(guard);
    let _ = sender_thread.join();
    let _ = callback_thread.join();
}

#[test]
fn resolved_output_uses_fallback_stream_config() {
    let fallback_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: 48_000,
        buffer_size: cpal::BufferSize::Fixed(512),
    };

    let resolved = resolved_output_from_stream_config(
        "fallback_host".to_string(),
        "fallback_device".to_string(),
        &fallback_config,
        true,
    );

    assert_eq!(resolved.sample_rate, 48_000);
    assert_eq!(resolved.channel_count, 1);
    assert_eq!(resolved.buffer_size_frames, Some(512));
    assert!(resolved.used_fallback);
}

#[test]
fn callback_clears_sources_with_pending_flag() {
    use crate::audio::Source;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    struct ConstantSource;

    impl Iterator for ConstantSource {
        type Item = f32;
        fn next(&mut self) -> Option<Self::Item> {
            Some(0.25)
        }
    }

    impl Source for ConstantSource {
        fn current_frame_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> u16 {
            1
        }
        fn sample_rate(&self) -> u32 {
            44100
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    let (_command_sender, command_receiver) = mpsc::sync_channel(1);
    let (error_sender, _error_receiver) = mpsc::channel();
    let volume_bits = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let active_sources = Arc::new(AtomicUsize::new(0));
    let clear_pending = Arc::new(AtomicBool::new(true));
    let command_generation = initial_command_generation();
    let mut state = CallbackState::new(
        command_receiver,
        error_sender,
        volume_bits,
        active_sources.clone(),
        clear_pending,
        command_generation,
    );
    state.sources.push((Box::new(ConstantSource), 1.0));

    let mut data = vec![0.0; 16];
    process_audio_callback(&mut state, &mut data);

    assert_eq!(active_sources.load(Ordering::Relaxed), 0);
    assert!(data.iter().all(|sample| *sample == 0.0));
}

#[test]
fn callback_drops_stale_queue_entries_after_pending_clear() {
    use crate::audio::Source;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    struct ConstantSource {
        value: f32,
    }

    impl Iterator for ConstantSource {
        type Item = f32;
        fn next(&mut self) -> Option<Self::Item> {
            Some(self.value)
        }
    }

    impl Source for ConstantSource {
        fn current_frame_len(&self) -> Option<usize> {
            None
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

    let (command_sender, command_receiver) = mpsc::sync_channel(8);
    let (error_sender, _error_receiver) = mpsc::channel();
    let volume_bits = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let active_sources = Arc::new(AtomicUsize::new(0));
    let clear_pending = Arc::new(AtomicBool::new(false));
    let command_generation = initial_command_generation();
    let mut state = CallbackState::new(
        command_receiver,
        error_sender,
        volume_bits,
        active_sources.clone(),
        clear_pending.clone(),
        command_generation.clone(),
    );
    state
        .sources
        .push((Box::new(ConstantSource { value: 0.75 }), 1.0));
    command_sender
        .send(StreamCommand::Append {
            generation: INITIAL_COMMAND_GENERATION,
            source: Box::new(ConstantSource { value: 0.25 }),
            volume: 1.0,
        })
        .unwrap();

    command_generation.store(INITIAL_COMMAND_GENERATION + 1, Ordering::Release);
    clear_pending.store(true, Ordering::Release);

    let mut data = vec![0.0; 8];
    process_audio_callback(&mut state, &mut data);

    assert!(data.iter().all(|sample| *sample == 0.0));
    assert_eq!(active_sources.load(Ordering::Relaxed), 0);

    command_sender
        .send(StreamCommand::Append {
            generation: INITIAL_COMMAND_GENERATION + 1,
            source: Box::new(ConstantSource { value: 0.5 }),
            volume: 1.0,
        })
        .unwrap();
    process_audio_callback(&mut state, &mut data);

    assert!(
        data.iter()
            .all(|sample| (*sample - 0.5).abs() < f32::EPSILON)
    );
    assert_eq!(active_sources.load(Ordering::Relaxed), 1);
}
