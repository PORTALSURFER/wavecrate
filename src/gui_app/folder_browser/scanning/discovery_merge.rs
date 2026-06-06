use super::{FileEntry, FolderEntry, FolderScanDiscovery, FolderScanItem};

pub(in crate::gui_app::folder_browser) fn merge_scan_discovery(
    root: &mut FolderEntry,
    event: &FolderScanDiscovery,
) -> bool {
    if let FolderScanItem::CompletedFolder(folder) = &event.item {
        if root.id == folder.id {
            if root == folder {
                return false;
            }
            *root = folder.clone();
            return true;
        }
        let Some(parent) = root.find_mut(&event.parent_id) else {
            return false;
        };
        return upsert_folder(&mut parent.children, folder.clone());
    }

    let Some(parent) = root.find_mut(&event.parent_id) else {
        return false;
    };
    match &event.item {
        FolderScanItem::Folder(folder) => {
            insert_discovered_folder(&mut parent.children, folder.clone())
        }
        FolderScanItem::CompletedFolder(_) => unreachable!("completed folders are handled above"),
        FolderScanItem::File(file) => upsert_file(&mut parent.files, file.clone()),
    }
}

fn insert_discovered_folder(folders: &mut Vec<FolderEntry>, folder: FolderEntry) -> bool {
    match folders.binary_search_by(|candidate| {
        candidate
            .name
            .to_ascii_lowercase()
            .cmp(&folder.name.to_ascii_lowercase())
    }) {
        Ok(_) => false,
        Err(index) => {
            folders.insert(index, folder);
            true
        }
    }
}

pub(in crate::gui_app::folder_browser) fn upsert_folder(
    folders: &mut Vec<FolderEntry>,
    folder: FolderEntry,
) -> bool {
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

pub(in crate::gui_app::folder_browser) fn upsert_file(
    files: &mut Vec<FileEntry>,
    file: FileEntry,
) -> bool {
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
