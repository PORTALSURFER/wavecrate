use super::*;

mod cleanup;
mod filenames;
mod open_roles;
mod schema;
mod user_library;

fn schema_version(connection: &Connection) -> i64 {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap()
}

fn current_schema_version() -> i64 {
    super::schema::SOURCE_DB_SCHEMA_VERSION
}
