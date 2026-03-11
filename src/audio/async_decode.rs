use std::marker::PhantomData;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ringbuf::{HeapRb, traits::*};

use crate::audio::Source;

const DEFAULT_BUFFER_SECONDS: f32 = 1.0;
const PRODUCER_BACKOFF: Duration = Duration::from_millis(1);
const PREFILL_DURATION: Duration = Duration::from_millis(5);
const PREFILL_TIMEOUT: Duration = Duration::from_millis(5);
const PREFILL_POLL: Duration = Duration::from_millis(1);

/// Streams samples from a source on a background thread into a lock-free ring buffer.
///
/// The decoder thread owns `S` and cooperatively checks `stop` between source reads and
/// ring-buffer backpressure waits. Consumer-facing methods join the worker once it has
/// quiesced so resources are reclaimed deterministically. Dropping `AsyncSource` always
/// requests stop, but it only blocks to join if the worker has already finished; otherwise
/// the worker is intentionally left detached so audio teardown never stalls on a blocked
/// decoder backend.
pub(crate) struct AsyncSource<S> {
    consumer: ringbuf::HeapCons<f32>,
    sample_rate: u32,
    channels: u16,
    total_duration: Option<Duration>,
    worker: Option<thread::JoinHandle<()>>,
    done: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<String>>>,
    _marker: PhantomData<S>,
}

impl<S> AsyncSource<S>
where
    S: Source + 'static,
{
    /// Spawn a decoder thread with a 1-second buffer sized to the source format.
    pub(crate) fn new(source: S) -> Self {
        Self::with_buffer_seconds(source, DEFAULT_BUFFER_SECONDS)
    }

    /// Spawn a decoder thread with a buffer sized to `buffer_seconds`.
    ///
    /// Non-finite or tiny buffer sizes are sanitized through [`buffer_samples`].
    pub(crate) fn with_buffer_seconds(source: S, buffer_seconds: f32) -> Self {
        let sample_rate = source.sample_rate();
        let channels = source.channels();
        let total_duration = source.total_duration();
        let buffer_samples = buffer_samples(sample_rate, channels, buffer_seconds);

        let rb = HeapRb::<f32>::new(buffer_samples.max(1));
        let (mut producer, consumer) = rb.split();

        let done = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let last_error = Arc::new(Mutex::new(None));
        let thread_done = Arc::clone(&done);
        let thread_stop = Arc::clone(&stop);
        let thread_error = Arc::clone(&last_error);

        let spawn_result = thread::Builder::new()
            .name("audio-decode".to_string())
            .spawn(move || {
                let decode_result = panic::catch_unwind(AssertUnwindSafe(|| {
                    let mut source = source;
                    loop {
                        if thread_stop.load(Ordering::Relaxed) {
                            break;
                        }
                        match source.next() {
                            Some(sample) => {
                                let mut sample = sample;
                                loop {
                                    if thread_stop.load(Ordering::Relaxed) {
                                        break;
                                    }
                                    match producer.try_push(sample) {
                                        Ok(()) => {
                                            break;
                                        }
                                        Err(returned) => {
                                            sample = returned;
                                            thread::sleep(PRODUCER_BACKOFF);
                                        }
                                    }
                                }
                            }
                            None => {
                                if let Some(err) = source.last_error()
                                    && let Ok(mut slot) = thread_error.lock()
                                {
                                    *slot = Some(err);
                                }
                                break;
                            }
                        }
                    }
                }));
                if let Err(payload) = decode_result
                    && let Ok(mut slot) = thread_error.lock()
                {
                    *slot = Some(format!(
                        "Async decode thread panicked: {}",
                        panic_payload_message(&payload)
                    ));
                }
                thread_done.store(true, Ordering::Relaxed);
            });

        let worker = match spawn_result {
            Ok(worker) => Some(worker),
            Err(err) => {
                if let Ok(mut slot) = last_error.lock() {
                    *slot = Some(format!("Async decode thread failed to start: {err}"));
                }
                done.store(true, Ordering::Relaxed);
                None
            }
        };

        Self {
            consumer,
            sample_rate,
            channels,
            total_duration,
            worker,
            done,
            stop,
            last_error,
            _marker: PhantomData,
        }
    }

    /// Join the worker once it is known to be finished.
    ///
    /// This is intentionally non-blocking for active workers so real-time audio teardown can
    /// request stop without waiting for arbitrary decoder backends to return.
    fn join_finished_worker(&mut self) {
        let can_join = self.done.load(Ordering::Acquire)
            || self
                .worker
                .as_ref()
                .is_some_and(thread::JoinHandle::is_finished);
        if !can_join {
            return;
        }

        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
            && let Ok(mut slot) = self.last_error.lock()
            && slot.is_none()
        {
            *slot = Some(String::from("Async decode thread panicked"));
        }
    }

    /// Prefill the ring buffer with a short slice of audio to avoid blocking playback.
    pub(crate) fn prefill(&mut self) -> usize {
        self.prefill_for_duration(PREFILL_DURATION, PREFILL_TIMEOUT)
    }

    /// Prefill the ring buffer for at least `duration`, waiting up to `timeout`.
    pub(crate) fn prefill_for_duration(&mut self, duration: Duration, timeout: Duration) -> usize {
        let target_samples = prefill_samples(self.sample_rate, self.channels, duration);
        self.prefill_samples(target_samples, timeout)
    }

    fn prefill_samples(&mut self, target_samples: usize, timeout: Duration) -> usize {
        if target_samples == 0 {
            return self.consumer.occupied_len();
        }
        let deadline = Instant::now() + timeout;
        loop {
            let available = self.consumer.occupied_len();
            if available >= target_samples {
                return available;
            }
            if self.done.load(Ordering::Relaxed) {
                self.join_finished_worker();
                return available;
            }
            if Instant::now() >= deadline {
                return available;
            }
            thread::sleep(PREFILL_POLL);
        }
    }
}

impl<S> Iterator for AsyncSource<S>
where
    S: Source + 'static,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.consumer.try_pop() {
            return Some(sample);
        }
        if self.done.load(Ordering::Relaxed) {
            self.join_finished_worker();
            return None;
        }
        Some(0.0)
    }
}

impl<S> Source for AsyncSource<S>
where
    S: Source + 'static,
{
    fn current_frame_len(&self) -> Option<usize> {
        if self.done.load(Ordering::Relaxed) {
            Some(self.consumer.occupied_len())
        } else {
            None
        }
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|slot| slot.clone())
    }
}

impl<S> Drop for AsyncSource<S> {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let can_join = self.done.load(Ordering::Acquire)
            || self
                .worker
                .as_ref()
                .is_some_and(thread::JoinHandle::is_finished);
        if !can_join {
            return;
        }

        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
            && let Ok(mut slot) = self.last_error.lock()
            && slot.is_none()
        {
            *slot = Some(String::from("Async decode thread panicked"));
        }
    }
}

/// Convert the requested buffering window into a sample count for the source format.
fn buffer_samples(sample_rate: u32, channels: u16, buffer_seconds: f32) -> usize {
    let channels = channels.max(1) as f32;
    let sample_rate = sample_rate.max(1) as f32;
    let buffer_seconds = if buffer_seconds.is_finite() {
        buffer_seconds.max(0.01)
    } else {
        DEFAULT_BUFFER_SECONDS
    };
    (sample_rate * channels * buffer_seconds).ceil() as usize
}

/// Convert a prefill duration into the number of interleaved samples to wait for.
fn prefill_samples(sample_rate: u32, channels: u16, duration: Duration) -> usize {
    let channels = channels.max(1) as f64;
    let sample_rate = sample_rate.max(1) as f64;
    let seconds = duration.as_secs_f64();
    (sample_rate * channels * seconds).ceil() as usize
}

/// Render a human-readable panic payload for the decoder worker.
fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        String::from("unknown panic payload")
    }
}

#[cfg(test)]
mod tests;
