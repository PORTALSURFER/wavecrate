# Folder Delete Recovery

`src/app/controller/library/source_folders/delete_recovery/**` owns crash recovery
for folder deletes that move data into a per-source app-owned trash area.

## Why this exists

Folder delete mutates filesystem state first and database/cache state second. If
Sempal crashes mid-delete, startup recovery must reconcile staged folders without
silently losing either source files or database intent.

## Stage contract

The delete journal is stored at:

- `<source-root>/.sempal_delete_staging/delete_journal.json`

Each journal row moves through these stages:

- `Intent`
  - The delete intent is durable, but the filesystem rename may not have finished.
  - Recovery expectation: the original folder should exist after recovery.
- `Staged`
  - The folder was moved into `.sempal_delete_staging`.
  - Recovery expectation: restore the staged folder back into the source tree.
- `Deleted`
  - Database updates completed and the delete is logically committed.
  - Recovery expectation: retain the staged folder in `.sempal_delete_staging` until an explicit restore or purge path resolves it.
- `RestorePendingDb`
  - Explicit retained restore started and may already have moved files back into the source tree, but database metadata replay is not durably finished yet.
  - Recovery expectation: finish any remaining merge work, then rebuild DB metadata from the retained delete snapshot before clearing the journal row.

## Recovery rules

- Journaled `Intent` / `Staged` entries restore the folder into the source root.
- If an `Intent` / `Staged` entry has no staged folder but the original folder is
  already present, recovery records `Already restored` and removes the journal row.
- Journaled `Deleted` entries remain in staging when the original folder is still
  absent, preserving the app-owned trash state across restarts and surfacing the
  folder in Recovery for explicit restore or purge.
- If a `Deleted` entry has no staged folder but the original folder is already
  present, recovery records `Already restored` and removes the journal row.
- Staged folders without journal rows are conservatively restored.
- Startup restore collisions are resolved by appending `.restored-N` to the folder
  name.
- Explicit restore from Recovery is merged file-by-file instead of blindly
  renaming the whole folder back into place:
  - exact byte-for-byte matches reuse the existing file and discard the staged copy
  - differing files keep both copies, using UTC timestamp suffixes such as
    `.recovered-20260326T105355Z` or `.replaced-20260326T105355Z`
  - modified time decides which differing file keeps the canonical/original path,
    but content equality is always proven by exact-byte comparison rather than by
    timestamps alone
- `Deleted` journal rows also persist the deleted wav metadata snapshot so an
  explicit restore after restart can reconstruct the source database state when
  the staged file becomes canonical again, while exact-match reuse preserves any
  newer metadata already attached to the existing canonical file.
- Explicit retained restore now keeps durable journal state until both the
  filesystem merge and metadata replay complete, so restart recovery can finish
  the operation if the app crashes after files move but before DB state is
  rebuilt.
- Explicit restore or purge after restart is a best-effort recovery action, not
  a resurrection of the prior-session undo/redo stack.

## Module ownership

- `journal.rs`
  - staging, journal persistence, rollback helpers
- `recovery.rs`
  - startup restore/retain policy and the recovery matrix tests
- `controller_apply.rs`
  - `AppController` startup kick-off, Recovery UI state, and explicit restore/purge actions
