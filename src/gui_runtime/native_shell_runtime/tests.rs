use super::*;

#[allow(clippy::module_inception)]
#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::PaintPrimitive;
    use radiant::theme::{DpiScale, ThemeTokens};
    use radiant::widgets::{CanvasMessage, WidgetInput, WidgetOutput};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    mod support;
    use support::*;

    #[test]
    fn native_run_options_map_field_for_field_to_radiant_runtime_options() {
        let options = NativeRunOptions {
            title: String::from("Wavecrate test host"),
            inner_size: Some([1280.0, 720.0]),
            min_inner_size: Some([640.0, 360.0]),
            maximized: true,
            decorations: false,
            icon: Some(WindowIconRgba {
                rgba: vec![255, 0, 0, 255],
                width: 1,
                height: 1,
            }),
            target_fps: 90,
            debug_layout: true,
        };

        let compat: radiant::gui_runtime::NativeRunOptions = options.into();

        assert_eq!(compat.window.title, "Wavecrate test host");
        assert_eq!(compat.window.geometry.inner_size, Some([1280.0, 720.0]));
        assert_eq!(compat.window.geometry.min_inner_size, Some([640.0, 360.0]));
        assert!(compat.window.behavior.maximized);
        assert!(!compat.window.behavior.decorations);
        assert_eq!(compat.frame.target_fps, 90);
        assert!(compat.frame.debug_layout);
        assert!(compat.window.behavior.drag_and_drop);
        assert_eq!(
            compat.gpu,
            radiant::gui_runtime::NativeGpuOptions::default()
        );
        assert_eq!(
            compat.text.font_paths,
            vec![crate::gui_runtime::wavecrate_ui_font_path()]
        );
        assert!(compat.text.font_paths[0].exists());
        let icon = compat.window.icon.expect("icon should be forwarded");
        assert_eq!(icon.rgba, vec![255, 0, 0, 255]);
        assert_eq!(icon.width, 1);
        assert_eq!(icon.height, 1);
    }

    #[test]
    fn wavecrate_generic_runtime_bridge_routes_messages_repaint_exit_and_snapshots() {
        let repaint_installed = Arc::new(AtomicBool::new(false));
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::clone(&repaint_installed),
            exit_status: Some(String::from("clean")),
        });

        let surface = bridge.project_surface();
        let layout = radiant::layout::layout_tree(
            &surface.layout_node(),
            radiant::gui::types::Rect::from_min_size(
                radiant::gui::types::Point::new(0.0, 0.0),
                radiant::gui::types::Vector2::new(1280.0, 720.0),
            ),
        );
        let plan = surface.paint_plan(&layout, &ThemeTokens::default());
        let retained = plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::CustomSurface(surface) => surface.retained,
                _ => None,
            });
        let retained = retained.expect("generic bridge should project retained shell metadata");
        assert_eq!(
            retained.dirty_mask,
            u64::from(crate::app_core::actions::NativeDirtySegments::all().bits())
        );
        assert!(
            retained.volatile,
            "Wavecrate retained shell overlays must opt out of Radiant full-frame cache hits"
        );
        let frame = bridge
            .render_retained_surface(
                retained,
                radiant::gui::types::Rect::from_min_size(
                    radiant::gui::types::Point::new(0.0, 0.0),
                    radiant::gui::types::Vector2::new(1280.0, 720.0),
                ),
                radiant::gui::types::Vector2::new(1280.0, 720.0),
            )
            .expect("generic runtime should ask Wavecrate for real retained shell paint");
        assert!(
            frame.primitives.len() > 8 && !frame.text_runs.is_empty(),
            "retained shell paint should contain the real Wavecrate frame, not a placeholder canvas"
        );

        let message = surface
            .dispatch_widget_output(
                1,
                WidgetOutput::typed(CanvasMessage::Input {
                    input: WidgetInput::PointerPress {
                        position: radiant::gui::types::Point::new(4.0, 5.0),
                        button: radiant::widgets::PointerButton::Primary,
                        modifiers: Default::default(),
                    },
                }),
            )
            .expect("generic canvas should map input into a Wavecrate action");
        assert!(matches!(
            message,
            WavecrateRuntimeMessage::RetainedInput(WidgetInput::PointerPress { .. })
        ));
        assert!(bridge.update(message).requests_repaint());
        assert_ne!(bridge.inner.reduced, vec![UiAction::HandleEscape]);
        assert_eq!(bridge.inner.reduced, vec![UiAction::ToggleTransport]);
        bridge.inner.reduced.clear();

        let hover_message = WavecrateRuntimeMessage::RetainedInput(WidgetInput::PointerMove {
            position: radiant::gui::types::Point::new(12.0, 16.0),
        });
        assert!(
            bridge.update(hover_message).requests_repaint(),
            "retained hover moves should repaint even when Wavecrate classifies the hover as a local overlay update"
        );

        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::FocusChanged(true),
        ));
        bridge.update(WavecrateRuntimeMessage::Action(
            UiAction::FocusBrowserSearch,
        ));
        bridge.inner.reduced.clear();
        assert!(
            bridge
                .update(WavecrateRuntimeMessage::RetainedInput(
                    WidgetInput::Character('k')
                ))
                .requests_repaint()
        );
        assert_eq!(
            bridge.inner.reduced,
            vec![UiAction::SetBrowserSearch {
                query: String::from("k")
            }]
        );

        bridge.install_repaint_signal(Arc::new(TestRepaintSignal));
        assert!(repaint_installed.load(Ordering::Acquire));

        let exit = bridge.on_runtime_exit().expect("shutdown artifact");
        assert_eq!(exit["status"], "clean");

        let snapshot = bridge.capture_gui_automation_snapshot([1280.0, 720.0]);
        assert_eq!(snapshot.root.id.0, "shell.root");

        bridge.update(WavecrateRuntimeMessage::Action(UiAction::HandleEscape));
        let shortcut = bridge.resolve_key_press(
            None,
            RadiantKeyPress {
                key: radiant::gui::input::KeyCode::G,
                command: false,
                shift: false,
                alt: false,
            },
            RadiantFocusSurface::None,
        );
        assert!(shortcut.handled);
        assert_eq!(
            shortcut.pending_chord,
            Some(RadiantKeyPress {
                key: radiant::gui::input::KeyCode::G,
                command: false,
                shift: false,
                alt: false,
            })
        );

        let shortcut = bridge.resolve_key_press(
            None,
            RadiantKeyPress {
                key: radiant::gui::input::KeyCode::Space,
                command: false,
                shift: false,
                alt: false,
            },
            RadiantFocusSurface::None,
        );
        assert!(shortcut.handled);
        assert_eq!(
            shortcut.action,
            Some(WavecrateRuntimeMessage::Action(UiAction::PlayFromStart))
        );
    }

    #[test]
    fn retained_auxiliary_drag_pans_zoomed_waveform_view() {
        let repaint_installed = Arc::new(AtomicBool::new(false));
        let mut model = NativeAppModel::default();
        model.waveform.view_start_micros = 250_000;
        model.waveform.view_end_micros = 500_000;
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(model),
            reduced: Vec::new(),
            repaint_installed,
            exit_status: None,
        });
        let _ = bridge.project_surface();
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let anchor = layout.waveform_plot.center();

        assert!(
            bridge
                .update(WavecrateRuntimeMessage::RetainedInput(
                    WidgetInput::PointerPress {
                        position: anchor,
                        button: PointerButton::Auxiliary,
                        modifiers: Default::default(),
                    }
                ))
                .requests_repaint()
        );
        assert!(
            bridge
                .update(WavecrateRuntimeMessage::RetainedInput(
                    WidgetInput::PointerMove {
                        position: Point::new(
                            anchor.x - layout.waveform_plot.width() * 0.25,
                            anchor.y
                        ),
                    }
                ))
                .requests_repaint()
        );

        assert!(matches!(
            bridge.inner.reduced.last(),
            Some(UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos: None,
            }) if *center_micros > 375_000
        ));
    }

    #[test]
    fn focused_browser_pill_editor_shields_typing_from_shortcuts_and_commits_commas() {
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        bridge.update(WavecrateRuntimeMessage::Action(
            UiAction::FocusBrowserTagSidebarInput,
        ));

        let shortcut = bridge.resolve_key_press(
            Some(RadiantKeyPress {
                key: RadiantKeyCode::G,
                command: false,
                shift: false,
                alt: false,
            }),
            RadiantKeyPress {
                key: RadiantKeyCode::G,
                command: false,
                shift: false,
                alt: false,
            },
            RadiantFocusSurface::None,
        );
        assert!(
            !shortcut.handled && shortcut.action.is_none() && shortcut.pending_chord.is_none(),
            "focused tag text entry should clear pending chords without consuming printable text"
        );

        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::Character('k'),
        ));
        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::Character('i'),
        ));
        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::Character(','),
        ));
        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::Character('h'),
        ));

        assert_eq!(
            bridge.inner.reduced,
            vec![
                UiAction::FocusBrowserTagSidebarInput,
                UiAction::SetBrowserTagSidebarInput {
                    value: String::from("k"),
                },
                UiAction::SetBrowserTagSidebarInput {
                    value: String::from("ki"),
                },
                UiAction::SetBrowserTagSidebarInput {
                    value: String::from("ki"),
                },
                UiAction::CommitBrowserTagSidebarInput,
                UiAction::SetBrowserTagSidebarInput {
                    value: String::from("h"),
                },
            ]
        );
    }

    #[test]
    fn retained_text_edit_commands_preserve_selection_and_paste_flow() {
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });

        bridge.update(WavecrateRuntimeMessage::Action(
            UiAction::FocusBrowserSearch,
        ));
        bridge.inner.reduced.clear();
        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::TextEdit(TextEditCommand::InsertText(String::from("kick"))),
        ));
        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::TextEdit(TextEditCommand::SelectAll),
        ));
        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::TextEdit(TextEditCommand::InsertText(String::from("snare"))),
        ));

        assert_eq!(
            bridge.inner.reduced.last(),
            Some(&UiAction::SetBrowserSearch {
                query: String::from("snare"),
            })
        );
    }

    #[test]
    fn delete_key_routes_to_focused_browser_or_folder_when_no_text_target_is_active() {
        let mut model = runtime_contract::AppModel {
            focus_context: runtime_contract::FocusContextModel::ContentList,
            ..runtime_contract::AppModel::default()
        };
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        bridge.set_retained_model_for_tests(model.clone());

        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::TextEdit(TextEditCommand::Delete),
        ));
        assert_eq!(
            bridge.inner.reduced.last(),
            Some(&UiAction::DeleteBrowserSelection)
        );

        model.focus_context = runtime_contract::FocusContextModel::NavigationTree;
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        bridge.set_retained_model_for_tests(model);
        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::TextEdit(TextEditCommand::Delete),
        ));

        assert_eq!(
            bridge.inner.reduced.last(),
            Some(&UiAction::DeleteFocusedFolder)
        );
    }

    #[test]
    fn folder_tree_f2_resolves_to_folder_rename_action() {
        let mut model = runtime_contract::AppModel {
            focus_context: runtime_contract::FocusContextModel::NavigationTree,
            ..runtime_contract::AppModel::default()
        };
        model.sources.focused_tree_row = Some(1);
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        bridge.set_retained_model_for_tests(model);

        let shortcut = bridge.resolve_key_press(
            None,
            RadiantKeyPress {
                key: RadiantKeyCode::F2,
                command: false,
                shift: false,
                alt: false,
            },
            RadiantFocusSurface::None,
        );

        assert!(shortcut.handled);
        assert_eq!(
            shortcut.action,
            Some(WavecrateRuntimeMessage::Action(UiAction::StartFolderRename))
        );
    }

    #[test]
    fn starting_folder_rename_syncs_retained_text_from_inline_input_value() {
        let mut model = runtime_contract::AppModel::default();
        model.sources.tree_rows.push(
            runtime_contract::FolderRowModel::rename_draft(1, "drums", "Folder name", None, true)
                .with_select_all_on_focus(true),
        );
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        bridge.set_retained_model_for_tests(model);

        assert_eq!(
            bridge.text_target_value_after_action_for_tests(&UiAction::StartFolderRename),
            Some(String::from("drums"))
        );
    }

    #[test]
    fn visible_prompt_accepts_yes_and_cancels_no_without_text_input() {
        let mut model = runtime_contract::AppModel::default();
        model.confirm_prompt.visible = true;
        model.confirm_prompt.input_value = None;
        let mut yes_bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        yes_bridge.set_retained_model_for_tests(model.clone());

        yes_bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::Character('Y'),
        ));
        assert_eq!(yes_bridge.inner.reduced, vec![UiAction::ConfirmPrompt]);

        let mut no_bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        no_bridge.set_retained_model_for_tests(model);
        no_bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::Character('n'),
        ));
        assert_eq!(no_bridge.inner.reduced, vec![UiAction::CancelPrompt]);
    }

    #[test]
    fn focused_browser_pill_editor_selection_deletes_draft_before_chip_backspace() {
        let mut model = NativeAppModel::default();
        model.browser.tag_sidebar.input_value = String::from("abc");
        model
            .browser
            .tag_sidebar
            .accepted_pills
            .push(BrowserTagPillModel {
                id: String::from("Kick"),
                label: String::from("Kick"),
                state: crate::app_core::actions::NativeBrowserTagState::On,
            });
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(model),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        bridge.update(WavecrateRuntimeMessage::Action(
            UiAction::FocusBrowserTagSidebarInput,
        ));
        bridge.update(WavecrateRuntimeMessage::LocalTextEdit);
        let _ = bridge.resolve_key_press(
            None,
            RadiantKeyPress {
                key: RadiantKeyCode::A,
                command: true,
                shift: false,
                alt: false,
            },
            RadiantFocusSurface::None,
        );
        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::KeyPress(WidgetKey::Backspace),
        ));

        assert_eq!(
            bridge.inner.reduced.last(),
            Some(&UiAction::SetBrowserTagSidebarInput {
                value: String::new(),
            })
        );
        assert!(
            !bridge.inner.reduced.iter().any(|action| matches!(
                action,
                UiAction::ToggleBrowserSidebarNormalTag { label } if label == "Kick"
            )),
            "Backspace with selected draft text should edit text before removing an accepted chip"
        );

        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::KeyPress(WidgetKey::Backspace),
        ));
        assert!(bridge.inner.reduced.iter().any(|action| matches!(
            action,
            UiAction::ToggleBrowserSidebarNormalTag { label } if label == "Kick"
        )));
    }

    #[test]
    /// Retained canvas frames include local overlays that do not change the app projection.
    fn retained_shell_render_includes_hover_and_playhead_overlays() {
        let repaint_installed = Arc::new(AtomicBool::new(false));
        let mut model = runtime_contract::AppModel::default();
        model
            .browser
            .rows
            .push(runtime_contract::BrowserRowModel::new(
                0,
                "hovered sample",
                1,
                false,
                false,
            ));
        model.browser.visible_count = 1;
        model.waveform.playhead_milli = Some(250);
        model.waveform.playhead_micros = Some(250_000);
        model.transport_running = true;

        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(model.into()),
            reduced: Vec::new(),
            repaint_installed,
            exit_status: None,
        });
        let retained = retained_shell_descriptor(&mut bridge);
        let viewport = radiant::gui::types::Vector2::new(1280.0, 720.0);
        let rect = radiant::gui::types::Rect::from_min_size(
            radiant::gui::types::Point::new(0.0, 0.0),
            viewport,
        );
        let base_frame = bridge
            .render_retained_surface(retained, rect, viewport)
            .expect("initial retained shell frame");
        let layout = ShellLayout::build(viewport);
        let style = StyleTokens::for_viewport_width(viewport.x);
        assert!(
            frame_contains_playhead_marker(&base_frame, &layout, &style),
            "retained shell frame should include the waveform playhead motion overlay"
        );

        let hover_point = radiant::gui::types::Point::new(
            layout.browser_rows.min.x + 8.0,
            layout.browser_rows.min.y + 8.0,
        );
        assert!(
            bridge
                .update(WavecrateRuntimeMessage::RetainedInput(
                    WidgetInput::PointerMove {
                        position: hover_point,
                    },
                ))
                .requests_repaint()
        );
        let hover_frame = bridge
            .render_retained_surface(retained, rect, viewport)
            .expect("hovered retained shell frame");

        assert!(
            hover_frame.primitives.len() > base_frame.primitives.len(),
            "retained shell frame should append hover overlays after retained pointer moves"
        );
    }

    #[test]
    fn retained_shell_render_includes_clickable_options_panel_overlay() {
        let repaint_installed = Arc::new(AtomicBool::new(false));
        let mut model = runtime_contract::AppModel::default();
        model.options_panel.visible = true;
        model.paired_device.primary_group = runtime_contract::SummaryFieldModel {
            label: String::from("Output Host"),
            value_label: String::from("WASAPI"),
        };
        model.paired_device.primary_item = runtime_contract::SummaryFieldModel {
            label: String::from("Output"),
            value_label: String::from("Speakers"),
        };
        model.paired_device.primary_number = runtime_contract::SummaryFieldModel {
            label: String::from("Sample Rate"),
            value_label: String::from("48 kHz"),
        };

        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(model.into()),
            reduced: Vec::new(),
            repaint_installed,
            exit_status: None,
        });
        let retained = retained_shell_descriptor(&mut bridge);
        let viewport = radiant::gui::types::Vector2::new(1280.0, 720.0);
        let rect = radiant::gui::types::Rect::from_min_size(
            radiant::gui::types::Point::new(0.0, 0.0),
            viewport,
        );
        let frame = bridge
            .render_retained_surface(retained, rect, viewport)
            .expect("retained shell frame with options panel");
        assert!(
            !frame.text_runs.iter().any(|run| run.text == "Audio Engine"),
            "options panel should not render its old inner title"
        );
        let output_host_label = frame
            .text_runs
            .iter()
            .find(|run| run.text.starts_with("Output Host:"))
            .expect("output host picker button should render");
        let click = radiant::gui::types::Point::new(
            output_host_label.position.x + 2.0,
            output_host_label.position.y + 2.0,
        );

        bridge.update(WavecrateRuntimeMessage::RetainedInput(
            WidgetInput::PointerPress {
                position: click,
                button: radiant::widgets::PointerButton::Primary,
                modifiers: Default::default(),
            },
        ));

        assert!(
            bridge
                .inner
                .reduced
                .iter()
                .any(|action| matches!(action, UiAction::OpenAudioOutputHostPicker)),
            "clicking a rendered options-panel button should route to the audio picker action"
        );
    }

    #[test]
    fn retained_shell_animation_refresh_uses_motion_only_projection() {
        let mut app_model = runtime_contract::AppModel::default();
        app_model.transport_running = true;
        app_model.waveform.playhead_milli = Some(100);
        app_model.waveform.playhead_micros = Some(100_000);
        let native_model: NativeAppModel = app_model.clone().into();

        let mut motion_model = runtime_contract::NativeMotionModel::from_app_model(&app_model);
        motion_model.waveform_playhead_milli = Some(450);
        motion_model.waveform_playhead_micros = Some(450_000);

        let mut bridge = WavecrateRuntimeBridge::new(MotionOnlyRecordingBridge {
            model: Arc::new(native_model),
            motion_model: Some(motion_model.into()),
            model_pull_count: 0,
            motion_pull_count: 0,
        });
        let retained = retained_motion_descriptor(&mut bridge);

        assert_eq!(bridge.inner.model_pull_count, 1);
        assert!(
            bridge.needs_animation(),
            "playback motion should keep the generic runtime on animation frames"
        );
        assert_eq!(bridge.inner.motion_pull_count, 1);
        let _ = bridge.project_surface();
        assert_eq!(
            bridge.inner.model_pull_count, 1,
            "animation refreshes should not force a full app-model pull"
        );

        let viewport = radiant::gui::types::Vector2::new(1280.0, 720.0);
        let rect = radiant::gui::types::Rect::from_min_size(
            radiant::gui::types::Point::new(0.0, 0.0),
            viewport,
        );
        let frame = bridge
            .render_retained_surface(retained, rect, viewport)
            .expect("motion-only retained shell frame");
        let layout = ShellLayout::build(viewport);
        let style = StyleTokens::for_viewport_width(viewport.x);

        assert!(
            frame_contains_playhead_marker(&frame, &layout, &style),
            "motion-only refresh should still render the playhead overlay"
        );
    }

    #[test]
    fn retained_hover_refresh_skips_app_model_pull() {
        let mut bridge = WavecrateRuntimeBridge::new(MotionOnlyRecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            motion_model: None,
            model_pull_count: 0,
            motion_pull_count: 0,
        });

        let _ = bridge.project_surface();
        assert_eq!(bridge.inner.model_pull_count, 1);
        assert!(
            bridge
                .update(WavecrateRuntimeMessage::RetainedInput(
                    WidgetInput::PointerMove {
                        position: radiant::gui::types::Point::new(12.0, 16.0),
                    },
                ))
                .requests_repaint()
        );
        let _ = bridge.project_surface();

        assert_eq!(
            bridge.inner.model_pull_count, 1,
            "local retained hover refreshes should repaint overlays without pulling the app model"
        );
    }

    #[test]
    fn retained_action_refresh_reuses_emit_projection() {
        let mut bridge = WavecrateRuntimeBridge::new(MotionOnlyRecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            motion_model: None,
            model_pull_count: 0,
            motion_pull_count: 0,
        });

        assert!(
            bridge
                .update(WavecrateRuntimeMessage::RetainedInput(
                    WidgetInput::PointerPress {
                        position: radiant::gui::types::Point::new(4.0, 5.0),
                        button: radiant::widgets::PointerButton::Primary,
                        modifiers: Default::default(),
                    },
                ))
                .requests_repaint()
        );
        assert_eq!(bridge.inner.model_pull_count, 1);
        let _ = bridge.project_surface();

        assert_eq!(
            bridge.inner.model_pull_count, 1,
            "retained pointer actions should not pull a second model during repaint projection"
        );
    }

    #[test]
    fn retained_shell_render_rebuilds_only_dirty_static_segments() {
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        let viewport = radiant::gui::types::Vector2::new(1280.0, 720.0);
        let rect = radiant::gui::types::Rect::from_min_size(
            radiant::gui::types::Point::new(0.0, 0.0),
            viewport,
        );
        let initial = retained_shell_descriptor(&mut bridge);
        let _ = bridge
            .render_retained_surface(initial, rect, viewport)
            .expect("initial retained shell frame");
        let browser_rows_before = bridge
            .static_segment_frame_for_tests(StaticFrameSegment::BrowserRowsWindow)
            .clone();
        let status_before = bridge
            .static_segment_frame_for_tests(StaticFrameSegment::StatusBar)
            .clone();

        let mut model = bridge.retained_model_for_tests().clone();
        model.status_text = String::from("status changed");
        bridge.set_retained_model_for_tests(model);
        let _ = bridge
            .render_retained_surface(
                RetainedSurfaceDescriptor {
                    key: 1,
                    revision: initial.revision + 1,
                    dirty_mask: u64::from(DirtySegments::STATUS_BAR),
                    volatile: true,
                },
                rect,
                viewport,
            )
            .expect("status-only retained shell frame");

        assert_eq!(
            bridge.static_segment_frame_for_tests(StaticFrameSegment::BrowserRowsWindow),
            &browser_rows_before,
            "status-only dirt should not rebuild the browser rows static segment"
        );
        assert_ne!(
            bridge.static_segment_frame_for_tests(StaticFrameSegment::StatusBar),
            &status_before,
            "status dirt should rebuild the status static segment"
        );
    }

    #[test]
    fn retained_shell_renders_against_radiant_logical_dpi_viewport() {
        let dpi_scale = DpiScale::new(2.0);
        let physical_framebuffer = radiant::gui::types::Vector2::new(1920.0, 1080.0);
        let viewport = radiant::gui::types::Vector2::new(
            dpi_scale.physical_to_logical(physical_framebuffer.x),
            dpi_scale.physical_to_logical(physical_framebuffer.y),
        );
        let rect = radiant::gui::types::Rect::from_min_size(
            radiant::gui::types::Point::new(0.0, 0.0),
            viewport,
        );
        let mut bridge = WavecrateRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });

        let retained = retained_shell_descriptor(&mut bridge);
        let frame = bridge
            .render_retained_surface(retained, rect, viewport)
            .expect("retained shell frame");
        let style = StyleTokens::for_viewport_width(viewport.x);
        let layout = ShellLayout::build_with_style(viewport, &style);

        assert_eq!(layout.root.rect.max, Point::new(viewport.x, viewport.y));
        assert!((layout.ui_scale - 1.0).abs() < 0.0001);
        assert!(
            frame
                .text_runs
                .iter()
                .all(|run| run.position.x <= viewport.x && run.position.y <= viewport.y),
            "Wavecrate retained shell should paint in Radiant logical points, not physical pixels"
        );
    }

    #[test]
    fn wavecrate_root_dependency_uses_default_radiant_package() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cargo = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("root manifest");

        assert!(
            cargo.contains("radiant = { path = \"vendor/radiant\" }"),
            "Wavecrate should consume the local Radiant package directly"
        );
    }

    #[test]
    fn windows_resource_manifest_declares_per_monitor_dpi_awareness() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let rc = fs::read_to_string(manifest_dir.join("build/windows/wavecrate.rc"))
            .expect("windows resource script");
        let manifest =
            fs::read_to_string(manifest_dir.join("build/windows/wavecrate.exe.manifest"))
                .expect("windows application manifest");

        assert!(
            rc.contains("1 24 \"build\\\\windows\\\\wavecrate.exe.manifest\""),
            "Windows resources should embed the app manifest"
        );
        assert!(
            manifest.contains("PerMonitorV2, PerMonitor"),
            "Windows app manifest should opt Wavecrate into per-monitor DPI awareness"
        );
        assert!(
            manifest.contains(">true/pm</dpiAware>"),
            "Windows app manifest should retain the legacy per-monitor DPI declaration"
        );
    }

    #[test]
    fn retained_runtime_bridge_forwards_native_file_drops_to_host() {
        let mut bridge = WavecrateRuntimeBridge::new(NativeDropRecordingBridge::default());
        let dropped_path = PathBuf::from("C:/samples/kick.wav");

        let command = bridge.native_file_drop(radiant::runtime::NativeFileDrop::dropped(
            dropped_path.clone(),
            Some(Point::new(12.0, 34.0)),
            None,
        ));

        assert!(command.requests_repaint());
        assert_eq!(
            bridge.inner.events,
            vec![NativeFileDropEvent {
                phase: NativeFileDropPhase::Drop,
                path: Some(dropped_path),
                position: Some((12.0, 34.0)),
            }]
        );
    }
}
