//! Cache-key and retained-fingerprint types used by the native Vello runtime.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in super::super) struct ImageUploadBlobCacheKey {
    pub(in super::super) pixels_ptr: usize,
    pub(in super::super) width: u32,
    pub(in super::super) height: u32,
}

pub(in super::super) struct SharedPixelBytes(pub(in super::super) Arc<[u8]>);

impl AsRef<[u8]> for SharedPixelBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct HoverOverlayCacheFingerprint {
    pub(in super::super) layout_width_bits: u32,
    pub(in super::super) layout_height_bits: u32,
    pub(in super::super) layout_scale_bits: u32,
    pub(in super::super) shell: HoverOverlayFingerprint,
    pub(in super::super) model_signature: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct FocusOverlayCacheFingerprint {
    pub(in super::super) layout_width_bits: u32,
    pub(in super::super) layout_height_bits: u32,
    pub(in super::super) layout_scale_bits: u32,
    pub(in super::super) shell: FocusOverlayFingerprint,
    pub(in super::super) model_signature: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct ModalOverlayCacheFingerprint {
    pub(in super::super) layout_width_bits: u32,
    pub(in super::super) layout_height_bits: u32,
    pub(in super::super) layout_scale_bits: u32,
    pub(in super::super) shell: ModalOverlayFingerprint,
    pub(in super::super) model_signature: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct WaveformMotionOverlayCacheFingerprint {
    pub(in super::super) layout_width_bits: u32,
    pub(in super::super) layout_height_bits: u32,
    pub(in super::super) layout_scale_bits: u32,
    pub(in super::super) shell: WaveformMotionOverlayFingerprint,
    pub(in super::super) motion_signature: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct ChromeMotionOverlayCacheFingerprint {
    pub(in super::super) layout_width_bits: u32,
    pub(in super::super) layout_height_bits: u32,
    pub(in super::super) layout_scale_bits: u32,
    pub(in super::super) shell: ChromeMotionOverlayFingerprint,
    pub(in super::super) motion_signature: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct StaticSegmentCacheFingerprint {
    pub(in super::super) segment: StaticFrameSegment,
    pub(in super::super) layout_width_bits: u32,
    pub(in super::super) layout_height_bits: u32,
    pub(in super::super) layout_scale_bits: u32,
    pub(in super::super) style_signature: u64,
    pub(in super::super) segment_revision: u64,
}

pub(in super::super) fn touch_image_upload_blob_cache_key(
    cache_order: &mut VecDeque<ImageUploadBlobCacheKey>,
    key: ImageUploadBlobCacheKey,
) {
    if let Some(position) = cache_order.iter().position(|existing| *existing == key) {
        let _ = cache_order.remove(position);
    }
    cache_order.push_back(key);
}
