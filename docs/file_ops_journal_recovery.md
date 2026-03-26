# File-Op Journal Recovery

This note documents the crash-recovery contract for
`src/sample_sources/db/file_ops_journal.rs`.

## Purpose

Copy and move flows can crash after filesystem work but before both source and
target databases describe the final state. The file-op journal records enough
metadata to reconcile those partial writes on the next startup.

The journal is deliberately descriptive, not authoritative. Recovery always
checks the filesystem and database state again so repeated startup scans remain
idempotent.

## Stage Contract

`FileOpStage` is append-only:

1. `Intent`
   The journal row exists before any filesystem mutation is assumed.
2. `Staged`
   Data exists at `staged_relative` and has not been finalized into the target
   path yet.
3. `TargetDb`
   The target-side DB update has committed.
4. `SourceDb`
   The source-side DB cleanup has committed for move operations.

The stored stage tells recovery how far the original operation expected to get,
but reconcile still trusts the observed filesystem state first.

## Reconcile Rules

- If a staged file exists and the target file is missing, recovery finalizes the
  staged file into the target path.
- If both staged and target files exist, recovery removes the stale staged copy.
- If the target file exists, recovery upserts the target DB row and restores the
  persisted tag/loop/lock/last-played metadata from the journal entry.
- If the target file does not exist, recovery removes any stale target DB row.
- For moves, if the source file is gone and the target exists, recovery removes
  the stale source DB row.
- If move recovery still needs to clean the source DB row but the source root is
  unavailable at replay time, recovery keeps the journal row and retries later
  instead of treating the operation as complete.
- For moves, if the source file still exists and the target does not, recovery
  leaves the source DB row intact and treats the operation as not finalized.

## Required Invariants

- Journal insertion happens before filesystem mutation.
- Journal stage updates happen after each durable boundary completes.
- Reconcile must be safe to run multiple times.
- Reconcile must prefer data preservation over aggressive cleanup when observed
  filesystem state is ambiguous.
