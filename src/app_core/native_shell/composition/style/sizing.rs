//! Geometry and typography tokens for the native shell.

use crate::theme::clamp_ui_scale;

mod base;
mod tier_deltas;

#[cfg(test)]
mod tests;

/// Resolve the shell sizing pack for the requested viewport tier.
pub(super) fn sizing_for_tier(layout_tier: super::LayoutScaleTier) -> SizingTokens {
    tier_deltas::sizing_for_tier(layout_tier)
}

/// Compact sizing tokens used by the native shell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SizingTokens {
    /// Minimum logical viewport width used by shell layout clamping.
    pub min_viewport_width: f32,
    /// Minimum logical viewport height used by shell layout clamping.
    pub min_viewport_height: f32,
    /// Root frame inset around the full shell viewport.
    pub frame_inset: f32,
    /// Shared gap between major shell panels.
    pub panel_gap: f32,
    /// Fixed top-bar band height.
    pub top_bar_height: f32,
    /// Height of the unified top-bar row inside the top bar.
    pub top_bar_title_row_height: f32,
    /// Minimum unified top-bar row height when vertical space is constrained.
    pub top_bar_title_row_min_height: f32,
    /// Reserved gap between title and controls rows. Zero keeps the top bar unified.
    pub top_bar_title_row_bottom_gap: f32,
    /// Fixed status-bar band height.
    pub status_bar_height: f32,
    /// Sidebar width ratio against body width.
    pub sidebar_ratio: f32,
    /// Sidebar minimum width.
    pub sidebar_min_width: f32,
    /// Sidebar maximum width.
    pub sidebar_max_width: f32,
    /// Main content minimum width.
    pub content_min_width: f32,
    /// Waveform card height ratio against content height.
    pub waveform_ratio: f32,
    /// Waveform card minimum height.
    pub waveform_min_height: f32,
    /// Waveform card maximum height.
    pub waveform_max_height: f32,
    /// Gap between triage columns.
    pub column_gap: f32,
    /// Maximum rendered browser rows per triage column.
    pub browser_rows_max_per_column: usize,
    /// Minimum width allowed for each triage column.
    pub column_min_width: f32,
    /// Height of the browser tab band.
    pub browser_tabs_height: f32,
    /// Minimum tabs-band height when browser panel space is constrained.
    pub browser_tabs_min_height: f32,
    /// Height of the browser toolbar band.
    pub browser_toolbar_height: f32,
    /// Minimum toolbar height when browser panel space is constrained.
    pub browser_toolbar_min_height: f32,
    /// Height of the browser table-header band.
    pub browser_table_header_height: f32,
    /// Minimum table-header height when browser panel space is constrained.
    pub browser_table_header_min_height: f32,
    /// Height of the browser footer band.
    pub browser_footer_height: f32,
    /// Minimum browser footer height to keep summary labels visible.
    pub browser_footer_min_height: f32,
    /// Maximum browser footer height to preserve row density.
    pub browser_footer_max_height: f32,
    /// Minimum width reserved for browser search controls.
    pub browser_search_field_min_width: f32,
    /// Preferred browser search width as a ratio of toolbar width.
    pub browser_search_field_ratio: f32,
    /// Width reserved for the browser row index column.
    pub browser_index_col_width: f32,
    /// Width reserved for the browser metadata-bucket column.
    pub browser_bucket_col_width: f32,
    /// Shared panel inset for nested regions.
    pub panel_inset: f32,
    /// Small horizontal gutter applied to header/status text anchors.
    pub header_label_gutter: f32,
    /// Gap between browser rows.
    pub browser_row_gap: f32,
    /// Browser row card height.
    pub browser_row_height: f32,
    /// Gap between source rows.
    pub source_row_gap: f32,
    /// Source row card height.
    pub source_row_height: f32,
    /// Maximum number of source rows rendered in compact sidebar mode.
    pub source_rows_max: usize,
    /// Minimum number of source rows preserved when folder rows are also visible.
    pub source_rows_min_when_split: usize,
    /// Gap between folder rows.
    pub folder_row_gap: f32,
    /// Folder row card height.
    pub folder_row_height: f32,
    /// Maximum number of folder rows rendered in the sidebar tree.
    pub tree_rows_max: usize,
    /// Minimum number of folder rows preserved when source rows are visible.
    pub tree_rows_min: usize,
    /// Gap between source/folder sections in the sidebar.
    pub sidebar_section_gap: f32,
    /// Stroke width for source/folder section dividers.
    pub source_section_divider_width: f32,
    /// Shared vertical spacing between section headers and their row stacks.
    pub header_to_rows_gap: f32,
    /// Top padding inside row-stack regions.
    pub panel_section_padding_top: f32,
    /// Bottom padding inside row-stack regions.
    pub panel_section_padding_bottom: f32,
    /// Top block height reserved for folder section header + metadata.
    pub folder_header_block_height: f32,
    /// Recovery badge height in the folder section header.
    pub recovery_badge_height: f32,
    /// Minimum width reserved for folder recovery badges.
    pub recovery_badge_min_width: f32,
    /// Horizontal padding inside the recovery badge.
    pub recovery_badge_padding_x: f32,
    /// Horizontal indent step for nested folder rows.
    pub folder_indent_step: f32,
    /// Space between compact metadata text rows.
    pub text_row_gap: f32,
    /// Gap between title and metadata lines in stacked header labels.
    pub title_meta_gap: f32,
    /// Horizontal text inset inside row cards.
    pub text_inset_x: f32,
    /// Vertical text inset inside row cards.
    pub text_inset_y: f32,
    /// Extra horizontal inset used for row labels inside bordered cards.
    pub row_corner_inset: f32,
    /// Top block height reserved for source header + search line.
    pub source_header_block_height: f32,
    /// Top block height reserved for triage column headers.
    pub column_header_block_height: f32,
    /// Top block height reserved for waveform title + metadata.
    pub waveform_header_block_height: f32,
    /// Bottom padding reserved for source list footer hints.
    pub source_bottom_padding: f32,
    /// Bottom padding reserved for triage columns.
    pub column_bottom_padding: f32,
    /// Minimum content width reserved while clamping sidebar width.
    pub content_tail_min_width: f32,
    /// Minimum content height reserved for the browser panel.
    pub content_browser_min_height: f32,
    /// Minimum waveform card height when content is constrained.
    pub waveform_card_floor_height: f32,
    /// Browser action button width.
    pub action_button_width: f32,
    /// Browser action button height.
    pub action_button_height: f32,
    /// Gap between browser action buttons.
    pub action_button_gap: f32,
    /// Gap between top-bar title and action clusters.
    pub top_bar_cluster_gap: f32,
    /// Width of the top-bar volume meter track.
    pub top_volume_meter_width: f32,
    /// Height of the top-bar volume meter track.
    pub top_volume_meter_height: f32,
    /// Minimum width reserved for top-bar actions cluster.
    pub top_bar_action_cluster_min_width: f32,
    /// Maximum width reserved for top-bar actions cluster.
    pub top_bar_action_cluster_max_width: f32,
    /// Minimum width reserved for the title cluster beside actions.
    pub top_bar_action_cluster_title_reserve_width: f32,
    /// Horizontal gap between status-bar text segments.
    pub status_segment_gap: f32,
    /// Outer padding for modal overlays.
    pub overlay_padding: f32,
    /// Prompt dialog width.
    pub prompt_width: f32,
    /// Prompt dialog minimum height.
    pub prompt_min_height: f32,
    /// Overlay button width.
    pub overlay_button_width: f32,
    /// Overlay button height.
    pub overlay_button_height: f32,
    /// Progress bar height.
    pub progress_bar_height: f32,
    /// Drag overlay banner height.
    pub drag_overlay_height: f32,
    /// Sidebar action button width.
    pub sidebar_action_button_width: f32,
    /// Sidebar action button height.
    pub sidebar_action_button_height: f32,
    /// Gap between sidebar action buttons.
    pub sidebar_action_button_gap: f32,
    /// Border stroke width.
    pub border_width: f32,
    /// Stroke width used for focused controls.
    pub focus_stroke_width: f32,
    /// Hover fill blend factor for panel/card surfaces.
    pub hover_fill_alpha: f32,
    /// Waveform scanline step width.
    pub waveform_scan_step: f32,
    /// Primary title font size.
    pub font_title: f32,
    /// Section header font size.
    pub font_header: f32,
    /// Body label font size.
    pub font_body: f32,
    /// Metadata font size.
    pub font_meta: f32,
    /// Status bar font size.
    pub font_status: f32,
    /// Base transport indicator radius.
    pub lamp_radius_base: f32,
    /// Additional transport indicator pulse amplitude.
    pub lamp_radius_amp: f32,
}

impl SizingTokens {
    /// Scale geometry and font tokens for a logical UI scale factor.
    ///
    /// This keeps tier selection stable while preserving density ratios and
    /// interaction values that should remain independent from geometry.
    pub(crate) fn with_ui_scale(mut self, ui_scale: f32) -> Self {
        let scale = clamp_ui_scale(ui_scale);
        if (scale - 1.0).abs() < f32::EPSILON {
            return self;
        }

        self.frame_inset *= scale;
        self.panel_gap *= scale;
        self.top_bar_height *= scale;
        self.top_bar_title_row_height *= scale;
        self.top_bar_title_row_min_height *= scale;
        self.top_bar_title_row_bottom_gap *= scale;
        self.status_bar_height *= scale;
        self.sidebar_min_width *= scale;
        self.sidebar_max_width *= scale;
        self.content_min_width *= scale;
        self.waveform_min_height *= scale;
        self.waveform_max_height *= scale;
        self.column_gap *= scale;
        self.column_min_width *= scale;
        self.browser_tabs_height *= scale;
        self.browser_tabs_min_height *= scale;
        self.browser_toolbar_height *= scale;
        self.browser_toolbar_min_height *= scale;
        self.browser_table_header_height *= scale;
        self.browser_table_header_min_height *= scale;
        self.browser_footer_height *= scale;
        self.browser_footer_min_height *= scale;
        self.browser_footer_max_height *= scale;
        self.browser_search_field_min_width *= scale;
        self.browser_index_col_width *= scale;
        self.browser_bucket_col_width *= scale;
        self.panel_inset *= scale;
        self.header_label_gutter *= scale;
        self.browser_row_gap *= scale;
        self.browser_row_height *= scale;
        self.source_row_gap *= scale;
        self.source_row_height *= scale;
        self.folder_row_gap *= scale;
        self.folder_row_height *= scale;
        self.sidebar_section_gap *= scale;
        self.source_section_divider_width *= scale;
        self.header_to_rows_gap *= scale;
        self.panel_section_padding_top *= scale;
        self.panel_section_padding_bottom *= scale;
        self.folder_header_block_height *= scale;
        self.recovery_badge_height *= scale;
        self.recovery_badge_min_width *= scale;
        self.recovery_badge_padding_x *= scale;
        self.folder_indent_step *= scale;
        self.text_row_gap *= scale;
        self.title_meta_gap *= scale;
        self.text_inset_x *= scale;
        self.text_inset_y *= scale;
        self.row_corner_inset *= scale;
        self.source_header_block_height *= scale;
        self.column_header_block_height *= scale;
        self.waveform_header_block_height *= scale;
        self.source_bottom_padding *= scale;
        self.column_bottom_padding *= scale;
        self.content_tail_min_width *= scale;
        self.content_browser_min_height *= scale;
        self.waveform_card_floor_height *= scale;
        self.action_button_width *= scale;
        self.action_button_height *= scale;
        self.action_button_gap *= scale;
        self.top_bar_cluster_gap *= scale;
        self.top_volume_meter_width *= scale;
        self.top_volume_meter_height *= scale;
        self.top_bar_action_cluster_min_width *= scale;
        self.top_bar_action_cluster_max_width *= scale;
        self.top_bar_action_cluster_title_reserve_width *= scale;
        self.status_segment_gap *= scale;
        self.overlay_padding *= scale;
        self.prompt_width *= scale;
        self.prompt_min_height *= scale;
        self.overlay_button_width *= scale;
        self.overlay_button_height *= scale;
        self.progress_bar_height *= scale;
        self.drag_overlay_height *= scale;
        self.sidebar_action_button_width *= scale;
        self.sidebar_action_button_height *= scale;
        self.sidebar_action_button_gap *= scale;
        self.border_width *= scale;
        self.focus_stroke_width *= scale;
        self.waveform_scan_step *= scale;
        self.font_title *= scale;
        self.font_header *= scale;
        self.font_body *= scale;
        self.font_meta *= scale;
        self.font_status *= scale;
        self.lamp_radius_base *= scale;
        self.lamp_radius_amp *= scale;
        self.frame_inset = 0.0;
        self.panel_gap = 0.0;
        self
    }
}
