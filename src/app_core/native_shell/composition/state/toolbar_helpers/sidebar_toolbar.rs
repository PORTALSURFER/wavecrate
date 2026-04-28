//! Sidebar action, source-add, and context-menu helper geometry.

use super::super::*;

pub(in crate::gui::native_shell::state) fn render_source_add_button_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    button_rect: Rect,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) {
    let fill = source_add_button_fill(style, hovered, flashed, motion_wave);
    let border = source_add_button_border(style, hovered, flashed, motion_wave);
    let icon_color = source_add_button_icon_color(style, hovered, flashed, motion_wave);
    let label_rect = compute_action_button_text_rect(button_rect, sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: button_rect,
            color: fill,
        }),
    );
    push_border(primitives, button_rect, border, sizing.border_width);
    emit_text(
        text_runs,
        TextRun {
            text: String::from("+"),
            position: label_rect.min,
            font_size: sizing.font_meta,
            color: icon_color,
            max_width: Some(label_rect.width().max(8.0)),
            align: TextAlign::Center,
        },
    );
}

pub(in crate::gui::native_shell::state) fn source_add_button_fill(
    style: &StyleTokens,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = style.surface_overlay;
    let hover = blend_color(idle, style.accent_mint, 0.14 + (motion_wave * 0.04));
    let flash = blend_color(hover, style.text_primary, 0.16);
    if flashed {
        flash
    } else if hovered {
        hover
    } else {
        idle
    }
}

pub(in crate::gui::native_shell::state) fn source_add_button_border(
    style: &StyleTokens,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = blend_color(
        style.border_emphasis,
        style.text_primary,
        style.state_hover_soft,
    );
    let hover = blend_color(idle, style.accent_mint, 0.34 + (motion_wave * 0.08));
    if flashed {
        blend_color(hover, style.text_primary, 0.38)
    } else if hovered {
        hover
    } else {
        idle
    }
}

pub(in crate::gui::native_shell::state) fn source_add_button_icon_color(
    style: &StyleTokens,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = style.accent_mint;
    let hover = blend_color(idle, style.text_primary, 0.24 + (motion_wave * 0.06));
    if flashed {
        blend_color(hover, style.text_primary, 0.4)
    } else if hovered {
        hover
    } else {
        idle
    }
}

pub(in crate::gui::native_shell::state) fn source_add_button_rect(
    header_rect: Rect,
    sizing: SizingTokens,
) -> Option<Rect> {
    resolve_sidebar_header_surface_layout(
        header_rect,
        sizing,
        &SidebarHeaderSurfaceContent::default(),
    )
    .add_button_rect
}

pub(in crate::gui::native_shell::state) fn sidebar_sections(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> SidebarSections {
    let resolved = compute_sidebar_row_sections(
        layout.sidebar_rows,
        style.sizing,
        SidebarRowCounts {
            source_rows: rendered_source_rows(style, model),
            upper_folder_rows: model.sources.upper_folder_pane.folder_rows.len(),
            lower_folder_rows: model.sources.lower_folder_pane.folder_rows.len(),
        },
    );
    SidebarSections {
        upper: SidebarPaneSections {
            bounds: resolved.upper_folder_pane.bounds,
            source_rows: resolved.upper_folder_pane.source_rows,
            folder_header: resolved.upper_folder_pane.header,
            folder_rows: resolved.upper_folder_pane.rows,
        },
        lower: SidebarPaneSections {
            bounds: resolved.lower_folder_pane.bounds,
            source_rows: resolved.lower_folder_pane.source_rows,
            folder_header: resolved.lower_folder_pane.header,
            folder_rows: resolved.lower_folder_pane.rows,
        },
    }
}

pub(in crate::gui::native_shell::state) fn source_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<ActionButton> {
    let definitions = [
        (
            "New",
            model.sources.folder_actions.can_create_folder,
            UiAction::StartNewFolder,
            style.text_primary,
        ),
        (
            "Root",
            model.sources.folder_actions.can_create_folder_at_root,
            UiAction::StartNewFolderAtRoot,
            style.text_muted,
        ),
        (
            "Rename",
            model.sources.folder_actions.can_rename_folder,
            UiAction::StartFolderRename,
            style.accent_warning,
        ),
        (
            "Delete",
            model.sources.folder_actions.can_delete_folder,
            UiAction::DeleteFocusedFolder,
            style.accent_copper,
        ),
        (
            "Restore",
            model.sources.folder_actions.can_restore_retained_deletes,
            UiAction::RestoreRetainedFolderDeletes,
            style.accent_mint,
        ),
        (
            "Purge",
            model.sources.folder_actions.can_purge_retained_deletes,
            UiAction::PurgeRetainedFolderDeletes,
            style.accent_copper,
        ),
        (
            "Clear",
            model.sources.folder_actions.can_clear_recovery_log,
            UiAction::ClearFolderDeleteRecoveryLog,
            style.text_muted,
        ),
    ];
    let action_layouts = resolve_sidebar_footer_surface_layout(
        layout.sidebar_footer,
        style.sizing,
        &SidebarFooterSurfaceContent {
            actions: definitions
                .iter()
                .map(|(label, _, _, _)| SidebarFooterActionSpec { label })
                .collect(),
            ..SidebarFooterSurfaceContent::default()
        },
    )
    .action_buttons;
    action_layouts
        .into_iter()
        .zip(definitions.into_iter())
        .map(
            |(layout, (label, enabled, action, text_color))| ActionButton {
                rect: layout.rect,
                label,
                icon: None,
                enabled,
                active: false,
                action,
                text_color,
            },
        )
        .collect()
}

/// Build source context-menu panel geometry and action buttons.
pub(in crate::gui::native_shell::state) fn source_context_menu_spec(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    menu: Option<SourceContextMenuState>,
) -> Option<(Rect, Vec<ActionButton>)> {
    let menu = menu?;
    if menu.row_index >= model.sources.rows.len() {
        return None;
    }
    let source_index = menu.row_index;
    let pane = Some(menu.pane);
    let definitions = [
        (
            "Reload",
            true,
            UiAction::ReloadSourceRow {
                pane,
                index: source_index,
            },
            style.text_primary,
        ),
        (
            "Hard sync",
            true,
            UiAction::HardSyncSourceRow {
                pane,
                index: source_index,
            },
            style.accent_warning,
        ),
        (
            "Open folder",
            true,
            UiAction::OpenSourceFolderRow {
                pane,
                index: source_index,
            },
            style.accent_mint,
        ),
        (
            "Remove source",
            true,
            UiAction::RemoveSourceRow {
                pane,
                index: source_index,
            },
            style.accent_copper,
        ),
    ];
    let sizing = style.sizing;
    let panel_padding = sizing.panel_inset.max(4.0);
    let button_width = sizing.sidebar_action_button_width.max(168.0);
    let button_height = sizing.sidebar_action_button_height.max(18.0);
    let button_gap = sizing.sidebar_action_button_gap.max(2.0);
    let button_count = definitions.len();
    let panel_width = button_width + panel_padding * 2.0;
    let panel_height = (button_height * button_count as f32)
        + (button_gap * button_count.saturating_sub(1) as f32)
        + panel_padding * 2.0;
    let min_x = layout.sidebar.min.x + sizing.panel_inset;
    let max_x = (layout.sidebar.max.x - sizing.panel_inset - panel_width).max(min_x);
    let min_y = layout.sidebar.min.y + sizing.panel_inset;
    let max_y = (layout.sidebar.max.y - sizing.panel_inset - panel_height).max(min_y);
    let panel_min = Point::new(
        menu.anchor.x.clamp(min_x, max_x),
        menu.anchor.y.clamp(min_y, max_y),
    );
    let panel_rect = Rect::from_min_max(
        panel_min,
        Point::new(panel_min.x + panel_width, panel_min.y + panel_height),
    );
    let mut buttons = Vec::with_capacity(button_count);
    let button_x = panel_rect.min.x + panel_padding;
    let mut button_y = panel_rect.min.y + panel_padding;
    for (label, enabled, action, text_color) in definitions {
        let rect = Rect::from_min_max(
            Point::new(button_x, button_y),
            Point::new(button_x + button_width, button_y + button_height),
        );
        buttons.push(ActionButton {
            rect,
            label,
            icon: None,
            enabled,
            active: false,
            action,
            text_color,
        });
        button_y += button_height + button_gap;
    }
    Some((panel_rect, buttons))
}

/// Build browser context-menu panel geometry and action buttons.
pub(in crate::gui::native_shell::state) fn browser_context_menu_spec(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    menu: Option<BrowserContextMenuState>,
) -> Option<(Rect, Vec<ActionButton>)> {
    let menu = menu?;
    if model.map.active || menu.visible_row >= model.browser.rows.len() {
        return None;
    }
    let definitions = [(
        "Auto Rename",
        true,
        UiAction::AutoRenameBrowserSelection {
            visible_row: Some(menu.visible_row),
        },
        style.accent_mint,
    )];
    let sizing = style.sizing;
    let panel_padding = sizing.panel_inset.max(4.0);
    let button_width = sizing.sidebar_action_button_width.max(168.0);
    let button_height = sizing.sidebar_action_button_height.max(18.0);
    let panel_width = button_width + panel_padding * 2.0;
    let panel_height = button_height + panel_padding * 2.0;
    let min_x = layout.browser_rows.min.x + sizing.panel_inset;
    let max_x = (layout.browser_rows.max.x - sizing.panel_inset - panel_width).max(min_x);
    let min_y = layout.browser_rows.min.y + sizing.panel_inset;
    let max_y = (layout.browser_rows.max.y - sizing.panel_inset - panel_height).max(min_y);
    let panel_min = Point::new(
        menu.anchor.x.clamp(min_x, max_x),
        menu.anchor.y.clamp(min_y, max_y),
    );
    let panel_rect = Rect::from_min_max(
        panel_min,
        Point::new(panel_min.x + panel_width, panel_min.y + panel_height),
    );
    let rect = Rect::from_min_max(
        Point::new(
            panel_rect.min.x + panel_padding,
            panel_rect.min.y + panel_padding,
        ),
        Point::new(
            panel_rect.min.x + panel_padding + button_width,
            panel_rect.min.y + panel_padding + button_height,
        ),
    );
    Some((
        panel_rect,
        definitions
            .into_iter()
            .map(|(label, enabled, action, text_color)| ActionButton {
                rect,
                label,
                icon: None,
                enabled,
                active: false,
                action,
                text_color,
            })
            .collect(),
    ))
}
