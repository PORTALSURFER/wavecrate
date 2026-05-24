use super::SizingTokens;
use crate::theme::clamp_ui_scale;

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

        self.scale_shell_chrome(scale);
        self.scale_browser_chrome(scale);
        self.scale_sidebar_chrome(scale);
        self.scale_content_chrome(scale);
        self.scale_action_chrome(scale);
        self.scale_overlay_chrome(scale);
        self.scale_strokes_and_motion(scale);
        self.scale_fonts(scale);
        self.preserve_zero_gap_shell_contract();
        self
    }

    fn scale_shell_chrome(&mut self, scale: f32) {
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
        self.panel_inset *= scale;
        self.header_label_gutter *= scale;
        self.status_segment_gap *= scale;
    }

    fn scale_browser_chrome(&mut self, scale: f32) {
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
        self.browser_row_gap *= scale;
        self.browser_row_height *= scale;
    }

    fn scale_sidebar_chrome(&mut self, scale: f32) {
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
    }

    fn scale_content_chrome(&mut self, scale: f32) {
        self.waveform_min_height *= scale;
        self.waveform_max_height *= scale;
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
    }

    fn scale_action_chrome(&mut self, scale: f32) {
        self.action_button_width *= scale;
        self.action_button_height *= scale;
        self.action_button_gap *= scale;
        self.top_bar_cluster_gap *= scale;
        self.top_volume_meter_width *= scale;
        self.top_volume_meter_height *= scale;
        self.top_bar_action_cluster_min_width *= scale;
        self.top_bar_action_cluster_max_width *= scale;
        self.top_bar_action_cluster_title_reserve_width *= scale;
        self.sidebar_action_button_width *= scale;
        self.sidebar_action_button_height *= scale;
        self.sidebar_action_button_gap *= scale;
    }

    fn scale_overlay_chrome(&mut self, scale: f32) {
        self.overlay_padding *= scale;
        self.prompt_width *= scale;
        self.prompt_min_height *= scale;
        self.overlay_button_width *= scale;
        self.overlay_button_height *= scale;
        self.progress_bar_height *= scale;
        self.drag_overlay_height *= scale;
    }

    fn scale_strokes_and_motion(&mut self, scale: f32) {
        self.border_width *= scale;
        self.focus_stroke_width *= scale;
        self.waveform_scan_step *= scale;
        self.lamp_radius_base *= scale;
        self.lamp_radius_amp *= scale;
    }

    fn scale_fonts(&mut self, scale: f32) {
        self.font_title *= scale;
        self.font_header *= scale;
        self.font_body *= scale;
        self.font_meta *= scale;
        self.font_status *= scale;
    }

    fn preserve_zero_gap_shell_contract(&mut self) {
        self.frame_inset = 0.0;
        self.panel_gap = 0.0;
    }
}
