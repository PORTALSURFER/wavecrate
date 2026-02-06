//! Mutable interaction state and paint generation for the native shell.

use super::{
    layout::{ShellLayout, ShellNodeKind},
    paint::{FillCircle, FillRect, NativeViewFrame, Primitive, TextAlign, TextRun},
    style::StyleTokens,
};
use crate::gui::{
    input::KeyCode,
    types::{Point, Rect},
};

/// Mutable interaction + animation state for the native shell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeShellState {
    selected_column: usize,
    hovered: Option<ShellNodeKind>,
    transport_running: bool,
    pulse_phase: f32,
}

impl NativeShellState {
    /// Create a default shell state.
    pub(crate) fn new() -> Self {
        Self {
            selected_column: 1,
            hovered: None,
            transport_running: true,
            pulse_phase: 0.0,
        }
    }

    /// Return whether the shell currently needs continuous animation.
    pub(crate) fn needs_animation(&self) -> bool {
        self.transport_running
    }

    /// Update animation clocks by a frame delta.
    pub(crate) fn tick(&mut self, delta_seconds: f32) {
        if self.transport_running {
            self.pulse_phase =
                (self.pulse_phase + delta_seconds * 2.6).rem_euclid(std::f32::consts::TAU);
        }
    }

    /// Handle pointer movement and update hovered view target.
    pub(crate) fn handle_cursor_move(&mut self, layout: &ShellLayout, point: Point) -> bool {
        let next_hover = layout.hit_test(point);
        if next_hover == self.hovered {
            return false;
        }
        self.hovered = next_hover;
        true
    }

    /// Handle a primary button click at the pointer position.
    pub(crate) fn handle_primary_click(&mut self, layout: &ShellLayout, point: Point) -> bool {
        let Some(column) = layout.column_at_point(point) else {
            return false;
        };
        if self.selected_column == column {
            return false;
        }
        self.selected_column = column;
        true
    }

    /// Handle backend-agnostic key input.
    pub(crate) fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::ArrowLeft => {
                self.selected_column = (self.selected_column + 2) % 3;
                true
            }
            KeyCode::ArrowRight => {
                self.selected_column = (self.selected_column + 1) % 3;
                true
            }
            KeyCode::Num1 => {
                if self.selected_column == 0 {
                    false
                } else {
                    self.selected_column = 0;
                    true
                }
            }
            KeyCode::Num2 => {
                if self.selected_column == 1 {
                    false
                } else {
                    self.selected_column = 1;
                    true
                }
            }
            KeyCode::Num3 => {
                if self.selected_column == 2 {
                    false
                } else {
                    self.selected_column = 2;
                    true
                }
            }
            KeyCode::Enter => {
                self.transport_running = !self.transport_running;
                true
            }
            _ => false,
        }
    }

    /// Build a native frame from state + layout + style tokens.
    pub(crate) fn build_frame_with_style(
        &self,
        layout: &ShellLayout,
        style: &StyleTokens,
    ) -> NativeViewFrame {
        let mut primitives = Vec::new();
        let mut text_runs = Vec::new();

        primitives.push(Primitive::Rect(FillRect {
            rect: layout.top_bar,
            color: if self.hovered == Some(ShellNodeKind::TopBar) {
                style.bg_tertiary
            } else {
                style.bg_secondary
            },
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.sidebar,
            color: if self.hovered == Some(ShellNodeKind::Sidebar) {
                style.bg_tertiary
            } else {
                style.bg_secondary
            },
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.content,
            color: style.bg_primary,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.waveform_card,
            color: if self.hovered == Some(ShellNodeKind::WaveformCard) {
                style.bg_tertiary
            } else {
                style.bg_secondary
            },
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.status_bar,
            color: style.bg_secondary,
        }));

        let waveform_inner = layout.waveform_card.inset(8.0);
        let scan_step = 14.0;
        let mut x = waveform_inner.min.x;
        while x < waveform_inner.max.x {
            let strong = ((x - waveform_inner.min.x) / scan_step).floor() as i32 % 4 == 0;
            let line_color = if strong {
                style.grid_strong
            } else {
                style.grid_soft
            };
            primitives.push(Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(x, waveform_inner.min.y),
                    Point::new((x + 1.0).min(waveform_inner.max.x), waveform_inner.max.y),
                ),
                color: line_color,
            }));
            x += scan_step;
        }

        for (index, column_rect) in layout.columns.iter().copied().enumerate() {
            let hovered = self.hovered == Some(ShellNodeKind::TriageColumn(index));
            let selected = self.selected_column == index;
            let fill = if selected {
                style.bg_tertiary
            } else {
                style.bg_secondary
            };
            primitives.push(Primitive::Rect(FillRect {
                rect: column_rect,
                color: fill,
            }));
            push_border(
                &mut primitives,
                column_rect,
                if hovered {
                    style.accent_warning
                } else if selected {
                    style.accent_mint
                } else {
                    style.border
                },
            );

            for row_rect in build_column_rows(column_rect.inset(8.0), 8, 6.0) {
                primitives.push(Primitive::Rect(FillRect {
                    rect: row_rect,
                    color: if selected {
                        style.grid_strong
                    } else {
                        style.grid_soft
                    },
                }));
            }
        }

        push_border(&mut primitives, layout.top_bar, style.border);
        push_border(&mut primitives, layout.sidebar, style.border);
        push_border(&mut primitives, layout.waveform_card, style.border);
        push_border(&mut primitives, layout.status_bar, style.border);

        let lamp_radius = 5.0 + (((self.pulse_phase.sin() + 1.0) * 0.5) * 4.0);
        let lamp_color = if self.transport_running {
            style.accent_mint
        } else {
            style.accent_copper
        };
        primitives.push(Primitive::Circle(FillCircle {
            center: Point::new(layout.top_bar.max.x - 20.0, layout.top_bar.min.y + 22.0),
            radius: lamp_radius,
            color: lamp_color,
        }));

        text_runs.push(TextRun {
            text: String::from("Sempal Native Shell"),
            position: Point::new(layout.top_bar.min.x + 12.0, layout.top_bar.min.y + 10.0),
            font_size: 16.0,
            color: style.text_primary,
            max_width: Some((layout.top_bar.width() - 120.0).max(100.0)),
            align: TextAlign::Left,
        });
        text_runs.push(TextRun {
            text: String::from("backend: native_vello"),
            position: Point::new(layout.top_bar.min.x + 12.0, layout.top_bar.min.y + 24.0),
            font_size: 12.0,
            color: style.text_muted,
            max_width: Some((layout.top_bar.width() - 24.0).max(100.0)),
            align: TextAlign::Right,
        });
        text_runs.push(TextRun {
            text: String::from("Sources"),
            position: Point::new(layout.sidebar.min.x + 12.0, layout.sidebar.min.y + 10.0),
            font_size: 14.0,
            color: style.text_primary,
            max_width: Some((layout.sidebar.width() - 24.0).max(80.0)),
            align: TextAlign::Left,
        });
        text_runs.push(TextRun {
            text: String::from("Waveform"),
            position: Point::new(
                layout.waveform_card.min.x + 12.0,
                layout.waveform_card.min.y + 10.0,
            ),
            font_size: 13.0,
            color: style.text_muted,
            max_width: Some((layout.waveform_card.width() - 24.0).max(80.0)),
            align: TextAlign::Left,
        });
        for (index, column) in layout.columns.iter().enumerate() {
            let label = match index {
                0 => "Trash",
                1 => "Samples",
                _ => "Keep",
            };
            text_runs.push(TextRun {
                text: label.to_string(),
                position: Point::new(column.min.x + 10.0, column.min.y + 8.0),
                font_size: 13.0,
                color: if self.selected_column == index {
                    style.accent_mint
                } else {
                    style.text_muted
                },
                max_width: Some((column.width() - 20.0).max(60.0)),
                align: TextAlign::Left,
            });
        }

        let status_text = if self.transport_running {
            format!(
                "Transport: running | Selected column: {}",
                self.selected_column + 1
            )
        } else {
            format!(
                "Transport: stopped | Selected column: {}",
                self.selected_column + 1
            )
        };
        text_runs.push(TextRun {
            text: status_text,
            position: Point::new(
                layout.status_bar.min.x + 10.0,
                layout.status_bar.min.y + 4.0,
            ),
            font_size: 12.0,
            color: style.text_muted,
            max_width: Some((layout.status_bar.width() - 20.0).max(80.0)),
            align: TextAlign::Left,
        });
        text_runs.push(TextRun {
            text: String::from("trash | samples | keep"),
            position: Point::new(
                layout.status_bar.min.x + 10.0,
                layout.status_bar.min.y + 4.0,
            ),
            font_size: 12.0,
            color: style.text_primary,
            max_width: Some((layout.status_bar.width() - 20.0).max(80.0)),
            align: TextAlign::Center,
        });

        NativeViewFrame {
            clear_color: style.clear_color,
            primitives,
            text_runs,
        }
    }

    /// Build a native frame using default style tokens.
    pub(crate) fn build_frame(&self, layout: &ShellLayout) -> NativeViewFrame {
        self.build_frame_with_style(layout, &StyleTokens::default())
    }
}

fn push_border(primitives: &mut Vec<Primitive>, rect: Rect, color: crate::gui::types::Rgba8) {
    if rect.width() <= 2.0 || rect.height() <= 2.0 {
        return;
    }
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(rect.min, Point::new(rect.max.x, rect.min.y + 1.0)),
        color,
    }));
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - 1.0), rect.max),
        color,
    }));
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(rect.min, Point::new(rect.min.x + 1.0, rect.max.y)),
        color,
    }));
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(Point::new(rect.max.x - 1.0, rect.min.y), rect.max),
        color,
    }));
}

fn build_column_rows(column: Rect, rows: usize, gap: f32) -> Vec<Rect> {
    if rows == 0 {
        return Vec::new();
    }
    let total_gap = gap * (rows.saturating_sub(1) as f32);
    let row_height = ((column.height() - total_gap) / rows as f32).max(6.0);
    let mut y = column.min.y;
    let mut output = Vec::with_capacity(rows);
    for row_index in 0..rows {
        let remaining = rows - row_index;
        let max_y = if remaining == 1 {
            column.max.y
        } else {
            (y + row_height).min(column.max.y)
        };
        output.push(Rect::from_min_max(
            Point::new(column.min.x, y),
            Point::new(column.max.x, max_y),
        ));
        y = (max_y + gap).min(column.max.y);
    }
    output
}
