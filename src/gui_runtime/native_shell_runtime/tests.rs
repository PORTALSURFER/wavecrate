use super::*;

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
        };

        let compat: radiant::gui_runtime::NativeRunOptions = options.into();

        assert_eq!(compat.title, "Sempal test host");
        assert_eq!(compat.inner_size, Some([1280.0, 720.0]));
        assert_eq!(compat.min_inner_size, Some([640.0, 360.0]));
        assert!(compat.maximized);
        assert!(!compat.decorations);
        assert_eq!(compat.target_fps, 90);
        let icon = compat.icon.expect("icon should be forwarded");
        assert_eq!(icon.rgba, vec![255, 0, 0, 255]);
        assert_eq!(icon.width, 1);
        assert_eq!(icon.height, 1);
    }

    #[test]
    /// Guard the Sempal launch path against regressing to a local native Vello runtime module.
    fn sempal_runtime_glue_launches_through_generic_radiant_runtime() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let adapter = native_shell_runtime_sources(manifest_dir);
        let public_runtime =
            fs::read_to_string(manifest_dir.join("src/gui_runtime/mod.rs")).expect("runtime mod");
        let removed_runtime_module = format!("mod {}{};", "native_", "vello");

        assert!(
            adapter.contains("crate::app_core::native_shell::runtime_contract")
                && adapter.contains("run_native_vello_runtime_with_artifacts")
                && adapter.contains("SempalRuntimeBridge::new(bridge)")
                && adapter.contains("local_automation_snapshot_from_native_shell")
                && adapter
                    .contains("crate::app_core::native_shell::runtime_contract::capture_native_shell_shot_snapshot"),
            "Sempal compatibility conversion, generic runtime launch, automation, and shot snapshots should stay in the runtime adapter"
        );
        assert!(
            !adapter.contains(&format!("{}{}", "radiant::runtime_contract::", "legacy_shell"))
                && !adapter.contains(&format!(
                    "{}{}",
                    "run_legacy_native_vello_", "app_with_artifacts"
                )),
            "Sempal runtime glue must not route through a legacy-shell facade or local legacy runner"
        );
        assert!(
            !public_runtime.contains(&removed_runtime_module),
            "Sempal runtime module tree must not include the removed local native Vello runner"
        );
        assert!(
            public_runtime.contains("Sempal GUI runtime host integration")
                && public_runtime.contains("Product shell composition, automation snapshots")
                && public_runtime.contains("Launching Sempal native Vello runtime"),
            "runtime boundary docs and logs should describe Sempal-owned compatibility glue, not a Radiant legacy runtime"
        );
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
                WidgetOutput::Canvas(CanvasMessage::Input {
                    input: WidgetInput::PointerPress {
                        position: radiant::gui::types::Point::new(4.0, 5.0),
                        button: radiant::widgets::PointerButton::Primary,
                    },
                }),
            )
            .expect("generic canvas should map input into a Sempal action");
        assert!(matches!(
            message,
            SempalRuntimeMessage::RetainedInput(RetainedCanvasInput::PointerPress { .. })
        ));
        assert!(bridge.update(message).requests_repaint());
        assert_ne!(bridge.inner.reduced, vec![UiAction::HandleEscape]);
        assert_eq!(bridge.inner.reduced, vec![UiAction::ToggleTransport]);
        bridge.inner.reduced.clear();

        let hover_message = SempalRuntimeMessage::RetainedInput(RetainedCanvasInput::PointerMove {
            position: radiant::gui::types::Point::new(12.0, 16.0),
        });
        assert!(
            bridge.update(hover_message).requests_repaint(),
            "retained hover moves should repaint even when Sempal classifies the hover as a local overlay update"
        );

        bridge.update(SempalRuntimeMessage::RetainedInput(
            RetainedCanvasInput::FocusChanged(true),
        ));
        bridge.update(SempalRuntimeMessage::Action(UiAction::FocusBrowserSearch));
        bridge.inner.reduced.clear();
        assert!(
            bridge
                .update(SempalRuntimeMessage::RetainedInput(
                    RetainedCanvasInput::Character('k')
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
    /// Retained canvas frames include local overlays that do not change the app projection.
    fn retained_shell_render_includes_hover_and_playhead_overlays() {
        let repaint_installed = Arc::new(AtomicBool::new(false));
        let mut model = runtime_contract::AppModel::default();
        model.browser.rows.push(runtime_contract::BrowserRowModel::new(
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
                    RetainedCanvasInput::PointerMove {
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
    fn sempal_root_dependency_no_longer_enables_radiant_legacy_shell() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cargo = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("root manifest");
        let adapter = native_shell_runtime_sources(manifest_dir);

        assert!(
            cargo.contains("radiant = { path = \"vendor/radiant\" }")
                && !cargo.contains("features = [\"legacy-shell\"]"),
            "Sempal should consume Radiant without the legacy-shell feature after OPT-277"
        );
        assert!(
            adapter.contains(
                "impl<B: NativeAppBridge> RuntimeBridge<SempalRuntimeMessage> for SempalRuntimeBridge<B>"
            ) && adapter.contains("fn resolve_key_press(")
                && adapter.contains("fn install_repaint_signal(")
                && adapter.contains("fn on_runtime_exit("),
            "Sempal should own a generic Radiant RuntimeBridge adapter for shortcut, repaint, and exit routing"
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

    fn native_shell_runtime_sources(manifest_dir: &Path) -> String {
        let mut sources =
            fs::read_to_string(manifest_dir.join("src/gui_runtime/native_shell_runtime.rs"))
                .expect("native shell runtime facade");
        for path in [
            "src/gui_runtime/native_shell_runtime/action_mapping.rs",
            "src/gui_runtime/native_shell_runtime/automation.rs",
            "src/gui_runtime/native_shell_runtime/bridge.rs",
            "src/gui_runtime/native_shell_runtime/input_routing.rs",
            "src/gui_runtime/native_shell_runtime/launch.rs",
            "src/gui_runtime/native_shell_runtime/model_mapping.rs",
        ] {
            sources.push('\n');
            sources.push_str(&fs::read_to_string(manifest_dir.join(path)).expect(path));
        }
        sources
    }
}

