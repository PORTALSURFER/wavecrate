use super::{
    BTreeSet, ORPHAN_CACHE_MAX_REMOVED, ORPHAN_CACHE_MAX_SCANNED, ORPHAN_CACHE_MIN_AGE,
    ORPHAN_CACHE_SCAN_CURSOR, Ordering, PathBuf, RETAINED_SOURCE_MAX_SCANNED, ReadinessStore,
    SystemTime, UNIX_EPOCH, fs, invalidate_persisted_waveform_cache_ref,
};

pub(super) fn retained_waveform_cache_ref_is_owned(cache_ref: &str) -> Result<bool, String> {
    let retained_sources = wavecrate::sample_sources::library::retained_sources()
        .map_err(|error| error.to_string())?;
    let mut visited = BTreeSet::new();
    for retained in retained_sources {
        let database_path = retained.db_path().map_err(|error| error.to_string())?;
        if !visited.insert(database_path.clone()) {
            continue;
        }
        if !database_path.is_file() {
            return Err(format!(
                "retained source database is unavailable: {}",
                database_path.display()
            ));
        }
        let mut connection = rusqlite::Connection::open_with_flags(
            &database_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| error.to_string())?;
        let owned = ReadinessStore::new(&mut connection)
            .legacy_playback_artifact_ref_is_owned(cache_ref)
            .map_err(|error| error.to_string())?;
        if owned {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn prune_unreferenced_waveform_cache() -> Result<usize, String> {
    let retained_sources = wavecrate::sample_sources::library::retained_sources()
        .map_err(|error| error.to_string())?;
    if retained_sources.len() > RETAINED_SOURCE_MAX_SCANNED {
        return Err(format!(
            "retained source count {} exceeds bounded GC scan limit {RETAINED_SOURCE_MAX_SCANNED}",
            retained_sources.len()
        ));
    }
    let mut referenced = BTreeSet::<PathBuf>::new();
    for source in retained_sources {
        let database_path = source.db_path().map_err(|error| error.to_string())?;
        if !database_path.is_file() {
            return Err(format!(
                "retained source database is unavailable: {}",
                database_path.display()
            ));
        }
        let mut connection = rusqlite::Connection::open_with_flags(
            &database_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| format!("open retained cache manifest: {error}"))?;
        let refs = ReadinessStore::new(&mut connection)
            .legacy_playback_artifact_refs()
            .map_err(|error| error.to_string())?;
        referenced.extend(refs.into_iter().map(PathBuf::from));
    }

    let cache_dir = wavecrate::app_dirs::waveform_cache_dir().map_err(|error| error.to_string())?;
    let entries = match fs::read_dir(&cache_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.to_string()),
    };
    let cutoff = SystemTime::now()
        .checked_sub(ORPHAN_CACHE_MIN_AGE)
        .unwrap_or(UNIX_EPOCH);
    let mut removed = 0_usize;
    let cursor = ORPHAN_CACHE_SCAN_CURSOR.load(Ordering::Relaxed);
    let mut scanned = 0_usize;
    let mut delete_limit_reached = false;
    for entry in entries
        .flatten()
        .skip(cursor)
        .take(ORPHAN_CACHE_MAX_SCANNED)
    {
        if removed >= ORPHAN_CACHE_MAX_REMOVED {
            delete_limit_reached = true;
            break;
        }
        scanned = scanned.saturating_add(1);
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "wfc") || referenced.contains(&path)
        {
            continue;
        }
        let old_enough = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .is_some_and(|modified| modified <= cutoff);
        if !old_enough {
            continue;
        }
        invalidate_persisted_waveform_cache_ref(&path);
        if !path.exists() {
            removed = removed.saturating_add(1);
        }
    }
    ORPHAN_CACHE_SCAN_CURSOR.store(
        if scanned < ORPHAN_CACHE_MAX_SCANNED && !delete_limit_reached {
            0
        } else {
            cursor.saturating_add(scanned)
        },
        Ordering::Relaxed,
    );
    Ok(removed)
}
