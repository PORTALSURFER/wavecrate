//! Retained view-tree layout and hit-testing for the native shell.

use super::{
    ShellLayoutRuntime,
    layout_adapter::{compute_status_bar_segments, compute_top_bar_band_sections},
    style::StyleTokens,
};
use crate::gui::types::{Point, Rect, Vector2};

mod contracts;
mod geometry;
mod tree;

#[cfg(test)]
pub(crate) use contracts::LayoutContractSnapshot;
use geometry::{
    band_header, build_browser_compat_columns, build_column_sections,
    waveform_scrollbar_lane_height,
};
use tree::build_shell_root;

/// Horizontal inset applied to the waveform plot and scrollbar lane.
const WAVEFORM_VIEW_SIDE_INSET: f32 = 10.0;

/// Stable identifier for nodes in the retained shell tree.
pub(crate) type ViewNodeId = u64;

/// Semantic node kinds used by the native shell tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShellNodeKind {
    Root,
    TopBar,
    Sidebar,
    Content,
    WaveformCard,
    BrowserPanel,
    BrowserTabs,
    BrowserTable,
    StatusBar,
}

/// A retained view node with stable identity, geometry, and optional children.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShellNode {
    pub id: ViewNodeId,
    pub kind: ShellNodeKind,
    pub rect: Rect,
    pub children: Vec<ShellNode>,
}

impl ShellNode {
    fn hit_test(&self, point: Point) -> Option<ShellNodeKind> {
        if !self.rect.contains(point) {
            return None;
        }
        for child in self.children.iter().rev() {
            if let Some(hit) = child.hit_test(point) {
                return Some(hit);
            }
        }
        Some(self.kind)
    }
}

/// Computed shell layout for one viewport size.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShellLayout {
    pub root: ShellNode,
    pub top_bar: Rect,
    pub top_bar_title_row: Rect,
    pub top_bar_controls_row: Rect,
    pub top_bar_title_cluster: Rect,
    pub top_bar_action_cluster: Rect,
    pub sidebar: Rect,
    pub sidebar_header: Rect,
    pub sidebar_rows: Rect,
    pub sidebar_footer: Rect,
    pub content: Rect,
    pub waveform_card: Rect,
    pub waveform_header: Rect,
    pub waveform_plot: Rect,
    pub waveform_scrollbar_lane: Rect,
    pub browser_panel: Rect,
    pub browser_tabs: Rect,
    pub browser_toolbar: Rect,
    pub browser_table_header: Rect,
    pub browser_rows: Rect,
    pub browser_footer: Rect,
    pub columns: [Rect; 3],
    pub column_headers: [Rect; 3],
    pub column_rows: [Rect; 3],
    pub status_bar: Rect,
    pub status_left_segment: Rect,
    pub status_center_segment: Rect,
    pub status_right_segment: Rect,
    pub status_progress_segment: Rect,
    /// UI scale factor used to derive the layout’s active token set.
    pub ui_scale: f32,
}

impl ShellLayout {
    /// Build shell layout for the provided logical viewport dimensions.
    #[cfg(test)]
    pub(crate) fn build(viewport: Vector2) -> Self {
        let style = StyleTokens::for_viewport_width(viewport.x);
        Self::build_with_style(viewport, &style)
    }

    /// Build shell layout for the provided viewport and style token set.
    #[cfg(test)]
    pub(crate) fn build_with_style(viewport: Vector2, style: &StyleTokens) -> Self {
        let mut runtime = ShellLayoutRuntime::default();
        Self::build_with_style_and_runtime(viewport, style, &mut runtime)
    }

    /// Build shell layout for the provided viewport/style using a persistent runtime cache.
    pub(crate) fn build_with_style_and_runtime(
        viewport: Vector2,
        style: &StyleTokens,
        runtime: &mut ShellLayoutRuntime,
    ) -> Self {
        let viewport_width = viewport.x.max(style.sizing.min_viewport_width);
        let viewport_height = viewport.y.max(style.sizing.min_viewport_height);
        let sizing = style.sizing;
        let base_style = StyleTokens::for_viewport_width(viewport_width);
        let ui_scale = if base_style.sizing.font_title > 0.0 {
            (sizing.font_title / base_style.sizing.font_title).clamp(1.0, 3.0)
        } else {
            1.0
        };
        let sections =
            runtime.compute_shell_sections(Vector2::new(viewport_width, viewport_height), style);
        let root_rect = sections.root;
        let top_bar = Rect::from_min_max(
            root_rect.min,
            Point::new(root_rect.max.x, root_rect.min.y + sizing.top_bar_height),
        );
        let top_bar_bands = compute_top_bar_band_sections(top_bar, sizing);
        let top_bar_title_row = top_bar_bands.top_bar_title_row;
        let top_bar_controls_row = top_bar_bands.top_bar_controls_row;
        let top_bar_title_cluster = top_bar_bands.top_bar_title_cluster;
        let top_bar_action_cluster = top_bar_bands.top_bar_action_cluster;
        let status_bar = Rect::from_min_max(
            Point::new(root_rect.min.x, root_rect.max.y - sizing.status_bar_height),
            root_rect.max,
        );
        let status_segments = compute_status_bar_segments(status_bar, sizing);
        let status_left_segment = status_segments.left;
        let status_center_segment = status_segments.center;
        let status_right_segment = status_segments.right;
        let status_progress_segment = status_segments.progress;
        let body_min_y = top_bar.max.y;
        let body_max_y = status_bar.min.y;
        let sidebar = Rect::from_min_max(
            Point::new(sections.sidebar.min.x, body_min_y),
            Point::new(sections.sidebar.max.x, body_max_y),
        );
        let sidebar_bands = runtime.compute_sidebar_band_sections(sidebar, sizing);
        let sidebar_header = sidebar_bands.sidebar_header;
        let sidebar_footer = sidebar_bands.sidebar_footer;
        let sidebar_rows = Rect::from_min_max(
            Point::new(
                sidebar_bands.sidebar_rows.min.x,
                sidebar_header.max.y.min(sidebar_footer.min.y),
            ),
            Point::new(
                sidebar_bands.sidebar_rows.max.x,
                sidebar_footer
                    .min
                    .y
                    .max(sidebar_header.max.y.min(sidebar_footer.min.y)),
            ),
        );
        let content = Rect::from_min_max(
            Point::new(sidebar.max.x, body_min_y),
            Point::new(root_rect.max.x, body_max_y),
        );
        let waveform_card = Rect::from_min_max(
            Point::new(content.min.x, content.min.y),
            Point::new(
                content.max.x,
                sections.waveform_card.max.y.min(content.max.y),
            ),
        );
        let browser_panel =
            Rect::from_min_max(Point::new(content.min.x, waveform_card.max.y), content.max);
        let browser_bands = runtime.compute_browser_band_sections(browser_panel, sizing);
        let browser_tabs = browser_bands.browser_tabs;
        let browser_footer = browser_bands.browser_footer;
        let browser_toolbar = Rect::from_min_max(
            Point::new(browser_bands.browser_toolbar.min.x, browser_tabs.max.y),
            Point::new(
                browser_bands.browser_toolbar.max.x,
                (browser_tabs.max.y + browser_bands.browser_toolbar.height())
                    .min(browser_footer.min.y),
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

        // Keep legacy triage partitions as invisible compatibility geometry for
        // routing actions that still speak in triage-column terms.
        let columns = build_browser_compat_columns(browser_rows, sizing);

        let waveform_header = band_header(waveform_card, sizing.waveform_header_block_height);
        let waveform_body_top = waveform_header
            .max
            .y
            .clamp(waveform_card.min.y, waveform_card.max.y);
        let waveform_body = Rect::from_min_max(
            Point::new(waveform_card.min.x, waveform_body_top),
            waveform_card.max,
        );
        let waveform_side_inset =
            WAVEFORM_VIEW_SIDE_INSET.min((waveform_body.width() - 1.0).max(0.0) * 0.5);
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
        let waveform_scrollbar_lane_height =
            waveform_scrollbar_lane_height(waveform_view_body, sizing.waveform_header_block_height);
        let waveform_scrollbar_lane = Rect::from_min_max(
            Point::new(
                waveform_view_body.min.x,
                waveform_view_body.max.y - waveform_scrollbar_lane_height,
            ),
            waveform_view_body.max,
        );
        let waveform_plot = Rect::from_min_max(
            waveform_view_body.min,
            Point::new(waveform_view_body.max.x, waveform_scrollbar_lane.min.y),
        );

        let (column_headers, column_rows) = build_column_sections(columns, sizing);
        let root = build_shell_root(
            root_rect,
            top_bar,
            sidebar,
            content,
            waveform_card,
            browser_panel,
            browser_tabs,
            browser_rows,
            status_bar,
        );

        Self {
            root,
            top_bar,
            top_bar_title_row,
            top_bar_controls_row,
            top_bar_title_cluster,
            top_bar_action_cluster,
            sidebar,
            sidebar_header,
            sidebar_rows,
            sidebar_footer,
            content,
            waveform_card,
            waveform_header,
            waveform_plot,
            waveform_scrollbar_lane,
            browser_panel,
            browser_tabs,
            browser_toolbar,
            browser_table_header,
            browser_rows,
            browser_footer,
            columns,
            column_headers,
            column_rows,
            status_bar,
            status_left_segment,
            status_center_segment,
            status_right_segment,
            status_progress_segment,
            ui_scale,
        }
    }

    /// Hit-test against the retained tree.
    pub(crate) fn hit_test(&self, point: Point) -> Option<ShellNodeKind> {
        self.root.hit_test(point)
    }

    /// Resolve triage column index for a point, if any.
    pub(crate) fn column_at_point(&self, point: Point) -> Option<usize> {
        if !self.browser_rows.contains(point) {
            return None;
        }
        let ratio = ((point.x - self.browser_rows.min.x) / self.browser_rows.width().max(1.0))
            .clamp(0.0, 0.999_9);
        Some((ratio * 3.0).floor() as usize)
    }

    /// Build a compact metric snapshot used by parity/layout contract tests.
    #[cfg(test)]
    pub(crate) fn contract_snapshot(&self, style: &StyleTokens) -> LayoutContractSnapshot {
        contracts::snapshot(self, style)
    }
}
