use super::*;
use std::io;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    fn captured(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl<'a> MakeWriter<'a> for SharedBuffer {
    type Writer = SharedBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedBufferWriter(self.0.clone())
    }
}

struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for SharedBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn remove(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        // Tests in this module need the default incident-log policy, so keep
        // the opt-in hot-path override absent for the captured scope.
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        unsafe {
            match self.previous.as_ref() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

fn capture_debug_logs<F>(run: F) -> String
where
    F: FnOnce(),
{
    let buffer = SharedBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(buffer.clone())
        .finish();
    crate::logging::set_debug_logging_enabled_for_tests(true);
    tracing::subscriber::with_default(subscriber, run);
    crate::logging::set_debug_logging_enabled_for_tests(false);
    buffer.captured()
}

#[test]
fn default_action_debug_log_suppresses_browser_view_start_scroll_bursts() {
    let _hotpath_guard = EnvVarGuard::remove(crate::hotpath_telemetry::HOTPATH_TELEMETRY_ENV);
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    let captured = capture_debug_logs(|| {
        for visible_row in 0..32 {
            controller.apply_ui_action(NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::SetBrowserViewStart { visible_row },
            ));
        }
        controller.apply_ui_action(NativeUiAction::Shell(
            crate::app_core::actions::NativeShellAction::FocusBrowserPanel,
        ));
    });

    assert!(
        !captured.contains("action=\"set_browser_view_start\""),
        "projection-only scroll updates should not flood default debug logs: {captured}"
    );
    assert!(
        captured.contains("action=\"focus_browser_panel\""),
        "meaningful browser focus actions should remain in default debug logs: {captured}"
    );
}
