//! Generic waveform-header surface projection for the native-shell compat path.
//!
//! This keeps the waveform title/metadata band on the same public
//! `radiant::layout`, `radiant::runtime`, and `radiant::widgets` hosting
//! pattern used by the top bar, status bar, and sidebar chrome bands while the
//! waveform plot, overlays, and edit geometry remain on the compatibility path.

use super::style::SizingTokens;
use crate::{
    app::NativeMotionModel,
    gui::types::{Point, Rect},
    layout::layout_tree,
    runtime::UiSurface,
};
use radiant::prelude as ui;
use radiant::prelude::IntoView;

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
    let transport = model.waveform_transport();
    let viewport = model.waveform_viewport();
    let presentation = model.waveform_presentation();
    let raster_preview = model.waveform_image_preview();
    let chrome = model.signal_chrome();
    let playhead_text = transport
        .playhead_milli
        .map(format_milli_value)
        .unwrap_or_else(|| String::from("—"));
    let cursor_text = transport
        .cursor_milli
        .map(format_milli_value)
        .unwrap_or_else(|| String::from("—"));
    let view_text = format!(
        "{}..{}",
        format_milli_value(viewport.start_milli),
        format_milli_value(viewport.end_milli)
    );
    let tempo_text = presentation.primary_label.as_deref().unwrap_or("— BPM");
    let zoom_text = presentation.viewport_label.as_deref().unwrap_or("100%");
    WaveformHeaderSurfaceContent {
        title: raster_preview
            .loaded_label
            .clone()
            .unwrap_or_else(|| String::from("Waveform")),
        metadata: format!(
            "{} | tempo: {} | zoom: {} | playhead: {} | cursor: {} | view: {}",
            chrome.status_hint, tempo_text, zoom_text, playhead_text, cursor_text, view_text,
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
    let title_height = sizing.font_header.max(1.0);
    let meta_height = sizing.font_meta.max(1.0);
    let content_column = ui::column([
        ui::text(&content.title)
            .id(WAVEFORM_HEADER_TITLE_ID)
            .size(1.0, title_height)
            .baseline((title_height * 0.75).max(0.0))
            .fill_width()
            .height(title_height),
        ui::text(&content.metadata)
            .id(WAVEFORM_HEADER_METADATA_ID)
            .size(1.0, meta_height)
            .baseline((meta_height * 0.75).max(0.0))
            .fill_width()
            .height(meta_height),
        ui::spacer().id(WAVEFORM_HEADER_FILL_ID).fill(),
    ])
    .id(WAVEFORM_HEADER_COLUMN_ID)
    .spacing(sizing.text_row_gap.max(0.0))
    .fill();
    UiSurface::new(
        ui::column([content_column])
            .id(WAVEFORM_HEADER_ROOT_ID)
            .padding_x((sizing.text_inset_x + sizing.header_label_gutter).max(0.0))
            .padding_y(sizing.text_inset_y.max(0.0))
            .fill()
            .into_node(),
    )
}

fn format_milli_value(value: u16) -> String {
    format!("{:.3}", f32::from(value.min(1000)) / 1000.0)
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    rect.clamp_to(bounds)
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::AppModel, app_core::native_shell::composition::style::StyleTokens};

    fn assert_widget_node(surface: &UiSurface<()>, id: u64) {
        assert_eq!(
            surface
                .find_widget(id)
                .expect("widget node should exist")
                .id(),
            id
        );
    }

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
    fn waveform_header_surface_projects_radiant_primitives() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let surface = build_waveform_header_surface(&content(), style.sizing);
        assert_widget_node(&surface, WAVEFORM_HEADER_TITLE_ID);
        assert_widget_node(&surface, WAVEFORM_HEADER_METADATA_ID);
        assert_widget_node(&surface, WAVEFORM_HEADER_FILL_ID);
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
