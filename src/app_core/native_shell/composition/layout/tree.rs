use super::{ShellNode, ShellNodeKind};
use crate::gui::types::Rect;

/// Build the retained shell tree from the already-computed chrome rects.
pub(super) fn build_shell_root(
    root_rect: Rect,
    top_bar: Rect,
    sidebar: Rect,
    content: Rect,
    waveform_card: Rect,
    browser_panel: Rect,
    browser_tabs: Rect,
    browser_rows: Rect,
    status_bar: Rect,
) -> ShellNode {
    ShellNode {
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
                children: vec![
                    ShellNode {
                        id: 5,
                        kind: ShellNodeKind::WaveformCard,
                        rect: waveform_card,
                        children: Vec::new(),
                    },
                    ShellNode {
                        id: 100,
                        kind: ShellNodeKind::BrowserPanel,
                        rect: browser_panel,
                        children: vec![
                            ShellNode {
                                id: 101,
                                kind: ShellNodeKind::BrowserTabs,
                                rect: browser_tabs,
                                children: Vec::new(),
                            },
                            ShellNode {
                                id: 102,
                                kind: ShellNodeKind::BrowserTable,
                                rect: browser_rows,
                                children: Vec::new(),
                            },
                        ],
                    },
                ],
            },
            ShellNode {
                id: 6,
                kind: ShellNodeKind::StatusBar,
                rect: status_bar,
                children: Vec::new(),
            },
        ],
    }
}
