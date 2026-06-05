use super::*;

#[derive(Clone)]
struct TestSource {
    samples: Vec<f32>,
    pos: usize,
    sample_rate: u32,
    channels: u16,
    delay: Duration,
    error: Option<String>,
    panic_at: Option<usize>,
    start_barrier: Option<Arc<std::sync::Barrier>>,
    start_barrier_waited: bool,
    dropped: Option<Arc<AtomicBool>>,
}

impl Drop for TestSource {
    fn drop(&mut self) {
        if let Some(flag) = self.dropped.as_ref() {
            flag.store(true, Ordering::Release);
        }
    }
}

impl Iterator for TestSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.start_barrier_waited
            && let Some(barrier) = self.start_barrier.as_ref()
        {
            self.start_barrier_waited = true;
            barrier.wait();
        }
        if self.delay > Duration::ZERO {
            thread::sleep(self.delay);
        }
        if self.panic_at == Some(self.pos) {
            panic!("test decode panic");
        }
        if self.pos < self.samples.len() {
            let sample = self.samples[self.pos];
            self.pos += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl Source for TestSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len().saturating_sub(self.pos))
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }

    fn last_error(&self) -> Option<String> {
        self.error.clone()
    }
}

fn wait_for_flag(flag: &AtomicBool, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if flag.load(Ordering::Acquire) {
            return true;
        }
        thread::sleep(Duration::from_millis(1));
    }
    flag.load(Ordering::Acquire)
}

fn wait_for_non_zero_sample(
    async_source: &mut AsyncSource<TestSource>,
    timeout: Duration,
) -> Option<f32> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(sample) = async_source.next()
            && sample != 0.0
        {
            return Some(sample);
        }
        thread::sleep(Duration::from_millis(5));
    }
    None
}

#[test]
fn async_source_emits_samples_after_decode() {
    let source = TestSource {
        samples: vec![0.1, 0.2, 0.3],
        pos: 0,
        sample_rate: 10,
        channels: 1,
        delay: Duration::ZERO,
        error: None,
        panic_at: None,
        start_barrier: None,
        start_barrier_waited: false,
        dropped: None,
    };
    let mut async_source = AsyncSource::with_buffer_seconds(source, 1.0);
    let available =
        async_source.prefill_for_duration(Duration::from_millis(300), Duration::from_millis(100));
    assert!(
        available >= 3,
        "expected three prefetched samples, got {available}"
    );
    let mut collected = Vec::with_capacity(3);
    for _ in 0..3 {
        collected.push(async_source.next().expect("prefilled sample"));
    }
    assert_eq!(collected, vec![0.1, 0.2, 0.3]);
}

#[test]
fn async_source_returns_silence_on_underrun() {
    let source = TestSource {
        samples: vec![0.5],
        pos: 0,
        sample_rate: 10,
        channels: 1,
        delay: Duration::from_millis(30),
        error: None,
        panic_at: None,
        start_barrier: Some(Arc::new(std::sync::Barrier::new(2))),
        start_barrier_waited: false,
        dropped: None,
    };
    let start_barrier = source.start_barrier.clone().expect("barrier present");
    let mut async_source = AsyncSource::with_buffer_seconds(source, 0.1);
    let first = async_source.next().unwrap();
    assert_eq!(first, 0.0);
    start_barrier.wait();
    let second =
        wait_for_non_zero_sample(&mut async_source, Duration::from_millis(250)).unwrap_or(0.0);
    assert_eq!(second, 0.5);
}

#[test]
fn async_source_prefill_waits_for_samples() {
    let source = TestSource {
        samples: vec![0.4],
        pos: 0,
        sample_rate: 10,
        channels: 1,
        delay: Duration::from_millis(5),
        error: None,
        panic_at: None,
        start_barrier: None,
        start_barrier_waited: false,
        dropped: None,
    };
    let mut async_source = AsyncSource::with_buffer_seconds(source, 0.1);
    let available =
        async_source.prefill_for_duration(Duration::from_millis(1), Duration::from_millis(100));
    assert!(available >= 1);
    assert_eq!(async_source.next(), Some(0.4));
}

#[test]
fn async_source_waits_for_consumer_when_buffer_full() {
    let source = TestSource {
        samples: vec![0.1, 0.2],
        pos: 0,
        sample_rate: 1,
        channels: 1,
        delay: Duration::ZERO,
        error: None,
        panic_at: None,
        start_barrier: None,
        start_barrier_waited: false,
        dropped: None,
    };
    let mut async_source = AsyncSource::with_buffer_seconds(source, 0.1);
    let first =
        wait_for_non_zero_sample(&mut async_source, Duration::from_millis(250)).unwrap_or(0.0);
    assert_eq!(first, 0.1);
    let second = wait_for_non_zero_sample(&mut async_source, Duration::from_millis(250));
    assert_eq!(second, Some(0.2));
}

#[test]
fn async_source_propagates_errors() {
    let source = TestSource {
        samples: vec![0.7],
        pos: 0,
        sample_rate: 10,
        channels: 1,
        delay: Duration::ZERO,
        error: Some("decode failed".to_string()),
        panic_at: None,
        start_barrier: None,
        start_barrier_waited: false,
        dropped: None,
    };
    let mut async_source = AsyncSource::with_buffer_seconds(source, 1.0);
    thread::sleep(Duration::from_millis(20));
    while async_source.next().is_some() {}
    assert_eq!(async_source.last_error(), Some("decode failed".to_string()));
}

#[test]
fn async_source_drop_requests_worker_stop_and_releases_source() {
    let dropped = Arc::new(AtomicBool::new(false));
    let source = TestSource {
        samples: vec![0.1, 0.2, 0.3, 0.4],
        pos: 0,
        sample_rate: 1,
        channels: 1,
        delay: Duration::ZERO,
        error: None,
        panic_at: None,
        start_barrier: None,
        start_barrier_waited: false,
        dropped: Some(Arc::clone(&dropped)),
    };

    let async_source = AsyncSource::with_buffer_seconds(source, 0.1);
    thread::sleep(Duration::from_millis(10));
    drop(async_source);

    assert!(
        wait_for_flag(&dropped, Duration::from_millis(100)),
        "decoder worker should observe stop and drop the source promptly"
    );
}

#[test]
fn async_source_reports_worker_panics_and_ends_stream() {
    let source = TestSource {
        samples: vec![0.9],
        pos: 0,
        sample_rate: 10,
        channels: 1,
        delay: Duration::ZERO,
        error: None,
        panic_at: Some(1),
        start_barrier: None,
        start_barrier_waited: false,
        dropped: None,
    };

    let mut async_source = AsyncSource::with_buffer_seconds(source, 1.0);
    assert_eq!(
        async_source.prefill_for_duration(Duration::from_millis(5), Duration::from_millis(50)),
        1
    );
    assert_eq!(async_source.next(), Some(0.9));
    for _ in 0..50 {
        if async_source.next().is_none() {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(async_source.next(), None);
    let error = async_source
        .last_error()
        .expect("worker panic should be reported");
    assert!(error.starts_with("Async decode thread panicked:"));
}
