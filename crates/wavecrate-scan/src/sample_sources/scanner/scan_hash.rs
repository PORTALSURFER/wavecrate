use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::{
    ContentAuditSkipReason, PendingRenameEntry, SourceWriteBatch, WavEntry,
};
use wavecrate_library::sample_sources::{SourceEntryFileType, classify_source_entry};

use super::scan::{ChangedSample, RenamedSample, ScanError, ScanStats, UpdatedSample};
use super::scan_fs::{compute_content_hash, ensure_root_dir, read_facts};
use super::scan_writer::{ScanWritePhase, ScanWriter, UncoordinatedScanWriter};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HashBackfill {
    relative_path: PathBuf,
    file_size: u64,
    modified_ns: i64,
    content_hash: String,
    file_identity: Option<String>,
}

const CONTENT_AUDIT_RETRY_SECONDS: i64 = 15 * 60;

/// Resource ceilings for one resumable content-verification batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentAuditBudget {
    /// Maximum wall time for one admitted slice.
    pub max_elapsed: Duration,
    /// Maximum bytes hashed by one admitted slice, after allowing one oversize file.
    pub max_bytes: u64,
    /// Maximum entries attempted by one admitted slice.
    pub max_entries: usize,
    /// Desired maximum age of a complete verification rotation.
    pub target_coverage_age: Duration,
    /// Portion of the entry ceiling reserved for due retries.
    pub retry_entries: usize,
}

impl ContentAuditBudget {
    /// Build a compatibility budget bounded only by entry count.
    pub fn entry_limited(max_entries: usize) -> Self {
        Self {
            max_elapsed: Duration::MAX,
            max_bytes: u64::MAX,
            max_entries,
            target_coverage_age: Duration::from_secs(30 * 24 * 60 * 60),
            retry_entries: max_entries.div_ceil(4).max(1),
        }
    }

    /// Derive a finite daily slice from source scale while retaining hard time, byte, and entry
    /// ceilings. Playback/foreground work and slower source classes use the conservative lane.
    pub fn adaptive(
        total_entries: usize,
        activity: ContentAuditActivity,
        storage: ContentAuditStorage,
        accelerated: bool,
    ) -> Self {
        Self::adaptive_for_target(
            total_entries,
            activity,
            storage,
            accelerated,
            Duration::from_secs(30 * 24 * 60 * 60),
        )
    }

    /// Derive an adaptive slice for an explicit content-coverage objective.
    pub fn adaptive_for_target(
        total_entries: usize,
        activity: ContentAuditActivity,
        storage: ContentAuditStorage,
        accelerated: bool,
        target_coverage_age: Duration,
    ) -> Self {
        const AUDIT_INTERVAL_SECONDS: u64 = 24 * 60 * 60;
        let target_intervals = usize::try_from(
            target_coverage_age
                .as_secs()
                .div_ceil(AUDIT_INTERVAL_SECONDS)
                .max(1),
        )
        .unwrap_or(usize::MAX);
        let desired_entries = total_entries.div_ceil(target_intervals).max(1);
        let constrained = activity.playback_active
            || activity.foreground_active
            || storage == ContentAuditStorage::ExternalOrNetwork;
        let (max_elapsed, max_bytes, hard_entry_cap) = match (constrained, accelerated) {
            (true, false) => (Duration::from_secs(1), 64 * 1024 * 1024, 1_024),
            (true, true) => (Duration::from_secs(2), 128 * 1024 * 1024, 2_048),
            (false, false) => (Duration::from_secs(5), 512 * 1024 * 1024, 4_096),
            (false, true) => (Duration::from_secs(10), 1024 * 1024 * 1024, 8_192),
        };
        let minimum_entries = if total_entries > 1 { 2 } else { 1 };
        let max_entries = desired_entries
            .saturating_mul(if accelerated { 4 } else { 1 })
            .clamp(minimum_entries, hard_entry_cap);
        Self {
            max_elapsed,
            max_bytes,
            max_entries,
            target_coverage_age,
            retry_entries: max_entries.div_ceil(4).max(1),
        }
    }
}

/// Foreground resource activity used to select conservative hashing ceilings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContentAuditActivity {
    /// Whether playback is actively consuming source resources.
    pub playback_active: bool,
    /// Whether foreground browsing/loading is active.
    pub foreground_active: bool,
}

/// Coarse storage class used to avoid aggressive hashing on slow sources.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContentAuditStorage {
    /// A source on the normal local storage path.
    #[default]
    Local,
    /// A removable, mounted, or network-addressed source.
    ExternalOrNetwork,
}

impl ContentAuditStorage {
    /// Conservatively classify a source root from platform mount information.
    pub fn classify(root: &std::path::Path) -> Self {
        #[cfg(target_os = "windows")]
        {
            classify_windows_storage(root)
        }
        #[cfg(target_os = "macos")]
        {
            classify_macos_storage(root)
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = root;
            classify_unknown_storage()
        }
    }
}

#[cfg(target_os = "macos")]
fn classify_macos_storage(root: &std::path::Path) -> ContentAuditStorage {
    use std::os::unix::fs::MetadataExt;

    let Ok(source) = std::fs::metadata(root) else {
        return ContentAuditStorage::ExternalOrNetwork;
    };
    let Ok(local_anchor) = std::fs::metadata("/Users") else {
        return ContentAuditStorage::ExternalOrNetwork;
    };
    classify_macos_device(source.dev(), local_anchor.dev())
}

#[cfg(any(target_os = "macos", test))]
fn classify_macos_device(source_device: u64, local_device: u64) -> ContentAuditStorage {
    if source_device == local_device {
        ContentAuditStorage::Local
    } else {
        ContentAuditStorage::ExternalOrNetwork
    }
}

#[cfg(any(not(any(target_os = "windows", target_os = "macos")), test))]
fn classify_unknown_storage() -> ContentAuditStorage {
    ContentAuditStorage::ExternalOrNetwork
}

#[cfg(any(target_os = "windows", test))]
const WINDOWS_DRIVE_REMOVABLE: u32 = 2;
#[cfg(any(target_os = "windows", test))]
const WINDOWS_DRIVE_FIXED: u32 = 3;
#[cfg(any(target_os = "windows", test))]
const WINDOWS_DRIVE_REMOTE: u32 = 4;

#[cfg(any(target_os = "windows", test))]
fn classify_windows_drive_type(drive_type: u32) -> ContentAuditStorage {
    match drive_type {
        WINDOWS_DRIVE_FIXED => ContentAuditStorage::Local,
        WINDOWS_DRIVE_REMOVABLE | WINDOWS_DRIVE_REMOTE => ContentAuditStorage::ExternalOrNetwork,
        _ => ContentAuditStorage::ExternalOrNetwork,
    }
}

#[cfg(target_os = "windows")]
fn classify_windows_storage(root: &std::path::Path) -> ContentAuditStorage {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Component, Prefix};
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    use windows::core::PCWSTR;

    if root.to_string_lossy().starts_with(r"\\") {
        return ContentAuditStorage::ExternalOrNetwork;
    }
    let Some(Component::Prefix(prefix)) = root.components().next() else {
        return ContentAuditStorage::ExternalOrNetwork;
    };
    let drive = match prefix.kind() {
        Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
        _ => return ContentAuditStorage::ExternalOrNetwork,
    };
    let drive_root = std::ffi::OsString::from(format!("{}:\\", char::from(drive)));
    let mut wide = drive_root.encode_wide().collect::<Vec<_>>();
    wide.push(0);
    // SAFETY: `wide` is a live, nul-terminated UTF-16 root path for the duration of the call.
    let drive_type = unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) };
    classify_windows_drive_type(drive_type)
}

fn cancel_requested(cancel: Option<&AtomicBool>) -> bool {
    cancel.is_some_and(|cancel| cancel.load(Ordering::Relaxed))
}

pub(super) fn verify_content_batch(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    budget: ContentAuditBudget,
    now: i64,
) -> Result<ScanStats, ScanError> {
    verify_content_batch_with_writer(db, cancel, budget, now, &UncoordinatedScanWriter)
}

pub(super) fn verify_content_batch_with_writer(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    budget: ContentAuditBudget,
    now: i64,
    writer: &impl ScanWriter,
) -> Result<ScanStats, ScanError> {
    verify_content_batch_with_post_hash_hook(db, cancel, budget, now, writer, |_| {})
}

fn verify_content_batch_with_post_hash_hook(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    budget: ContentAuditBudget,
    now: i64,
    writer: &impl ScanWriter,
    mut post_hash: impl FnMut(&std::path::Path),
) -> Result<ScanStats, ScanError> {
    let started = Instant::now();
    verify_content_batch_with_hooks(db, cancel, budget, now, writer, &mut post_hash, &mut || {
        started.elapsed()
    })
}

fn verify_content_batch_with_hooks(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    budget: ContentAuditBudget,
    now: i64,
    writer: &impl ScanWriter,
    post_hash: &mut impl FnMut(&std::path::Path),
    elapsed: &mut impl FnMut() -> Duration,
) -> Result<ScanStats, ScanError> {
    let (manifest_revision, manifest_before) = super::manifest::capture_manifest_with_revision(db)?;
    let root = ensure_root_dir(db)?;
    let entries = manifest_before.clone();
    let checkpoint = {
        let _writer = writer.lock(ScanWritePhase::Manifest);
        db.begin_or_resume_content_audit(now, manifest_revision)?
    };
    let states = db.content_audit_entry_states()?;
    let forward_is_due = entries.iter().any(|entry| {
        states.get(&entry.relative_path).is_none_or(|state| {
            !state.verifies(entry, checkpoint.rotation_id) && state.skip_reason.is_none()
        })
    });
    let retry_limit = budget.retry_entries.min(
        budget
            .max_entries
            .saturating_sub(usize::from(forward_is_due)),
    );
    let due_retries = entries
        .iter()
        .filter(|entry| {
            states.get(&entry.relative_path).is_some_and(|state| {
                !state.verifies(entry, checkpoint.rotation_id) && state.retry_is_due(now)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let retry_start = due_retries
        .iter()
        .position(|entry| {
            entry.relative_path.to_string_lossy().as_ref() > checkpoint.retry_cursor.as_str()
        })
        .unwrap_or(0);
    let retry = due_retries
        .iter()
        .cycle()
        .skip(retry_start)
        .take(due_retries.len())
        .take(retry_limit)
        .cloned()
        .collect::<Vec<_>>();
    let start = entries
        .iter()
        .position(|entry| {
            entry.relative_path.to_string_lossy().as_ref() > checkpoint.cursor.as_str()
        })
        .unwrap_or(0);
    let forward = entries
        .iter()
        .cycle()
        .skip(start)
        .take(entries.len())
        .filter(|entry| {
            states.get(&entry.relative_path).is_none_or(|state| {
                !state.verifies(entry, checkpoint.rotation_id) && state.skip_reason.is_none()
            })
        })
        .take(budget.max_entries.saturating_sub(retry.len()))
        .cloned()
        .collect::<Vec<_>>();
    let mut selected = Vec::with_capacity(retry.len() + forward.len());
    let mut retry = VecDeque::from(retry);
    let mut forward = VecDeque::from(forward);
    let mut retry_paths = HashSet::new();
    let mut planned_retry_next = checkpoint.retry_next;
    while selected.len() < budget.max_entries && (!retry.is_empty() || !forward.is_empty()) {
        let take_retry = !retry.is_empty() && (planned_retry_next || forward.is_empty());
        if take_retry {
            let entry = retry.pop_front().expect("retry queue is non-empty");
            retry_paths.insert(entry.relative_path.clone());
            selected.push(entry);
            planned_retry_next = false;
        } else if let Some(entry) = forward.pop_front() {
            selected.push(entry);
            planned_retry_next = true;
        }
    }
    let mut stats = ScanStats::default();
    let mut verified = Vec::new();
    let mut skipped = Vec::new();
    let mut attempted_bytes = 0_u64;
    let mut last_forward = None;
    let mut last_retry = None;
    let mut retry_next = checkpoint.retry_next;
    for entry in &selected {
        if cancel_requested(cancel) {
            break;
        }
        if !verified.is_empty() || !skipped.is_empty() {
            if elapsed() >= budget.max_elapsed
                || attempted_bytes.saturating_add(entry.file_size) > budget.max_bytes
            {
                break;
            }
        }
        if retry_paths.contains(&entry.relative_path) {
            last_retry = Some(entry.relative_path.clone());
            retry_next = false;
        } else {
            last_forward = Some(entry.relative_path.clone());
            retry_next = true;
        }
        let absolute = root.join(&entry.relative_path);
        if !is_supported_regular_audio_file(&absolute) {
            let reason = if absolute.exists() {
                ContentAuditSkipReason::Unsupported
            } else {
                ContentAuditSkipReason::Unavailable
            };
            skipped.push((entry.clone(), reason, 0));
            continue;
        }
        let before_hash = match read_facts(&root, &absolute) {
            Ok(facts) => facts,
            Err(_) => {
                skipped.push((entry.clone(), ContentAuditSkipReason::Unavailable, 0));
                continue;
            }
        };
        let content_hash = match compute_content_hash(&absolute, cancel) {
            Ok(hash) => hash,
            Err(ScanError::Canceled) => break,
            Err(_) => {
                attempted_bytes = attempted_bytes.saturating_add(before_hash.size);
                skipped.push((
                    entry.clone(),
                    ContentAuditSkipReason::HashFailed,
                    before_hash.size,
                ));
                continue;
            }
        };
        post_hash(&absolute);
        attempted_bytes = attempted_bytes.saturating_add(before_hash.size);
        let after_hash = match read_facts(&root, &absolute) {
            Ok(facts) => facts,
            Err(_) => {
                skipped.push((
                    entry.clone(),
                    ContentAuditSkipReason::Unavailable,
                    before_hash.size,
                ));
                continue;
            }
        };
        if !before_hash.same_content_snapshot(&after_hash) {
            skipped.push((
                entry.clone(),
                ContentAuditSkipReason::ChangedDuringHash,
                before_hash.size,
            ));
            continue;
        }
        verified.push((entry.clone(), after_hash, content_hash));
        stats.hashes_computed += 1;
    }
    let cancelled = cancel_requested(cancel);
    let committed_snapshot = if !verified.is_empty() || !skipped.is_empty() {
        let _writer = writer.lock(ScanWritePhase::DeferredHash);
        let mut batch = db.write_batch()?;
        if !batch.matches_revision(manifest_revision)? {
            return Err(ScanError::StaleRevision {
                expected: manifest_revision,
                actual: db.get_revision()?,
            });
        }
        let mut committed_verified = Vec::with_capacity(verified.len());
        for (previous, facts, content_hash) in verified {
            match read_facts(&root, &root.join(&previous.relative_path)) {
                Ok(committed_facts) if facts.same_content_snapshot(&committed_facts) => {
                    committed_verified.push((previous, facts, content_hash));
                }
                Ok(_) => skipped.push((previous, ContentAuditSkipReason::ChangedDuringHash, 0)),
                Err(_) => {
                    skipped.push((previous, ContentAuditSkipReason::Unavailable, 0));
                }
            }
        }
        for (previous, facts, content_hash) in &committed_verified {
            batch.record_content_audit_verified(
                &previous.relative_path,
                checkpoint.rotation_id,
                now,
                facts.size,
                facts.modified_ns,
                facts.file_identity.as_deref(),
            )?;
            if previous.content_hash.as_deref() == Some(content_hash.as_str())
                && previous.file_size == facts.size
                && previous.modified_ns == facts.modified_ns
                && previous.file_identity == facts.file_identity
            {
                continue;
            }
            if previous.file_identity != facts.file_identity {
                tracing::debug!(
                    path = %previous.relative_path.display(),
                    previous_identity = ?previous.file_identity,
                    current_identity = ?facts.file_identity,
                    "Source content audit refreshed filesystem identity"
                );
            }
            batch.upsert_file_with_hash(
                &previous.relative_path,
                facts.size,
                facts.modified_ns,
                content_hash,
            )?;
            batch.set_file_identity(&previous.relative_path, facts.file_identity.as_deref())?;
            stats.updated += 1;
            stats.updated_samples.push(UpdatedSample {
                relative_path: previous.relative_path.clone(),
                file_size: facts.size,
                modified_ns: facts.modified_ns,
                content_hash: Some(content_hash.clone()),
            });
            if previous.content_hash.as_deref() != Some(content_hash.as_str()) {
                stats.content_changed += 1;
                stats.changed_samples.push(ChangedSample {
                    relative_path: previous.relative_path.clone(),
                    file_size: facts.size,
                    modified_ns: facts.modified_ns,
                    content_hash: content_hash.clone(),
                });
            }
        }
        for (entry, reason, bytes_read) in &skipped {
            batch.record_content_audit_skipped(
                &entry.relative_path,
                now,
                now.saturating_add(CONTENT_AUDIT_RETRY_SECONDS),
                *reason,
                *bytes_read,
            )?;
        }
        batch.checkpoint_content_audit(
            last_forward.as_deref(),
            last_retry.as_deref(),
            retry_next,
            manifest_revision,
            attempted_bytes,
            now,
        )?;
        batch.commit_with_manifest_snapshot()?
    } else {
        db.manifest_snapshot_with_revision()?
    };
    super::manifest::publish_committed_delta(&mut stats, manifest_before, committed_snapshot);
    let report = {
        let _writer = writer.lock(ScanWritePhase::Manifest);
        db.content_audit_report(now)?
    };
    if report.remaining_entries == 0 && report.total_entries > 0 {
        let manifest_before = stats.manifest_before.clone();
        let _writer = writer.lock(ScanWritePhase::Manifest);
        let mut batch = db.write_batch()?;
        batch.complete_content_audit_rotation(now, stats.committed_delta.revision)?;
        let committed = batch.commit_with_manifest_snapshot()?;
        super::manifest::publish_committed_delta(&mut stats, manifest_before, committed);
    }
    stats.content_audit = Some({
        let _writer = writer.lock(ScanWritePhase::Manifest);
        db.content_audit_report(now)?
    });
    if cancelled {
        Err(ScanError::Incomplete {
            committed: Box::new(stats),
            error: ScanError::Canceled.to_string(),
        })
    } else {
        Ok(stats)
    }
}

fn is_supported_regular_audio_file(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| {
        let file_type = metadata.file_type();
        classify_source_entry(
            path,
            SourceEntryFileType::from_no_followed_type(
                file_type.is_dir(),
                file_type.is_file(),
                file_type.is_symlink(),
            ),
        )
        .has_supported_audio()
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeferredHashScope {
    AllUnhashed,
    RenameCandidates,
}

pub(super) fn deep_hash_scan(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    rename_candidates: &HashSet<PathBuf>,
    scope: DeferredHashScope,
    max_hashes: Option<usize>,
    exact_path: Option<&std::path::Path>,
) -> Result<ScanStats, ScanError> {
    deep_hash_scan_with_writer(
        db,
        cancel,
        rename_candidates,
        scope,
        max_hashes,
        exact_path,
        &UncoordinatedScanWriter,
    )
}

pub(super) fn deep_hash_scan_with_writer(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    rename_candidates: &HashSet<PathBuf>,
    scope: DeferredHashScope,
    max_hashes: Option<usize>,
    exact_path: Option<&std::path::Path>,
    writer: &impl ScanWriter,
) -> Result<ScanStats, ScanError> {
    deep_hash_scan_with_post_hash_hook(
        db,
        cancel,
        rename_candidates,
        scope,
        max_hashes,
        exact_path,
        writer,
        |_| {},
    )
}

pub(super) fn reconcile_hashed_rename_candidates_with_writer(
    db: &SourceDatabase,
    rename_candidates: &HashSet<PathBuf>,
    cancel: Option<&AtomicBool>,
    writer: &impl ScanWriter,
) -> Result<Vec<RenamedSample>, ScanError> {
    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let root = ensure_root_dir(db)?;
    let rename_candidates = rename_candidates.clone();
    if rename_candidates.is_empty() {
        return Ok(Vec::new());
    }

    let entries_by_path = db
        .list_files()?
        .into_iter()
        .filter(|entry| {
            !entry.missing && is_supported_regular_audio_file(&root.join(&entry.relative_path))
        })
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect::<HashMap<_, _>>();
    let manifest_entries = db.list_manifest_entries()?;
    let pending_entries = db
        .list_pending_renames()?
        .into_iter()
        .filter(|entry| !root.join(&entry.relative_path).exists())
        .collect::<Vec<_>>();
    if pending_entries.is_empty() {
        return Ok(Vec::new());
    }

    let mut present_by_hash = HashMap::new();
    let mut pending_by_hash = HashMap::new();
    let mut present_by_file_identity = HashMap::new();
    let mut pending_by_file_identity = HashMap::new();
    for entry in manifest_entries {
        if !entries_by_path.contains_key(&entry.relative_path) {
            continue;
        }
        if let Some(hash) = entry.content_hash.as_deref() {
            present_by_hash
                .entry(hash.to_string())
                .or_insert_with(Vec::new)
                .push(entry.relative_path.clone());
        }
        if rename_candidates.contains(&entry.relative_path)
            && let Some(file_identity) = entry.file_identity.as_deref()
        {
            present_by_file_identity
                .entry(file_identity.to_string())
                .or_insert_with(Vec::new)
                .push(entry.relative_path.clone());
        }
    }
    for entry in pending_entries {
        if let Some(hash) = entry.content_hash.as_deref() {
            pending_by_hash
                .entry(hash.to_string())
                .or_insert_with(Vec::new)
                .push(entry.clone());
        }
        if let Some(file_identity) = entry.file_identity.as_deref() {
            pending_by_file_identity
                .entry(file_identity.to_string())
                .or_insert_with(Vec::new)
                .push(entry);
        }
    }

    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let _writer = writer.lock(ScanWritePhase::DeferredHash);
    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let mut batch = db.write_batch()?;
    let retained_candidates = retain_matching_rename_candidates(
        &mut batch,
        &present_by_hash,
        &pending_by_hash,
        &rename_candidates,
    )?;
    let mut renamed_samples = reconcile_same_file_renames(
        &mut batch,
        &entries_by_path,
        &present_by_file_identity,
        &pending_by_file_identity,
        &HashSet::new(),
    )?;
    let already_reconciled = reconciled_paths(&renamed_samples);
    renamed_samples.extend(reconcile_missing_renames(
        &mut batch,
        &entries_by_path,
        &present_by_hash,
        &pending_by_hash,
        &rename_candidates,
        &already_reconciled,
    )?);
    if renamed_samples.is_empty() && retained_candidates == 0 {
        return Ok(renamed_samples);
    }
    batch.commit()?;
    Ok(renamed_samples)
}

fn deep_hash_scan_with_post_hash_hook(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    rename_candidates: &HashSet<PathBuf>,
    scope: DeferredHashScope,
    max_hashes: Option<usize>,
    exact_path: Option<&std::path::Path>,
    writer: &impl ScanWriter,
    mut post_hash: impl FnMut(&std::path::Path),
) -> Result<ScanStats, ScanError> {
    let manifest_before = super::manifest::capture_manifest(db)?;
    let root = ensure_root_dir(db)?;
    let mut rename_candidates = rename_candidates.clone();
    rename_candidates.extend(db.list_pending_rename_destinations()?);
    let entries = if let Some(exact_path) = exact_path {
        db.entry_for_path(exact_path)?.into_iter().collect()
    } else if scope == DeferredHashScope::AllUnhashed && rename_candidates.is_empty() {
        db.list_pending_hash_files(max_hashes.unwrap_or(usize::MAX))?
    } else {
        db.list_files()?
    };
    let mut entries_by_path: HashMap<PathBuf, WavEntry> = entries
        .into_iter()
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect();
    let has_unhashed_files = scope == DeferredHashScope::AllUnhashed
        && entries_by_path.values().any(|entry| {
            !entry.missing
                && entry.content_hash.is_none()
                && root.join(&entry.relative_path).is_file()
        });
    if !has_unhashed_files && rename_candidates.is_empty() {
        let mut stats = ScanStats::default();
        let committed_snapshot = db.manifest_snapshot_with_revision()?;
        super::manifest::publish_committed_delta(&mut stats, manifest_before, committed_snapshot);
        return Ok(stats);
    }
    let pending_entries = db.list_pending_renames()?;
    let mut stats = ScanStats::default();
    let mut present_by_hash = HashMap::new();
    let mut pending_by_hash = HashMap::new();
    let mut present_by_file_identity = HashMap::new();
    let mut pending_by_file_identity = HashMap::new();
    let mut hash_backfills = Vec::new();

    for entry in entries_by_path.values() {
        if entry.missing
            || rename_candidates.contains(&entry.relative_path)
            || !is_supported_regular_audio_file(&root.join(&entry.relative_path))
        {
            continue;
        }
        let Some(hash) = entry.content_hash.as_deref() else {
            continue;
        };
        present_by_hash
            .entry(hash.to_string())
            .or_insert_with(Vec::new)
            .push(entry.relative_path.clone());
    }
    for entry in pending_entries {
        if let Some(file_identity) = entry.file_identity.as_deref() {
            pending_by_file_identity
                .entry(file_identity.to_string())
                .or_insert_with(Vec::new)
                .push(entry.clone());
        }
        let Some(hash) = entry.content_hash.as_deref() else {
            continue;
        };
        pending_by_hash
            .entry(hash.to_string())
            .or_insert_with(Vec::new)
            .push(entry);
    }

    for entry in entries_by_path.values_mut() {
        if let Some(cancel) = cancel
            && cancel.load(Ordering::Relaxed)
        {
            return Err(ScanError::Canceled);
        }
        if entry.missing {
            continue;
        }
        let is_rename_candidate = rename_candidates.contains(&entry.relative_path);
        if entry.content_hash.is_some() && !is_rename_candidate {
            continue;
        }
        if scope == DeferredHashScope::RenameCandidates && !is_rename_candidate {
            continue;
        }
        let was_unhashed = entry.content_hash.is_none();
        if was_unhashed
            && !is_rename_candidate
            && max_hashes.is_some_and(|limit| stats.hashes_computed >= limit)
        {
            continue;
        }
        let absolute = root.join(&entry.relative_path);
        if !is_supported_regular_audio_file(&absolute) {
            continue;
        }
        let facts = read_facts(&root, &absolute)?;
        let hash = compute_content_hash(&absolute, cancel)?;
        post_hash(&absolute);
        let after_hash = read_facts(&root, &absolute)?;
        if !facts.same_content_snapshot(&after_hash) {
            continue;
        }
        hash_backfills.push(HashBackfill {
            relative_path: entry.relative_path.clone(),
            file_size: after_hash.size,
            modified_ns: after_hash.modified_ns,
            content_hash: hash.clone(),
            file_identity: after_hash.file_identity,
        });
        entry.file_size = after_hash.size;
        entry.modified_ns = after_hash.modified_ns;
        entry.content_hash = Some(hash.clone());
        present_by_hash
            .entry(hash)
            .or_insert_with(Vec::new)
            .push(entry.relative_path.clone());
        if was_unhashed {
            stats.hashes_computed += 1;
        }
    }

    for backfill in &hash_backfills {
        if !rename_candidates.contains(&backfill.relative_path) {
            continue;
        }
        let Some(file_identity) = backfill.file_identity.as_ref() else {
            continue;
        };
        present_by_file_identity
            .entry(file_identity.clone())
            .or_insert_with(Vec::new)
            .push(backfill.relative_path.clone());
    }

    if let Some(cancel) = cancel
        && cancel.load(Ordering::Relaxed)
    {
        return Err(ScanError::Canceled);
    }

    let _writer = writer.lock(ScanWritePhase::DeferredHash);
    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let mut batch = db.write_batch()?;
    for backfill in &hash_backfills {
        batch.upsert_file_with_hash(
            &backfill.relative_path,
            backfill.file_size,
            backfill.modified_ns,
            &backfill.content_hash,
        )?;
        batch.set_file_identity(&backfill.relative_path, backfill.file_identity.as_deref())?;
    }
    retain_matching_rename_candidates(
        &mut batch,
        &present_by_hash,
        &pending_by_hash,
        &rename_candidates,
    )?;

    let mut renamed_samples = reconcile_same_file_renames(
        &mut batch,
        &entries_by_path,
        &present_by_file_identity,
        &pending_by_file_identity,
        &HashSet::new(),
    )?;
    let already_reconciled = reconciled_paths(&renamed_samples);
    renamed_samples.extend(reconcile_missing_renames(
        &mut batch,
        &entries_by_path,
        &present_by_hash,
        &pending_by_hash,
        &rename_candidates,
        &already_reconciled,
    )?);
    stats.renames_reconciled = renamed_samples.len();
    stats.renamed_samples = renamed_samples;

    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let committed_snapshot = batch.commit_with_manifest_snapshot()?;
    super::manifest::publish_committed_delta(&mut stats, manifest_before, committed_snapshot);
    Ok(stats)
}

fn reconcile_same_file_renames(
    batch: &mut SourceWriteBatch<'_>,
    entries_by_path: &HashMap<PathBuf, WavEntry>,
    present_by_file_identity: &HashMap<String, Vec<PathBuf>>,
    pending_by_file_identity: &HashMap<String, Vec<PendingRenameEntry>>,
    already_reconciled: &HashSet<PathBuf>,
) -> Result<Vec<RenamedSample>, ScanError> {
    let mut reconciled = Vec::new();
    for (file_identity, pending_entries) in pending_by_file_identity {
        let [pending_entry] = pending_entries.as_slice() else {
            continue;
        };
        let Some(present_paths) = present_by_file_identity.get(file_identity) else {
            continue;
        };
        let [present_path] = present_paths.as_slice() else {
            continue;
        };
        if pending_entry.relative_path == *present_path
            || already_reconciled.contains(&pending_entry.relative_path)
            || already_reconciled.contains(present_path)
        {
            continue;
        }
        let Some(present_entry) = entries_by_path.get(present_path) else {
            continue;
        };
        if present_entry.file_size != pending_entry.file_size
            || present_entry.modified_ns != pending_entry.modified_ns
        {
            continue;
        }
        let Some(hash) = present_entry.content_hash.as_deref() else {
            continue;
        };
        apply_deep_rename(batch, present_entry, pending_entry, hash)?;
        reconciled.push(RenamedSample {
            old_relative_path: pending_entry.relative_path.clone(),
            new_relative_path: present_entry.relative_path.clone(),
            file_size: present_entry.file_size,
            modified_ns: present_entry.modified_ns,
            content_hash: Some(hash.to_string()),
        });
    }
    Ok(reconciled)
}

fn reconcile_missing_renames(
    batch: &mut SourceWriteBatch<'_>,
    entries_by_path: &HashMap<PathBuf, WavEntry>,
    present_by_hash: &HashMap<String, Vec<PathBuf>>,
    pending_by_hash: &HashMap<String, Vec<PendingRenameEntry>>,
    rename_candidates: &HashSet<PathBuf>,
    already_reconciled: &HashSet<PathBuf>,
) -> Result<Vec<RenamedSample>, ScanError> {
    let mut reconciled = Vec::new();
    for (hash, pending_entries) in pending_by_hash {
        let [pending_entry] = pending_entries.as_slice() else {
            continue;
        };
        if already_reconciled.contains(&pending_entry.relative_path) {
            continue;
        }
        let Some(present_paths) = present_by_hash.get(hash) else {
            continue;
        };
        let candidates = present_paths
            .iter()
            .filter(|path| rename_candidates.contains(*path) && !already_reconciled.contains(*path))
            .collect::<Vec<_>>();
        let [present_path] = candidates.as_slice() else {
            continue;
        };
        let present_path = *present_path;
        if pending_entry.relative_path == *present_path {
            continue;
        }
        let Some(present_entry) = entries_by_path.get(present_path) else {
            continue;
        };
        apply_deep_rename(batch, present_entry, pending_entry, hash)?;
        reconciled.push(RenamedSample {
            old_relative_path: pending_entry.relative_path.clone(),
            new_relative_path: present_entry.relative_path.clone(),
            file_size: present_entry.file_size,
            modified_ns: present_entry.modified_ns,
            content_hash: Some(hash.clone()),
        });
    }
    Ok(reconciled)
}

fn retain_matching_rename_candidates(
    batch: &mut SourceWriteBatch<'_>,
    present_by_hash: &HashMap<String, Vec<PathBuf>>,
    pending_by_hash: &HashMap<String, Vec<PendingRenameEntry>>,
    rename_candidates: &HashSet<PathBuf>,
) -> Result<usize, ScanError> {
    let mut retained = 0;
    for hash in pending_by_hash.keys() {
        let Some(present_paths) = present_by_hash.get(hash) else {
            continue;
        };
        for path in present_paths
            .iter()
            .filter(|path| rename_candidates.contains(*path))
        {
            batch.retain_pending_rename_destination(path, hash)?;
            retained += 1;
        }
    }
    Ok(retained)
}

fn reconciled_paths(renamed_samples: &[RenamedSample]) -> HashSet<PathBuf> {
    renamed_samples
        .iter()
        .flat_map(|renamed| {
            [
                renamed.old_relative_path.clone(),
                renamed.new_relative_path.clone(),
            ]
        })
        .collect()
}

fn apply_deep_rename(
    batch: &mut SourceWriteBatch<'_>,
    present_entry: &WavEntry,
    pending_entry: &PendingRenameEntry,
    hash: &str,
) -> Result<(), ScanError> {
    batch.clear_pending_rename(&pending_entry.relative_path)?;
    batch.clear_pending_rename_destination(&present_entry.relative_path)?;
    batch.upsert_file_with_hash_and_tag(
        &present_entry.relative_path,
        present_entry.file_size,
        present_entry.modified_ns,
        hash,
        pending_entry.metadata.tag,
        false,
    )?;
    batch.restore_rename_metadata(&present_entry.relative_path, &pending_entry.metadata)?;
    batch.remap_analysis_sample_identity(
        &pending_entry.relative_path,
        &present_entry.relative_path,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        path::Path,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
    };

    use crate::sample_sources::SourceDatabase;

    use super::*;

    #[derive(Clone, Default)]
    struct ObservedWriter {
        active: Arc<AtomicBool>,
        locks: Arc<AtomicUsize>,
    }

    struct ObservedWriterGuard(Arc<AtomicBool>);

    impl Drop for ObservedWriterGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }

    impl ScanWriter for ObservedWriter {
        type Guard = ObservedWriterGuard;

        fn lock(&self, _phase: ScanWritePhase) -> Self::Guard {
            assert!(!self.active.swap(true, Ordering::AcqRel));
            self.locks.fetch_add(1, Ordering::AcqRel);
            ObservedWriterGuard(Arc::clone(&self.active))
        }
    }

    fn content_fixture(count: usize, bytes: usize) -> (tempfile::TempDir, SourceDatabase) {
        let directory = tempfile::tempdir().expect("content source");
        let database =
            SourceDatabase::open_for_source_write(directory.path()).expect("source database");
        for index in 0..count {
            let relative = PathBuf::from(format!("sample-{index:05}.wav"));
            std::fs::write(directory.path().join(&relative), vec![index as u8; bytes])
                .expect("write content fixture");
            let facts = read_facts(directory.path(), &directory.path().join(&relative))
                .expect("content facts");
            database
                .upsert_file(&relative, facts.size, facts.modified_ns)
                .expect("insert content manifest");
        }
        (directory, database)
    }

    #[test]
    fn adaptive_content_budget_has_a_finite_large_source_horizon() {
        let idle = ContentAuditBudget::adaptive(
            10_000,
            ContentAuditActivity::default(),
            ContentAuditStorage::Local,
            false,
        );
        assert_eq!(idle.max_entries, 334);
        assert_eq!(idle.max_elapsed, Duration::from_secs(5));
        assert_eq!(idle.max_bytes, 512 * 1024 * 1024);

        let busy_external = ContentAuditBudget::adaptive(
            29_000,
            ContentAuditActivity {
                playback_active: true,
                foreground_active: true,
            },
            ContentAuditStorage::ExternalOrNetwork,
            false,
        );
        assert_eq!(busy_external.max_entries, 967);
        assert_eq!(busy_external.max_elapsed, Duration::from_secs(1));
        assert_eq!(busy_external.max_bytes, 64 * 1024 * 1024);
        assert_eq!(
            ContentAuditBudget::adaptive(
                30,
                ContentAuditActivity::default(),
                ContentAuditStorage::Local,
                false,
            )
            .max_entries,
            2,
            "multi-entry adaptive slices must reserve retry and forward capacity"
        );

        let seven_day = ContentAuditBudget::adaptive_for_target(
            10_000,
            ContentAuditActivity::default(),
            ContentAuditStorage::Local,
            false,
            Duration::from_secs(7 * 24 * 60 * 60),
        );
        assert_eq!(seven_day.max_entries, 1_429);
        assert_eq!(
            seven_day.target_coverage_age,
            Duration::from_secs(7 * 24 * 60 * 60)
        );
    }

    #[test]
    fn windows_mapped_removable_and_unknown_drive_types_are_conservative() {
        assert_eq!(
            classify_windows_drive_type(WINDOWS_DRIVE_FIXED),
            ContentAuditStorage::Local
        );
        for drive_type in [
            WINDOWS_DRIVE_REMOTE,
            WINDOWS_DRIVE_REMOVABLE,
            0, // DRIVE_UNKNOWN
            1, // DRIVE_NO_ROOT_DIR
            5, // DRIVE_CDROM
            6, // DRIVE_RAMDISK
        ] {
            assert_eq!(
                classify_windows_drive_type(drive_type),
                ContentAuditStorage::ExternalOrNetwork
            );
        }
        assert_eq!(
            classify_unknown_storage(),
            ContentAuditStorage::ExternalOrNetwork,
            "platforms without mount classification must fail conservatively"
        );
    }

    #[test]
    fn macos_only_grants_local_budget_to_the_normal_local_mount() {
        assert_eq!(
            classify_macos_device(7, 7),
            ContentAuditStorage::Local,
            "the normal local device receives the local budget"
        );
        assert_eq!(
            classify_macos_device(8, 7),
            ContentAuditStorage::ExternalOrNetwork,
            "a distinct mounted filesystem receives the conservative budget"
        );
    }

    #[test]
    fn content_audit_resumes_without_recounting_committed_entries() {
        let (_directory, database) = content_fixture(10, 32);
        let budget = ContentAuditBudget {
            max_elapsed: Duration::MAX,
            max_bytes: u64::MAX,
            max_entries: 3,
            target_coverage_age: Duration::from_secs(30 * 24 * 60 * 60),
            retry_entries: 1,
        };

        let first = verify_content_batch(&database, None, budget, 100).expect("first slice");
        let first_report = first.content_audit.expect("first coverage");
        assert_eq!(first_report.verified_entries, 3);
        assert_eq!(first_report.remaining_entries, 7);

        drop(database);
        let database = SourceDatabase::open_for_source_write(_directory.path()).expect("reopen");
        let second = verify_content_batch(&database, None, budget, 101).expect("second slice");
        let second_report = second.content_audit.expect("second coverage");
        assert_eq!(second_report.verified_entries, 6);
        assert_eq!(second_report.remaining_entries, 4);
        assert_eq!(second.hashes_computed, 3);
    }

    #[test]
    fn final_rotation_completion_preserves_the_committed_content_delta() {
        let (directory, database) = content_fixture(1, 32);
        let relative = Path::new("sample-00000.wav");
        let facts = read_facts(directory.path(), &directory.path().join(relative)).unwrap();
        let mut batch = database.write_batch().expect("stale content batch");
        batch
            .upsert_file_with_hash(
                relative,
                facts.size,
                facts.modified_ns,
                "stale-content-hash",
            )
            .expect("seed stale content generation");
        batch.commit().expect("commit stale content generation");

        let stats =
            verify_content_batch(&database, None, ContentAuditBudget::entry_limited(1), 100)
                .expect("rotation-completing content verification");

        assert_eq!(stats.content_changed, 1);
        assert_eq!(stats.committed_delta.changed.len(), 1);
        assert_eq!(
            stats.committed_delta.changed[0].relative_path,
            Path::new("sample-00000.wav")
        );
        assert_eq!(
            stats.committed_delta.revision,
            database.get_revision().unwrap(),
            "published delta must carry the final committed rotation revision"
        );
    }

    #[test]
    fn content_audit_byte_budget_allows_one_oversize_file_then_stops() {
        let (_directory, database) = content_fixture(4, 32);
        let budget = ContentAuditBudget {
            max_elapsed: Duration::MAX,
            max_bytes: 16,
            max_entries: 4,
            target_coverage_age: Duration::from_secs(30 * 24 * 60 * 60),
            retry_entries: 1,
        };

        let stats = verify_content_batch(&database, None, budget, 100).expect("byte slice");

        assert_eq!(stats.hashes_computed, 1);
        let report = stats.content_audit.expect("coverage");
        assert_eq!(report.verified_entries, 1);
        assert_eq!(report.bytes_read, 32);
    }

    #[test]
    fn content_audit_time_budget_stops_at_the_next_file_boundary() {
        let (_directory, database) = content_fixture(3, 32);
        let elapsed = std::cell::Cell::new(Duration::ZERO);
        let mut post_hash = |_: &std::path::Path| elapsed.set(Duration::from_secs(1));
        let mut observe_elapsed = || elapsed.get();
        let budget = ContentAuditBudget {
            max_elapsed: Duration::from_secs(1),
            max_bytes: u64::MAX,
            max_entries: 3,
            target_coverage_age: Duration::from_secs(30 * 24 * 60 * 60),
            retry_entries: 1,
        };

        let stats = verify_content_batch_with_hooks(
            &database,
            None,
            budget,
            100,
            &UncoordinatedScanWriter,
            &mut post_hash,
            &mut observe_elapsed,
        )
        .expect("time-bounded slice");

        assert_eq!(stats.hashes_computed, 1);
        let report = stats.content_audit.expect("coverage");
        assert_eq!(report.verified_entries, 1);
        assert_eq!(report.remaining_entries, 2);
    }

    #[test]
    fn cancellation_commits_verified_checkpoint_and_resume_skips_it() {
        let (_directory, database) = content_fixture(3, 32);
        let cancel = AtomicBool::new(false);
        let budget = ContentAuditBudget::entry_limited(3);

        let result = verify_content_batch_with_post_hash_hook(
            &database,
            Some(&cancel),
            budget,
            100,
            &UncoordinatedScanWriter,
            |_| cancel.store(true, Ordering::Release),
        );
        let ScanError::Incomplete { committed, .. } = result.expect_err("cancel content slice")
        else {
            panic!("cancellation after a hash must retain committed coverage");
        };
        assert_eq!(
            committed
                .content_audit
                .as_ref()
                .expect("cancel coverage")
                .verified_entries,
            1
        );

        cancel.store(false, Ordering::Release);
        let resumed = verify_content_batch(
            &database,
            Some(&cancel),
            ContentAuditBudget::entry_limited(1),
            101,
        )
        .expect("resume content slice");
        assert_eq!(resumed.hashes_computed, 1);
        assert_eq!(
            resumed
                .content_audit
                .expect("resumed coverage")
                .verified_entries,
            2
        );
    }

    #[test]
    fn changed_during_hash_remains_due_with_retry_reason() {
        let (directory, database) = content_fixture(2, 32);
        let changing = directory.path().join("sample-00000.wav");
        let result = verify_content_batch_with_post_hash_hook(
            &database,
            None,
            ContentAuditBudget::entry_limited(1),
            100,
            &UncoordinatedScanWriter,
            |path| {
                assert_eq!(path, changing);
                std::fs::write(path, [7_u8; 64]).expect("mutate during hash");
            },
        )
        .expect("skip unstable content");

        let report = result.content_audit.expect("coverage");
        assert_eq!(report.verified_entries, 0);
        assert_eq!(report.skipped_retry_entries, 1);
        let states = database.content_audit_entry_states().expect("entry states");
        assert_eq!(
            states[Path::new("sample-00000.wav")].skip_reason.as_deref(),
            Some("changed_during_hash")
        );

        let forward =
            verify_content_batch(&database, None, ContentAuditBudget::entry_limited(1), 101)
                .expect("continue past delayed retry");
        assert_eq!(forward.hashes_computed, 1);
        assert_eq!(
            forward
                .content_audit
                .expect("forward coverage")
                .skipped_retry_entries,
            1
        );
        assert_eq!(
            database.content_audit_entry_states().unwrap()[Path::new("sample-00000.wav")].attempts,
            1
        );

        let retry =
            verify_content_batch(&database, None, ContentAuditBudget::entry_limited(1), 1_000)
                .expect("retry stable content");
        assert_eq!(retry.hashes_computed, 1);
        let state = database.content_audit_entry_states().unwrap();
        assert_eq!(state[Path::new("sample-00000.wav")].skip_reason, None);
        assert_eq!(state[Path::new("sample-00000.wav")].attempts, 2);
    }

    #[test]
    fn due_retry_cannot_consume_the_only_forward_progress_slot() {
        let (directory, database) = content_fixture(2, 32);
        let changing = directory.path().join("sample-00000.wav");
        verify_content_batch_with_post_hash_hook(
            &database,
            None,
            ContentAuditBudget::entry_limited(1),
            100,
            &UncoordinatedScanWriter,
            |path| {
                assert_eq!(path, changing);
                std::fs::write(path, [7_u8; 64]).expect("mutate during hash");
            },
        )
        .expect("record unstable entry");

        let forward =
            verify_content_batch(&database, None, ContentAuditBudget::entry_limited(1), 1_000)
                .expect("reserve the only slot for forward progress");

        assert_eq!(forward.hashes_computed, 1);
        let states = database.content_audit_entry_states().unwrap();
        assert_eq!(
            states[Path::new("sample-00000.wav")].attempts,
            1,
            "the due retry must remain queued while forward work owns the only slot"
        );
        assert!(
            states[Path::new("sample-00001.wav")].verifies(
                &database
                    .list_manifest_entries()
                    .unwrap()
                    .into_iter()
                    .find(|entry| entry.relative_path == Path::new("sample-00001.wav"))
                    .unwrap(),
                1,
            )
        );
    }

    #[test]
    fn oversized_retry_yields_the_next_slice_to_forward_progress() {
        let (directory, database) = content_fixture(2, 32);
        let retry_path = directory.path().join("sample-00000.wav");
        verify_content_batch_with_post_hash_hook(
            &database,
            None,
            ContentAuditBudget::entry_limited(1),
            100,
            &UncoordinatedScanWriter,
            |path| {
                assert_eq!(path, retry_path);
                std::fs::write(path, [7_u8; 64]).expect("make first attempt unstable");
            },
        )
        .expect("record unstable entry");
        let constrained = ContentAuditBudget {
            max_elapsed: Duration::MAX,
            max_bytes: 16,
            max_entries: 2,
            target_coverage_age: Duration::from_secs(30 * 24 * 60 * 60),
            retry_entries: 1,
        };

        let retry_slice = verify_content_batch_with_post_hash_hook(
            &database,
            None,
            constrained,
            1_000,
            &UncoordinatedScanWriter,
            |path| {
                assert_eq!(path, retry_path);
                std::fs::write(path, [9_u8; 96]).expect("keep retry unstable");
            },
        )
        .expect("bounded retry slice");
        assert_eq!(retry_slice.hashes_computed, 0);
        assert!(
            !retry_slice
                .content_audit
                .expect("retry coverage")
                .retry_next,
            "an attempted retry must persist forward as the next lane"
        );
        let after_retry = database.content_audit_entry_states().unwrap();
        assert_eq!(after_retry[Path::new("sample-00000.wav")].attempts, 2);
        assert!(!after_retry.contains_key(Path::new("sample-00001.wav")));

        drop(database);
        let database =
            SourceDatabase::open_for_source_write(directory.path()).expect("reopen source");
        let forward_slice =
            verify_content_batch(&database, None, constrained, 2_000).expect("forward slice");
        assert_eq!(forward_slice.hashes_computed, 1);
        let after_forward = database.content_audit_entry_states().unwrap();
        assert_eq!(
            after_forward[Path::new("sample-00000.wav")].attempts,
            2,
            "the still-due oversized retry must not run before reserved forward work"
        );
        assert_eq!(
            after_forward[Path::new("sample-00001.wav")].skip_reason,
            None
        );
    }

    #[test]
    fn retry_cursor_rotates_past_a_persistently_unavailable_path() {
        let (directory, database) = content_fixture(3, 32);
        std::fs::remove_file(directory.path().join("sample-00000.wav")).unwrap();
        std::fs::remove_file(directory.path().join("sample-00001.wav")).unwrap();
        verify_content_batch(&database, None, ContentAuditBudget::entry_limited(2), 100)
            .expect("record two unavailable entries");

        verify_content_batch(&database, None, ContentAuditBudget::entry_limited(2), 1_000)
            .expect("retry the first path while reserving forward progress");
        let first_retry = database.content_audit_report(1_000).unwrap();
        assert_eq!(first_retry.retry_cursor, "sample-00000.wav");

        verify_content_batch(&database, None, ContentAuditBudget::entry_limited(1), 2_000)
            .expect("rotate the retry slot to the later path");
        let states = database.content_audit_entry_states().unwrap();
        assert_eq!(states[Path::new("sample-00000.wav")].attempts, 2);
        assert_eq!(
            states[Path::new("sample-00001.wav")].attempts,
            2,
            "the later due retry must eventually own the bounded retry slot"
        );
        assert_eq!(
            database.content_audit_report(2_000).unwrap().retry_cursor,
            "sample-00001.wav"
        );
    }

    #[test]
    fn content_hashing_does_not_hold_the_runtime_database_writer() {
        let (_directory, database) = content_fixture(2, 32);
        let writer = ObservedWriter::default();
        let active = Arc::clone(&writer.active);

        verify_content_batch_with_post_hash_hook(
            &database,
            None,
            ContentAuditBudget::entry_limited(2),
            100,
            &writer,
            |_| assert!(!active.load(Ordering::Acquire)),
        )
        .expect("content slice");

        assert!(writer.locks.load(Ordering::Acquire) >= 3);
        assert!(!writer.active.load(Ordering::Acquire));
    }

    #[test]
    fn content_audit_rejects_a_stale_manifest_revision_before_commit() {
        let (_directory, database) = content_fixture(1, 32);

        let result = verify_content_batch_with_post_hash_hook(
            &database,
            None,
            ContentAuditBudget::entry_limited(1),
            100,
            &UncoordinatedScanWriter,
            |_| {
                database
                    .set_metadata("concurrent_manifest_writer", "1")
                    .expect("advance source revision");
            },
        );

        assert!(matches!(result, Err(ScanError::StaleRevision { .. })));
        assert!(database.content_audit_entry_states().unwrap().is_empty());
    }

    #[test]
    fn deep_hash_scan_checks_cancel_before_writer_lock() {
        let dir = tempfile::tempdir().expect("temp source");
        std::fs::write(dir.path().join("pending.wav"), b"pending").expect("write wav");
        let db = SourceDatabase::open_for_source_write(dir.path()).expect("source db");
        db.upsert_file(Path::new("pending.wav"), 7, 10)
            .expect("file row");
        let lock_db = SourceDatabase::open_for_source_write(dir.path()).expect("lock db");
        let _writer = lock_db.write_batch().expect("writer lock");
        let cancel = AtomicBool::new(true);

        let result = deep_hash_scan(
            &db,
            Some(&cancel),
            &HashSet::new(),
            DeferredHashScope::AllUnhashed,
            None,
            None,
        );

        assert!(matches!(result, Err(ScanError::Canceled)));
    }

    #[test]
    fn deep_hash_scan_bounds_a_large_library_batch() {
        let dir = tempfile::tempdir().expect("temp source");
        let db = SourceDatabase::open_for_source_write(dir.path()).expect("source db");
        for index in 0..512 {
            let relative = PathBuf::from(format!("pending-{index}.wav"));
            std::fs::write(dir.path().join(&relative), [index as u8; 32]).expect("write wav");
            db.upsert_file(&relative, 32, index)
                .expect("insert pending row");
        }

        let stats = deep_hash_scan(
            &db,
            None,
            &HashSet::new(),
            DeferredHashScope::AllUnhashed,
            Some(8),
            None,
        )
        .expect("bounded hash pass");

        assert_eq!(stats.hashes_computed, 8);
        assert_eq!(
            db.list_files()
                .expect("list files")
                .iter()
                .filter(|entry| entry.content_hash.is_some())
                .count(),
            8
        );
    }

    #[test]
    fn deep_hash_scan_exact_path_does_not_process_earlier_pending_rows() {
        let dir = tempfile::tempdir().expect("temp source");
        let db = SourceDatabase::open_for_source_write(dir.path()).expect("source db");
        for relative in [Path::new("a-first.wav"), Path::new("z-target.wav")] {
            std::fs::write(dir.path().join(relative), [9_u8; 32]).expect("write wav");
            db.upsert_file(relative, 32, 1).expect("insert pending row");
        }

        let stats = deep_hash_scan(
            &db,
            None,
            &HashSet::new(),
            DeferredHashScope::AllUnhashed,
            Some(1),
            Some(Path::new("z-target.wav")),
        )
        .expect("targeted hash pass");

        assert_eq!(stats.hashes_computed, 1);
        assert!(
            db.entry_for_path(Path::new("a-first.wav"))
                .unwrap()
                .unwrap()
                .content_hash
                .is_none()
        );
        assert!(
            db.entry_for_path(Path::new("z-target.wav"))
                .unwrap()
                .unwrap()
                .content_hash
                .is_some()
        );
    }

    #[test]
    fn deep_hash_scan_does_not_commit_a_file_mutated_during_hashing() {
        let dir = tempfile::tempdir().expect("temp source");
        let relative = Path::new("changing.wav");
        let absolute = dir.path().join(relative);
        std::fs::write(&absolute, [1_u8; 32]).expect("write initial wav");
        let original_modified = std::fs::metadata(&absolute)
            .expect("read initial metadata")
            .modified()
            .expect("read initial modified time");
        let db = SourceDatabase::open_for_source_write(dir.path()).expect("source db");
        db.upsert_file(relative, 32, 1).expect("insert pending row");

        let stats = deep_hash_scan_with_post_hash_hook(
            &db,
            None,
            &HashSet::new(),
            DeferredHashScope::AllUnhashed,
            Some(1),
            Some(relative),
            &UncoordinatedScanWriter,
            |path| {
                std::fs::write(path, [2_u8; 32]).expect("mutate during hashing");
                let file = std::fs::OpenOptions::new()
                    .write(true)
                    .open(path)
                    .expect("reopen mutated wav");
                file.set_times(std::fs::FileTimes::new().set_modified(original_modified))
                    .expect("restore modified time");
            },
        )
        .expect("defer unstable hash");

        assert_eq!(stats.hashes_computed, 0);
        assert!(
            db.entry_for_path(relative)
                .expect("read pending row")
                .expect("pending row")
                .content_hash
                .is_none(),
            "an unstable read must remain pending for a later hash pass"
        );
    }
}
