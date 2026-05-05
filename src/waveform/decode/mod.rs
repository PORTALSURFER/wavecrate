mod cache;
mod cache_token;
mod normalize;
mod peaks;
mod resample;
mod symphonia_reader;
mod wav_reader;

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use crate::waveform::{DecodedWaveform, WaveformDecodeError, WaveformRenderer};

const DEFAULT_DECODE_CACHE_LIMIT: usize = 8;

impl WaveformRenderer {
    /// Decode wav bytes into samples and duration without rendering.
    pub fn decode_from_bytes(&self, bytes: &[u8]) -> Result<DecodedWaveform, WaveformDecodeError> {
        let key = cache::hash_bytes(bytes);
        let lock_start = Instant::now();
        match self.decode_cache.lock() {
            Ok(mut cache_guard) => {
                cache::record_decode_cache_lock_wait(lock_start.elapsed());
                if let Some(cached) = cache_guard.get(&key) {
                    return Ok((*cached).clone());
                }
            }
            Err(_) => {
                cache::record_decode_cache_lock_wait(lock_start.elapsed());
                cache::record_decode_cache_lock_poison();
            }
        }

        let decoded = self.load_decoded(bytes)?;
        let lock_start = Instant::now();
        match self.decode_cache.lock() {
            Ok(mut cache_guard) => {
                cache::record_decode_cache_lock_wait(lock_start.elapsed());
                cache_guard.insert(key, Arc::new(decoded.clone()));
            }
            Err(_) => {
                cache::record_decode_cache_lock_wait(lock_start.elapsed());
                cache::record_decode_cache_lock_poison();
            }
        }
        Ok(decoded)
    }
}

/// Advance and return the next cache token used for waveform cache invalidation.
pub(crate) fn next_cache_token() -> u64 {
    cache_token::next_cache_token()
}

/// Construct the default decode cache with the built-in LRU capacity.
pub(super) fn default_decode_cache() -> Mutex<cache::DecodeCache> {
    Mutex::new(cache::DecodeCache::new(DEFAULT_DECODE_CACHE_LIMIT))
}

/// Re-export of the decode cache type for neighboring modules.
pub(crate) use cache::DecodeCache;
