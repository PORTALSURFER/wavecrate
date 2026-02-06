//! Backend-neutral native shell model used by the Vello runtime.
//!
//! The design mirrors a retained view tree (inspired by Xilem): we build a
//! deterministic layout tree, run hit testing against that tree, and then
//! derive draw primitives that the backend renderer (Vello here) can consume.

use super::input::KeyCode;
use super::types::{Point, Rect, Rgba8, Vector2};

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

const COLOR_BG_PRIMARY: Rgba8 = rgba(12, 11, 10, 255);
const COLOR_BG_SECONDARY: Rgba8 = rgba(20, 18, 16, 255);
const COLOR_BG_TERTIARY: Rgba8 = rgba(28, 26, 23, 255);
const COLOR_BORDER: Rgba8 = rgba(44, 40, 36, 255);
const COLOR_GRID_STRONG: Rgba8 = rgba(55, 50, 45, 255);
const COLOR_GRID_SOFT: Rgba8 = rgba(42, 38, 34, 255);
const COLOR_ACCENT_MINT: Rgba8 = rgba(152, 172, 158, 255);
const COLOR_ACCENT_COPPER: Rgba8 = rgba(186, 148, 108, 255);
const COLOR_ACCENT_WARNING: Rgba8 = rgba(194, 158, 108, 255);

/// Semantic node kinds used by the native shell tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShellNodeKind {
    Root,
    TopBar,
    Sidebar,
    Content,
    WaveformCard,
    TriageColumn(usize),
    StatusBar,
}

/// A retained view node with stable identity, geometry, and optional children.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShellNode {
    pub id: u64,
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
    pub sidebar: Rect,
    pub content: Rect,
    pub waveform_card: Rect,
    pub columns: [Rect; 3],
    pub status_bar: Rect,
}

impl ShellLayout {
    /// Build the shell layout for a viewport.
    pub(crate) fn build(viewport: Vector2) -> Self {
        let viewport_width = viewport.x.max(640.0);
        let viewport_height = viewport.y.max(420.0);

        let root_rect = Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(viewport_width, viewport_height),
        );
        let frame = root_rect.inset(10.0);
        let gap = 10.0;
        let top_bar = Rect::from_min_max(frame.min, Point::new(frame.max.x, frame.min.y + 44.0));
        let status_bar = Rect::from_min_max(Point::new(frame.min.x, frame.max.y - 24.0), frame.max);
        let body = Rect::from_min_max(
            Point::new(frame.min.x, top_bar.max.y + gap),
            Point::new(frame.max.x, status_bar.min.y - gap),
        );

        let max_sidebar = (body.width() - 260.0).max(180.0);
        let sidebar_width = (body.width() * 0.24).clamp(180.0, 320.0).min(max_sidebar);
        let sidebar =
            Rect::from_min_max(body.min, Point::new(body.min.x + sidebar_width, body.max.y));
        let content_min_x = (sidebar.max.x + gap).min(body.max.x - 64.0);
        let content = Rect::from_min_max(Point::new(content_min_x, body.min.y), body.max);

        let waveform_height = (content.height() * 0.42)
            .clamp(160.0, 340.0)
            .min((content.height() - 70.0).max(80.0));
        let waveform_card = Rect::from_min_max(
            content.min,
            Point::new(
                content.max.x,
                (content.min.y + waveform_height).min(content.max.y),
            ),
        );

        let triage_top = (waveform_card.max.y + gap).min(content.max.y - 1.0);
        let triage_rect = Rect::from_min_max(Point::new(content.min.x, triage_top), content.max);
        let column_gap = 8.0;
        let base_column_width = ((triage_rect.width() - (column_gap * 2.0)) / 3.0).max(40.0);

        let mut columns = [Rect::default(), Rect::default(), Rect::default()];
        for (index, column) in columns.iter_mut().enumerate() {
            let x0 = triage_rect.min.x + (base_column_width + column_gap) * index as f32;
            let x1 = if index == 2 {
                triage_rect.max.x
            } else {
                x0 + base_column_width
            };
            *column = Rect::from_min_max(
                Point::new(x0, triage_rect.min.y),
                Point::new(x1, triage_rect.max.y),
            );
        }

        let root = ShellNode {
            id: 1,
            kind: ShellNodeKind::Root,
            rect: root_rect,
            children: vec![
                ShellNode {
                    id: 2,
                    kind: ShellNodeKind::TopBar,
                    rect: top_bar,
                    children: Vec::new(),
                },
                ShellNode {
                    id: 3,
                    kind: ShellNodeKind::Sidebar,
                    rect: sidebar,
                    children: Vec::new(),
                },
                ShellNode {
                    id: 4,
                    kind: ShellNodeKind::Content,
                    rect: content,
                    children: {
                        let mut content_children = vec![ShellNode {
                            id: 5,
                            kind: ShellNodeKind::WaveformCard,
                            rect: waveform_card,
                            children: Vec::new(),
                        }];
                        for (index, rect) in columns.iter().copied().enumerate() {
                            content_children.push(ShellNode {
                                id: 100 + index as u64,
                                kind: ShellNodeKind::TriageColumn(index),
                                rect,
                                children: Vec::new(),
                            });
                        }
                        content_children
                    },
                },
                ShellNode {
                    id: 6,
                    kind: ShellNodeKind::StatusBar,
                    rect: status_bar,
                    children: Vec::new(),
                },
            ],
        };

        Self {
            root,
            top_bar,
            sidebar,
            content,
            waveform_card,
            columns,
            status_bar,
        }
    }

    /// Hit-test against the retained tree.
    pub(crate) fn hit_test(&self, point: Point) -> Option<ShellNodeKind> {
        self.root.hit_test(point)
    }

    /// Resolve the triage column index for a point, if any.
    pub(crate) fn column_at_point(&self, point: Point) -> Option<usize> {
        match self.hit_test(point) {
            Some(ShellNodeKind::TriageColumn(index)) => Some(index),
            _ => None,
        }
    }
}

/// Filled rectangle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FillRect {
    pub rect: Rect,
    pub color: Rgba8,
}

/// Filled circle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FillCircle {
    pub center: Point,
    pub radius: f32,
    pub color: Rgba8,
}

/// Backend-neutral scene primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Primitive {
    Rect(FillRect),
    Circle(FillCircle),
}

/// Frame scene generated from current shell state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShellFrame {
    pub clear_color: Rgba8,
    pub primitives: Vec<Primitive>,
}

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

    /// Build the current frame as backend-neutral primitives.
    pub(crate) fn build_frame(&self, layout: &ShellLayout) -> ShellFrame {
        let mut primitives = Vec::new();
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.top_bar,
            color: if self.hovered == Some(ShellNodeKind::TopBar) {
                COLOR_BG_TERTIARY
            } else {
                COLOR_BG_SECONDARY
            },
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.sidebar,
            color: if self.hovered == Some(ShellNodeKind::Sidebar) {
                COLOR_BG_TERTIARY
            } else {
                COLOR_BG_SECONDARY
            },
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.content,
            color: COLOR_BG_PRIMARY,
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.waveform_card,
            color: if self.hovered == Some(ShellNodeKind::WaveformCard) {
                COLOR_BG_TERTIARY
            } else {
                COLOR_BG_SECONDARY
            },
        }));
        primitives.push(Primitive::Rect(FillRect {
            rect: layout.status_bar,
            color: COLOR_BG_SECONDARY,
        }));

        let waveform_inner = layout.waveform_card.inset(8.0);
        let scan_step = 14.0;
        let mut x = waveform_inner.min.x;
        while x < waveform_inner.max.x {
            let strong = ((x - waveform_inner.min.x) / scan_step).floor() as i32 % 4 == 0;
            let line_color = if strong {
                COLOR_GRID_STRONG
            } else {
                COLOR_GRID_SOFT
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
                COLOR_BG_TERTIARY
            } else {
                COLOR_BG_SECONDARY
            };
            primitives.push(Primitive::Rect(FillRect {
                rect: column_rect,
                color: fill,
            }));
            push_border(
                &mut primitives,
                column_rect,
                if hovered {
                    COLOR_ACCENT_WARNING
                } else if selected {
                    COLOR_ACCENT_MINT
                } else {
                    COLOR_BORDER
                },
            );

            for row_rect in build_column_rows(column_rect.inset(8.0), 8, 6.0) {
                primitives.push(Primitive::Rect(FillRect {
                    rect: row_rect,
                    color: if selected {
                        COLOR_GRID_STRONG
                    } else {
                        COLOR_GRID_SOFT
                    },
                }));
            }
        }

        push_border(&mut primitives, layout.top_bar, COLOR_BORDER);
        push_border(&mut primitives, layout.sidebar, COLOR_BORDER);
        push_border(&mut primitives, layout.waveform_card, COLOR_BORDER);
        push_border(&mut primitives, layout.status_bar, COLOR_BORDER);

        let lamp_radius = 5.0 + (((self.pulse_phase.sin() + 1.0) * 0.5) * 4.0);
        let lamp_color = if self.transport_running {
            COLOR_ACCENT_MINT
        } else {
            COLOR_ACCENT_COPPER
        };
        primitives.push(Primitive::Circle(FillCircle {
            center: Point::new(layout.top_bar.max.x - 20.0, layout.top_bar.min.y + 22.0),
            radius: lamp_radius,
            color: lamp_color,
        }));

        ShellFrame {
            clear_color: COLOR_BG_PRIMARY,
            primitives,
        }
    }
}

fn push_border(primitives: &mut Vec<Primitive>, rect: Rect, color: Rgba8) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_exposes_non_overlapping_columns() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        assert!(layout.columns[0].max.x <= layout.columns[1].min.x);
        assert!(layout.columns[1].max.x <= layout.columns[2].min.x);
        assert!(layout.columns.iter().all(|column| column.width() > 40.0));
    }

    #[test]
    fn hit_test_prefers_column_node_inside_content() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let center = Point::new(
            (layout.columns[0].min.x + layout.columns[0].max.x) * 0.5,
            (layout.columns[0].min.y + layout.columns[0].max.y) * 0.5,
        );
        assert_eq!(
            layout.hit_test(center),
            Some(ShellNodeKind::TriageColumn(0))
        );
    }

    #[test]
    fn primary_click_selects_clicked_column() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let mut state = NativeShellState::new();
        let point = Point::new(
            (layout.columns[2].min.x + layout.columns[2].max.x) * 0.5,
            (layout.columns[2].min.y + layout.columns[2].max.y) * 0.5,
        );
        assert!(state.handle_primary_click(&layout, point));
        let frame = state.build_frame(&layout);
        assert!(frame.primitives.len() > 10);
    }

    #[test]
    fn arrow_keys_wrap_selection() {
        let mut state = NativeShellState::new();
        assert!(state.handle_key(KeyCode::ArrowRight));
        assert!(state.handle_key(KeyCode::ArrowRight));
        assert!(state.handle_key(KeyCode::ArrowRight));
        assert!(state.handle_key(KeyCode::ArrowLeft));
    }
}
