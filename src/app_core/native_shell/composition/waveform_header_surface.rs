//! Generic waveform-header surface projection for the native-shell compat path.
//!
//! This keeps the waveform title/metadata band on the same public
//! `radiant::layout`, `radiant::runtime`, and `radiant::widgets` hosting
//! pattern used by the top bar, status bar, and sidebar chrome bands while the
//! waveform plot, overlays, and edit geometry remain on the compatibility path.

use super::style::SizingTokens;
use crate::{
    app::NativeMotionModel,
    gui::types::{Point, Rect, Vector2},
    layout::{
        Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, MainAlign, OverflowPolicy,
        SizeModeCross, SizeModeMain, SlotParams, layout_tree,
    },
    runtime::{SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{CanvasWidget, TextWidget, WidgetSizing, WidgetSpec},
};

const WAVEFORM_HEADER_ROOT_ID: u64 = 1120;
const WAVEFORM_HEADER_COLUMN_ID: u64 = 1121;
const WAVEFORM_HEADER_TITLE_ID: u64 = 1122;
const WAVEFORM_HEADER_METADATA_ID: u64 = 1123;
const WAVEFORM_HEADER_FILL_ID: u64 = 1124;

/// User-facing content projected into the generic waveform-header surface.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WaveformHeaderSurfaceContent {
    /// Primary loaded-sample title shown on the first row.
    pub title: String,
    /// Compact waveform metadata shown on the second row.
    pub metadata: String,
}

/// Resolved text layout for the generic waveform-header surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WaveformHeaderSurfaceLayout {
    /// Title text widget bounds.
    pub title_text_rect: Rect,
    /// Metadata text widget bounds.
    pub metadata_text_rect: Rect,
}

/// Build waveform-header copy from the projected motion model.
pub(crate) fn waveform_header_surface_content(
    model: &NativeMotionModel,
) -> WaveformHeaderSurfaceContent {
    let playhead_text = model
        .waveform_playhead_milli
        .map(format_milli_value)
        .unwrap_or_else(|| String::from("—"));
    let cursor_text = model
        .waveform_cursor_milli
        .map(format_milli_value)
        .unwrap_or_else(|| String::from("—"));
    let view_text = format!(
        "{}..{}",
        format_milli_value(model.waveform_view_start_milli),
        format_milli_value(model.waveform_view_end_milli)
    );
    let tempo_text = model.waveform_tempo_label.as_deref().unwrap_or("— BPM");
    let zoom_text = model.waveform_zoom_label.as_deref().unwrap_or("100%");
    WaveformHeaderSurfaceContent {
        title: model
            .waveform_loaded_label
            .clone()
            .unwrap_or_else(|| String::from("Waveform")),
        metadata: format!(
            "{} | tempo: {} | zoom: {} | playhead: {} | cursor: {} | view: {}",
            model.waveform_transport_hint,
            tempo_text,
            zoom_text,
            playhead_text,
            cursor_text,
            view_text,
        ),
    }
}

/// Resolve the generic waveform-header surface layout inside one shell band.
pub(crate) fn resolve_waveform_header_surface_layout(
    header_rect: Rect,
    sizing: SizingTokens,
    content: &WaveformHeaderSurfaceContent,
) -> WaveformHeaderSurfaceLayout {
    let surface = build_waveform_header_surface(content, sizing);
    let output = layout_tree(&surface.layout_node(), header_rect);
    let empty = Rect::from_min_max(header_rect.min, header_rect.min);
    WaveformHeaderSurfaceLayout {
        title_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, WAVEFORM_HEADER_TITLE_ID, empty),
            header_rect,
        ),
        metadata_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, WAVEFORM_HEADER_METADATA_ID, empty),
            header_rect,
        ),
    }
}

fn build_waveform_header_surface(
    content: &WaveformHeaderSurfaceContent,
    sizing: SizingTokens,
) -> UiSurface<()> {
    UiSurface::new(SurfaceNode::container(
        WAVEFORM_HEADER_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: (sizing.text_inset_x + sizing.header_label_gutter).max(0.0),
                right: sizing.text_inset_x.max(0.0),
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                WAVEFORM_HEADER_COLUMN_ID,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: sizing.text_row_gap.max(0.0),
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::new(
                        text_slot(sizing.font_header),
                        text_widget(
                            WAVEFORM_HEADER_TITLE_ID,
                            &content.title,
                            sizing.font_header.max(1.0),
                        ),
                    ),
                    SurfaceChild::new(
                        text_slot(sizing.font_meta),
                        text_widget(
                            WAVEFORM_HEADER_METADATA_ID,
                            &content.metadata,
                            sizing.font_meta.max(1.0),
                        ),
                    ),
                    SurfaceChild::new(
                        SlotParams::fill(),
                        SurfaceNode::widget(
                            WidgetSpec::Canvas(CanvasWidget::new(
                                WAVEFORM_HEADER_FILL_ID,
                                WidgetSizing::fixed(Vector2::new(1.0, 1.0)),
                            )),
                            WidgetMessageMapper::None,
                        ),
                    ),
                ],
            ),
        )],
    ))
}

fn text_widget(id: u64, text: &str, font_size: f32) -> SurfaceNode<()> {
    SurfaceNode::widget(
        WidgetSpec::Text(TextWidget::new(
            id,
            text,
            WidgetSizing::fixed(Vector2::new(1.0, font_size.max(1.0)))
                .with_baseline((font_size * 0.75).max(0.0)),
        )),
        WidgetMessageMapper::None,
    )
}

fn text_slot(font_size: f32) -> SlotParams {
    let font_size = font_size.max(1.0);
    SlotParams {
        size_main: SizeModeMain::Fixed(font_size),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(0.0, f32::INFINITY, font_size, font_size),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

fn format_milli_value(value: u16) -> String {
    format!("{:.3}", f32::from(value.min(1000)) / 1000.0)
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::AppModel, gui::native_shell::style::StyleTokens, widgets::WidgetKind};

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    fn content() -> WaveformHeaderSurfaceContent {
        waveform_header_surface_content(&NativeMotionModel::from_app_model(&AppModel::default()))
    }

    #[test]
    fn waveform_header_surface_uses_public_text_and_canvas_widgets() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let surface = build_waveform_header_surface(&content(), style.sizing);
        assert_eq!(
            surface
                .find_widget(WAVEFORM_HEADER_TITLE_ID)
                .expect("title")
                .widget()
                .kind(),
            WidgetKind::Text
        );
        assert_eq!(
            surface
                .find_widget(WAVEFORM_HEADER_METADATA_ID)
                .expect("metadata")
                .widget()
                .kind(),
            WidgetKind::Text
        );
        assert_eq!(
            surface
                .find_widget(WAVEFORM_HEADER_FILL_ID)
                .expect("fill")
                .widget()
                .kind(),
            WidgetKind::Canvas
        );
    }

    #[test]
    fn waveform_header_surface_layout_keeps_rows_inside_header() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let header = Rect::from_min_max(Point::new(220.0, 32.0), Point::new(1260.0, 64.0));
        let layout = resolve_waveform_header_surface_layout(header, style.sizing, &content());
        assert_inside(header, layout.title_text_rect);
        assert_inside(header, layout.metadata_text_rect);
        assert!(layout.title_text_rect.max.y <= layout.metadata_text_rect.max.y);
    }

    #[test]
    fn waveform_header_surface_layout_covers_loading_loaded_and_compact_states() {
        let states = [
            (820.0, AppModel::default()),
            (1440.0, {
                let mut model = AppModel::default();
                model.waveform.loaded_label = Some(String::from("kick.wav"));
                model.waveform.tempo_label = Some(String::from("128.0 BPM"));
                model.waveform.zoom_label = Some(String::from("125%"));
                model.waveform.cursor_milli = Some(315);
                model.waveform.playhead_milli = Some(620);
                model.waveform_chrome.transport_hint = String::from("Loop enabled");
                model
            }),
            (360.0, {
                let mut model = AppModel::default();
                model.waveform.loaded_label = Some(String::from("very_long_loaded_take_name.wav"));
                model.waveform.loading = true;
                model
            }),
        ];
        for (viewport_width, model) in states {
            let style = StyleTokens::for_viewport_width(viewport_width);
            let header = Rect::from_min_max(
                Point::new(0.0, 0.0),
                Point::new(viewport_width, style.sizing.waveform_header_block_height),
            );
            let content =
                waveform_header_surface_content(&NativeMotionModel::from_app_model(&model));
            let layout = resolve_waveform_header_surface_layout(header, style.sizing, &content);
            assert_inside(header, layout.title_text_rect);
            assert_inside(header, layout.metadata_text_rect);
            assert!(layout.title_text_rect.height() > 0.0);
            assert!(layout.metadata_text_rect.height() > 0.0);
            assert!(layout.title_text_rect.min.y <= layout.metadata_text_rect.min.y);
        }
    }
}
