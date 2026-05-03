//! Retained geometry and hit-test cache accessors for the native shell.

use super::*;
use crate::compat_app_contract::FolderPaneIdModel;

pub(super) struct BrowserInteractionGeometry<'a> {
    pub(super) style: StyleTokens,
    pub(super) rows: &'a [CachedBrowserRow],
    pub(super) scrollbar: Option<BrowserScrollbarLayout>,
    pub(super) scrollbar_viewport_len: usize,
    pub(super) buttons: &'a [ActionButton],
    pub(super) chips: &'a [BrowserColumnChip],
    pub(super) toolbar: BrowserToolbarLayout,
}

impl NativeShellState {
    pub(super) fn cached_source_rows(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> &[CachedSourceRow] {
        let cache_key = sidebar_rows_cache_key(layout, style, model);
        if self.source_row_cache_key != Some(cache_key) {
            self.source_row_rects = rendered_source_row_rects(layout, style, model);
            self.source_row_cache_key = Some(cache_key);
        }
        &self.source_row_rects
    }

    pub(super) fn cached_tree_rows(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        pane: FolderPaneIdModel,
    ) -> &[CachedFolderRow] {
        let folder_pane = self.folder_pane_runtime_state_mut(pane);
        let cache_key = tree_rows_cache_key(
            layout,
            style,
            model,
            pane,
            folder_pane.window_start,
            folder_pane.autoscroll,
        );
        if folder_pane.cache_key != Some(cache_key) {
            let (rows, resolved_window_start) = rendered_tree_rows_with_state(
                layout,
                model,
                style,
                pane,
                folder_pane.window_start,
                folder_pane.autoscroll,
            );
            folder_pane.rows = rows;
            folder_pane.window_start = resolved_window_start;
            folder_pane.cache_key = Some(tree_rows_cache_key(
                layout,
                style,
                model,
                pane,
                resolved_window_start,
                folder_pane.autoscroll,
            ));
        }
        &self.folder_pane_runtime_state(pane).rows
    }

    pub(super) fn cached_folder_scrollbar(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pane: FolderPaneIdModel,
    ) -> Option<(FolderScrollbarLayout, usize)> {
        let style = style_for_layout(layout);
        let rows = self.cached_tree_rows(layout, &style, model, pane);
        let pane_model = model.sources.folder_pane(pane);
        let viewport_len = rows.len().min(pane_model.tree_rows.len());
        let sections = sidebar_sections(layout, &style, model);
        let scrollbar = folder_scrollbar_layout(
            sections.tree_rows(pane),
            rows,
            pane_model.tree_rows.len(),
            style.sizing,
        )?;
        Some((scrollbar, viewport_len))
    }

    pub(super) fn cached_browser_rows(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> &[CachedBrowserRow] {
        let previous_visible_start = self.browser_rows.first().map(|row| row.visible_row);
        let (window_start, _) = browser_rows_window_bounds_with_previous(
            layout,
            model,
            style.sizing,
            previous_visible_start,
        );
        let cache_key = browser_rows_cache_key(layout, style, model, window_start);
        let truncation_cache_key = browser_row_truncation_cache_key(layout, style, cache_key);
        if self.browser_row_truncation_cache_key != Some(truncation_cache_key) {
            self.browser_row_truncation_cache.clear();
            self.browser_row_truncation_cache_key = Some(truncation_cache_key);
        }
        self.browser_row_truncation_frame_counts = BrowserRowTruncationFrameCounts::default();
        if self.browser_rows_cache_key != Some(cache_key) {
            let (rows, resolved_window_start) =
                rendered_browser_rows_cached_with_window_start_and_previous(
                    layout,
                    model,
                    style,
                    &mut self.browser_row_truncation_cache,
                    &mut self.browser_row_truncation_frame_counts,
                    previous_visible_start,
                );
            let resolved_cache_key =
                browser_rows_cache_key(layout, style, model, resolved_window_start);
            self.browser_rows = rows;
            sync_cached_browser_row_selection(&mut self.browser_rows, model, resolved_window_start);
            self.browser_rows_window_start = resolved_window_start;
            self.browser_rows_cache_key = Some(resolved_cache_key);
        } else {
            sync_cached_browser_row_selection(&mut self.browser_rows, model, window_start);
            self.browser_rows_window_start = window_start;
        }
        &self.browser_rows
    }

    pub(super) fn cached_browser_scrollbar(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<(BrowserScrollbarLayout, usize)> {
        let style = style_for_layout(layout);
        self.cached_browser_scrollbar_for_style(layout, &style, model)
    }

    pub(super) fn cached_browser_scrollbar_for_style(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> Option<(BrowserScrollbarLayout, usize)> {
        self.cached_browser_rows(layout, style, model);
        let Some(rows_key) = self.browser_rows_cache_key else {
            self.browser_scrollbar = None;
            self.browser_scrollbar_viewport_len = 0;
            self.browser_scrollbar_cache_key = None;
            return None;
        };
        let cache_key = BrowserScrollbarCacheKey { rows_key };
        if self.browser_scrollbar_cache_key != Some(cache_key) {
            let rows = &self.browser_rows;
            let viewport_len = rows.len().min(model.browser.visible_count);
            let list_rect = browser_rows_list_rect(layout.browser_rows, style.sizing, model);
            self.browser_scrollbar = browser_scrollbar_layout(
                list_rect,
                rows,
                model.browser.visible_count,
                style.sizing,
            );
            self.browser_scrollbar_viewport_len = viewport_len;
            self.browser_scrollbar_cache_key = Some(cache_key);
        }
        self.browser_scrollbar
            .map(|scrollbar| (scrollbar, self.browser_scrollbar_viewport_len))
    }

    pub(super) fn cached_browser_action_hit_test(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> (&[ActionButton], &[BrowserColumnChip], BrowserToolbarLayout) {
        let cache_key = browser_action_hit_test_cache_key(layout, model);
        if self.browser_action_hit_test_cache_key != Some(cache_key) {
            let toolbar = browser_toolbar_layout(layout, style, model);
            self.browser_action_buttons = browser_action_buttons(layout, style, model, &toolbar);
            self.browser_column_chips =
                browser_column_chips(layout, style, model, &self.browser_action_buttons);
            self.browser_toolbar_layout = Some(toolbar);
            self.browser_action_hit_test_cache_key = Some(cache_key);
        } else if self.browser_toolbar_layout.is_none() {
            let toolbar = browser_toolbar_layout(layout, style, model);
            self.browser_toolbar_layout = Some(toolbar);
        }
        let toolbar = self
            .browser_toolbar_layout
            .expect("browser action hit-test cache must retain toolbar layout");
        (
            &self.browser_action_buttons,
            &self.browser_column_chips,
            toolbar,
        )
    }

    pub(super) fn cached_browser_interaction_geometry(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> BrowserInteractionGeometry<'_> {
        let style = style_for_layout(layout);
        self.cached_browser_rows(layout, &style, model);
        self.cached_browser_scrollbar_for_style(layout, &style, model);
        self.cached_browser_action_hit_test(layout, &style, model);
        BrowserInteractionGeometry {
            style,
            rows: &self.browser_rows,
            scrollbar: self.browser_scrollbar,
            scrollbar_viewport_len: self.browser_scrollbar_viewport_len,
            buttons: &self.browser_action_buttons,
            chips: &self.browser_column_chips,
            toolbar: self
                .browser_toolbar_layout
                .expect("browser interaction geometry must retain toolbar layout"),
        }
    }

    pub(super) fn cached_waveform_toolbar_buttons(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
    ) -> &[WaveformToolbarButton] {
        let cache_key = waveform_toolbar_hit_test_cache_key(
            layout,
            model,
            self.waveform_bpm_input_active,
            self.waveform_bpm_input_display.as_deref(),
        );
        if self.waveform_toolbar_hit_test_cache_key != Some(cache_key) {
            self.waveform_toolbar_buttons = waveform_toolbar_buttons(
                layout,
                style,
                model,
                self.waveform_bpm_input_active,
                self.waveform_bpm_input_display.as_deref(),
            );
            self.waveform_toolbar_hit_test_cache_key = Some(cache_key);
        }
        &self.waveform_toolbar_buttons
    }

    pub(super) fn folder_pane_runtime_state_mut(
        &mut self,
        pane: FolderPaneIdModel,
    ) -> &mut FolderPaneRuntimeState {
        match pane {
            FolderPaneIdModel::Upper => &mut self.upper_folder_pane,
            FolderPaneIdModel::Lower => &mut self.lower_folder_pane,
        }
    }

    pub(super) fn folder_pane_runtime_state(
        &self,
        pane: FolderPaneIdModel,
    ) -> &FolderPaneRuntimeState {
        match pane {
            FolderPaneIdModel::Upper => &self.upper_folder_pane,
            FolderPaneIdModel::Lower => &self.lower_folder_pane,
        }
    }
}

fn sync_cached_browser_row_selection(
    rows: &mut [CachedBrowserRow],
    model: &AppModel,
    window_start: usize,
) {
    let selected_visible_row = model.browser.selected_visible_row;
    let model_rows = model.browser.rows.as_slice();
    let window_end = window_start
        .saturating_add(rows.len())
        .min(model_rows.len());
    if window_end.saturating_sub(window_start) == rows.len() {
        for (row, model_row) in rows.iter_mut().zip(&model_rows[window_start..window_end]) {
            row.selected = model_row.selected || selected_visible_row == Some(row.visible_row);
        }
        return;
    }
    for row in rows {
        let selected = selected_visible_row == Some(row.visible_row);
        row.selected = selected;
    }
}
