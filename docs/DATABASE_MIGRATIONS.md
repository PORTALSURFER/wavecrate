# Database Migration Contract

Wavecrate treats persisted SQLite state as a user-trust surface. Schema changes
must preserve existing ratings, tags, collection state, curation timestamps,
file-operation recovery state, analysis status, and cache references wherever
the old data can be interpreted safely.

## Database Families

- Source databases live in each indexed source folder as `.wavecrate.db`.
  They own source-local file metadata, tags assigned to files in that source,
  ratings, curation state, scan status, pending rename records, file-operation
  recovery state, revision-fenced versioned readiness targets/artifact completions, readiness-owned
  job metadata, and source-local analysis/cache references.
- The global library database lives under Wavecrate's app config directory as
  `library.db`. It owns configured source references and global analysis/cache
  state that is not source-local.

## Source Database Path Policy

Source-local database writes must stay inside the configured source/database
root. Writable opens, read-only opens, legacy filename migration, and SQLite
WAL/SHM sidecar handling must reject symlinked `.wavecrate.db`,
`.wavecrate_samples.db`, and related sidecar paths before handing the path to
SQLite or filesystem rename operations. Existing regular DB files and newly
created DB parents must resolve under the canonical database root.

## Required Change Pattern

Every schema change must update these pieces together:

1. Base DDL for newly created databases.
2. The migration or repair path for existing databases.
3. The schema contract test for the affected database family.
4. Compatibility behavior for read-only source opens when UI reads can touch
   pre-migration source databases.
5. Focused tests that prove important existing rows keep their data.

For source databases, additive columns/tables must be represented in the
current-stamp repair path. A matching `PRAGMA user_version` is not enough to
skip structural repairs, because development builds can create current-stamped
databases that are missing newer additive columns. Current-stamp repairs must be
low-cost and idempotent.

For non-additive or destructive changes, bump the source DB schema version and
add a fixture-style migration test that starts from the old shape, opens the
database through the real `SourceDatabase` entrypoint, and verifies both schema
shape and data preservation. Prefer additive expansion plus backfill over table
rebuilds unless the old shape cannot remain valid.

The global library database currently runs idempotent base DDL and additive
migrations on open. Future non-additive library changes should introduce an
explicit versioned migration before shipping.

## Read-Only Source Compatibility

`SourceDatabase::open_read_only` must not apply migrations. Any query reachable
from a read-only UI or worker path must either:

- project safe defaults for optional columns that may not exist yet, or
- avoid the feature-specific query when the required table or column is absent.

Do not add direct `SELECT new_column` reads to read-only paths without a legacy
fixture proving an older `.wavecrate.db` still opens and shows usable metadata.

## Tests

Schema contract tests live in `tests/unit/source_db_migration_tests/` for source
databases and in `crates/wavecrate-library/src/sample_sources/library/tests.rs`
for the global library database.

Use these focused checks after persistence changes:

```sh
cargo test -p wavecrate-library source_db_migration_tests
cargo test -p wavecrate-library sample_sources::db::read
cargo test -p wavecrate-library sample_sources::library
cargo check -p wavecrate-library
```

Run the broader wrapper lane for release-risk or cross-module persistence work:

```sh
bash scripts/ci.sh agent
```

Use the PowerShell equivalent on Windows.
