use radiant::prelude as ui;

use super::identity::folder_row_key;
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::SIDEBAR_ROW_STYLE;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::VisibleFolder;
use crate::native_app::sample_library::folder_browser::view_contract::{
    TREE_DEPTH_INDENT, TREE_ROW_HEIGHT,
};

const FOLDER_EXPANDER_WIDTH: f32 = 28.0;
const FOLDER_TREE_HIGHLIGHTED_LABEL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 238,
    b: 224,
    a: 255,
};
pub(super) const FOLDER_TREE_EMPTY_LABEL: ui::Rgba8 = ui::Rgba8 {
    r: 142,
    g: 148,
    b: 156,
    a: 255,
};
pub(super) const FOLDER_TREE_SELECTED_HOVER_MARKER_ALPHA: u8 = 245;
pub(super) const FOLDER_TREE_SELECTED_HOVER_MARKER_WIDTH: f32 = 3.0;
const FOLDER_LOCK_ICON_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 232,
    g: 221,
    b: 190,
    a: 235,
};
const FOLDER_LOCK_INHERITED_ICON_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 158,
    g: 164,
    b: 172,
    a: 220,
};

pub(super) fn folder_row(folder: &VisibleFolder) -> ui::View<GuiMessage> {
    let id = folder.id.clone();
    if let Some(rename) = FolderRenameProjection::from_folder(folder) {
        return folder_rename_row(folder, &id, rename);
    }

    standard_folder_row(folder, id)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FolderRenameProjection<'a> {
    draft: &'a str,
    input_id: u64,
    caret: usize,
}

impl<'a> FolderRenameProjection<'a> {
    fn from_folder(folder: &'a VisibleFolder) -> Option<Self> {
        let draft = folder.rename_draft.as_deref()?;
        Some(Self {
            draft,
            input_id: folder.rename_input_id?,
            caret: draft.chars().count(),
        })
    }
}

fn folder_rename_row(
    folder: &VisibleFolder,
    id: &str,
    rename: FolderRenameProjection<'_>,
) -> ui::View<GuiMessage> {
    ui::row([
        ui::tree_guide_indent(folder.depth, folder_tree_guide_style()),
        ui::text_input(rename.draft.to_owned())
            .selection(0, rename.caret)
            .message_event(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
            })
            .id(rename.input_id)
            .fill_width()
            .height(22.0),
    ])
    .key(folder_row_key(id))
    .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
    .fill_width()
    .height(TREE_ROW_HEIGHT)
    .spacing(1.0)
    .hoverable()
}

fn standard_folder_row(folder: &VisibleFolder, id: String) -> ui::View<GuiMessage> {
    let row = ui::tree_row(folder.name.clone())
        .depth(folder.depth)
        .expanded(folder.expanded)
        .has_children(folder.has_children && !folder.is_source_root)
        .selected(folder.selected)
        .focused(folder.focused)
        .drag_drop_state(folder_tree_drag_drop_state(folder))
        .row_height(TREE_ROW_HEIGHT)
        .expander_width(FOLDER_EXPANDER_WIDTH)
        .style(SIDEBAR_ROW_STYLE)
        .guide_style(folder_tree_guide_style())
        .selected_hover_marker(folder_tree_selected_hover_marker())
        .highlighted_label_color(folder_tree_highlighted_label_color(folder));

    let row = if let Some(label_color) = folder_tree_label_color(folder) {
        row.label_color(label_color)
    } else {
        row
    };
    let row = if folder.locked {
        row.trailing_icon(folder_lock_icon(folder.lock_inherited))
    } else {
        row
    };

    row.row_key(folder_row_key(&id))
        .on_toggle({
            let id = id.clone();
            move || {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ToggleFolderExpansion(id.clone()))
            }
        })
        .interactive_actions(folder_row_actions(id))
}

fn folder_row_actions(id: String) -> ui::InteractiveRowActions<GuiMessage> {
    ui::row_actions()
        .primary_with_modifiers_key(id.clone(), |id, modifiers| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(id, modifiers))
        })
        .double_key(id.clone(), |id| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(id, Default::default()))
        })
        .secondary_key(id.clone(), |id, position| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::OpenFolderContextMenu(id, position))
        })
        .drag_key(id.clone(), |id, drag| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::DragFolder(id, drag))
        })
        .tracked_drop_candidate_key(
            id,
            |id| GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnFolder(id)),
            |id, position| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::HoverDropTarget(id, position))
            },
            |id, position| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ClearDropTargetUnless(id, position))
            },
        )
}

pub(super) fn folder_tree_label_color(folder: &VisibleFolder) -> Option<ui::Rgba8> {
    folder.empty.then_some(FOLDER_TREE_EMPTY_LABEL)
}

fn folder_tree_highlighted_label_color(folder: &VisibleFolder) -> ui::Rgba8 {
    if folder.empty {
        FOLDER_TREE_EMPTY_LABEL
    } else {
        FOLDER_TREE_HIGHLIGHTED_LABEL
    }
}

fn folder_tree_drag_drop_state(folder: &VisibleFolder) -> ui::TreeRowDragDropState {
    ui::TreeRowDragDropState {
        drag_active: folder.drag_active,
        drag_source: folder.drag_source,
        drop_candidate: folder.drop_candidate,
        drop_target: folder.drop_target,
        drop_target_active: folder.drop_target_active,
    }
}

pub(super) fn folder_tree_guide_style() -> ui::StyledTreeGuideStyle {
    ui::StyledTreeGuideStyle::new(TREE_DEPTH_INDENT, TREE_ROW_HEIGHT, SIDEBAR_ROW_STYLE)
}

#[cfg(test)]
pub(super) fn folder_tree_palette_for_tests(theme: &ui::ThemeTokens) -> ui::DenseRowPalette {
    ui::dense_row_palette_from_style(theme, SIDEBAR_ROW_STYLE)
}

pub(super) fn folder_tree_selected_hover_marker() -> ui::DenseRowMarkerStyle {
    ui::DenseRowMarkerStyle::new(
        ui::DenseRowMarkerParts::leading(FOLDER_TREE_SELECTED_HOVER_MARKER_WIDTH)
            .edge_inset(1.0)
            .vertical_inset(3.0),
        ui::ThemeTokens::default()
            .accent_mint
            .with_alpha(FOLDER_TREE_SELECTED_HOVER_MARKER_ALPHA),
    )
}

fn folder_lock_icon(inherited: bool) -> ui::SvgIcon {
    let color = if inherited {
        FOLDER_LOCK_INHERITED_ICON_COLOR
    } else {
        FOLDER_LOCK_ICON_COLOR
    };
    FOLDER_LOCK_ICON.icon(color)
}

static FOLDER_LOCK_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M4.1 7.1V5.6C4.1 3.45 5.65 2 8 2s3.9 1.45 3.9 3.6v1.5" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round"/>
  <rect x="3" y="6.75" width="10" height="7" rx="1.2" fill="currentColor"/>
  <rect x="7.3" y="9" width="1.4" height="2.7" rx=".55" fill="rgb(24,24,24)"/>
</svg>"#,
);
