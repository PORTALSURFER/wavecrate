use super::*;
use rusqlite::{OptionalExtension, params};
use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};
use tempfile::tempdir;

mod metadata;
mod opening;
mod writes;

fn with_home_env_override<T>(home: &Path, test: impl FnOnce() -> T) -> T {
    static HOME_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _lock = match HOME_ENV_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(lock) => lock,
        Err(_) => panic!("HOME env override lock was poisoned"),
    };
    let prev_home = std::env::var_os("HOME");
    let prev_homedrive = std::env::var_os("HOMEDRIVE");
    let prev_hompath = std::env::var_os("HOMEPATH");
    let prev_user_profile = std::env::var_os("USERPROFILE");

    unsafe {
        std::env::set_var("HOME", home);
    }

    struct HomeEnvGuard {
        prev_home: Option<OsString>,
        prev_homedrive: Option<OsString>,
        prev_hompath: Option<OsString>,
        prev_user_profile: Option<OsString>,
    }

    impl Drop for HomeEnvGuard {
        fn drop(&mut self) {
            match self.prev_home.take() {
                Some(home) => unsafe { std::env::set_var("HOME", home) },
                None => unsafe { std::env::remove_var("HOME") },
            }
            match self.prev_homedrive.take() {
                Some(value) => unsafe { std::env::set_var("HOMEDRIVE", value) },
                None => unsafe { std::env::remove_var("HOMEDRIVE") },
            }
            match self.prev_hompath.take() {
                Some(value) => unsafe { std::env::set_var("HOMEPATH", value) },
                None => unsafe { std::env::remove_var("HOMEPATH") },
            }
            match self.prev_user_profile.take() {
                Some(value) => unsafe { std::env::set_var("USERPROFILE", value) },
                None => unsafe { std::env::remove_var("USERPROFILE") },
            }
        }
    }

    let _home_guard = HomeEnvGuard {
        prev_home,
        prev_homedrive,
        prev_hompath,
        prev_user_profile,
    };

    test()
}

fn revision_value(db: &SourceDatabase) -> i64 {
    db.connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'revision'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap()
        .parse::<i64>()
        .unwrap()
}
