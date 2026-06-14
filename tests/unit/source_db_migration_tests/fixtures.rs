use super::*;
use rusqlite::Connection;
use tempfile::tempdir;

pub(super) fn with_legacy_db(setup_sql: &str) -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let db_file = dir.path().join(DB_FILE_NAME);
    let conn = Connection::open(&db_file).unwrap();
    conn.execute_batch(setup_sql).unwrap();
    dir
}

pub(super) fn column_names(connection: &Connection, table_name: &str) -> Vec<String> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut stmt = connection.prepare(&pragma).unwrap();
    stmt.query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}
