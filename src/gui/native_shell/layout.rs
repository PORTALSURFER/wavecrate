//! Retained view-tree layout and hit-testing for the native shell.

use crate::gui::types::{Point, Rect, Vector2};

/// Stable identifier for nodes in the retained shell tree.
pub(crate) type ViewNodeId = u64;

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
    pub id: ViewNodeId,
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
    /// Build shell layout for the provided logical viewport dimensions.
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
                        let mut children = vec![ShellNode {
                            id: 5,
                            kind: ShellNodeKind::WaveformCard,
                            rect: waveform_card,
                            children: Vec::new(),
                        }];
                        for (index, rect) in columns.iter().copied().enumerate() {
                            children.push(ShellNode {
                                id: 100 + index as u64,
                                kind: ShellNodeKind::TriageColumn(index),
                                rect,
                                children: Vec::new(),
                            });
                        }
                        children
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

    /// Resolve triage column index for a point, if any.
    pub(crate) fn column_at_point(&self, point: Point) -> Option<usize> {
        match self.hit_test(point) {
            Some(ShellNodeKind::TriageColumn(index)) => Some(index),
            _ => None,
        }
    }
}
