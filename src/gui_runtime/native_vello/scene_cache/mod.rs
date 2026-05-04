//! Focused retained-scene cache contracts for the native Vello runtime.

use super::*;

mod diff_plan;
mod keys;
mod scene_entries;
mod signatures;

pub(super) use diff_plan::StaticSegmentStateGraph;
pub(super) use keys::{
    ChromeMotionOverlayCacheFingerprint, FocusOverlayCacheFingerprint,
    HoverOverlayCacheFingerprint, ImageUploadBlobCacheKey, ModalOverlayCacheFingerprint,
    SharedPixelBytes, StaticSegmentCacheFingerprint, WaveformMotionOverlayCacheFingerprint,
    touch_image_upload_blob_cache_key,
};
pub(super) use scene_entries::StaticSegmentSceneCache;
pub(super) use signatures::{
    chrome_motion_overlay_model_signature, focus_overlay_model_signature,
    hover_overlay_model_signature, modal_overlay_model_signature, static_segment_style_signature,
    waveform_motion_overlay_model_signature,
};
