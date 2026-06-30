use std::sync::{OnceLock, mpsc};

use crate::native_app::app::{NativeAppState, WaveformState};

type DeferredDropJob = Box<dyn FnOnce() + Send + 'static>;
static DEFERRED_DROP_SENDER: OnceLock<Option<mpsc::Sender<DeferredDropJob>>> = OnceLock::new();

pub(super) fn defer_large_drop<T: Send + 'static>(value: T) {
    let job: DeferredDropJob = Box::new(move || drop(value));
    let Some(sender) = deferred_drop_sender() else {
        job();
        return;
    };
    if let Err(err) = sender.send(job) {
        (err.0)();
    }
}

impl NativeAppState {
    pub(super) fn replace_waveform_deferred(&mut self, waveform: WaveformState) {
        self.log_sample_identity_waveform_checkpoint(
            "browser.sample_load.replace_waveform_deferred",
            "replace_waveform_deferred",
            Some(&waveform.path()),
            &waveform,
            Some("before_replace"),
        );
        let previous = std::mem::replace(&mut self.waveform.current, waveform);
        defer_large_drop(previous);
        self.log_sample_identity_checkpoint(
            "browser.sample_load.replace_waveform_deferred_done",
            "replace_waveform_deferred",
            Some(&self.waveform.current.path()),
            Some("after_replace"),
        );
    }
}

fn deferred_drop_sender() -> Option<&'static mpsc::Sender<DeferredDropJob>> {
    DEFERRED_DROP_SENDER
        .get_or_init(|| {
            let (sender, receiver) = mpsc::channel::<DeferredDropJob>();
            match std::thread::Builder::new()
                .name(String::from("wavecrate-deferred-drop"))
                .spawn(move || {
                    while let Ok(job) = receiver.recv() {
                        job();
                    }
                }) {
                Ok(_) => Some(sender),
                Err(err) => {
                    tracing::warn!("Failed to spawn deferred waveform drop worker: {err}");
                    None
                }
            }
        })
        .as_ref()
}

#[cfg(test)]
fn deferred_drop_sender_initialized_for_tests() -> bool {
    DEFERRED_DROP_SENDER.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant};

    #[test]
    fn deferred_drop_uses_reusable_worker() {
        let dropped = Arc::new(AtomicBool::new(false));
        struct Probe(Arc<AtomicBool>);
        impl Drop for Probe {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        defer_large_drop(Probe(Arc::clone(&dropped)));

        let deadline = Instant::now() + Duration::from_secs(2);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(dropped.load(Ordering::Acquire));
        assert!(deferred_drop_sender_initialized_for_tests());
    }
}
