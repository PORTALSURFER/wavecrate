#![allow(missing_docs)]

use radiant::{
    prelude as ui,
    widgets::{ButtonMessage, WidgetStyle, WidgetTone},
};
use rfd::FileDialog;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::RebuildMessage;

const MAX_SCAN_DEPTH: usize = 3;
const MAX_CHILD_FOLDERS: usize = 80;
const TREE_ROW_HEIGHT: f32 = 23.0;

#[derive(Clone, Debug)]
pub(super) struct FolderBrowserState {
    selected_source: String,
    sources: Vec<SourceEntry>,
    selected_folder: String,
    expanded_folders: HashSet<String>,
    folders: Vec<FolderEntry>,
}

impl FolderBrowserState {
    pub(super) fn load_default() -> Self {
        Self::from_root(default_root_path())
    }

    fn from_root(root: PathBuf) -> Self {
        let sources = vec![SourceEntry::new("assets", "Assets", root)];
        Self::from_sources(sources, String::from("assets"))
    }

    fn from_sources(sources: Vec<SourceEntry>, selected_source: String) -> Self {
        let source = sources
            .iter()
            .find(|source| source.id == selected_source)
            .or_else(|| sources.first())
            .expect("folder browser needs at least one source");
        let root_folder = load_root_folder(source.root.clone());
        let root_id = root_folder.id.clone();
        Self {
            selected_source: source.id.clone(),
            sources,
            selected_folder: root_id.clone(),
            expanded_folders: [root_id].into_iter().collect(),
            folders: vec![root_folder],
        }
    }

    #[cfg(test)]
    pub(super) fn root_path(&self) -> &Path {
        self.folders
            .first()
            .map(|folder| Path::new(&folder.id))
            .unwrap_or_else(|| Path::new(""))
    }

    #[cfg(test)]
    pub(super) fn source_labels(&self) -> Vec<String> {
        self.sources
            .iter()
            .map(|source| source.label.clone())
            .collect()
    }

    pub(super) fn selected_files(&self) -> &[FileEntry] {
        self.selected_folder()
            .map(|folder| folder.files.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn selected_audio_files(&self) -> Vec<&FileEntry> {
        self.selected_files()
            .iter()
            .filter(|file| file.is_audio())
            .collect()
    }

    pub(super) fn apply_message(&mut self, message: FolderBrowserMessage) {
        match message {
            FolderBrowserMessage::AddSource => self.add_source_from_dialog(),
            FolderBrowserMessage::SelectSource(id) => self.select_source(id),
            FolderBrowserMessage::ActivateFolder(id) => self.activate_folder(id),
            FolderBrowserMessage::ToggleFolder(id) => self.toggle_folder(id),
        }
    }

    fn add_source_from_dialog(&mut self) {
        let Some(path) = FileDialog::new().set_title("Add source").pick_folder() else {
            return;
        };
        self.add_source_path(path);
    }

    fn add_source_path(&mut self, root: PathBuf) {
        if let Some(source) = self.sources.iter().find(|source| source.root == root) {
            self.select_source(source.id.clone());
            return;
        }
        let id = path_id(&root);
        let label = folder_label(&root);
        self.sources.push(SourceEntry::new(id.clone(), label, root));
        self.select_source(id);
    }

    fn select_source(&mut self, id: String) {
        if self.selected_source == id {
            return;
        }
        let Some(source) = self.sources.iter().find(|source| source.id == id) else {
            return;
        };
        let root_folder = load_root_folder(source.root.clone());
        let root_id = root_folder.id.clone();
        self.selected_source = source.id.clone();
        self.selected_folder = root_id.clone();
        self.expanded_folders.clear();
        self.expanded_folders.insert(root_id);
        self.folders = vec![root_folder];
    }

    fn selected_folder(&self) -> Option<&FolderEntry> {
        self.find_folder(&self.selected_folder)
            .or_else(|| self.folders.first())
    }

    fn find_folder(&self, id: &str) -> Option<&FolderEntry> {
        self.folders.iter().find_map(|folder| folder.find(id))
    }

    fn folder_has_children(&self, id: &str) -> bool {
        self.find_folder(id).is_some_and(FolderEntry::has_children)
    }

    fn is_expanded(&self, id: &str) -> bool {
        self.expanded_folders.contains(id)
    }

    fn activate_folder(&mut self, id: String) {
        if !self.folder_has_children(&id) {
            self.select_folder(id);
            return;
        }
        if !self.is_expanded(&id) {
            self.expanded_folders.insert(id.clone());
            self.select_folder(id);
        } else if self.selected_folder == id {
            self.expanded_folders.remove(&id);
        } else {
            self.select_folder(id);
        }
    }

    fn toggle_folder(&mut self, id: String) {
        if self.folder_has_children(&id) && !self.expanded_folders.remove(&id) {
            self.expanded_folders.insert(id);
        }
    }

    fn select_folder(&mut self, id: String) {
        self.selected_folder = id;
    }

    fn visible_folders(&self) -> Vec<VisibleFolder> {
        let mut folders = Vec::new();
        for folder in &self.folders {
            self.push_visible_folder(folder, 0, &mut folders);
        }
        folders
    }

    fn push_visible_folder(
        &self,
        folder: &FolderEntry,
        depth: usize,
        folders: &mut Vec<VisibleFolder>,
    ) {
        folders.push(VisibleFolder {
            id: folder.id.clone(),
            name: folder.name.clone(),
            depth,
            has_children: folder.has_children(),
            expanded: self.is_expanded(&folder.id),
            selected: self.selected_folder == folder.id,
        });
        if self.is_expanded(&folder.id) {
            for child in &folder.children {
                self.push_visible_folder(child, depth + 1, folders);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceEntry {
    id: String,
    label: String,
    root: PathBuf,
}

impl SourceEntry {
    fn new(id: impl Into<String>, label: impl Into<String>, root: PathBuf) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            root,
        }
    }
}

#[derive(Clone, Debug)]
struct FolderEntry {
    id: String,
    name: String,
    children: Vec<FolderEntry>,
    files: Vec<FileEntry>,
}

impl FolderEntry {
    fn find(&self, id: &str) -> Option<&FolderEntry> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileEntry {
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) size: String,
}

impl FileEntry {
    fn is_audio(&self) -> bool {
        self.kind == "Audio"
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VisibleFolder {
    id: String,
    name: String,
    depth: usize,
    has_children: bool,
    expanded: bool,
    selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum FolderBrowserMessage {
    AddSource,
    SelectSource(String),
    ActivateFolder(String),
    ToggleFolder(String),
}

pub(super) fn folder_browser_view(state: &FolderBrowserState) -> ui::View<RebuildMessage> {
    ui::column([
        source_selector(state),
        ui::text("Folders").height(22.0).fill_width(),
        ui::scroll(
            ui::column(
                state
                    .visible_folders()
                    .into_iter()
                    .map(folder_row)
                    .collect::<Vec<_>>(),
            )
            .fill_width()
            .spacing(1.0),
        )
        .fill(),
        selected_folder_status(state),
    ])
    .spacing(3.0)
    .padding(4.0)
    .style(WidgetStyle::default())
    .fill_height()
}

fn source_selector(state: &FolderBrowserState) -> ui::View<RebuildMessage> {
    ui::column([
        ui::row([
            ui::text("Sources").height(20.0).fill_width(),
            ui::button("+")
                .primary()
                .message(RebuildMessage::FolderBrowser(
                    FolderBrowserMessage::AddSource,
                ))
                .key("source-add-button")
                .size(28.0, 22.0),
        ])
        .spacing(3.0)
        .fill_width()
        .height(24.0),
        ui::column(
            state
                .sources
                .iter()
                .map(|source| source_row(state, source))
                .collect::<Vec<_>>(),
        )
        .spacing(2.0)
        .fill_width(),
    ])
    .spacing(3.0)
    .fill_width()
}

fn source_row(state: &FolderBrowserState, source: &SourceEntry) -> ui::View<RebuildMessage> {
    let id = source.id.clone();
    let selected = state.selected_source == source.id;
    let mut row = ui::button(source.label.clone())
        .message(RebuildMessage::FolderBrowser(
            FolderBrowserMessage::SelectSource(id.clone()),
        ))
        .key(format!("source-row-{id}"))
        .fill_width()
        .height(24.0);
    if selected {
        row = row.primary();
    } else {
        row = row.subtle();
    }
    row.style(if selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .fill_width()
}

fn folder_row(folder: VisibleFolder) -> ui::View<RebuildMessage> {
    let id = folder.id.clone();
    let toggle_id = folder.id.clone();
    let expander = if folder.expanded { "[-]" } else { "[+]" };
    let label_message =
        RebuildMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(id.clone()));
    let mut label = ui::button(folder.name)
        .message(label_message)
        .key(format!("folder-label-{id}"))
        .fill_width()
        .height(22.0);
    if folder.selected {
        label = label.primary();
    } else {
        label = label.subtle();
    }

    ui::row([
        ui::text("").size((folder.depth as f32) * 12.0, 22.0),
        if folder.has_children {
            ui::button(expander)
                .mapped(move |message| match message {
                    ButtonMessage::Activate => RebuildMessage::FolderBrowser(
                        FolderBrowserMessage::ToggleFolder(toggle_id.clone()),
                    ),
                    ButtonMessage::SecondaryActivate { .. } | ButtonMessage::Drag(_) => {
                        RebuildMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(
                            toggle_id.clone(),
                        ))
                    }
                })
                .key(format!("folder-toggle-{id}"))
                .size(32.0, 22.0)
                .subtle()
        } else {
            ui::text("")
                .key(format!("folder-toggle-spacer-{id}"))
                .size(32.0, 22.0)
        },
        label,
    ])
    .key(format!("folder-row-{id}"))
    .style(if folder.selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .fill_width()
    .height(TREE_ROW_HEIGHT)
    .spacing(1.0)
    .hoverable()
}

fn selected_folder_status(state: &FolderBrowserState) -> ui::View<RebuildMessage> {
    let file_count = state.selected_files().len();
    let audio_count = state.selected_audio_files().len();
    let label = state
        .selected_folder()
        .map(|folder| {
            format!(
                "{} | {audio_count} audio | {file_count} item{}",
                folder.name,
                plural(file_count)
            )
        })
        .unwrap_or_else(|| String::from("No folder selected"));
    ui::text(label).height(20.0).fill_width().truncate()
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn default_root_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn load_root_folder(root: PathBuf) -> FolderEntry {
    load_folder(&root, 0).unwrap_or_else(|| FolderEntry {
        id: path_id(&root),
        name: folder_label(&root),
        children: Vec::new(),
        files: Vec::new(),
    })
}

fn load_folder(path: &Path, depth: usize) -> Option<FolderEntry> {
    if depth > MAX_SCAN_DEPTH {
        return None;
    }
    let entries = read_sorted_entries(path);
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .take(MAX_CHILD_FOLDERS)
        .filter_map(|entry| load_folder(entry, depth + 1))
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(file_entry)
        .collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

fn file_entry(path: &PathBuf) -> FileEntry {
    let metadata = fs::metadata(path).ok();
    let size_bytes = metadata.as_ref().map(fs::Metadata::len).unwrap_or_default();
    FileEntry {
        name: file_label(path),
        kind: file_kind(path),
        size: format_size(size_bytes),
    }
}

fn file_kind(path: &Path) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav" | "aif" | "aiff" | "flac" | "mp3") => String::from("Audio"),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp") => String::from("Image"),
        Some("json" | "txt" | "md" | "toml" | "rs") => String::from("Text"),
        _ => String::from("File"),
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} B")
    }
}

fn read_sorted_entries(path: &Path) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries = read_dir
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        file_label(a)
            .to_ascii_lowercase()
            .cmp(&file_label(b).to_ascii_lowercase())
    });
    entries
}

fn path_id(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn folder_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
