//! Scene-cache rebuild and composition helpers for the native Vello runtime.

mod composition;
mod fingerprints;
mod rebuild;
mod static_segments;

use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Resolve a retained image-upload blob for one RGBA payload.
    pub(crate) fn cached_image_upload_blob(
        cache: &mut HashMap<ImageUploadBlobCacheKey, Blob<u8>>,
        cache_order: &mut VecDeque<ImageUploadBlobCacheKey>,
        pixels: &Arc<[u8]>,
        width: u32,
        height: u32,
    ) -> Blob<u8> {
        let key = ImageUploadBlobCacheKey {
            pixels_ptr: pixels.as_ptr() as usize,
            width,
            height,
        };
        if let Some(blob) = cache.get(&key) {
            touch_image_upload_blob_cache_key(cache_order, key);
            return blob.clone();
        }
        while cache.len() >= IMAGE_UPLOAD_BLOB_CACHE_LIMIT {
            let Some(stale_key) = cache_order.pop_front() else {
                cache.clear();
                break;
            };
            cache.remove(&stale_key);
        }
        let blob = Blob::new(Arc::new(SharedPixelBytes(Arc::clone(pixels))));
        cache.insert(key, blob.clone());
        cache_order.push_back(key);
        blob
    }

    /// Return bridge-provided revision for one static segment.
    pub(crate) fn static_segment_revision(
        &self,
        segment_revisions: SegmentRevisions,
        segment: StaticFrameSegment,
    ) -> u64 {
        match segment {
            StaticFrameSegment::StatusBar => segment_revisions.status_bar,
            StaticFrameSegment::BrowserFrame => segment_revisions.browser_frame,
            StaticFrameSegment::BrowserRowsWindow => segment_revisions.browser_rows_window,
            StaticFrameSegment::MapPanel => segment_revisions.map_panel,
            StaticFrameSegment::WaveformOverlay => segment_revisions.waveform_overlay,
            StaticFrameSegment::GlobalStatic => segment_revisions.global_static,
        }
    }

    /// Return deterministic static segment identifier from cache-array index.
    pub(crate) fn static_segment_from_cache_index(index: usize) -> StaticFrameSegment {
        match index {
            0 => StaticFrameSegment::GlobalStatic,
            1 => StaticFrameSegment::WaveformOverlay,
            2 => StaticFrameSegment::BrowserFrame,
            3 => StaticFrameSegment::BrowserRowsWindow,
            4 => StaticFrameSegment::MapPanel,
            5 => StaticFrameSegment::StatusBar,
            _ => unreachable!("invalid static segment index {index}"),
        }
    }

    /// Refresh cached motion-model projection from the latest full app model.
    pub(crate) fn refresh_motion_model_from_model(&mut self) {
        self.motion_model = Some(NativeMotionModel::from_app_model(&self.model));
    }
}
