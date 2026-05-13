
# Wavecrate Project Target: Fast Sample Extraction, Curation, and Library Usage

## Working Product Name

The current working product name is **Wavecrate**.

Documentation should use **Wavecrate** when referring to the application and **sample file**, **audio file**, or **WAV file** when referring to files in the library.

## Vision

Wavecrate should become a focused desktop application for browsing, auditioning, extracting, editing, organizing, and using large audio sample libraries without breaking the user’s listening flow.

The application should help users turn messy audio material into a clean, structured, well-managed sample library. A user should be able to record long jams or experiments in tools such as Ableton Live, Bitwig, hardware recorders, or other audio software, save them as ordinary audio files, then use Wavecrate to find the useful parts, cut those parts into usable sample files, clean them up, name them consistently, tag them, rate them, organize them, and use them in music production.

Wavecrate is not a generic GUI library, a DAW, a plugin host, or a file-manager skin. It is a sample-focused workstation built around immediate auditioning, precise waveform interaction, destructive sample editing with safe warnings and undo, fast metadata workflows, similarity-based discovery, safe library hygiene, and reliable file management.

The target is a dense, responsive, predictable, and trustworthy tool for repeated professional use on large local sample collections.

## Product Identity

Wavecrate is three things working together:

1. A sample manager for browsing, tagging, filtering, rating, moving, renaming, and organizing sample files.
2. An auditioning tool for quickly moving through sounds, hearing them immediately, comparing related material, and deciding what is worth keeping.
3. A lightweight destructive sample editor for turning long or short audio files into clean, named, reusable samples without leaving the browsing flow.

The product is built around three primary workflows.

### 1. Sample Extraction Loop

This loop turns long audio files into usable smaller sample files.

There is no special “source recording” file type. A long jam recording is just an audio file in the library. It may be longer than other sample files, but Wavecrate should treat it as a normal sample file with the same browsing, auditioning, editing, tagging, rating, and file-management capabilities.

This workflow should support:

1. Open or scan long audio files such as hardware jams, synth experiments, resampling sessions, modular patches, field recordings, rehearsal takes, or other recordings.
2. Navigate quickly through large WAV or AIFF/AIF files.
3. Audition from any position without waiting.
4. Inspect the waveform at useful zoom levels.
5. Use analysis aids such as BPM detection, tempo grids, transient detection, silence detection, waveform overviews, and energy/section cues where practical.
6. Create play selections for auditioning sections.
7. Loop selected regions for closer listening.
8. Slide or move loop selections through the waveform to audition different parts quickly.
9. Mark interesting sections.
10. Adjust region boundaries accurately, including grid-aware or transient-aware adjustment where practical.
11. Extract selected regions into new audio files that sound exactly like what was auditioned.
12. Keep visual history marks for regions that were already extracted.
13. Continue searching the original audio file without losing flow.

Extraction is non-destructive because it creates a new sample file from a selected region. The original audio file remains in place unless the user performs an explicit destructive edit on it.

### 2. Sample Library Curation Loop

This loop turns many existing or newly extracted sample files into a clean, structured, well-managed library.

The user should be able to work through large lists of sample files, audition them quickly, decide what they are, decide whether they are useful, clean them up, name them consistently, tag them, rate them, and place them in the right location.

This workflow should support:

1. Browse, search, filter, or explore sample files by folder, tag, rating, metadata, age, review state, or similarity.
2. Select a sample file and hear it immediately.
3. Inspect and edit the waveform.
4. Apply micro-edits such as trimming, fades, silence removal, gain adjustment, normalization, and timing/BPM metadata correction where practical.
5. Add or remove tags quickly.
6. Rate or triage the sample file through the keep/trash rating system.
7. Use generated database display names based on tags, labels, prefixes, BPM, source, and other metadata.
8. Deliberately apply generated names to disk filenames when desired.
9. Move, copy, collect, export, or route sample files into the right folders.
10. Continue through the library without losing flow.

The purpose of this loop is to turn a pile of audio files into a clearly named, rated, tagged, searchable, and organized library.

### 3. Sample Library Usage Loop

This loop uses Wavecrate as a fast sample library while making music.

The user should be able to open Wavecrate during production, quickly find sounds that fit the track, audition related material, and move the chosen sample file into a DAW or other creative tool with minimal friction.

This workflow should support:

1. Browse, search, filter, or explore the library by folder, tag, rating, metadata, age, similarity, or 2D map position.
2. Audition samples immediately without disrupting the creative flow.
3. Set and lock a target audition BPM where useful.
4. Audition BPM-tagged samples warped to the target BPM where practical.
5. Compare groups of related sounds, including sounds near each other in the similarity map.
6. Quickly narrow results by sound type, character tags, BPM, rating, age, source, or similarity to a reference sample.
7. Preview enough waveform and metadata context to choose the right sound quickly.
8. Copy, drag, export, reveal, or otherwise hand off the selected sample file to a DAW or external tool.
9. Return to browsing and auditioning immediately.

The purpose of this loop is to make Wavecrate useful as an active production companion, similar in spirit to Sononym-style sample discovery, where finding and using the right sound is fast, fluid, and musically focused.

Everything in the application should support one or more of these workflows.

## Scope and Interpretation

This document is a durable product and architecture target for Wavecrate. It is not a one-shot implementation task.

Use it when reviewing features, refactors, UI changes, audio-engine work, database work, background processing, similarity systems, editing workflows, rating and triage behavior, naming systems, logging, diagnostics, and the Wavecrate/Radiant boundary.

Prefer incremental changes that move Wavecrate closer to this target while preserving working validation lanes. A change does not need to solve the whole product direction at once, but it should not make the intended direction harder.

## Core Product Goals

Wavecrate should provide:

1. Fast source and folder browsing for large sample libraries.
2. Correct handling of both short sample files and long audio files.
3. Support for WAV and AIFF/AIF as the target audio formats.
4. A sample extraction workflow for cutting useful regions out of long audio files into new sample files.
5. A sample library curation workflow for editing, tagging, rating, naming, moving, and cleaning up sample files.
6. A sample library usage workflow for finding, auditioning, and handing off sounds to a DAW or external creative tool.
7. Immediate sample auditioning from keyboard, mouse, selection changes, region selections, and similarity workflows.
8. A robust audio engine for playback, decoding, seeking, looping, range auditioning, edit auditioning, BPM-aware auditioning, and future extraction into a standalone audio-engine library.
9. Clear waveform visualization with precise playhead, cursor, play selection, edit selection, range, marker, loop, fade, grid, transient, extracted-region, and edit-state feedback.
10. Lightweight destructive sample editing for both macro extraction support and micro cleanup, including trimming, cutting, splitting, muting, fading, gain adjustment, normalization, silence removal, timing metadata correction, and range export.
11. Audio-analysis tools that help users find useful material, such as BPM detection, tempo grids, transient detection, silence detection, waveform overviews, aging/listen-history indicators, and similarity analysis where practical.
12. Reliable file, folder, export, copy, drag, trash, and DAW handoff workflows with clear recovery behavior where relevant.
13. Fast tagging, rating, filtering, metadata, indexing, display naming, and persistence workflows for large libraries.
14. A tag-category and database-display-name system that helps enforce consistent sample naming.
15. Library triage tools based on keep/trash ratings, aging, listen history, untagged filters, and temporary color collections.
16. Similarity analysis for discovering related sounds through browser filters and a visual 2D similarity map.
17. Dense, low-friction UI optimized for repeated scanning, auditioning, cutting, editing, rating, naming, tagging, organizing, finding, and using sounds.
18. Background scanning, decoding, waveform preparation, analysis, edit rendering, similarity indexing, and metadata work that never blocks the GUI thread.
19. Predictable status and progress feedback for long-running work.
20. Detailed logging and diagnostics for tracing failures, stalls, background jobs, and unexpected state changes.
21. A session-local undo/redo system for destructive edits, file operations, metadata operations, rating operations, and meaningful UI interaction state.
22. Clean separation between Wavecrate product logic, Radiant GUI-library logic, reusable audio-engine logic, and persistence/indexing logic.
23. Tests, diagnostics, and validation workflows that protect real user behavior, performance, and recovery guarantees.

## Product Principles

Wavecrate should optimize for:

- audition-first workflows
- realtime-feeling navigation and playback
- fast extraction from long audio files
- fast destructive sample preparation and cleanup
- dense, structural layouts over decorative surfaces
- clear warnings for destructive actions until the user opts into advanced destructive workflow mode
- session-local undoable operations wherever practical
- explicit status when work is running
- correctness under large libraries and slow disks
- recoverability for destructive edits and file operations through session undo and safe trash movement
- predictable keyboard and pointer behavior
- similarity-driven discovery without hiding the underlying file/library structure
- fast sample handoff to DAWs and external creative tools
- fast tagging, rating, naming, organizing, and filtering with minimal friction
- gentle library-hygiene workflows that encourage users to keep reviewing, rating, and cleaning their libraries
- traceable behavior when something goes wrong

Perceived stalls are product bugs. If a source scan, decode, rename, edit render, waveform update, BPM analysis, transient analysis, similarity analysis job, database/index update, logging flush, or metadata update can take noticeable time, it belongs off the GUI thread with clear state handoff back to the UI.

## Non-Goals

Wavecrate should not become:

- a general-purpose GUI framework
- a DAW or multitrack arrangement tool
- a plugin host
- a VST/AU/CLAP effects host
- a modular effects processing environment
- a managed central sample-library system that hides the real filesystem
- a database administration tool
- a collection of one-off UI experiments
- a machine-learning-based audio intelligence platform in the current target
- a general-purpose audio converter for arbitrary file formats

Supported audio formats for the current target are WAV and AIFF/AIF. MP3, FLAC, and other audio file formats are non-goals unless explicitly added later.

Wavecrate may contain DAW-like primitives such as waveforms, ranges, meters, fades, markers, loops, transport controls, tempo grids, transient markers, warp auditioning, and timeline overlays. These exist only to support sample exploration, extraction, preparation, and handoff.

Wavecrate may support copy, drag-and-drop, export, reveal-in-explorer, or file handoff into DAWs and other creative tools. This does not make Wavecrate a DAW integration layer or plugin host.

Wavecrate may include simple built-in sample operations such as fades, cuts, mutes, gain changes, normalization, trimming, splitting, silence removal, BPM metadata correction, audition-time warping, and exports. These are editing and auditioning tools, not an open-ended effects system.

Runtime model inference and ML-based similarity are not part of the current target.

## Platform Target

Wavecrate should be developed Windows-first.

The initial primary development and testing target is:

- Windows

Future platform targets may include:

- macOS
- Linux

Cross-platform support does not need to be implemented immediately, but the architecture should avoid unnecessary Windows-only assumptions in core product logic, audio-engine logic, persistence logic, and reusable GUI integration.

Platform-specific code should be isolated behind clear boundaries. File watching, drag-and-drop, DAW handoff, audio-device handling, paths, temporary recovery folders, trash-folder behavior, shell integration, and windowing behavior should be designed so future macOS and Linux support can be added without rewriting core systems.

## Audio Format and Channel Target

Wavecrate should support the following audio formats:

- WAV
- AIFF/AIF

MP3, FLAC, and other audio formats are out of scope for the current target.

This is a deliberate simplification. Wavecrate should prioritize reliable destructive editing, extraction, metadata writing, waveform analysis, and DAW-compatible file behavior over broad format support.

Wavecrate should align its practical audio-file compatibility with common DAW workflows, especially Ableton Live-style sample workflows. It should support common bit depths, sample rates, and channel layouts used in music production.

The target includes:

- common PCM bit depths such as 16-bit and 24-bit where supported by the format
- higher precision PCM or floating-point formats where they are part of normal DAW sample workflows
- common music-production sample rates such as 44.1 kHz, 48 kHz, 88.2 kHz, 96 kHz, and higher rates where practical
- mono files
- stereo files

Wavecrate should preserve source channel layout during normal destructive edits. Editing a stereo file in the mono-style editor must not collapse it to mono. In the mono-first workflow, edits should affect both stereo channels equally.

The waveform view should default to a mono-style overview because it is compact and fast for browsing. A later stereo split-view mode should show channels separately and allow channel-specific selection, editing, and extraction.

Implementation should be phased:

1. First, implement a complete mono-style editor that works correctly for mono and stereo files, with stereo edits applied equally to both channels.
2. Later, add stereo split-view editing where users can independently view, select, mark, edit, and extract left/right channels where useful.

## Core Domain Model

Wavecrate should keep the domain model simple and file-based.

### Audio File / Sample File

An audio file is the core object. Long recordings, short one-shots, loops, extracted slices, and edited sounds are all just audio files in the library.

There should be no functional distinction between a “source recording” and a “sample.” A long jam recording is simply a longer audio file. An extracted region is simply a new audio file. An edited file remains the same audio file unless the user duplicates or exports it.

### Region

A region is a marked time range inside an audio file.

Regions may be used for auditioning, looping, extraction, editing, markers, or visual history. Regions should be visually clear in the waveform view and should support precise boundary adjustment.

### Play Selection

A play selection is a waveform range used for auditioning and looping.

The play selection should be easy to create with the primary pointer interaction, such as left-click/drag. It should be easy to move or slide the play selection through the waveform so users can loop and audition different parts of a long file quickly.

### Edit Selection

An edit selection is a waveform range used specifically for destructive editing.

The edit selection should be distinct from the play selection and may be created with a separate interaction, such as right-click/drag. This allows the user to audition one area while editing another area, or to create a more precise edit range without disturbing the audition loop.

Edit commands should follow this priority:

1. If an edit selection exists, edit commands apply to the edit selection.
2. If no edit selection exists but a play selection exists, edit commands apply to the play selection.
3. If neither selection exists, edit commands apply according to the command’s explicit behavior, such as whole-file normalization or current-cursor operations.

The play selection and edit selection should not interfere with each other visually or behaviorally.

### Extracted Region History

When a user extracts a region from a longer audio file into a new sample file, the original audio file should show a visual history mark for the extracted range where practical.

This helps users avoid repeatedly extracting the same section and makes it easier to continue scanning unexplored parts of a long file.

### Wavecrate Sample ID

Every indexed audio file should have a stable internal Wavecrate Sample ID.

The filename is not the stable identity. Disk filenames may change, display names may change, and generated names may change. The internal Sample ID should remain the stable identifier for Wavecrate metadata, rating history, analysis cache, similarity data, aging/listen history, and other persisted state.

Wavecrate should store this ID in the database and should also attempt to embed it directly into supported audio files.

For WAV files, Wavecrate should use a custom RIFF chunk for the primary embedded ID. The chunk should be application-specific, versioned, and small. Unknown RIFF chunks should be preserved where practical when Wavecrate rewrites the file.

For AIFF/AIF files, Wavecrate should use an application-specific metadata chunk where practical, following the same principle: small, versioned, and safe to ignore by other applications.

The embedded metadata should contain at minimum:

- Wavecrate Sample ID
- metadata schema version
- optional creation/update timestamp
- optional checksum or file fingerprint reference where useful

The database remains the authoritative fallback. If embedded metadata is missing, stripped, duplicated, or conflicting, Wavecrate should reconcile using the database record, file path, file fingerprint, audio properties, and user-visible recovery behavior.

Wavecrate must validate embedded IDs by round-tripping files through common DAW workflows, especially Ableton Live and Bitwig. Validation should confirm that audio data, timing, channel layout, readability, and metadata recovery remain intact.

If embedded ID writing proves unsafe for WAV or AIFF/AIF, Wavecrate should disable embedding for that format and treat the target as needing adjustment rather than risking file damage.

### Disk Filename

The disk filename is the actual file name on disk.

Wavecrate should be able to show and manipulate real disk filenames, but the disk filename should not be treated as the only or primary product label.

### Display Name / Database Name

The display name is the clean name shown by Wavecrate based on metadata such as tags, tag categories, label, prefix, BPM, source, and uniqueness suffix.

Wavecrate should support view modes that show either the real disk filename or the generated database/display name. Applying a generated display name to the disk filename should be a deliberate file operation, not an invisible background behavior.

### Label

A label is a user-editable text field that can act as the human-readable identity of a sample file inside Wavecrate.

The label may default to the original filename without extension. Users can refine it later. The label can be used in generated display names.

### Prefix

A prefix is a structured metadata field that can identify an artist, creator, project, pack, session, or other user-selected namespace.

The prefix can be used in generated display names and can help users distinguish their own sounds from other material.

### Tags

Tags describe sound type, structure, character, source, or other useful properties.

Tags should be category-aware where practical so they can support filtering and naming in a predictable order.

### Rating State

Rating state is based on the keep/trash rating system. It is separate from tags.

Rating state should be fast to apply, visually obvious, persistent, and useful for sorting, filtering, cleanup, and library hygiene.

### Aging / Listen History

Aging and listen history track when a sample file was last auditioned, whether it has never been listened to, and how often it is used or auditioned.

This should help users find new files, neglected files, recently used files, and frequently used files.

### Temporary Color Collections

Temporary color collections are lightweight shelves inspired by Ableton-style color collections.

They are not the primary file organization system and do not replace folders. They should help users temporarily collect files for a project, task, comparison pass, cleanup pass, or sound selection workflow.

## File Ownership and Source of Truth

Wavecrate should remain grounded in the real filesystem.

The filesystem is the source of truth for file existence, folder structure, and physical file location. Wavecrate indexes existing folders and operates on real files in those folders.

Wavecrate should not require a central managed library folder. Users should be able to point Wavecrate at existing folders and work with the actual files there.

Wavecrate may create, move, rename, duplicate, export, collect, copy, or trash files, but those operations should physically happen on disk. If a file is moved from one folder to another in Wavecrate, the file should actually move on disk.

The database is the source of truth for Wavecrate-specific metadata, including tags, ratings, display names, labels, prefixes, analysis results, similarity descriptors, waveform cache references, aging/listen history, extracted-region history, and session undo/history state where appropriate.

Wavecrate should detect and handle:

- missing files
- moved files
- renamed files
- changed files
- duplicate files
- stale metadata
- stale analysis cache
- stale waveform cache
- database records pointing to unavailable files
- files without embedded Wavecrate Sample IDs
- files with embedded IDs that conflict with database state

When filesystem and database state diverge, Wavecrate should provide clear reconciliation behavior instead of silently losing user metadata or pretending missing files still exist.

## Destructive Editing, Duplication, and Session Undo Policy

Wavecrate is intentionally a fast destructive sample editor.

When the user performs an edit such as mute, cut, delete, fade, normalize, gain change, trim, or silence removal, the edit should modify the current audio file in place unless the command is explicitly an extraction, export, duplicate, or copy operation.

This is a deliberate product direction. Wavecrate should not automatically create endless duplicate versions for every edit. Users who want to preserve an original file should duplicate it before editing.

Wavecrate should therefore provide a clear duplicate-file command that makes it easy to create a backup or alternate version before destructive editing.

Destructive editing should have two user-facing safety modes.

### Default Safety Mode

In default mode, destructive edits should warn the user clearly before modifying the file in place.

The warning should explain that the edit will modify the audio file on disk. The user should be able to confirm, cancel, and optionally enable advanced destructive workflow mode if they understand the behavior.

### Advanced Destructive Workflow / YOLO Mode

Advanced users should be able to enable a persistent destructive workflow mode, informally called YOLO mode.

When this mode is enabled, Wavecrate should stop showing repetitive destructive-edit warnings and should allow fast in-place editing. Users in this mode are expected to duplicate files themselves when they want backups.

YOLO mode should be explicit, persistent, and easy to identify in the UI. It should not be enabled accidentally.

### Session-Local Undo and Redo

Even though edits are destructive, Wavecrate should have a deeply integrated undo/redo system.

Undo/redo is session-local. It only needs to work while the application is running. Undo history does not need to persist across application restarts.

Undo/redo should cover as much of the application as practical, including:

- destructive audio edits
- selection changes
- play selection changes
- edit selection changes
- marker changes
- region changes
- extraction actions
- duplicate actions
- rename actions
- generated-name application actions
- tag changes
- rating changes
- temporary color collection changes
- metadata changes
- move/copy/export/trash operations
- folder operations where practical

Undo/redo should be transaction-based. A user action should either complete as a coherent operation or fail in a recoverable way. Partial failures should be logged and reported clearly.

The undo history should be deep enough to be useful while remaining bounded to avoid excessive memory or disk usage. A reasonable initial target is at least 50 meaningful user actions where practical, with the architecture allowing this limit to be adjusted later.

### Temporary Recovery Files

For destructive audio edits and file operations that need recovery support, Wavecrate should use temporary recovery files.

Wavecrate should create a standard application temp/recovery folder appropriate for the operating system. On Windows, this should live in a conventional user/application data or temp location, not inside arbitrary sample-library folders.

Temporary recovery files should support undo for the current session only. Files from previous sessions should be considered stale and eligible for cleanup.

Wavecrate should not slow down shutdown by doing heavy temp cleanup on exit. Instead, a background worker should clean stale recovery files while the application is running, such as on startup or during idle/background maintenance. The cleanup worker should remove old files that do not belong to the current session and are no longer valid for undo.

Recovery-file creation, cleanup, and failures should be logged.

## Extraction Policy

Extraction is not the same as destructive editing.

When a user selects a region of an audio file and extracts it, Wavecrate should create a new sample file containing that region. The original file should remain unchanged unless the user separately performs a destructive edit on it.

If audition-time processing is active, such as target-BPM warping, extraction should create what the user hears. If a user auditions a warped loop and extracts it, the created sample file should bake the auditioned warp result into the new file.

Extraction should support:

- extracting the current play selection
- extracting the current edit selection where appropriate
- drag-out extraction where practical
- enter/confirm extraction where practical
- naming or labeling extracted files
- applying tags and metadata to extracted files
- assigning a new Wavecrate Sample ID to extracted files
- writing embedded Sample ID metadata where practical
- recording extracted-region history on the original file
- avoiding filename collisions through predictable numbering

## Trash and Cleanup Policy

Wavecrate should use a configured trash folder for cleanup.

Trashing a file means moving it from its current source folder into the configured trash folder. It does not mean permanent deletion.

Wavecrate should not permanently delete files automatically. Permanent deletion is outside the normal cleanup workflow.

The trash folder must be configured before Wavecrate can automatically move files there. If no trash folder is configured, trash actions should be blocked, downgraded to rating-only behavior, or prompt the user to configure the trash folder.

Trash workflow should support:

- explicit trash actions
- automatic trash movement when a file reaches the rejected threshold
- undo for recent trash moves during the current session
- clear logging of trash moves
- clear UI indication that a file was moved out of the active library
- predictable behavior if a destination file already exists in the trash folder

No special long-term untrash system is required beyond current-session undo and the fact that files remain physically present in the trash folder. Users can move files back manually if needed.

## Keep/Trash Rating System

Wavecrate should use a fast keep/trash rating scale rather than a generic star-rating system.

All files start unrated.

The user should be able to quickly apply a keep mark or a trash mark while auditioning. These marks should be visually clear in the sample browser.

The rating state should move in the direction of the latest rating action. It should not use simple cancellation back to unrated once a file has been rated.

The target rating behavior is:

- Unrated plus keep becomes keep level 1.
- Keep level 1 plus keep becomes keep level 2.
- Keep level 2 plus keep becomes keep level 3.
- Keep level 3 plus keep becomes accepted/favorite/locked.
- Unrated plus trash becomes trash level 1.
- Trash level 1 plus trash becomes trash level 2.
- Trash level 2 plus trash becomes trash level 3.
- Trash level 3 plus trash becomes rejected and eligible for trash-folder movement.
- Applying trash to a keep-rated file should move it to trash level 1, not back to unrated.
- Applying keep to a trash-rated file should move it to keep level 1, not back to unrated.

Accepted/favorite files should be visually distinct and locked from further rating changes. To change the rating of an accepted file, the user must manually unlock it first.

Rejected files should be moved to the configured trash folder automatically when cleanup policy allows it.

The UI should make partial ratings clear. A file with one, two, or three keep/trash marks should visually communicate that it is moving toward accepted or rejected state.

Rating controls should be keyboard-friendly and should not interrupt auditioning.

## Aging and Listen-History System

Wavecrate should track listen history and use it to support library hygiene.

The system should track information such as:

- never auditioned
- last auditioned recently
- last auditioned days ago
- last auditioned weeks ago
- last auditioned months ago
- audition count where useful

The sample browser should visually communicate aging state. Files that have not been listened to recently may be visually faded, greyed, or otherwise marked so users can quickly identify neglected material.

Aging should support sorting and filtering. Users should be able to find files that are never listened to, recently listened to, not listened to for weeks/months, or heavily auditioned.

Aging state should combine with tags, ratings, similarity, folders, and text filters.

## Temporary Color Collections

Wavecrate should support temporary color collections similar in spirit to Ableton-style collections.

The target is a small fixed set of color collections, such as 10 slots. Users should be able to name these color collections and quickly assign sample files to them.

Color collections should behave like temporary shelves, not as the main organization model.

Possible uses include:

- collecting samples for a current track
- collecting kick candidates
- collecting sounds to clean up later
- collecting samples to move into a folder
- comparing similar sounds
- staging files before export or DAW handoff

Color collections should be fast to assign, clear in the UI, sortable/filterable, and easy to clear when no longer needed.

Physical folders remain the main durable organization system. Color collections are a lightweight metadata layer for temporary workflow support.

## Sample Browsing, Auditioning, and Usage Target

Browsing, auditioning, and using samples should feel immediate, even on large libraries.

Wavecrate should support:

- fast source and folder navigation
- incremental source scanning
- responsive sample lists for large folders and libraries
- keyboard-first movement through samples
- pointer-based selection and auditioning
- immediate playback on selection when enabled
- quick restart, stop, seek, loop, and range playback
- target-BPM auditioning where practical
- copy, drag, export, reveal, or handoff of selected sample files into DAWs and external tools
- visible selected, playing, loading, unavailable, failed, edited, unsaved, unreviewed, aged, keep-rated, trash-rated, accepted, rejected, and trashed states
- stable behavior when the user moves quickly through many samples

Fast browsing is more important than decorative UI. The user should be able to scan thousands of files, hear sounds quickly, find sounds that fit a production context, and move chosen samples into their DAW without breaking flow.

## DAW and External Tool Handoff Target

Wavecrate should make it easy to use found sounds in music production.

The initial target is practical Windows-first file-based handoff, not deep DAW integration.

Wavecrate should support workflows such as:

- drag a sample file into a DAW
- copy a sample file
- copy a file path
- reveal a sample file in Explorer
- export or copy a prepared sample file to a chosen folder
- extract a region and immediately drag/copy/export the new file
- hand off the exact audio the user auditioned, including baked audition-time warp when extraction/export requires it

Initial implementation should prioritize drag-and-drop into DAWs such as Ableton Live and Bitwig where practical, plus reveal-in-Explorer and copy-file-path.

Handoff should be fast and predictable. It should not require users to understand Wavecrate internals.

Future macOS/Linux handoff should be possible through platform-specific adapters without rewriting core product logic.

## Sample Editing and Cleanup Target

Wavecrate should support lightweight, sample-centric editing directly in the waveform view.

Core editing workflows should include:

- trimming start and end points
- cutting or deleting ranges
- splitting a sample file into regions or pieces
- muting or silencing selected sections
- removing or reducing unwanted silence
- adding fade-ins and fade-outs
- adjusting fade length and curve where practical
- normalizing a whole sample file or selected region
- applying simple gain changes to a sample file or range
- correcting or setting BPM metadata where practical
- aligning loop boundaries to grid or transients where practical
- creating named markers or regions
- naming selected regions or extracted slices
- rating selected regions or extracted slices
- exporting selected ranges or slices as new files
- saving destructive edits with clear warning/YOLO behavior

Editing should be designed around fast preparation of individual sample files and long recordings. The user should be able to clean up a tail, remove silence, isolate a hit, split a loop into useful pieces, cut a long jam into named regions, normalize a quiet file, mute an unwanted section, and export prepared versions without leaving the browsing flow.

The first complete editing target is mono-style editing. Stereo files should retain their stereo data, but edits should affect both channels equally until stereo split-view editing is added later.

## BPM, Grid, Warp, and Timing Target

Wavecrate should support BPM-aware auditioning and extraction.

The system should support:

- BPM detection where practical
- manually setting or correcting BPM metadata
- tempo-grid display when BPM is known
- grid-aware selection and loop adjustment
- transient-aware loop boundary adjustment where practical
- setting a target audition BPM
- locking the target audition BPM
- auditioning BPM-tagged loops warped to the target BPM where practical
- extracting or exporting warped audio exactly as heard when the user extracts from a warped audition state

Time-stretching/warping is part of the audition and extraction workflow, not a general DAW-style warping environment.

The initial warp target should be practical and beat-oriented, similar in spirit to a simple beats-style warp mode. Special high-quality, multi-algorithm, formant-preserving, or advanced warping modes are non-goals for the current target.

Wavecrate should not become a full arrangement, clip-warping, or multitrack timing editor. The goal is to audition loops in a production-relevant tempo context and to bake that auditioned result into a new sample file when the user extracts or exports it.

## Audio Engine Target

Wavecrate should include a robust audio engine for fast, predictable auditioning, destructive editing, extraction, and sample-library usage.

The audio engine should support:

- immediate playback of selected sample files
- low-latency start, stop, seek, loop, and retrigger behavior
- accurate playback of play selections, edit selections, regions, markers, slices, loops, and pending edit operations
- target-BPM auditioning where practical
- reliable decoding of WAV and AIFF/AIF
- editing/write support for WAV and AIFF/AIF
- reusable decoded audio, waveform, and peak data where practical
- background preparation of audio data without blocking the GUI thread
- stale-result-safe handoff when selection or edit state changes while work is running
- clear error reporting for unsupported files, decode failures, device failures, metadata write failures, and playback failures
- diagnostic events that make playback, decode, seek, loop, warp, render, and device bugs traceable

The audio engine should be designed with future extraction in mind. Wavecrate can drive its immediate requirements, but the core engine should avoid unnecessary dependency on Wavecrate-specific UI, metadata, tag, similarity, naming, or source-management concepts.

The practical target is a clean internal boundary today that allows reusable audio-engine components to become their own standalone library later without rewriting the playback, editing, and extraction stack.

## Similarity and Discovery Target

Wavecrate should include a similarity engine for finding related sounds by audio character, not only by filename, folder, or manually assigned tags.

The similarity system should remain local, deterministic, cacheable, and optimized for fast sample exploration.

The default engine should use:

- analysis-normalized audio
- stable versioned DSP feature vectors
- L2-normalized descriptor embeddings
- cosine approximate-nearest-neighbor search
- lightweight reranking that balances broad timbral similarity with practical envelope and loudness cues

Runtime model inference is not part of the current target.

Map projection and clustering are secondary exploration aids built from the same persisted embeddings. They should preserve compatibility while using implementation names that reflect the current projection method.

The similarity system should support:

- background audio analysis for feature extraction
- persistent storage of analysis results
- stale-result-safe updates when files change or analysis jobs complete out of order
- finding sample files similar to the selected sample file
- sorting the browser by similarity to a selected reference file
- visual similarity indicators in the sample browser
- filtering the sample list by similarity to a selected reference sound
- combining similarity filters with tags, ratings, folders, search text, metadata, aging/listen state, and file properties
- browsing a 2D similarity map for visual discovery of related sounds
- clear status for analysis progress, unavailable analysis, stale analysis, and failed analysis

The 2D similarity map should provide an XO-like exploration surface for samples. Similar-sounding samples should appear near each other so the user can scan clusters, audition nearby files, and discover variations quickly.

The sample browser should provide Sononym-like similarity workflows. The user should be able to choose a reference sample and quickly find related kicks, hats, loops, textures, impacts, vocals, or other sound types through list-based filters and sorting.

Similarity should assist discovery without becoming opaque. The UI should make it clear when results come from similarity analysis, when analysis is still pending, and when ordinary file, tag, rating, aging, search, or metadata filters are also active.

Similarity should support both management and production use. It should help users clean up libraries, but it should also help music producers quickly audition clusters of related sounds and choose material to drag, copy, export, or otherwise hand off to a DAW.

## Tagging and Metadata Target

Tagging should be fast, flexible, category-aware, and low-friction.

Wavecrate should support:

- adding and removing tags quickly from one or many sample files or regions
- custom tags created directly by the user
- custom tags added to existing tag categories
- a dynamic tag library built from tags already used in the library
- autocomplete suggestions based on existing tags
- clear pill visuals for assigned tags
- keyboard-friendly tag entry and removal
- typo-resistant tag workflows where practical
- tag-based filtering in the sample browser
- combination filters across tags, ratings, age, triage states, similarity, folders, search text, and metadata

The tag library should not require a fixed predefined taxonomy, but Wavecrate should provide clear built-in tag categories so naming, filtering, and visual organization remain structured.

Recommended tag categories:

- **Structure/type:** one-shot, loop, phrase, texture, riser, impact, fill, layer, tool, recording.
- **Sound/source:** kick, snare, clap, hat, ride, bass, synth, stab, vocal, percussion, noise, field, modular, drum loop.
- **Character/flavor:** distorted, clean, soft, hard, dark, bright, noisy, metallic, deep, punchy, loose, tight, raw, warm.
- **Technical/music metadata:** BPM, key, pitch, length, mono/stereo, source session where practical.
- **Prefix/namespace:** artist, creator, project, pack, or user-defined prefix where useful.

Users should be able to add their own tags to these categories. The categories should guide the system without preventing custom vocabulary.

For example, if the user starts typing `ki`, the tag input should suggest an existing `kick` tag. Selecting the suggestion should apply the existing tag instead of creating a duplicate spelling variant. If the intended tag does not exist, the user should be able to create it immediately.

Tagging UI should favor compact, readable pill components. Tags should be visible enough to be useful during scanning, but not so visually heavy that they dominate the sample list or waveform/editor area.

## Display Naming and Disk Rename Target

Wavecrate should make it easy to create a consistently named and organized sample library while avoiding accidental breakage of external file references.

Wavecrate should distinguish between:

- the actual disk filename
- the generated display/database name
- the user-editable label
- the stable internal Wavecrate Sample ID

The generated display name should be based on structured metadata such as:

- prefix/namespace
- structure/type tag
- sound/source tag
- character/flavor tags
- BPM where known
- key or pitch where known
- user label
- source/session information where useful
- uniqueness suffix

A recommended initial naming order is:

```text
[prefix]_[type]_[sound]_[character-tags]_[bpm]_[label]_[number]
````

Examples:

```text
wanja_loop_kick_distorted_raw_140_metal-floor_001.wav
wanja_oneshot_hat_bright_noisy_short_017.wav
modular_texture_noise_dark_wide_006.wav
```

Missing metadata should simply be omitted. The naming system should not invent misleading metadata.

The generated name should be available as a Wavecrate display mode in the browser. Users should be able to switch between viewing the real disk filename and the generated display/database name.

Applying the generated display name to the actual disk filename should be an explicit action. This action may be triggered from a context menu, command, batch operation, or future workflow mode, but it should be understood as a real filesystem rename.

Renaming should:

* preserve the stable internal Wavecrate Sample ID
* avoid collisions through predictable numbering
* handle invalid filename characters
* be undoable during the current session where practical
* be logged clearly
* warn the user where external DAW/project references may break

A fully automatic disk auto-rename mode is not the default target. The safer target is automatic database/display naming plus explicit apply-to-disk rename.

## Physical Organization Target

Wavecrate should rely primarily on real files and folders.

Users organize durable collections by creating folders and moving/copying sample files into those folders. Wavecrate should expose this clearly rather than hiding it behind a central managed library.

Wavecrate should support:

* creating folders
* moving files between folders
* copying files between folders
* duplicating files
* renaming files
* applying generated display names to disk filenames
* exporting extracted or edited files to chosen folders
* moving rejected files to the configured trash folder
* revealing files in Explorer

The folder tree should reflect actual folders on disk. The sample browser should reflect actual files on disk plus Wavecrate-specific metadata from the database.

## Database, Persistence, and Indexing Target

Wavecrate should use a fast, robust persistence layer for sources, tags, ratings, age/listen history, metadata, display names, labels, prefixes, embedded Sample ID state, analysis state, similarity data, waveform cache state, extracted-region history, edit state where needed, and current-session recovery information.

The database and indexing system should support:

* responsive queries for large sample libraries
* fast tag filtering, rating filtering, age filtering, text filtering, metadata filtering, and similarity lookup
* efficient updates when tags, ratings, age state, metadata, names, analysis results, source contents, embedded IDs, or edit state change
* clear schema ownership and migration behavior
* transactional or recovery-safe updates for user-trust surfaces
* consistency between persisted state and UI projection
* diagnostic information for failed writes, stale records, migration issues, metadata embedding failures, and indexing problems

Tag, rating, aging, naming, and metadata workflows should remain fast as the library grows. Filtering should feel interactive, and persistence should be treated as part of the product performance surface rather than a passive storage detail.

## File Operations and Recovery Target

Wavecrate should treat the filesystem, source database, edit state, tags, ratings, names, similarity index, action history, and logs as user-trust surfaces.

File, folder, metadata, rename, and edit operations should:

* operate on real files and folders
* preserve metadata where possible
* avoid silent data loss
* make destructive operations explicit unless YOLO mode is enabled
* use session-local undoable transactions where practical
* use clear recovery paths for partial failure
* keep UI projection and persisted state aligned
* report failures in user-actionable terms
* avoid blocking the GUI while work is planned or executed
* make derived files, exports, overwritten files, renamed files, trashed files, duplicated files, and moved files understandable to the user

Background workers should be cancellable or stale-result-safe when the user changes selection, sources, folders, filters, tags, ratings, names, edit selections, play selections, or map views before work completes.

Similarity data, BPM data, transient data, aging data, waveform data, embedded Sample ID state, and display-name data should be invalidated or refreshed when the underlying file changes. Tag, rating, naming, age, and metadata updates should be persisted predictably and reflected consistently across browser, waveform/editor, filters, and map views.

## Logging and Diagnostics Target

Wavecrate should have detailed logging throughout the application so bugs, stalls, failed operations, and unexpected state changes can be traced.

Logging should cover:

* application startup and shutdown
* source scanning and file discovery
* database reads, writes, migrations, and indexing work
* embedded Sample ID reads, writes, conflicts, and failures
* audio device setup, decoding, playback, seeking, looping, warping, and render failures
* waveform preparation and cache usage
* BPM detection, grid generation, transient detection, silence detection, and analysis failures
* play selection and edit selection state changes where useful
* extraction actions and extracted-region history
* destructive edit creation, rendering, overwrite, and failure recovery
* session undo/redo transaction creation, success, failure, and rollback
* temp recovery file creation, usage, cleanup, and failure
* tag creation, autocomplete, filtering, display naming, disk rename application, and metadata persistence
* rating, aging/listen history, review-state, and trash-workflow changes
* similarity analysis, indexing, sorting, filtering, and 2D map generation
* temporary color collection changes
* background job lifecycle, cancellation, stale completions, and handoff back to the UI
* file operations such as rename, duplicate, move, copy, trash, restore via undo, collect, reveal, and export
* DAW/external handoff actions
* user-actionable errors and internal diagnostic details

Logs should be structured enough to support debugging. Important operations should include useful context such as source IDs, Wavecrate Sample IDs, region IDs, file paths where safe, job IDs, selection versions, edit versions, action transaction IDs, cache keys, timing information, and failure reasons.

Logging should help answer practical debugging questions: what happened, in what order, on which thread or worker, for which file or source, how long it took, and why it failed. Logging itself must not cause UI stalls, excessive disk writes, or unreadable noise.

## UI Target

The Wavecrate UI should be compact, stable, and optimized for scanning, auditioning, extracting, editing, naming, tagging, rating, organizing, and using sample files.

The UI should provide:

* tight margins and predictable panel geometry
* resizable sidebars and durable split positions
* folder tree, sample list, waveform/editor, tags, ratings, age indicators, filters, similarity controls, naming controls, color collections, and status surfaces available without excessive navigation
* clear selected, playing, loading, edited, unsaved, failed, analyzing, unreviewed, aged, keep-rated, trash-rated, accepted, rejected, trashed, and unavailable states
* keyboard navigation for common browse, audition, loop, mark, cut, edit, duplicate, rename, tag, rate, collect, triage, filter, copy, and handoff actions
* pointer interactions that show exact positions, play selections, edit selections, ranges, fades, markers, loops, grids, transients, extracted-region marks, and edit handles
* compact tag pills and autocomplete controls
* region, marker, loop, grid, transient, BPM, and target-BPM controls where relevant
* fast keep/trash rating controls that do not interrupt auditioning
* aging/listen-history visuals that make neglected or recently used files clear
* generated-name display mode and disk-filename display mode
* deliberate apply-display-name-to-disk controls
* current destructive-edit safety mode indicator, especially when YOLO mode is enabled
* mono-style waveform view by default
* future stereo split-view mode for independent channel inspection and editing
* copy, drag, export, reveal, and DAW handoff controls that are fast and predictable
* similarity controls that can be used from both list and map workflows
* concise status surfaces for long-running work
* no marketing-style hero layout, decorative cards, or ornamental whitespace

The waveform/editor should be a primary work surface, not a decorative preview. It should support precise cursor movement, play selection, edit selection, range selection, playhead display, loop display, edit handles, fades, markers, grids, transient cues, extracted-region history, and clear feedback for destructive edits.

Status bars should stay concise. Long-running operations should report what is happening without monopolizing the interface.

## Performance Target

Wavecrate should handle large sources without freezing or rebuilding unnecessary UI work.

Important performance rules:

* source scanning must stream discoveries to the UI incrementally
* long audio files must remain navigable without loading unnecessary full-resolution UI data at once
* folder and sample views should be virtualized or windowed for large datasets
* sample decode and waveform preparation must run in background work
* BPM detection, transient detection, silence detection, similarity analysis, and 2D map generation must run in background work
* destructive edit rendering and file overwrite work must avoid blocking the GUI thread
* normalization scans, extraction renders, warp renders, and derived-file exports must run in background work
* database writes, indexing, migrations, metadata embedding, and maintenance work must not block the GUI thread
* stale background completions must not overwrite newer selection, edit, filter, tag, rating, naming, database, aging, or triage state
* playback should reuse already-loaded bytes where possible
* waveform previews should reuse cached analysis and peak data where practical
* tag, rating, metadata, naming, aging, triage, and similarity filters should remain responsive on large libraries through indexing, caching, or incremental computation
* recovery-file management should not block shutdown or normal UI interaction
* repaint requests should match actual visual changes
* GPU waveform overlays should be composited by Radiant, not faked with extra application surfaces
* logging should provide useful traceability without becoming a performance bottleneck

Performance-sensitive paths should have focused tests, diagnostics, examples, benchmarks, or manual validation notes when practical.

## Architecture Boundaries

Wavecrate owns product behavior. Radiant owns reusable GUI capability. The audio engine should move toward reusable audio capability. The persistence layer should provide fast, reliable storage and indexing without leaking database details into every product workflow.

### Wavecrate Owns

* source configuration and sample-library policy
* audio-file discovery and supported media rules
* audition behavior and product-level playback policy
* sample extraction workflows and user-facing analysis behavior
* destructive sample editing semantics
* play selection and edit selection product rules
* keep/trash rating behavior
* aging/listen-history behavior
* temporary color collection behavior
* sample metadata, tags, ratings, age states, filters, and custom tag policy
* similarity workflows and user-facing discovery behavior
* display-name generation policy, disk-rename policy, and folder-routing policy
* file operations such as rename, duplicate, move, copy, trash, reveal, collect, export, overwrite, and recovery workflows
* DAW/external handoff policy
* destructive warning and YOLO mode policy
* Wavecrate-specific status wording and command behavior

### Radiant Owns

* layout panels and split panes
* virtualized trees, lists, and detail tables
* text input, editable rows, keyboard focus, and shortcuts
* pill/tag input primitives, autocomplete popovers, and selection chips
* GPU surface overlays and waveform/timeline rendering primitives
* range selection, cursor, marker, playhead, loop, grid, and timeline interaction primitives when they are domain-neutral
* icon buttons, SVG/image resource caching, progress widgets, and status surfaces
* repaint, invalidation, subscriptions, and background-resource ergonomics
* generic map/canvas interaction primitives where useful for the 2D similarity map

### Audio Engine Owns

* reusable playback primitives
* decoding and buffering for WAV and AIFF/AIF
* seeking, looping, retriggering, and range playback primitives
* audition-time warp primitives where reusable
* audio data preparation for waveform and edit auditioning
* render-preparation primitives needed for destructive edits, extraction, export, and baked warp output
* mono-style edit application across mono/stereo data
* future stereo split-channel editing primitives where useful
* audio-analysis primitives where they are reusable beyond Wavecrate
* audio-device and playback diagnostics

### Persistence Layer Owns

* durable storage
* indexing
* schema migrations
* transactional or recovery-safe updates
* query performance for tags, metadata, similarity, rating, naming, aging, color collections, and source state
* embedded Sample ID state
* persistence diagnostics

### Undo/Recovery System Owns

* session-local undo/redo history
* action transactions
* recovery file registration
* recovery file cleanup coordination
* undo/redo diagnostics

If Wavecrate has to build a custom UI primitive only because Radiant lacks the right general API, prefer improving Radiant and then migrating Wavecrate back to the generic primitive.

If Wavecrate has to add playback, decoding, looping, warping, destructive rendering, or reusable audio-analysis code only because the audio engine lacks a reusable primitive, prefer improving the audio-engine boundary instead of embedding one-off audio behavior directly into product code.

## Code Quality and Maintainability Target

Wavecrate should be simple, focused, and maintainable internally. The codebase should make the product direction easier to execute, not harder.

Code should be organized around clear responsibilities:

* sample discovery and source scanning
* audio playback and decoding
* waveform preparation and editing
* large-audio-file analysis and extraction
* similarity analysis and indexing
* tags, ratings, aging, display names, metadata, and persistence
* embedded metadata and Sample ID handling
* file operations and recovery
* undo/redo transactions
* UI state and product commands
* logging and diagnostics
* Radiant integration

Files and modules should be small, focused, and organized by responsibility. Each file should have a clear reason to exist. Each module should expose a clean surface and hide internal implementation details where practical. Module boundaries should follow real architecture boundaries, not arbitrary file splitting.

Wavecrate code should follow these standards:

* Keep functions small and single-purpose.
* Keep structs focused on one responsibility.
* Keep traits minimal, meaningful, and justified.
* Split complex methods into named helpers.
* Separate large impl blocks where it improves clarity.
* Expose only intentional public API.
* Keep internal types internal.
* Make error handling explicit and understandable.
* Keep control flow readable.
* Encode or document important invariants.
* Keep state mutation clear and predictable.
* Make side effects easy to identify.
* Keep hot-path code simple and efficient.
* Prefer clear composition over broad inheritance-like trait systems.
* Avoid premature abstraction.
* Avoid cleverness where straightforward code is better.
* Prefer explicit data flow.
* Minimize global mutable state.
* Remove dead code.
* Remove unused experiments unless intentionally preserved and documented.
* Make every abstraction earn its place.
* Avoid large rewrites unless they clearly reduce complexity, unlock important architecture, improve performance, or remove real blockers.

Code smells to avoid:

* god files and god objects
* long functions
* deep nesting
* repeated logic
* ambiguous names
* hidden side effects
* unclear ownership
* overly broad traits
* tight coupling between unrelated modules
* product behavior leaking into Radiant
* Radiant implementation details leaking into Wavecrate workflows
* audio-engine internals leaking into unrelated product code
* persistence details leaking into every workflow
* temporary hacks becoming architecture
* stale legacy UI assumptions shaping the new architecture

Wavecrate should be improved incrementally. Refactors should usually preserve working behavior, keep validation lanes intact, and leave the codebase in a clearer state than before. Do not turn quality work into endless renaming; renaming is useful only when it improves API clarity, architectural understanding, or developer experience.

Tests should support refactoring rather than prevent it. Avoid tests that only lock in names, incidental file layout, or implementation details. Prefer tests that protect real workflows, recovery behavior, stale-result safety, persistence correctness, playback behavior, destructive editing behavior, extraction behavior, undo/redo behavior, similarity behavior, and performance-sensitive paths.

## Milestone Strategy

Wavecrate should move toward the target through clear phases. These phases are not rigid release promises, but they help keep implementation work practical.

### Phase 1: Reliable Browser, Playback, Database, and Diagnostics

Establish the core application foundation:

* Windows-first folder/source scanning
* responsive sample browser
* immediate playback
* WAV-first reliable decoding, playback, waveform display, and metadata writing
* AIFF/AIF decoding, playback, waveform display, and metadata writing
* mono-style waveform display
* stable Wavecrate Sample IDs
* database metadata foundation
* embedded Sample ID writing where practical
* basic tags
* basic keep/trash ratings
* aging/listen-history tracking
* logging and diagnostics
* Windows-first file operations

### Phase 2: Mono-Style Destructive Editing, Selections, Extraction, and Undo

Build the core waveform workflow:

* play selection
* edit selection
* selection-priority rules
* loop auditioning
* mono-style destructive edit commands that preserve stereo files while applying edits equally to both channels
* default warnings and YOLO mode
* duplicate-file workflow
* extraction from selected regions
* extracted-region history marks
* session-local undo/redo transaction system
* temp recovery file system and background cleanup

### Phase 3: Library Hygiene, Naming, and Organization

Make the library manageable over time:

* tag categories
* generated display/database names
* disk filename view versus generated-name view
* apply generated name to disk
* untagged filters
* aging filters and sorting
* four-keep/four-trash rating thresholds
* accepted/favorite lock behavior
* configured trash folder workflow
* temporary color collections
* folder organization workflows

### Phase 4: Analysis, BPM, Warp Auditioning, and DAW Handoff

Improve production usefulness:

* BPM detection
* BPM metadata editing
* tempo-grid display
* target BPM lock
* practical beats-style audition-time warping
* extraction/export of warped audio exactly as heard
* drag/copy/export/reveal workflows
* practical Windows-first DAW handoff

### Phase 5: Similarity Search and 2D Exploration

Build Sononym/XO-style discovery:

* deterministic descriptor pipeline
* versioned DSP feature vectors
* persisted descriptor cache
* cosine ANN search
* browser similarity sorting
* visual similarity indicators
* similarity filters
* 2D map projection
* cluster exploration
* map/list audition workflow

### Phase 6: Stereo Split Editing, Polish, Performance, and Cross-Platform Readiness

Harden and extend the product:

* stereo split-view waveform display
* independent channel selection where useful
* independent channel editing/extraction where useful
* large-library stress testing
* long-file waveform performance
* background worker reliability
* stale-result safety
* undo/redo robustness
* file reconciliation workflows
* improved diagnostics
* macOS/Linux architecture review
* documentation and validation passes

## Documentation and Validation

Durable product and architecture contracts belong in `docs/`. Planning and backlog state belongs in Linear.

Meaningful changes should usually include:

* focused tests for behavior that can regress
* smoke or agent validation before commit/push
* updated docs when a durable contract changes
* Radiant example updates when a new generic GUI API is introduced
* audio-engine validation when playback, decode, seek, loop, warp, render, channel handling, or device behavior changes
* persistence and migration validation when durable storage contracts change
* undo/redo validation when transactional behavior changes
* embedded metadata validation when Sample ID writing changes
* logging and diagnostic validation for important workflows and failure paths
* manual validation notes for workflows that are hard to automate

Important validation lanes should cover:

* browsing responsiveness on large libraries
* supported format import/audition behavior for WAV and AIFF/AIF files
* mono and stereo file playback correctness
* stereo file preservation during mono-style edits
* long audio file navigation, auditioning, looping, marking, extraction, and export
* immediate audition behavior during fast selection changes
* play selection and edit selection behavior
* waveform precision for playhead, cursor, range, loop, fade, marker, grid, transient, and extracted-history interactions
* destructive edit warnings and YOLO mode behavior
* session-local undo/redo for edits, file operations, metadata operations, rating operations, and selection changes
* temporary recovery file creation and background cleanup
* stale-result safety for decode, waveform, analysis, edit-render, database, naming, and similarity jobs
* tag creation, autocomplete, duplicate prevention, persistence, generated names, disk rename application, and filtering
* keep/trash rating scale, accepted lock behavior, aging/listen-history behavior, and trash-folder movement
* embedded Sample ID creation, reading, conflict handling, and fallback behavior
* temporary color collection behavior
* target-BPM auditioning and baked warped extraction/export
* similarity list sorting, filtering, and 2D map interaction
* file mutation, duplicate, export, overwrite, rename, trash, move, copy, reveal, and partial-failure recovery
* DAW/external handoff workflows
* logging usefulness for tracing failures and background operations

## Completion Criteria

Wavecrate is moving toward the target when:

* browsing remains responsive on large sample sources
* short sample files and long audio files are both handled accurately and smoothly
* WAV and AIFF/AIF files are supported according to the phased format target
* MP3, FLAC, and other unsupported formats are clearly ignored, rejected, or reported as unsupported
* mono and stereo files play correctly
* mono-style editing preserves stereo files while applying edits equally to both channels
* sample selection starts playback quickly and reliably
* waveform marks, playhead, cursor, play selections, edit selections, loops, grids, transients, fades, edits, and markers are precise and stable
* long audio files can be navigated, analyzed, auditioned, looped, marked, cut into useful pieces, named, rated, tagged, and exported without leaving the browsing flow
* extracted regions create new sample files while preserving the original file unless the user performs a destructive edit
* extracted-region history helps users see what they already used from a long file
* destructive edits are fast, warning-protected by default, YOLO-mode capable for advanced users, and undoable during the current session where practical
* users can duplicate files before destructive editing when they want backups
* temp recovery files support session undo without slowing down shutdown
* stale recovery files are cleaned up by a background worker
* existing sample libraries can be quickly auditioned, cleaned up, tagged, rated, display-named, physically renamed when desired, moved, collected, and organized without losing flow
* producers can quickly find, audition, compare, copy, drag, export, reveal, or otherwise hand off samples to a DAW without losing creative flow
* target-BPM auditioning helps users hear loops in production context where practical
* extraction/export of warped audition material creates what the user heard
* the audio engine is robust internally and cleanly separated enough to become a standalone library later
* tag, rating, aging, naming, metadata, database, and indexing workflows remain fast on large libraries
* embedded Wavecrate Sample IDs are written where practical and safely fall back to database tracking where not
* long-running work is visible but non-blocking
* similarity analysis helps users find related sounds from both list and map workflows without runtime ML inference
* tags are quick to add, remove, autocomplete, categorize, visualize, and filter by
* keep/trash ratings, aging/listen-history states, accepted locks, and trash-folder movement help keep the library clean over time without unsafe permanent deletion
* generated display names and deliberate disk-renaming behavior are predictable and safe
* temporary color collections help users stage and compare material without replacing physical folder organization
* file operations preserve trust and recovery behavior
* logs make failures and slow operations traceable
* files, modules, functions, structs, and traits stay small, focused, and maintainable
* code smells are actively reduced instead of normalized
* tests protect real workflows without locking in incidental implementation details
* Wavecrate code owns sample-domain decisions
* Radiant code owns reusable GUI/runtime primitives
* reusable audio-engine code is not unnecessarily coupled to Wavecrate product behavior
* the new GUI can replace legacy UI paths without losing core workflows
