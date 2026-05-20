use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::{LayoutOutput, Vector2};
use radiant::prelude as ui;
use radiant::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintText, PaintTextAlign, PaintTextRun,
};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    FocusBehavior, PaintBounds, PointerButton, TextWrap, Widget, WidgetCommon, WidgetInput,
    WidgetOutput, WidgetSizing,
};

use super::{
    AUDIO_ENGINE_PILL_HEIGHT, AUDIO_ENGINE_PILL_ID, AUDIO_ENGINE_PILL_WIDTH,
    AUDIO_SETTINGS_POPUP_HEIGHT, AUDIO_SETTINGS_POPUP_WIDTH, GuiAppState, GuiMessage,
    VOLUME_SLIDER_HEIGHT, VOLUME_SLIDER_ID, VOLUME_SLIDER_WIDTH,
};

pub(super) fn top_status_bar(state: &GuiAppState) -> ui::View<GuiMessage> {
    ui::row([
        volume_slider(state.volume),
        ui::spacer().height(20.0).fill_width(),
        audio_engine_pill(state.audio_engine_pill_label(), state.audio_settings_open),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn audio_engine_pill(label: String, active: bool) -> ui::View<GuiMessage> {
    audio_engine_pill_with_id(label, active, AUDIO_ENGINE_PILL_ID, "top-audio-engine-pill")
}

fn audio_engine_pill_with_id(
    label: String,
    active: bool,
    id: u64,
    key: &'static str,
) -> ui::View<GuiMessage> {
    ui::custom_widget(AudioEnginePill::new(label, active), |output| {
        output.typed_ref::<GuiMessage>().cloned()
    })
    .id(id)
    .key(key)
    .size(AUDIO_ENGINE_PILL_WIDTH, AUDIO_ENGINE_PILL_HEIGHT)
}

pub(super) fn volume_slider(volume: f32) -> ui::View<GuiMessage> {
    ui::slider(volume)
        .compact()
        .message(GuiMessage::SetVolume)
        .id(VOLUME_SLIDER_ID)
        .key("top-volume-slider")
        .size(VOLUME_SLIDER_WIDTH, VOLUME_SLIDER_HEIGHT)
}

pub(super) fn audio_settings_popover(state: &GuiAppState) -> ui::View<GuiMessage> {
    let panel = ui::column(audio_settings_panel_rows(state))
        .key("audio-settings-panel")
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Strong,
        })
        .spacing(7.0)
        .padding(8.0)
        .width(AUDIO_SETTINGS_POPUP_WIDTH)
        .height(AUDIO_SETTINGS_POPUP_HEIGHT);
    ui::column(vec![
        ui::spacer().height(42.0),
        ui::row(vec![ui::spacer().fill_width(), panel])
            .padding_x(14.0)
            .fill_width()
            .height(AUDIO_SETTINGS_POPUP_HEIGHT),
        ui::spacer().fill_height(),
    ])
    .fill()
}

fn audio_settings_panel_rows(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut rows = vec![
        ui::row(vec![
            ui::text("Audio Engine").height(20.0).fill_width(),
            ui::button("x")
                .subtle()
                .message(GuiMessage::CloseAudioSettings)
                .width(24.0)
                .height(20.0),
        ])
        .fill_width()
        .height(22.0),
        ui::text(state.audio_engine_detail_label())
            .key("audio-settings-detail")
            .fill_width()
            .height(20.0)
            .truncate(),
    ];
    if let Some(error) = state.audio_settings_error.as_ref() {
        rows.push(
            ui::text(error.clone())
                .key("audio-settings-error")
                .style(ui::WidgetStyle {
                    tone: ui::WidgetTone::Danger,
                    prominence: ui::WidgetProminence::Subtle,
                })
                .fill_width()
                .height(20.0)
                .truncate(),
        );
    }
    rows.push(audio_settings_section(
        "Backend",
        audio_host_option_buttons(state),
        2,
    ));
    rows.push(audio_settings_section(
        "Output",
        audio_device_option_buttons(state),
        2,
    ));
    rows.push(audio_settings_section(
        "Sample Rate",
        audio_sample_rate_option_buttons(state),
        4,
    ));
    rows.push(cache_maintenance_section());
    rows
}

fn cache_maintenance_section() -> ui::View<GuiMessage> {
    ui::column(vec![
        ui::text("Maintenance")
            .style(ui::WidgetStyle {
                tone: ui::WidgetTone::Accent,
                prominence: ui::WidgetProminence::Subtle,
            })
            .fill_width()
            .height(18.0),
        ui::button("Clear Rebuildable Caches")
            .message(GuiMessage::ClearRebuildableCaches)
            .key("settings-clear-rebuildable-caches")
            .fill_width()
            .height(24.0),
    ])
    .spacing(3.0)
    .fill_width()
    .height(45.0)
}

fn audio_settings_section(
    label: &'static str,
    options: Vec<ui::View<GuiMessage>>,
    columns: usize,
) -> ui::View<GuiMessage> {
    let grid_height = audio_option_grid_height(options.len(), columns);
    let mut rows = vec![
        ui::text(label)
            .style(ui::WidgetStyle {
                tone: ui::WidgetTone::Accent,
                prominence: ui::WidgetProminence::Subtle,
            })
            .fill_width()
            .height(18.0),
    ];
    if options.is_empty() {
        rows.push(ui::text("Unavailable").fill_width().height(20.0));
    } else {
        rows.push(
            ui::grid(options, columns.max(1))
                .fill_width()
                .height(grid_height),
        );
    }
    ui::column(rows)
        .spacing(3.0)
        .fill_width()
        .height(21.0 + grid_height)
}

fn audio_host_option_buttons(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut buttons = vec![audio_option_button(
        "System default".to_string(),
        state.audio_output_config.host.is_none(),
        GuiMessage::SetAudioOutputHost(None),
    )];
    buttons.extend(state.audio_hosts.iter().map(|host| {
        audio_option_button(
            default_option_label(host.label.as_str(), host.is_default),
            state.audio_output_config.host.as_deref() == Some(host.id.as_str()),
            GuiMessage::SetAudioOutputHost(Some(host.id.clone())),
        )
    }));
    buttons
}

fn audio_device_option_buttons(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut buttons = vec![audio_option_button(
        "Host default".to_string(),
        state.audio_output_config.device.is_none(),
        GuiMessage::SetAudioOutputDevice(None),
    )];
    buttons.extend(state.audio_devices.iter().map(|device| {
        audio_option_button(
            default_option_label(device.name.as_str(), device.is_default),
            state.audio_output_config.device.as_deref() == Some(device.name.as_str()),
            GuiMessage::SetAudioOutputDevice(Some(device.name.clone())),
        )
    }));
    buttons
}

fn audio_sample_rate_option_buttons(state: &GuiAppState) -> Vec<ui::View<GuiMessage>> {
    let mut buttons = vec![audio_option_button(
        "Device default".to_string(),
        state.audio_output_config.sample_rate.is_none(),
        GuiMessage::SetAudioOutputSampleRate(None),
    )];
    buttons.extend(state.audio_sample_rates.iter().copied().map(|rate| {
        audio_option_button(
            format_sample_rate_label(rate),
            state.audio_output_config.sample_rate == Some(rate),
            GuiMessage::SetAudioOutputSampleRate(Some(rate)),
        )
    }));
    buttons
}

fn audio_option_button(label: String, selected: bool, message: GuiMessage) -> ui::View<GuiMessage> {
    ui::button(label)
        .style(ui::WidgetStyle {
            tone: if selected {
                ui::WidgetTone::Accent
            } else {
                ui::WidgetTone::Neutral
            },
            prominence: if selected {
                ui::WidgetProminence::Strong
            } else {
                ui::WidgetProminence::Subtle
            },
        })
        .message(message)
        .fill_width()
        .height(20.0)
}

fn default_option_label(label: &str, is_default: bool) -> String {
    if is_default {
        format!("{label} (default)")
    } else {
        label.to_string()
    }
}

fn audio_option_grid_height(option_count: usize, columns: usize) -> f32 {
    let columns = columns.max(1);
    let rows = option_count.max(1).div_ceil(columns);
    rows as f32 * 20.0 + rows.saturating_sub(1) as f32 * 4.0
}

pub(super) fn format_sample_rate_label(sample_rate: u32) -> String {
    if sample_rate >= 1000 && sample_rate.is_multiple_of(1000) {
        format!("{} kHz", sample_rate / 1000)
    } else if sample_rate >= 1000 {
        format!("{:.1} kHz", sample_rate as f32 / 1000.0)
    } else {
        format!("{sample_rate} Hz")
    }
}

#[derive(Clone, Debug)]
pub(super) struct AudioEnginePill {
    common: WidgetCommon,
    label: String,
}

impl AudioEnginePill {
    pub(super) fn new(label: String, active: bool) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(
                AUDIO_ENGINE_PILL_WIDTH,
                AUDIO_ENGINE_PILL_HEIGHT,
            )),
        );
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_state_layers = false;
        common.state.active = active;
        Self { common, label }
    }
}

impl Widget for AudioEnginePill {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let activated = self.common.state.pressed && bounds.contains(position);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                activated.then(|| WidgetOutput::typed(GuiMessage::ToggleAudioSettings))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                if !focused {
                    self.common.state.pressed = false;
                }
                None
            }
            WidgetInput::KeyPress(key) if self.common.state.focused => match key {
                radiant::widgets::WidgetKey::Enter | radiant::widgets::WidgetKey::Space => {
                    Some(WidgetOutput::typed(GuiMessage::ToggleAudioSettings))
                }
                _ => None,
            },
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                }
                None
            }
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let hovered_or_pressed = self.common.state.hovered || self.common.state.pressed;
        let fill = if self.common.state.active {
            Rgba8 {
                r: 50,
                g: 54,
                b: 56,
                a: 245,
            }
        } else if hovered_or_pressed {
            Rgba8 {
                r: 42,
                g: 43,
                b: 44,
                a: 245,
            }
        } else {
            Rgba8 {
                r: 31,
                g: 32,
                b: 33,
                a: 235,
            }
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: fill,
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(bounds.min.x + 0.5, bounds.min.y + 0.5),
                Point::new(bounds.max.x - 0.5, bounds.max.y - 0.5),
            ),
            color: Rgba8 {
                r: 78,
                g: 79,
                b: 80,
                a: if hovered_or_pressed { 230 } else { 165 },
            },
            width: 1.0,
        }));
        if self.common.state.focused {
            primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    Point::new(bounds.min.x - 1.0, bounds.min.y - 1.0),
                    Point::new(bounds.max.x + 1.0, bounds.max.y + 1.0),
                ),
                color: Rgba8 {
                    r: 255,
                    g: 112,
                    b: 86,
                    a: 190,
                },
                width: 1.0,
            }));
        }
        let font_size = 9.0;
        let text_rect = Rect::from_min_max(
            Point::new(bounds.min.x + 5.0, bounds.min.y),
            Point::new(bounds.max.x - 5.0, bounds.max.y),
        );
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.common.id,
            text: PaintText::from(self.label.as_str()),
            rect: text_rect,
            font_size,
            baseline: Some(((text_rect.height() - font_size) * 0.5 + font_size * 0.78).round()),
            color: Rgba8 {
                r: 183,
                g: 184,
                b: 184,
                a: 235,
            },
            align: PaintTextAlign::Center,
            wrap: TextWrap::None,
        }));
    }
}
