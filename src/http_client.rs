//! Shared HTTP client configuration and bounded response helpers.

use std::io::{self, Read, Write};
use std::sync::OnceLock;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// Retry settings for network operations with exponential backoff.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RetryConfig {
    /// Maximum number of attempts, including the first try.
    pub max_attempts: usize,
    /// Base delay used for the exponential backoff.
    pub base_delay: Duration,
    /// Maximum delay allowed between attempts.
    pub max_delay: Duration,
}

/// Retry classification returned by operation-specific error classifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RetryDecision {
    /// Do not retry this error.
    Stop,
    /// Retry using the configured exponential backoff delay.
    Retry,
    /// Retry after an operation-provided delay, capped by `RetryConfig::max_delay`.
    RetryAfter(Duration),
}

/// Return a shared HTTP agent with consistent timeouts.
pub(crate) fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(CONNECT_TIMEOUT)
            .timeout_read(READ_TIMEOUT)
            .timeout_write(WRITE_TIMEOUT)
            .build()
    })
}

/// Retry an operation with bounded exponential backoff when the predicate allows it.
pub(crate) fn retry_with_backoff<T, E, F, R>(
    config: RetryConfig,
    action: F,
    mut should_retry: R,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    R: FnMut(&E) -> bool,
{
    retry_with_policy(config, action, |err| {
        if should_retry(err) {
            RetryDecision::Retry
        } else {
            RetryDecision::Stop
        }
    })
}

/// Retry an operation with bounded policy-controlled backoff.
pub(crate) fn retry_with_policy<T, E, F, R>(
    config: RetryConfig,
    action: F,
    classify_error: R,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    R: FnMut(&E) -> RetryDecision,
{
    retry_with_policy_using(config, action, classify_error, std::thread::sleep)
}

/// Retry an operation with an injectable sleeper for deterministic tests.
pub(crate) fn retry_with_policy_using<T, E, F, R, S>(
    config: RetryConfig,
    mut action: F,
    mut classify_error: R,
    mut sleep: S,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    R: FnMut(&E) -> RetryDecision,
    S: FnMut(Duration),
{
    let mut attempt = 0usize;
    loop {
        attempt += 1;
        match action() {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempt >= config.max_attempts {
                    return Err(err);
                }
                let decision = classify_error(&err);
                let Some(delay) = retry_delay(config, decision, attempt) else {
                    return Err(err);
                };
                if delay > Duration::from_secs(0) {
                    sleep(delay);
                }
            }
        }
    }
}

/// Read a response into memory, enforcing a maximum byte size.
pub(crate) fn read_response_bytes(
    response: ureq::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, io::Error> {
    check_content_length(&response, max_bytes)?;
    let reader = response.into_reader();
    read_response_reader(reader, max_bytes)
}

/// Stream a response to the provided writer, enforcing a maximum byte size.
pub(crate) fn copy_response_to_writer(
    response: ureq::Response,
    writer: &mut impl Write,
    max_bytes: usize,
) -> Result<(), io::Error> {
    check_content_length(&response, max_bytes)?;
    let reader = response.into_reader();
    copy_response_reader(reader, writer, max_bytes)
}

/// Read a UTF-8 response body into a string, enforcing a maximum byte size.
pub(crate) fn read_response_text(
    response: ureq::Response,
    max_bytes: usize,
) -> Result<String, io::Error> {
    let bytes = read_response_bytes(response, max_bytes)?;
    String::from_utf8(bytes).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Response body is not valid UTF-8: {err}"),
        )
    })
}

fn read_response_reader(reader: impl Read, max_bytes: usize) -> Result<Vec<u8>, io::Error> {
    let mut limited = reader.take(max_bytes as u64 + 1);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Response exceeded {max_bytes} bytes"),
        ));
    }
    Ok(bytes)
}

fn copy_response_reader(
    reader: impl Read,
    writer: &mut impl Write,
    max_bytes: usize,
) -> Result<(), io::Error> {
    let mut limited = reader.take(max_bytes as u64 + 1);
    let mut total = 0usize;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = limited.read(&mut buf)?;
        if read == 0 {
            break;
        }
        total += read;
        if total > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Response exceeded {max_bytes} bytes"),
            ));
        }
        writer.write_all(&buf[..read])?;
    }
    Ok(())
}

fn check_content_length(response: &ureq::Response, max_bytes: usize) -> Result<(), io::Error> {
    let Some(length) = response.header("Content-Length") else {
        return Ok(());
    };
    let Ok(length) = length.parse::<u64>() else {
        return Ok(());
    };
    check_content_length_value(length, max_bytes)
}

fn check_content_length_value(length: u64, max_bytes: usize) -> Result<(), io::Error> {
    if length > max_bytes as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Response too large: {length} bytes"),
        ));
    }
    Ok(())
}

/// Compute exponential backoff delay for the given attempt.
pub(crate) fn backoff_delay(base: Duration, max: Duration, attempt: usize) -> Duration {
    let exponent = u32::try_from(attempt.saturating_sub(1)).unwrap_or(u32::MAX);
    let factor = 1u32.checked_shl(exponent).unwrap_or(u32::MAX);
    let delay = base.checked_mul(factor).unwrap_or(max);
    if delay > max { max } else { delay }
}

fn retry_delay(config: RetryConfig, decision: RetryDecision, attempt: usize) -> Option<Duration> {
    match decision {
        RetryDecision::Stop => None,
        RetryDecision::Retry => Some(backoff_delay(config.base_delay, config.max_delay, attempt)),
        RetryDecision::RetryAfter(delay) => Some(delay.min(config.max_delay)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct ChunkedReader {
        inner: Cursor<Vec<u8>>,
        chunk_size: usize,
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let max = buf.len().min(self.chunk_size);
            self.inner.read(&mut buf[..max])
        }
    }

    #[test]
    fn read_response_bytes_rejects_content_length_over_max() {
        let err = check_content_length_value(100, 10).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_response_bytes_rejects_body_over_max() {
        let body = "a".repeat(32);
        let err = read_response_reader(body.as_bytes(), 16).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_response_bytes_accepts_under_limit() {
        let body = "hello";
        let bytes = read_response_reader(body.as_bytes(), 16).unwrap();
        assert_eq!(bytes, body.as_bytes());
    }

    #[test]
    fn copy_response_reader_honors_max_bytes() {
        let body = b"abcdef";
        let mut output = Vec::new();
        let reader = ChunkedReader {
            inner: Cursor::new(body.to_vec()),
            chunk_size: 2,
        };
        let err = copy_response_reader(reader, &mut output, 4).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(output, b"abcd".to_vec());
    }

    #[test]
    fn retry_with_backoff_stops_after_success() {
        let mut attempts = 0usize;
        let config = RetryConfig {
            max_attempts: 4,
            base_delay: Duration::from_millis(0),
            max_delay: Duration::from_millis(0),
        };
        let result: Result<u32, &'static str> = retry_with_backoff(
            config,
            || {
                attempts += 1;
                if attempts < 3 { Err("fail") } else { Ok(7) }
            },
            |_| true,
        );
        assert_eq!(result, Ok(7));
        assert_eq!(attempts, 3);
    }

    #[test]
    fn retry_with_backoff_honors_should_retry() {
        let mut attempts = 0usize;
        let config = RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(0),
            max_delay: Duration::from_millis(0),
        };
        let result: Result<u32, &'static str> = retry_with_backoff(
            config,
            || {
                attempts += 1;
                Err("fail")
            },
            |_| false,
        );
        assert_eq!(result, Err("fail"));
        assert_eq!(attempts, 1);
    }

    #[test]
    fn retry_with_policy_stops_after_limit_without_extra_sleep() {
        let mut attempts = 0usize;
        let mut delays = Vec::new();
        let config = RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
        };

        let result: Result<(), &'static str> = retry_with_policy_using(
            config,
            || {
                attempts += 1;
                Err("fail")
            },
            |_| RetryDecision::Retry,
            |delay| delays.push(delay),
        );

        assert_eq!(result, Err("fail"));
        assert_eq!(attempts, 3);
        assert_eq!(
            delays,
            vec![Duration::from_millis(10), Duration::from_millis(20)]
        );
    }

    #[test]
    fn retry_with_policy_uses_operation_retry_after_delay() {
        let mut attempts = 0usize;
        let mut delays = Vec::new();
        let config = RetryConfig {
            max_attempts: 2,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
        };

        let result: Result<u32, &'static str> = retry_with_policy_using(
            config,
            || {
                attempts += 1;
                if attempts == 1 { Err("rate") } else { Ok(9) }
            },
            |_| RetryDecision::RetryAfter(Duration::from_millis(250)),
            |delay| delays.push(delay),
        );

        assert_eq!(result, Ok(9));
        assert_eq!(delays, vec![Duration::from_millis(100)]);
    }
}
