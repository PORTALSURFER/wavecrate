//! Retained native-shell cache keys, counters, and small cache storage types.

use super::*;

mod frame_text;
mod interaction;
mod overlay_fingerprints;
mod segmented;
mod truncation;

pub(in crate::gui::native_shell::state) use frame_text::*;
pub(in crate::gui::native_shell::state) use interaction::*;
pub(in crate::gui::native_shell::state) use overlay_fingerprints::WaveformResizeHoverEdge;
pub(crate) use overlay_fingerprints::{
    ChromeMotionOverlayFingerprint, CursorMoveEffect, FocusOverlayFingerprint,
    HoverOverlayFingerprint, ModalOverlayFingerprint, WaveformMotionOverlayFingerprint,
    WaveformToolbarHoverHint,
};
#[cfg(test)]
pub(in crate::gui::native_shell::state) use overlay_fingerprints::{
    MotionOverlayFingerprint, StateOverlayFingerprint,
};
pub(in crate::gui::native_shell::state) use segmented::{
    PrimitiveSink, SegmentedPrimitiveSink, SegmentedStaticEmitContext, SegmentedTextRunSink,
    TextRunSink, emit_primitive, emit_text,
};
pub(crate) use segmented::{StaticFrameSegment, StaticFrameSegments};
pub(in crate::gui::native_shell::state) use truncation::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Vector2;
    use std::cell::RefCell;

    #[test]
    fn truncation_cache_evicts_oldest_entry_after_capacity() {
        let mut cache = BrowserRowTruncationCache::default();
        let mut counts = BrowserRowTruncationFrameCounts::default();

        for row_id in 0..=BROWSER_ROW_TRUNCATION_CACHE_CAPACITY as u32 {
            let key = BrowserRowTruncationEntryKey {
                row_id,
                width_bucket: 10,
                font_size_bucket: 12,
                text_kind: BrowserRowTextKind::Item,
            };
            let _ = cache.resolve(key, "abcdefgh", 12.0, 10.0, &mut counts);
        }

        assert_eq!(cache.values.len(), BROWSER_ROW_TRUNCATION_CACHE_CAPACITY);
        assert_eq!(
            counts.cache_miss_count,
            BROWSER_ROW_TRUNCATION_CACHE_CAPACITY as u32 + 1
        );
        assert!(!cache.values.contains_key(&BrowserRowTruncationEntryKey {
            row_id: 0,
            width_bucket: 10,
            font_size_bucket: 12,
            text_kind: BrowserRowTextKind::Item,
        }));
        assert!(cache.values.contains_key(&BrowserRowTruncationEntryKey {
            row_id: BROWSER_ROW_TRUNCATION_CACHE_CAPACITY as u32,
            width_bucket: 10,
            font_size_bucket: 12,
            text_kind: BrowserRowTextKind::Item,
        }));
    }

    #[test]
    fn segmented_emit_context_routes_only_targeted_status_bar_output() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let model = AppModel::default();
        let mut segments = StaticFrameSegments::default();
        let context = RefCell::new(SegmentedStaticEmitContext {
            layout: &layout,
            model: &model,
            segments: &mut segments,
            target_segment: Some(StaticFrameSegment::StatusBar),
        });

        emit_primitive(
            &mut SegmentedPrimitiveSink { context: &context },
            Primitive::Rect(FillRect {
                rect: layout.status_bar,
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            }),
        );
        emit_primitive(
            &mut SegmentedPrimitiveSink { context: &context },
            Primitive::Rect(FillRect {
                rect: layout.browser_panel,
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            }),
        );
        emit_text(
            &mut SegmentedTextRunSink { context: &context },
            TextRun {
                text: String::from("status"),
                position: layout.status_bar.min,
                font_size: 12.0,
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                max_width: None,
                align: TextAlign::Left,
            },
        );

        assert_eq!(
            context
                .borrow()
                .segments
                .frame(StaticFrameSegment::StatusBar)
                .primitives
                .len(),
            1
        );
        assert_eq!(
            context
                .borrow()
                .segments
                .frame(StaticFrameSegment::StatusBar)
                .text_runs
                .len(),
            1
        );
        assert!(
            context
                .borrow()
                .segments
                .frame(StaticFrameSegment::BrowserFrame)
                .primitives
                .is_empty()
        );
    }
}
