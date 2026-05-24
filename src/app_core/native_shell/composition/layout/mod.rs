//! Retained view-tree layout and hit-testing for the native shell.

use super::{ShellLayoutRuntime, style::StyleTokens};
use crate::gui::types::{Point, Rect, Vector2};

mod contracts;
mod geometry;
mod sections;
mod tree;

#[cfg(test)]
pub(crate) use contracts::LayoutContractSnapshot;
use geometry::{build_browser_compat_columns, build_column_sections};
use sections::{
    build_browser_layout, build_content_layout, build_layout_frame, build_sidebar_layout,
    build_top_status_layout, build_waveform_layout,
};
use tree::build_shell_root;

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
        let sizing = style.sizing;
        let frame = build_layout_frame(viewport, style, runtime);
        let top_status = build_top_status_layout(frame.root_rect, sizing);
        let sidebar = build_sidebar_layout(
            frame.sections.sidebar,
            top_status.top_bar,
            top_status.status_bar,
            sizing,
            runtime,
        );
        let content = build_content_layout(
            frame.root_rect,
            sidebar.sidebar,
            frame.sections.waveform_card,
            top_status.top_bar,
            top_status.status_bar,
        );
        let browser = build_browser_layout(content.browser_panel, sizing, runtime);

        // Keep legacy triage partitions as invisible compatibility geometry for
        // routing actions that still speak in triage-column terms.
        let columns = build_browser_compat_columns(browser.browser_rows, sizing);
        let waveform = build_waveform_layout(content.waveform_card, sizing);

        let (column_headers, column_rows) = build_column_sections(columns, sizing);
        let root = build_shell_root(
            frame.root_rect,
            top_status.top_bar,
            sidebar.sidebar,
            content.content,
            content.waveform_card,
            content.browser_panel,
            browser.browser_tabs,
            browser.browser_rows,
            top_status.status_bar,
        );

        Self {
            root,
            top_bar: top_status.top_bar,
            top_bar_title_row: top_status.top_bar_title_row,
            top_bar_controls_row: top_status.top_bar_controls_row,
            top_bar_title_cluster: top_status.top_bar_title_cluster,
            top_bar_action_cluster: top_status.top_bar_action_cluster,
            sidebar: sidebar.sidebar,
            sidebar_header: sidebar.sidebar_header,
            sidebar_rows: sidebar.sidebar_rows,
            sidebar_footer: sidebar.sidebar_footer,
            content: content.content,
            waveform_card: content.waveform_card,
            waveform_header: waveform.waveform_header,
            waveform_plot: waveform.waveform_plot,
            waveform_scrollbar_lane: waveform.waveform_scrollbar_lane,
            browser_panel: content.browser_panel,
            browser_tabs: browser.browser_tabs,
            browser_toolbar: browser.browser_toolbar,
            browser_table_header: browser.browser_table_header,
            browser_rows: browser.browser_rows,
            browser_footer: browser.browser_footer,
            columns,
            column_headers,
            column_rows,
            status_bar: top_status.status_bar,
            status_left_segment: top_status.status_left_segment,
            status_center_segment: top_status.status_center_segment,
            status_right_segment: top_status.status_right_segment,
            status_progress_segment: top_status.status_progress_segment,
            ui_scale: frame.ui_scale,
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
