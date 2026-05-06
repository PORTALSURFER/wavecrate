use super::*;

mod shared;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Route one left-pointer press through the production hit-testing path in tests.
    #[cfg(test)]
    pub(crate) fn handle_left_pointer_press_for_tests(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        spatial_drag_start: bool,
        action_emitted: &mut bool,
    ) -> bool {
        self.begin_pointer_press_cycle();
        self.last_cursor = Some(point);
        self.refresh_cached_model_for_pending_input();
        self.handle_left_pointer_press(layout, point, spatial_drag_start, action_emitted)
    }

    /// Route one right-pointer press through the production hit-testing path in tests.
    #[cfg(test)]
    pub(crate) fn handle_right_pointer_press_for_tests(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        action_emitted: &mut bool,
        source_menu_state_changed: &mut bool,
    ) -> bool {
        self.begin_pointer_press_cycle();
        self.last_cursor = Some(point);
        self.refresh_cached_model_for_pending_input();
        let mut browser_menu_state_changed = false;
        self.handle_right_pointer_press(
            layout,
            point,
            action_emitted,
            source_menu_state_changed,
            &mut browser_menu_state_changed,
        )
    }

    /// Route one cursor-move event through the production pointer path in tests.
    #[cfg(test)]
    pub(crate) fn handle_cursor_moved_for_tests(&mut self, point: Point) {
        self.handle_cursor_moved(point);
    }

    pub(super) fn handle_cursor_moved(&mut self, point: Point) {
        if self.last_cursor == Some(point) {
            return;
        }
        self.last_cursor = Some(point);
        self.note_cursor_activity(Instant::now());
        if self.maybe_start_content_item_drag(point) {
            return;
        }
        let session = self.active_pointer_session();
        if matches!(session, ActivePointerSession::WaveformDrag) {
            self.update_cursor_for_active_waveform_drag();
        } else {
            self.update_waveform_resize_cursor(point);
        }
        match session {
            ActivePointerSession::Volume => {
                if let Some(layout) = self.shell_layout.as_ref()
                    && let Some(action) =
                        self.shell_state
                            .top_bar_volume_drag_action(layout, &self.model, point)
                {
                    if let UiAction::SetVolume { value_milli } = action {
                        if self.last_emitted_volume_milli != Some(value_milli) {
                            self.last_emitted_volume_milli = Some(value_milli);
                            self.emit_volume_milli_immediately(value_milli);
                        }
                    } else {
                        self.emit_model_action(action);
                    }
                }
            }
            ActivePointerSession::FolderScrollbar => {
                let _ = self.process_folder_scrollbar_drag_immediately(point);
            }
            ActivePointerSession::ContentListScrollbar => {
                let _ = self.process_browser_scrollbar_drag_immediately(point);
            }
            ActivePointerSession::WaveformScrollbar => {
                let _ = self.process_waveform_scrollbar_drag_immediately(point);
            }
            ActivePointerSession::WaveformPan => {
                let _ = self.process_waveform_pan_drag_immediately(point);
            }
            ActivePointerSession::WaveformDrag => {
                let _ = self.process_waveform_drag_immediately(point);
            }
            ActivePointerSession::ContentItemDrag => {
                let _ = self.process_content_item_drag_immediately(point);
                let _ = self.maybe_launch_external_drag_session(false, false);
            }
            ActivePointerSession::SelectionDrag => {
                let _ = self.process_selection_drag_immediately(point);
                let _ = self.maybe_launch_external_drag_session(false, false);
            }
            ActivePointerSession::SpatialFocusDrag => {
                let _ = self.process_spatial_focus_drag_immediately(point);
            }
            ActivePointerSession::TextInputDrag => {
                if !self.process_text_input_drag(point) {
                    let (processed, _) = self.process_cursor_move_immediately(point);
                    if !processed {
                        self.queue_cursor(point);
                    }
                }
            }
            ActivePointerSession::Hover => {
                let (processed, _) = self.process_cursor_move_immediately(point);
                if !processed {
                    self.queue_cursor(point);
                }
            }
        }
    }

    pub(super) fn handle_mouse_pressed(&mut self, button: MouseButton) {
        if self.window.is_none() {
            return;
        }
        let Some(point) = self.last_cursor else {
            return;
        };
        let _ = self.with_shell_layout(|this, layout| {
            this.begin_pointer_press_cycle();
            let mut handled = false;
            let mut action_emitted = false;
            let mut source_menu_state_changed = false;
            let mut browser_menu_state_changed = false;
            match button {
                MouseButton::Left => {
                    this.refresh_cached_model_for_pending_input();
                    this.cancel_folder_inline_edit_for_external_pointer_target(layout, point);
                    let spatial_drag_start =
                        this.model.map.active && layout.browser_rows.contains(point);
                    if let Some(action) = this.shell_state.source_context_menu_action_at_point(
                        layout,
                        &this.model,
                        point,
                    ) {
                        this.emit_model_action(action);
                        action_emitted = true;
                        source_menu_state_changed |= this.shell_state.close_source_context_menu();
                        handled = true;
                    } else if let Some(action) = this
                        .shell_state
                        .browser_context_menu_action_at_point(layout, &this.model, point)
                    {
                        this.emit_model_action(action);
                        action_emitted = true;
                        browser_menu_state_changed |= this.shell_state.close_browser_context_menu();
                        handled = true;
                    } else {
                        source_menu_state_changed |= this.shell_state.close_source_context_menu();
                        browser_menu_state_changed |= this.shell_state.close_browser_context_menu();
                    }
                    if !handled {
                        if this.handle_folder_create_pointer_press(
                            layout,
                            point,
                            this.modifiers.shift_key(),
                        ) {
                            handled = true;
                        } else if this.handle_browser_search_pointer_press(
                            layout,
                            point,
                            this.modifiers.shift_key(),
                        ) {
                            handled = true;
                        } else if this.handle_waveform_bpm_pointer_press(
                            layout,
                            point,
                            this.modifiers.shift_key(),
                        ) {
                            handled = true;
                        }
                    }
                    if !handled {
                        handled = this.handle_left_pointer_press(
                            layout,
                            point,
                            spatial_drag_start,
                            &mut action_emitted,
                        );
                    }
                }
                MouseButton::Right => {
                    this.refresh_cached_model_for_pending_input();
                    this.cancel_folder_inline_edit_for_external_pointer_target(layout, point);
                    handled = this.handle_right_pointer_press(
                        layout,
                        point,
                        &mut action_emitted,
                        &mut source_menu_state_changed,
                        &mut browser_menu_state_changed,
                    );
                }
                MouseButton::Middle => {
                    if layout.waveform_plot.contains(point) {
                        this.begin_waveform_pan_drag(point.x);
                        handled = true;
                    }
                }
                _ => {}
            }
            if source_menu_state_changed || browser_menu_state_changed {
                this.apply_invalidation_scope(RuntimeInvalidationScope::StaticAndOverlays);
            } else if action_emitted && handled && !this.frame_state.has_pending_rebuild() {
                this.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
            }
        });
    }

    pub(super) fn handle_mouse_released(&mut self, button: MouseButton) {
        self.clear_pointer_release_state();
        self.finish_volume_drag(Some(button));
    }

    fn handle_left_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        spatial_drag_start: bool,
        action_emitted: &mut bool,
    ) -> bool {
        if self
            .shell_state
            .prompt_input_at_point(layout, &self.model, point)
        {
            self.activate_text_input_target(TextInputTarget::PromptInput);
            return true;
        }
        self.cancel_folder_inline_edit_for_external_pointer_target(layout, point);
        if self.handle_folder_create_pointer_press(layout, point, self.modifiers.shift_key()) {
            return true;
        }
        if self.text_input_target != TextInputTarget::None {
            self.deactivate_text_input_target();
        }
        if let Some(action) =
            self.shell_state
                .top_bar_volume_action_at_point(layout, &self.model, point)
        {
            if let UiAction::SetVolume { value_milli } = action {
                self.last_emitted_volume_milli = Some(value_milli);
                self.emit_volume_milli_immediately(value_milli);
            } else {
                self.emit_model_action(action);
            }
            *action_emitted = true;
            self.volume_drag_active = true;
            return true;
        }
        if let Some((pane, thumb_pointer_offset_y)) = self
            .shell_state
            .folder_scrollbar_thumb_offset_at_point(layout, &self.model, point)
        {
            self.begin_folder_scrollbar_drag(pane, thumb_pointer_offset_y);
            return true;
        }
        if let Some(thumb_pointer_offset_y) = self
            .shell_state
            .browser_scrollbar_thumb_offset_at_point(layout, &self.model, point)
        {
            self.begin_browser_scrollbar_drag(thumb_pointer_offset_y);
            return true;
        }
        if let Some(thumb_pointer_offset_x) = self
            .shell_state
            .waveform_scrollbar_thumb_offset_at_point(layout, &self.model, point)
        {
            let thumb_pointer_ratio_x = self
                .shell_state
                .waveform_scrollbar_thumb_ratio_at_point(layout, &self.model, point)
                .unwrap_or(0.0);
            self.begin_waveform_scrollbar_drag(thumb_pointer_offset_x, thumb_pointer_ratio_x);
            return true;
        }
        if self.process_waveform_scrollbar_track_click_immediately(point) {
            *action_emitted = true;
            return true;
        }
        if self.process_folder_scrollbar_track_click_immediately(point) {
            *action_emitted = true;
            return true;
        }
        if self.process_browser_scrollbar_track_click_immediately(point) {
            *action_emitted = true;
            return true;
        }
        // Pointer-to-waveform position mapping must use the latest visible view
        // after wheel/keyboard zoom changes, even if the next redraw has not
        // rebuilt the cached model yet.
        self.refresh_waveform_view_if_needed();
        if let Some(action) = action_from_pointer_with_motion(
            layout,
            &self.model,
            self.motion_model.as_ref(),
            &mut self.shell_state,
            point,
            self.modifiers,
        ) {
            if shared::should_emit_waveform_range_adjust_immediately(self, &action) {
                self.emit_waveform_drag_action_immediately(action);
                *action_emitted = true;
            } else {
                *action_emitted = self.handle_pointer_press_action_at_point(
                    action,
                    spatial_drag_start,
                    layout,
                    point,
                );
            }
            return true;
        }
        if self.shell_state.handle_primary_click(layout, point)
            && let Some(column) = layout.column_at_point(point)
        {
            self.emit_model_action(UiAction::SelectColumn { index: column });
            *action_emitted = true;
            return true;
        }
        false
    }

    fn cancel_folder_inline_edit_for_external_pointer_target(
        &mut self,
        layout: &ShellLayout,
        point: Point,
    ) {
        if self.text_input_target != TextInputTarget::FolderCreate
            || self
                .shell_state
                .folder_create_input_at_point(layout, &self.model, point)
        {
            return;
        }
        let action = UiAction::CancelFolderCreate;
        self.update_text_target_after_action(&action);
        self.emit_model_action(action);
        self.refresh_cached_model_for_pending_input();
    }

    fn handle_right_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        action_emitted: &mut bool,
        source_menu_state_changed: &mut bool,
        browser_menu_state_changed: &mut bool,
    ) -> bool {
        if let Some(action) =
            self.shell_state
                .source_context_menu_action_at_point(layout, &self.model, point)
        {
            self.emit_model_action(action);
            *action_emitted = true;
            *source_menu_state_changed |= self.shell_state.close_source_context_menu();
            return true;
        }
        if let Some(action) =
            self.shell_state
                .browser_context_menu_action_at_point(layout, &self.model, point)
        {
            self.emit_model_action(action);
            *action_emitted = true;
            *browser_menu_state_changed |= self.shell_state.close_browser_context_menu();
            return true;
        }
        if let Some((pane, index)) =
            self.shell_state
                .source_row_at_point(layout, &self.model, point)
        {
            self.emit_model_action(UiAction::FocusSourceRow { index });
            self.shell_state
                .open_source_context_menu_for_row(pane, index, point);
            *source_menu_state_changed = true;
            *action_emitted = true;
            return true;
        }
        *source_menu_state_changed |= self.shell_state.close_source_context_menu();
        *browser_menu_state_changed |= self.shell_state.close_browser_context_menu();
        if self.model.browser.duplicate_cleanup_active
            && let Some(visible_row) =
                self.shell_state
                    .browser_row_at_point(layout, &self.model, point)
        {
            self.emit_model_action(UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row });
            *action_emitted = true;
            return true;
        }
        if let Some(visible_row) = self
            .shell_state
            .browser_row_at_point(layout, &self.model, point)
        {
            self.shell_state
                .open_browser_context_menu_for_row(visible_row, point);
            *browser_menu_state_changed = true;
            *action_emitted = true;
            return true;
        }
        if matches!(layout.hit_test(point), Some(ShellNodeKind::WaveformCard)) {
            // Edit-selection pointer mapping shares the same waveform view bounds
            // as click-play, so refresh pending zoom/view changes before hit-testing.
            self.refresh_waveform_view_if_needed();
            if let Some(action) =
                crate::gui_runtime::native_vello::input::duplicate_cleanup_exemption_action_from_pointer(
                    layout,
                    &self.model,
                    point,
                )
            {
                self.emit_model_action(action);
                *action_emitted = true;
                return true;
            }
            let action =
                waveform_edit_action_from_pointer(layout, &self.model, point, self.modifiers);
            if shared::should_emit_waveform_range_adjust_immediately(self, &action) {
                self.emit_waveform_drag_action_immediately(action);
                *action_emitted = true;
            } else {
                *action_emitted =
                    self.handle_pointer_press_action_at_point(action, false, layout, point);
            }
            return true;
        }
        false
    }
}
