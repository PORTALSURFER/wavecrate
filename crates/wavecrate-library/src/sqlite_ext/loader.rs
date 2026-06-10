//! Low-level SQLite extension loading mechanics.

use std::path::Path;

use rusqlite::Connection;

/// Enable SQLite extension loading, load one extension, and disable loading.
pub(super) fn load_extension(conn: &Connection, path: &Path) -> Result<(), rusqlite::Error> {
    unsafe {
        conn.load_extension_enable()?;
    }
    let load_result = unsafe { conn.load_extension(path, Option::<&str>::None) };
    let _ = conn.load_extension_disable();
    load_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn invalid_extension_file_maps_to_sqlite_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid_ext");
        std::fs::write(&path, b"not a sqlite extension").unwrap();
        let conn = Connection::open_in_memory().unwrap();

        assert!(load_extension(&conn, &path).is_err());
    }
}
