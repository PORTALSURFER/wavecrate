use crate::app_core::native_shell::composition::{
    ShellLayoutRuntime,
    layout_adapter::{
        ShellSectionRects, compute_status_bar_segments, compute_top_bar_band_sections,
    },
    style::{SizingTokens, StyleTokens},
};
use crate::gui::types::{Point, Rect, Vector2};

use super::geometry::{band_header, waveform_scrollbar_lane_height};

pub(super) struct LayoutFrame {
    pub(super) root_rect: Rect,
    pub(super) sections: ShellSectionRects,
    pub(super) ui_scale: f32,
}

pub(super) struct TopStatusLayout {
    pub(super) top_bar: Rect,
    pub(super) top_bar_title_row: Rect,
    pub(super) top_bar_controls_row: Rect,
    pub(super) top_bar_title_cluster: Rect,
    pub(super) top_bar_action_cluster: Rect,
    pub(super) status_bar: Rect,
    pub(super) status_left_segment: Rect,
    pub(super) status_center_segment: Rect,
    pub(super) status_right_segment: Rect,
    pub(super) status_progress_segment: Rect,
}

pub(super) struct SidebarLayout {
    pub(super) sidebar: Rect,
    pub(super) sidebar_header: Rect,
    pub(super) sidebar_rows: Rect,
    pub(super) sidebar_footer: Rect,
}

pub(super) struct ContentLayout {
    pub(super) content: Rect,
    pub(super) waveform_card: Rect,
    pub(super) browser_panel: Rect,
}

pub(super) struct BrowserLayout {
    pub(super) browser_tabs: Rect,
    pub(super) browser_toolbar: Rect,
    pub(super) browser_table_header: Rect,
    pub(super) browser_rows: Rect,
    pub(super) browser_footer: Rect,
}

pub(super) struct WaveformLayout {
    pub(super) waveform_header: Rect,
    pub(super) waveform_plot: Rect,
    pub(super) waveform_scrollbar_lane: Rect,
}

pub(super) fn build_layout_frame(
    viewport: Vector2,
    style: &StyleTokens,
    runtime: &mut ShellLayoutRuntime,
) -> LayoutFrame {
    let viewport_width = viewport.x.max(style.sizing.min_viewport_width);
    let viewport_height = viewport.y.max(style.sizing.min_viewport_height);
    let sections =
        runtime.compute_shell_sections(Vector2::new(viewport_width, viewport_height), style);
    LayoutFrame {
        root_rect: sections.root,
        sections,
        ui_scale: compute_ui_scale(viewport_width, style),
    }
}

pub(super) fn build_top_status_layout(root_rect: Rect, sizing: SizingTokens) -> TopStatusLayout {
    let top_bar = Rect::from_min_max(
        root_rect.min,
        Point::new(root_rect.max.x, root_rect.min.y + sizing.top_bar_height),
    );
    let top_bar_bands = compute_top_bar_band_sections(top_bar, sizing);
    let status_bar = Rect::from_min_max(
        Point::new(root_rect.min.x, root_rect.max.y - sizing.status_bar_height),
        root_rect.max,
    );
    let status_segments = compute_status_bar_segments(status_bar, sizing);

    TopStatusLayout {
        top_bar,
        top_bar_title_row: top_bar_bands.top_bar_title_row,
        top_bar_controls_row: top_bar_bands.top_bar_controls_row,
        top_bar_title_cluster: top_bar_bands.top_bar_title_cluster,
        top_bar_action_cluster: top_bar_bands.top_bar_action_cluster,
        status_bar,
        status_left_segment: status_segments.left,
        status_center_segment: status_segments.center,
        status_right_segment: status_segments.right,
        status_progress_segment: status_segments.progress,
    }
}

pub(super) fn build_sidebar_layout(
    sidebar_seed: Rect,
    top_bar: Rect,
    status_bar: Rect,
    sizing: SizingTokens,
    runtime: &mut ShellLayoutRuntime,
) -> SidebarLayout {
    let sidebar = Rect::from_min_max(
        Point::new(sidebar_seed.min.x, top_bar.max.y),
        Point::new(sidebar_seed.max.x, status_bar.min.y),
    );
    let sidebar_bands = runtime.compute_sidebar_band_sections(sidebar, sizing);
    let sidebar_header = sidebar_bands.sidebar_header;
    let sidebar_footer = sidebar_bands.sidebar_footer;
    let sidebar_rows_top = sidebar_header.max.y.min(sidebar_footer.min.y);
    let sidebar_rows = Rect::from_min_max(
        Point::new(sidebar_bands.sidebar_rows.min.x, sidebar_rows_top),
        Point::new(
            sidebar_bands.sidebar_rows.max.x,
            sidebar_footer.min.y.max(sidebar_rows_top),
        ),
    );

    SidebarLayout {
        sidebar,
        sidebar_header,
        sidebar_rows,
        sidebar_footer,
    }
}

pub(super) fn build_content_layout(
    root_rect: Rect,
    sidebar: Rect,
    waveform_card_seed: Rect,
    top_bar: Rect,
    status_bar: Rect,
) -> ContentLayout {
    let content = Rect::from_min_max(
        Point::new(sidebar.max.x, top_bar.max.y),
        Point::new(root_rect.max.x, status_bar.min.y),
    );
    let waveform_card = Rect::from_min_max(
        Point::new(content.min.x, content.min.y),
        Point::new(content.max.x, waveform_card_seed.max.y.min(content.max.y)),
    );
    let browser_panel =
        Rect::from_min_max(Point::new(content.min.x, waveform_card.max.y), content.max);

    ContentLayout {
        content,
        waveform_card,
        browser_panel,
    }
}

pub(super) fn build_browser_layout(
    browser_panel: Rect,
    sizing: SizingTokens,
    runtime: &mut ShellLayoutRuntime,
) -> BrowserLayout {
    let browser_bands = runtime.compute_browser_band_sections(browser_panel, sizing);
    let browser_tabs = browser_bands.browser_tabs;
    let browser_footer = browser_bands.browser_footer;
    let browser_toolbar = Rect::from_min_max(
        Point::new(browser_bands.browser_toolbar.min.x, browser_tabs.max.y),
        Point::new(
            browser_bands.browser_toolbar.max.x,
            (browser_tabs.max.y + browser_bands.browser_toolbar.height()).min(browser_footer.min.y),
        ),
    );
    let browser_table_header = Rect::from_min_max(
        Point::new(
            browser_bands.browser_table_header.min.x,
            browser_toolbar.max.y,
        ),
        Point::new(
            browser_bands.browser_table_header.max.x,
            (browser_toolbar.max.y + browser_bands.browser_table_header.height())
                .min(browser_footer.min.y),
        ),
    );
    let browser_rows = Rect::from_min_max(
        Point::new(browser_bands.browser_rows.min.x, browser_table_header.max.y),
        Point::new(
            browser_bands.browser_rows.max.x,
            browser_footer.min.y.max(browser_table_header.max.y),
        ),
    );

    BrowserLayout {
        browser_tabs,
        browser_toolbar,
        browser_table_header,
        browser_rows,
        browser_footer,
    }
}

pub(super) fn build_waveform_layout(waveform_card: Rect, sizing: SizingTokens) -> WaveformLayout {
    let waveform_header = band_header(waveform_card, sizing.waveform_header_block_height);
    let waveform_body_top = waveform_header
        .max
        .y
        .clamp(waveform_card.min.y, waveform_card.max.y);
    let waveform_body = Rect::from_min_max(
        Point::new(waveform_card.min.x, waveform_body_top),
        waveform_card.max,
    );
    let waveform_side_inset = 10.0_f32.min((waveform_body.width() - 1.0).max(0.0) * 0.5);
    let waveform_view_body = Rect::from_min_max(
        Point::new(
            waveform_body.min.x + waveform_side_inset,
            waveform_body.min.y,
        ),
        Point::new(
            waveform_body.max.x - waveform_side_inset,
            waveform_body.max.y,
        ),
    );
    let scrollbar_height =
        waveform_scrollbar_lane_height(waveform_view_body, sizing.waveform_header_block_height);
    let waveform_scrollbar_lane = Rect::from_min_max(
        Point::new(
            waveform_view_body.min.x,
            waveform_view_body.max.y - scrollbar_height,
        ),
        waveform_view_body.max,
    );
    let waveform_plot = Rect::from_min_max(
        waveform_view_body.min,
        Point::new(waveform_view_body.max.x, waveform_scrollbar_lane.min.y),
    );

    WaveformLayout {
        waveform_header,
        waveform_plot,
        waveform_scrollbar_lane,
    }
}

fn compute_ui_scale(viewport_width: f32, style: &StyleTokens) -> f32 {
    let base_style = StyleTokens::for_viewport_width(viewport_width);
    if base_style.sizing.font_title <= 0.0 {
        return 1.0;
    }

    (style.sizing.font_title / base_style.sizing.font_title).clamp(1.0, 3.0)
}
