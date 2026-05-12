use super::*;

#[allow(clippy::module_inception)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::repaint::RepaintSignal;
    use radiant::runtime::PaintPrimitive;
    use radiant::theme::ThemeTokens;
    use radiant::widgets::{CanvasMessage, WidgetInput, WidgetOutput};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::{fs, path::Path};

    #[test]
    fn native_run_options_map_field_for_field_to_radiant_runtime_options() {
        let options = NativeRunOptions {
            title: String::from("Sempal test host"),
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

        assert_eq!(compat.title, "Sempal test host");
        assert_eq!(compat.inner_size, Some([1280.0, 720.0]));
        assert_eq!(compat.min_inner_size, Some([640.0, 360.0]));
        assert!(compat.maximized);
        assert!(!compat.decorations);
        assert_eq!(compat.target_fps, 90);
        assert!(compat.debug_layout);
        assert!(compat.drag_and_drop);
        assert_eq!(
            compat.gpu,
            radiant::gui_runtime::NativeGpuOptions::default()
        );
        assert_eq!(
            compat.text,
            radiant::gui_runtime::NativeTextOptions::default()
        );
        let icon = compat.icon.expect("icon should be forwarded");
        assert_eq!(icon.rgba, vec![255, 0, 0, 255]);
        assert_eq!(icon.width, 1);
        assert_eq!(icon.height, 1);
    }

    #[test]
    fn sempal_generic_runtime_bridge_routes_messages_repaint_exit_and_snapshots() {
        let repaint_installed = Arc::new(AtomicBool::new(false));
        let mut bridge = SempalRuntimeBridge::new(RecordingBridge {
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
            "Sempal retained shell overlays must opt out of Radiant full-frame cache hits"
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
            .expect("generic runtime should ask Sempal for real retained shell paint");
        assert!(
            frame.primitives.len() > 8 && !frame.text_runs.is_empty(),
            "retained shell paint should contain the real Sempal frame, not a placeholder canvas"
        );

        let message = surface
            .dispatch_widget_output(
                1,
                WidgetOutput::typed(CanvasMessage::Input {
                    input: WidgetInput::PointerPress {
                        position: radiant::gui::types::Point::new(4.0, 5.0),
                        button: radiant::widgets::PointerButton::Primary,
                    },
                }),
            )
            .expect("generic canvas should map input into a Sempal action");
        assert!(matches!(
            message,
            SempalRuntimeMessage::RetainedInput(WidgetInput::PointerPress { .. })
        ));
        assert!(bridge.update(message).requests_repaint());
        assert_ne!(bridge.inner.reduced, vec![UiAction::HandleEscape]);
        assert_eq!(bridge.inner.reduced, vec![UiAction::ToggleTransport]);
        bridge.inner.reduced.clear();

        let hover_message = SempalRuntimeMessage::RetainedInput(WidgetInput::PointerMove {
            position: radiant::gui::types::Point::new(12.0, 16.0),
        });
        assert!(
            bridge.update(hover_message).requests_repaint(),
            "retained hover moves should repaint even when Sempal classifies the hover as a local overlay update"
        );

        bridge.update(SempalRuntimeMessage::RetainedInput(
            WidgetInput::FocusChanged(true),
        ));
        bridge.update(SempalRuntimeMessage::Action(UiAction::FocusBrowserSearch));
        bridge.inner.reduced.clear();
        assert!(
            bridge
                .update(SempalRuntimeMessage::RetainedInput(WidgetInput::Character(
                    'k'
                )))
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

        bridge.update(SempalRuntimeMessage::Action(UiAction::HandleEscape));
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
    }

    #[test]
    fn focused_browser_pill_editor_shields_typing_from_shortcuts_and_commits_commas() {
        let mut bridge = SempalRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        bridge.update(SempalRuntimeMessage::Action(
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

        bridge.update(SempalRuntimeMessage::RetainedInput(WidgetInput::Character(
            'k',
        )));
        bridge.update(SempalRuntimeMessage::RetainedInput(WidgetInput::Character(
            'i',
        )));
        bridge.update(SempalRuntimeMessage::RetainedInput(WidgetInput::Character(
            ',',
        )));
        bridge.update(SempalRuntimeMessage::RetainedInput(WidgetInput::Character(
            'h',
        )));

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
        let mut bridge = SempalRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });

        bridge.update(SempalRuntimeMessage::Action(UiAction::FocusBrowserSearch));
        bridge.inner.reduced.clear();
        bridge.update(SempalRuntimeMessage::RetainedInput(WidgetInput::TextEdit(
            TextEditCommand::InsertText(String::from("kick")),
        )));
        bridge.update(SempalRuntimeMessage::RetainedInput(WidgetInput::TextEdit(
            TextEditCommand::SelectAll,
        )));
        bridge.update(SempalRuntimeMessage::RetainedInput(WidgetInput::TextEdit(
            TextEditCommand::InsertText(String::from("snare")),
        )));

        assert_eq!(
            bridge.inner.reduced.last(),
            Some(&UiAction::SetBrowserSearch {
                query: String::from("snare"),
            })
        );
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
        let mut bridge = SempalRuntimeBridge::new(RecordingBridge {
            model: Arc::new(model),
            reduced: Vec::new(),
            repaint_installed: Arc::new(AtomicBool::new(false)),
            exit_status: None,
        });
        bridge.update(SempalRuntimeMessage::Action(
            UiAction::FocusBrowserTagSidebarInput,
        ));
        bridge.update(SempalRuntimeMessage::LocalTextEdit);
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
        bridge.update(SempalRuntimeMessage::RetainedInput(WidgetInput::KeyPress(
            WidgetKey::Backspace,
        )));

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

        bridge.update(SempalRuntimeMessage::RetainedInput(WidgetInput::KeyPress(
            WidgetKey::Backspace,
        )));
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

        let mut bridge = SempalRuntimeBridge::new(RecordingBridge {
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
                .update(SempalRuntimeMessage::RetainedInput(
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
    fn retained_shell_animation_refresh_uses_motion_only_projection() {
        let mut app_model = runtime_contract::AppModel::default();
        app_model.transport_running = true;
        app_model.waveform.playhead_milli = Some(100);
        app_model.waveform.playhead_micros = Some(100_000);
        let native_model: NativeAppModel = app_model.clone().into();

        let mut motion_model = runtime_contract::NativeMotionModel::from_app_model(&app_model);
        motion_model.waveform_playhead_milli = Some(450);
        motion_model.waveform_playhead_micros = Some(450_000);

        let mut bridge = SempalRuntimeBridge::new(MotionOnlyRecordingBridge {
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
        let mut bridge = SempalRuntimeBridge::new(MotionOnlyRecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            motion_model: None,
            model_pull_count: 0,
            motion_pull_count: 0,
        });

        let _ = bridge.project_surface();
        assert_eq!(bridge.inner.model_pull_count, 1);
        assert!(
            bridge
                .update(SempalRuntimeMessage::RetainedInput(
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
        let mut bridge = SempalRuntimeBridge::new(MotionOnlyRecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            motion_model: None,
            model_pull_count: 0,
            motion_pull_count: 0,
        });

        assert!(
            bridge
                .update(SempalRuntimeMessage::RetainedInput(
                    WidgetInput::PointerPress {
                        position: radiant::gui::types::Point::new(4.0, 5.0),
                        button: radiant::widgets::PointerButton::Primary,
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
        let mut bridge = SempalRuntimeBridge::new(RecordingBridge {
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
    fn sempal_root_dependency_uses_default_radiant_package() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cargo = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("root manifest");

        assert!(
            cargo.contains("radiant = { path = \"vendor/radiant\" }"),
            "Sempal should consume the local Radiant package directly"
        );
    }

    struct RecordingBridge {
        model: Arc<NativeAppModel>,
        reduced: Vec<UiAction>,
        repaint_installed: Arc<AtomicBool>,
        exit_status: Option<String>,
    }

    impl NativeAppBridge for RecordingBridge {
        fn project_model(&mut self) -> Arc<NativeAppModel> {
            Arc::clone(&self.model)
        }

        fn reduce_action(&mut self, action: UiAction) {
            match &action {
                UiAction::SetBrowserSearch { query } => {
                    Arc::make_mut(&mut self.model).browser.search_query = query.clone();
                }
                UiAction::SetBrowserTagSidebarInput { value } => {
                    Arc::make_mut(&mut self.model)
                        .browser
                        .tag_sidebar
                        .input_value = value.clone();
                }
                UiAction::CommitBrowserTagSidebarInput => {
                    Arc::make_mut(&mut self.model)
                        .browser
                        .tag_sidebar
                        .input_value
                        .clear();
                }
                _ => {}
            }
            self.reduced.push(action);
        }

        fn install_repaint_signal(&mut self, _signal: Arc<dyn RepaintSignal>) {
            self.repaint_installed.store(true, Ordering::Release);
        }

        fn on_runtime_exit(&mut self) -> Option<crate::gui_runtime::NativeShutdownTimingArtifact> {
            Some(crate::gui_runtime::NativeShutdownTimingArtifact {
                status: self.exit_status.take()?,
                failure_reason: None,
                bridge_exit_flush_ms: None,
                config_persist_ms: None,
                controller_jobs_shutdown_ms: None,
                analysis_shutdown_ms: None,
                controller_shutdown_ms: None,
                runtime_exit_total_ms: None,
            })
        }
    }

    struct MotionOnlyRecordingBridge {
        model: Arc<NativeAppModel>,
        motion_model: Option<NativeMotionModel>,
        model_pull_count: usize,
        motion_pull_count: usize,
    }

    impl NativeAppBridge for MotionOnlyRecordingBridge {
        fn project_model(&mut self) -> Arc<NativeAppModel> {
            self.model_pull_count += 1;
            Arc::clone(&self.model)
        }

        fn pull_model_arc(&mut self) -> Arc<NativeAppModel> {
            self.project_model()
        }

        fn pull_motion_model(&mut self) -> Option<NativeMotionModel> {
            self.motion_pull_count += 1;
            self.motion_model.clone()
        }
    }

    struct TestRepaintSignal;

    impl RepaintSignal for TestRepaintSignal {
        fn request_repaint(&self) {}
    }

    /// Return the retained shell descriptor projected by the bridge surface.
    fn retained_shell_descriptor(
        bridge: &mut SempalRuntimeBridge<RecordingBridge>,
    ) -> RetainedSurfaceDescriptor {
        let surface = bridge.project_surface();
        let layout = radiant::layout::layout_tree(
            &surface.layout_node(),
            radiant::gui::types::Rect::from_min_size(
                radiant::gui::types::Point::new(0.0, 0.0),
                radiant::gui::types::Vector2::new(1280.0, 720.0),
            ),
        );
        let plan = surface.paint_plan(&layout, &ThemeTokens::default());
        plan.primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::CustomSurface(surface) => surface.retained,
                _ => None,
            })
            .expect("generic bridge should project retained shell metadata")
    }

    /// Return the retained shell descriptor projected by a motion-only bridge.
    fn retained_motion_descriptor(
        bridge: &mut SempalRuntimeBridge<MotionOnlyRecordingBridge>,
    ) -> RetainedSurfaceDescriptor {
        let surface = bridge.project_surface();
        let layout = radiant::layout::layout_tree(
            &surface.layout_node(),
            radiant::gui::types::Rect::from_min_size(
                radiant::gui::types::Point::new(0.0, 0.0),
                radiant::gui::types::Vector2::new(1280.0, 720.0),
            ),
        );
        let plan = surface.paint_plan(&layout, &ThemeTokens::default());
        plan.primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::CustomSurface(surface) => surface.retained,
                _ => None,
            })
            .expect("generic bridge should project retained shell metadata")
    }

    /// Return whether a frame contains the narrow waveform playhead marker.
    fn frame_contains_playhead_marker(
        frame: &PaintFrame,
        layout: &ShellLayout,
        style: &StyleTokens,
    ) -> bool {
        frame.primitives.iter().any(|primitive| match primitive {
            crate::gui::paint::Primitive::Rect(rect) => {
                rect.color == style.accent_copper
                    && rect.rect.min.x >= layout.waveform_plot.min.x
                    && rect.rect.max.x <= layout.waveform_plot.max.x
                    && rect.rect.min.y >= layout.waveform_plot.min.y
                    && rect.rect.max.y <= layout.waveform_plot.max.y
                    && rect.rect.width() <= (style.sizing.border_width * 2.0).max(2.0)
            }
            _ => false,
        })
    }
}
