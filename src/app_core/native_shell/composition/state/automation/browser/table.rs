use super::*;

pub(super) fn build_browser_table_automation(
    shell: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let rows = shell.cached_browser_rows(layout, style, model).to_vec();
    let first_visible_row = rows
        .first()
        .map(|row| row.visible_row.to_string())
        .unwrap_or_default();
    let last_visible_row = rows
        .last()
        .map(|row| row.visible_row.to_string())
        .unwrap_or_default();
    let rendered_row_count = rows.len().to_string();
    let mut table_node = simple_node(
        "browser.table",
        AutomationRole::Table,
        Some(String::from("Browser rows")),
        layout.browser_rows,
        Some(model.browser_chrome.item_count_label.clone()),
        true,
        matches!(
            model.focus_context,
            crate::app_core::native_shell::runtime_contract::FocusContextModel::ContentList
        ),
        vec![
            String::from("focus_browser_panel"),
            String::from("set_browser_view_start"),
        ],
    );
    table_node.metadata = metadata(&[
        ("first_visible_row", &first_visible_row),
        ("last_visible_row", &last_visible_row),
        ("rendered_row_count", &rendered_row_count),
    ]);
    let mut children: Vec<_> = rows.into_iter().map(browser_row_node).collect();
    if let Some((scrollbar, viewport_len)) = shell.cached_browser_scrollbar(layout, model) {
        let visible_count = model.browser.visible_count.to_string();
        let viewport_len = viewport_len.to_string();
        let view_start_row = first_visible_row.clone();
        table_node.metadata.extend(metadata(&[
            ("scrollbar_visible", "true"),
            ("viewport_len", &viewport_len),
            ("visible_count", &visible_count),
            ("view_start_row", &view_start_row),
        ]));
        children.extend(browser_scrollbar_automation(
            scrollbar,
            &viewport_len,
            &visible_count,
        ));
    } else {
        table_node
            .metadata
            .insert(String::from("scrollbar_visible"), String::from("false"));
    }
    table_node.children = children;
    table_node
}

fn browser_row_node(row: CachedBrowserRow) -> AutomationNodeSnapshot {
    AutomationNodeSnapshot {
        id: node_id(format!("browser.row.{}", row.visible_row)),
        role: AutomationRole::Row,
        label: Some(row.label.clone()),
        bounds: bounds(row.rect),
        value: (!row.bucket_label.is_empty()).then_some(row.bucket_label.clone()),
        enabled: true,
        selected: row.selected || row.focused,
        available_actions: vec![
            String::from("focus_browser_row"),
            String::from("toggle_browser_row_selection"),
            String::from("commit_focused_browser_row"),
        ],
        metadata: metadata(&[
            ("column", &row.column.to_string()),
            ("rating_level", &row.rating_level.to_string()),
            ("focused", bool_text(row.focused)),
            ("missing", bool_text(row.missing)),
            ("locked", bool_text(row.locked)),
            ("marked", bool_text(row.marked)),
            (
                "playback_age_bucket",
                playback_age_bucket_text(row.playback_age_bucket),
            ),
        ]),
        children: Vec::new(),
    }
}

fn playback_age_bucket_text(
    bucket: crate::app_core::native_shell::runtime_contract::PlaybackAgeBucket,
) -> &'static str {
    match bucket {
        crate::app_core::native_shell::runtime_contract::PlaybackAgeBucket::Fresh => "fresh",
        crate::app_core::native_shell::runtime_contract::PlaybackAgeBucket::OlderThanWeek => "week",
        crate::app_core::native_shell::runtime_contract::PlaybackAgeBucket::OlderThanMonth => {
            "month"
        }
        crate::app_core::native_shell::runtime_contract::PlaybackAgeBucket::NeverPlayed => "never",
    }
}

fn browser_scrollbar_automation(
    scrollbar: BrowserScrollbarLayout,
    viewport_len: &str,
    visible_count: &str,
) -> [AutomationNodeSnapshot; 2] {
    let track_metadata = metadata(&[
        ("viewport_len", viewport_len),
        ("visible_count", visible_count),
        ("part", "track"),
    ]);
    let thumb_metadata = metadata(&[
        ("viewport_len", viewport_len),
        ("visible_count", visible_count),
        ("part", "thumb"),
    ]);
    [
        AutomationNodeSnapshot {
            id: node_id("browser.scrollbar.track"),
            role: AutomationRole::Slider,
            label: Some(String::from("Browser scrollbar track")),
            bounds: bounds(scrollbar.track),
            value: None,
            enabled: true,
            selected: false,
            available_actions: vec![String::from("set_browser_view_start")],
            metadata: track_metadata,
            children: Vec::new(),
        },
        AutomationNodeSnapshot {
            id: node_id("browser.scrollbar.thumb"),
            role: AutomationRole::Slider,
            label: Some(String::from("Browser scrollbar thumb")),
            bounds: bounds(scrollbar.thumb),
            value: None,
            enabled: true,
            selected: false,
            available_actions: vec![String::from("set_browser_view_start")],
            metadata: thumb_metadata,
            children: Vec::new(),
        },
    ]
}
