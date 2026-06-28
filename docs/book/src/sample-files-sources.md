# Sample Files and Source Operations

Wavecrate keeps source browsing, file moves, and source maintenance close to the sample list. Most file and source operations are available from the relevant context menu.

## Sample Actions

Right-click a sample row to open the sample context menu.

- **Reveal in Explorer:** open the selected sample in the system file browser.
- **Copy Path:** copy the sample path.
- **Duplicate Same:** create a duplicate at the same playback length.
- **Duplicate Double:** create a duplicate with doubled playback length.
- **Move to Trash:** move the sample to the configured trash location or system trash behavior.
- **Remove from collection:** remove the sample from the active collection when the row is being viewed through a collection.

Missing collection entries show cleanup actions instead of normal file actions:

- **Clean missing entry:** remove the missing collection record for one file.
- **Clean all missing in collection:** remove all missing records from the active collection.

## File Copy, Cut, and Paste

Use file shortcuts when the sample browser owns the current interaction:

- `Command-C` copies the selected sample files, unless a waveform play selection is the active copy target.
- `Command-X` cuts selected files for a move.
- `Command-V` pastes cut files into the selected folder.
- `Command-A` selects all listed samples.
- `Delete` or `Backspace` deletes the selected item.

Wavecrate preserves source safety rules during moves. If a target source is protected, Wavecrate prompts before routing the operation through the primary destination.

## Source Actions

Right-click a source in the Sources list to open source actions.

- **Open in Explorer:** open the source folder in the system file browser.
- **Copy Path:** copy the source root path.
- **New Folder:** create a folder under the source root.
- **Protect Source** or **Unprotect Source:** change whether Wavecrate treats the source as protected original material.
- **Set as Primary** or **Clear Primary:** choose the primary writable destination used by protected-source workflows.
- **Refresh Source:** rescan the source folder tree.
- **Process Source:** queue source processing work such as cache and similarity preparation.
- **Remove Source:** remove the source from Wavecrate.

## Folder Actions

Right-click a folder in the sidebar to open folder actions.

- **Open in Explorer:** open the folder in the system file browser.
- **Copy Path:** copy the folder path.
- **New Folder:** create a child folder.
- **Rename Folder:** rename the folder.
- **Lock Folder**, **Lock Folder Here**, or **Unlock Folder:** control whether the folder can be changed by normal file operations.
- **Delete Folder:** delete the folder after confirmation.

Folder locks are useful when a subtree should stay visible for auditioning but should not be modified during a fast browsing pass.

## Move Conflicts

When a move would collide with an existing file or folder, Wavecrate opens a conflict prompt. Choose the resolution for the current item, or apply the same resolution to the remaining conflicts when that option is available.

Use this prompt deliberately. File moves and deletes affect files on disk.
