# Folder Delete Recovery

`src/app/controller/library/source_folders/delete_recovery/**` owns crash recovery
for folder deletes that temporarily move data into a per-source staging area.

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
- `DbCommitted`
  - Database updates completed and the delete is logically committed.
  - Recovery expectation: finalize by removing the staged folder and clearing the journal row.

## Recovery rules

- Journaled `Intent` / `Staged` entries restore the folder into the source root.
- If an `Intent` / `Staged` entry has no staged folder but the original folder is
  already present, recovery records `Already restored` and removes the journal row.
- Journaled `DbCommitted` entries finalize the staged folder; if the staged folder
  is already gone, recovery records `Already finalized`.
- Staged folders without journal rows are conservatively restored.
- Restore collisions are resolved by appending `.restored-N` to the folder name.

## Module ownership

- `journal.rs`
  - staging, journal persistence, rollback helpers
- `recovery.rs`
  - startup restore/finalize policy and the recovery matrix tests
- `controller_apply.rs`
  - `AppController` startup kick-off and UI/cache application of the report
