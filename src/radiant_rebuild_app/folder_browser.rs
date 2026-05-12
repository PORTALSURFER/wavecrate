#![allow(missing_docs)]

use radiant::{
    prelude as ui,
    widgets::{WidgetStyle, WidgetTone},
};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use super::RebuildMessage;

const MAX_SCAN_DEPTH: usize = 3;
const MAX_CHILD_FOLDERS: usize = 80;
const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_DEPTH_INDENT: f32 = 4.0;

#[derive(Clone, Debug)]
pub(super) struct FolderBrowserState {
    selected_source: String,
    sources: Vec<SourceEntry>,
    selected_folder: String,
    selected_file: Option<String>,
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
        let mut sources = sources;
        let source_index = sources
            .iter()
            .position(|source| source.id == selected_source)
            .or(if sources.is_empty() { None } else { Some(0) })
            .expect("folder browser needs at least one source");
        let root_folder = load_root_folder(sources[source_index].root.clone());
        sources[source_index].root_folder = Some(root_folder.clone());
        let root_id = root_folder.id.clone();
        Self {
            selected_source: sources[source_index].id.clone(),
            sources,
            selected_folder: root_id.clone(),
            selected_file: None,
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

    pub(super) fn selected_file_id(&self) -> Option<&str> {
        self.selected_file.as_deref()
    }

    pub(super) fn scan_is_active(&self, source_id: &str, task_id: u64) -> bool {
        self.sources
            .iter()
            .any(|source| source.id == source_id && source.loading_task == Some(task_id))
    }

    pub(super) fn apply_message(&mut self, message: FolderBrowserMessage) {
        match message {
            FolderBrowserMessage::AddSource | FolderBrowserMessage::SelectSource(_) => {}
            FolderBrowserMessage::ActivateFolder(id) => self.activate_folder(id),
        }
    }

    pub(super) fn begin_add_source_path(
        &mut self,
        root: PathBuf,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        if let Some(index) = self.sources.iter().position(|source| source.root == root) {
            let id = self.sources[index].id.clone();
            return self.begin_select_source(id, task_id);
        }
        let id = path_id(&root);
        let label = folder_label(&root);
        let mut source = SourceEntry::new(id.clone(), label.clone(), root.clone());
        source.loading_task = Some(task_id);
        self.sources.push(source);
        self.select_pending_source(id.clone(), placeholder_folder(&root));
        Some(FolderScanRequest {
            task_id,
            source_id: id,
            label,
            root,
        })
    }

    pub(super) fn begin_select_source(
        &mut self,
        id: String,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        let index = self.sources.iter().position(|source| source.id == id)?;
        if self.selected_source == id && self.sources[index].root_folder.is_some() {
            return None;
        }
        if let Some(root_folder) = self.sources[index].root_folder.clone() {
            self.select_loaded_source(id, root_folder);
            return None;
        }
        if self.sources[index].loading_task.is_some() {
            let root = self.sources[index].root.clone();
            self.select_pending_source(id, placeholder_folder(&root));
            return None;
        }
        self.sources[index].loading_task = Some(task_id);
        let source = self.sources[index].clone();
        self.select_pending_source(source.id.clone(), placeholder_folder(&source.root));
        Some(FolderScanRequest {
            task_id,
            source_id: source.id,
            label: source.label,
            root: source.root,
        })
    }

    pub(super) fn apply_scan_finished(&mut self, result: FolderScanResult) -> bool {
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == result.source_id)
        else {
            return false;
        };
        if source.loading_task != Some(result.task_id) {
            return false;
        }
        let source_id = source.id.clone();
        let should_select = self.selected_source == source_id;
        source.loading_task = None;
        source.root_folder = Some(result.folder.clone());
        if should_select {
            self.select_loaded_source(source_id, result.folder);
        }
        true
    }

    #[cfg(test)]
    pub(super) fn apply_scan_discovered(&mut self, event: FolderScanDiscovery) -> bool {
        self.apply_scan_discovered_batch(FolderScanDiscoveryBatch {
            task_id: event.task_id,
            source_id: event.source_id.clone(),
            events: vec![event],
        })
    }

    pub(super) fn apply_scan_discovered_batch(&mut self, batch: FolderScanDiscoveryBatch) -> bool {
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == batch.source_id)
        else {
            return false;
        };
        if source.loading_task != Some(batch.task_id) {
            return false;
        }

        let root_folder = source
            .root_folder
            .get_or_insert_with(|| placeholder_folder(&source.root));
        let mut changed = false;
        for event in &batch.events {
            changed |= merge_scan_discovery(root_folder, event);
        }
        if changed && self.selected_source == batch.source_id {
            self.folders = vec![root_folder.clone()];
        }
        changed
    }

    fn select_pending_source(&mut self, id: String, folder: FolderEntry) {
        let root_id = folder.id.clone();
        self.selected_source = id;
        self.selected_folder = root_id.clone();
        self.selected_file = None;
        self.expanded_folders.clear();
        self.expanded_folders.insert(root_id);
        self.folders = vec![folder];
    }

    fn select_loaded_source(&mut self, id: String, root_folder: FolderEntry) {
        let root_id = root_folder.id.clone();
        self.selected_source = id;
        self.selected_folder = root_id.clone();
        self.selected_file = None;
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

    fn select_folder(&mut self, id: String) {
        self.selected_folder = id;
        self.selected_file = None;
    }

    pub(super) fn select_file(&mut self, id: String) {
        if self.selected_files().iter().any(|file| file.id == id) {
            self.selected_file = Some(id);
        }
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
    root_folder: Option<FolderEntry>,
    loading_task: Option<u64>,
}

impl SourceEntry {
    fn new(id: impl Into<String>, label: impl Into<String>, root: PathBuf) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            root,
            root_folder: None,
            loading_task: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderEntry {
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

    fn find_mut(&mut self, id: &str) -> Option<&mut FolderEntry> {
        if self.id == id {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_mut(id))
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileEntry {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) size: String,
    pub(super) size_bytes: u64,
    pub(super) modified: String,
    pub(super) modified_rank: u64,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanRequest {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) label: String,
    pub(super) root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanProgress {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) label: String,
    pub(super) phase: String,
    pub(super) completed: usize,
    pub(super) total: usize,
    pub(super) detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FolderScanItem {
    Folder(FolderEntry),
    File(FileEntry),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanDiscovery {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) parent_id: String,
    pub(super) item: FolderScanItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanDiscoveryBatch {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) events: Vec<FolderScanDiscovery>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderScanResult {
    pub(super) task_id: u64,
    pub(super) source_id: String,
    pub(super) label: String,
    pub(super) folder: FolderEntry,
    pub(super) file_count: usize,
    pub(super) folder_count: usize,
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
    let label = if source.loading_task.is_some() {
        format!("{} (scanning)", source.label)
    } else {
        source.label.clone()
    };
    let mut row = ui::button(label)
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
    let expander = if folder.expanded { "[-]" } else { "[+]" };
    let indent = (folder.depth as f32) * TREE_DEPTH_INDENT;
    let label_message =
        RebuildMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(id.clone()));
    let label_text = if folder.has_children {
        format!("{expander} {}", folder.name)
    } else {
        format!("    {}", folder.name)
    };
    let mut label = ui::button(label_text)
        .message(label_message)
        .key(format!("folder-row-button-{id}"))
        .align_text(ui::TextAlign::Left)
        .fill_width()
        .height(22.0);
    if folder.selected {
        label = label.primary();
    } else {
        label = label.subtle();
    }

    ui::row([ui::spacer().width(indent).height(22.0), label])
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

fn placeholder_folder(root: &Path) -> FolderEntry {
    FolderEntry {
        id: path_id(root),
        name: folder_label(root),
        children: Vec::new(),
        files: Vec::new(),
    }
}

pub(super) fn scan_source_with_progress(
    request: FolderScanRequest,
    mut progress: impl FnMut(FolderScanProgress),
    mut discovered: impl FnMut(FolderScanDiscovery),
) -> FolderScanResult {
    let mut scan = ScanProgressCounter {
        completed: 0,
        files: 0,
        folders: 0,
    };
    progress(FolderScanProgress {
        task_id: request.task_id,
        source_id: request.source_id.clone(),
        label: request.label.clone(),
        phase: String::from("Scanning"),
        completed: 0,
        total: 0,
        detail: request.root.display().to_string(),
    });
    let folder = load_folder_with_progress(
        &request.root,
        0,
        &request,
        &mut scan,
        &mut progress,
        &mut discovered,
    )
    .unwrap_or_else(|| placeholder_folder(&request.root));
    FolderScanResult {
        task_id: request.task_id,
        source_id: request.source_id,
        label: request.label,
        folder,
        file_count: scan.files,
        folder_count: scan.folders,
    }
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

struct ScanProgressCounter {
    completed: usize,
    files: usize,
    folders: usize,
}

fn load_folder_with_progress(
    path: &Path,
    depth: usize,
    request: &FolderScanRequest,
    scan: &mut ScanProgressCounter,
    progress: &mut impl FnMut(FolderScanProgress),
    discovered: &mut impl FnMut(FolderScanDiscovery),
) -> Option<FolderEntry> {
    if depth > MAX_SCAN_DEPTH {
        return None;
    }
    let entries = read_sorted_entries(path);
    let parent_id = path_id(path);
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .take(MAX_CHILD_FOLDERS)
        .filter_map(|entry| {
            scan.completed += 1;
            scan.folders += 1;
            maybe_report_scan_progress(entry, request, scan, progress);
            discovered(FolderScanDiscovery {
                task_id: request.task_id,
                source_id: request.source_id.clone(),
                parent_id: parent_id.clone(),
                item: FolderScanItem::Folder(placeholder_folder(entry)),
            });
            let child =
                load_folder_with_progress(entry, depth + 1, request, scan, progress, discovered)?;
            discovered(FolderScanDiscovery {
                task_id: request.task_id,
                source_id: request.source_id.clone(),
                parent_id: parent_id.clone(),
                item: FolderScanItem::Folder(child.clone()),
            });
            Some(child)
        })
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(|entry| {
            scan.completed += 1;
            scan.files += 1;
            maybe_report_scan_progress(entry, request, scan, progress);
            let file = file_entry(entry);
            discovered(FolderScanDiscovery {
                task_id: request.task_id,
                source_id: request.source_id.clone(),
                parent_id: parent_id.clone(),
                item: FolderScanItem::File(file.clone()),
            });
            file
        })
        .collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

fn maybe_report_scan_progress(
    path: &Path,
    request: &FolderScanRequest,
    scan: &ScanProgressCounter,
    progress: &mut impl FnMut(FolderScanProgress),
) {
    if scan.completed == 1 || scan.completed.is_multiple_of(64) {
        progress(FolderScanProgress {
            task_id: request.task_id,
            source_id: request.source_id.clone(),
            label: request.label.clone(),
            phase: String::from("Scanning"),
            completed: scan.completed,
            total: 0,
            detail: path.display().to_string(),
        });
    }
}

fn merge_scan_discovery(root: &mut FolderEntry, event: &FolderScanDiscovery) -> bool {
    let Some(parent) = root.find_mut(&event.parent_id) else {
        return false;
    };
    match &event.item {
        FolderScanItem::Folder(folder) => upsert_folder(&mut parent.children, folder.clone()),
        FolderScanItem::File(file) => upsert_file(&mut parent.files, file.clone()),
    }
}

fn upsert_folder(folders: &mut Vec<FolderEntry>, folder: FolderEntry) -> bool {
    match folders.binary_search_by(|candidate| {
        candidate
            .name
            .to_ascii_lowercase()
            .cmp(&folder.name.to_ascii_lowercase())
    }) {
        Ok(index) if folders[index] == folder => false,
        Ok(index) => {
            folders[index] = folder;
            true
        }
        Err(index) => {
            folders.insert(index, folder);
            true
        }
    }
}

fn upsert_file(files: &mut Vec<FileEntry>, file: FileEntry) -> bool {
    match files.binary_search_by(|candidate| {
        candidate
            .name
            .to_ascii_lowercase()
            .cmp(&file.name.to_ascii_lowercase())
    }) {
        Ok(index) if files[index] == file => false,
        Ok(index) => {
            files[index] = file;
            true
        }
        Err(index) => {
            files.insert(index, file);
            true
        }
    }
}

fn file_entry(path: &PathBuf) -> FileEntry {
    let metadata = fs::metadata(path).ok();
    let size_bytes = metadata.as_ref().map(fs::Metadata::len).unwrap_or_default();
    let modified = metadata.and_then(|metadata| metadata.modified().ok());
    FileEntry {
        id: path_id(path),
        name: file_label(path),
        kind: file_kind(path),
        size: format_size(size_bytes),
        size_bytes,
        modified: modified_label(modified),
        modified_rank: modified_rank(modified),
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
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} B")
    }
}

fn modified_label(modified: Option<SystemTime>) -> String {
    let Some(modified) = modified else {
        return String::from("-");
    };
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);
    let days = age.as_secs() / 86_400;
    if days == 0 {
        String::from("Today")
    } else if days == 1 {
        String::from("1 day")
    } else {
        format!("{days} days")
    }
}

fn modified_rank(modified: Option<SystemTime>) -> u64 {
    modified
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map(|age| age.as_secs())
        .unwrap_or(u64::MAX)
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

#[cfg(test)]
mod tests {
    use super::{path_id, scan_source_with_progress, FolderBrowserState, FolderScanDiscoveryBatch};
    use std::{fs, path::PathBuf};

    #[test]
    fn source_scan_installs_finished_tree_after_placeholder_selection() {
        let root = temp_source_root("radiant-rebuild-source-scan");
        fs::create_dir_all(root.join("drums")).expect("create nested folder");
        fs::write(root.join("drums").join("kick.wav"), [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::load_default();
        let request = browser
            .begin_add_source_path(root.clone(), 42)
            .expect("new source should request scan");

        assert_eq!(browser.root_path(), root.as_path());
        assert!(browser.selected_audio_files().is_empty());

        let mut progress_events = Vec::new();
        let mut discovery_events = Vec::new();
        let result = scan_source_with_progress(
            request,
            |progress| progress_events.push(progress),
            |event| discovery_events.push(event),
        );
        assert!(browser.apply_scan_finished(result));

        browser.begin_select_source(root.to_string_lossy().to_string(), 43);
        browser.activate_folder(path_id(&root.join("drums")));
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["kick.wav"]
        );
        assert!(progress_events
            .iter()
            .any(|progress| progress.phase == "Scanning" && progress.total == 0));
        assert!(!discovery_events.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn source_scan_discoveries_populate_selected_tree_before_finish() {
        let root = temp_source_root("radiant-rebuild-source-streaming");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create nested folder");
        fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::load_default();
        let request = browser
            .begin_add_source_path(root.clone(), 77)
            .expect("new source should request scan");

        let mut progress_events = Vec::new();
        let mut discovery_events = Vec::new();
        let result = scan_source_with_progress(
            request,
            |progress| progress_events.push(progress),
            |event| discovery_events.push(event),
        );

        for event in discovery_events {
            browser.apply_scan_discovered(event);
        }
        browser.activate_folder(path_id(&drums));
        assert_eq!(
            browser
                .selected_audio_files()
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["kick.wav"]
        );

        assert!(browser.apply_scan_finished(result));
        assert!(progress_events.iter().all(|progress| progress.total == 0));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn batched_scan_discoveries_clone_selected_tree_once_per_batch() {
        let root = temp_source_root("radiant-rebuild-source-batch");
        let drums = root.join("drums");
        fs::create_dir_all(&drums).expect("create nested folder");
        fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write wav");
        fs::write(drums.join("snare.wav"), [0_u8; 8]).expect("write wav");
        let mut browser = FolderBrowserState::load_default();
        let request = browser
            .begin_add_source_path(root.clone(), 88)
            .expect("new source should request scan");

        let mut discovery_events = Vec::new();
        let result =
            scan_source_with_progress(request, |_| {}, |event| discovery_events.push(event));
        assert!(
            browser.apply_scan_discovered_batch(FolderScanDiscoveryBatch {
                task_id: 88,
                source_id: path_id(&root),
                events: discovery_events,
            })
        );
        browser.activate_folder(path_id(&drums));
        assert_eq!(browser.selected_audio_files().len(), 2);

        assert!(browser.apply_scan_finished(result));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn visible_folder_depths_are_stable_for_siblings() {
        let root = temp_source_root("radiant-rebuild-folder-depths");
        for child in ["alpha", "beta", "gamma"] {
            fs::create_dir_all(root.join("parent").join(child)).expect("create nested folder");
        }
        let browser = FolderBrowserState::from_root(root.clone());
        let mut browser = browser;
        browser.activate_folder(path_id(&root.join("parent")));

        let sibling_depths = browser
            .visible_folders()
            .into_iter()
            .filter(|folder| ["alpha", "beta", "gamma"].contains(&folder.name.as_str()))
            .map(|folder| folder.depth)
            .collect::<Vec<_>>();

        assert_eq!(sibling_depths, vec![2, 2, 2]);
        let _ = fs::remove_dir_all(root);
    }

    fn temp_source_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }
}
