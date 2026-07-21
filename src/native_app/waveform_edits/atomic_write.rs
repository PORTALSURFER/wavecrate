use std::{fs, io, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReplacePhase {
    Commit,
    Restore,
}

#[derive(Debug)]
pub(super) struct AtomicWriteFailure {
    message: String,
    recovery_copy_required: bool,
}

impl AtomicWriteFailure {
    fn ordinary(message: String) -> Self {
        Self {
            message,
            recovery_copy_required: false,
        }
    }

    fn recovery_required(message: String) -> Self {
        Self {
            message,
            recovery_copy_required: true,
        }
    }

    pub(super) fn recovery_copy_required(&self) -> bool {
        self.recovery_copy_required
    }
}

impl std::fmt::Display for AtomicWriteFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

trait AtomicEditIo {
    fn before_sample_write(&self, _sample_index: usize) -> Result<(), String> {
        Ok(())
    }

    fn before_finalize(&self) -> Result<(), String> {
        Ok(())
    }

    fn replace(&self, staged: &Path, target: &Path, _phase: ReplacePhase) -> io::Result<()> {
        replace_file(staged, target)
    }

    fn sync_parent(&self, parent: &Path) -> io::Result<()> {
        sync_parent_dir(parent)
    }
}

struct RealAtomicEditIo;

impl AtomicEditIo for RealAtomicEditIo {
    #[cfg(debug_assertions)]
    fn before_sample_write(&self, sample_index: usize) -> Result<(), String> {
        if sample_index > 0 && injected_failure() == Some("write") {
            return Err(String::from("Injected late waveform sample-write failure"));
        }
        Ok(())
    }

    #[cfg(debug_assertions)]
    fn before_finalize(&self) -> Result<(), String> {
        if injected_failure() == Some("finalize") {
            return Err(String::from("Injected waveform finalize failure"));
        }
        Ok(())
    }

    #[cfg(debug_assertions)]
    fn replace(&self, staged: &Path, target: &Path, phase: ReplacePhase) -> io::Result<()> {
        let requested = injected_failure();
        if (phase == ReplacePhase::Commit && matches!(requested, Some("replace" | "restore")))
            || (phase == ReplacePhase::Restore && requested == Some("restore"))
        {
            return Err(io::Error::other(format!(
                "Injected waveform {} failure",
                match phase {
                    ReplacePhase::Commit => "replace",
                    ReplacePhase::Restore => "restore",
                }
            )));
        }
        replace_file(staged, target)
    }
}

#[cfg(debug_assertions)]
fn injected_failure() -> Option<&'static str> {
    match std::env::var("WAVECRATE_INJECT_WAVEFORM_EDIT_FAILURE").as_deref() {
        Ok("write") => Some("write"),
        Ok("finalize") => Some("finalize"),
        Ok("replace") => Some("replace"),
        Ok("restore") => Some("restore"),
        _ => None,
    }
}

pub(super) fn write_wav_atomically(
    target: &Path,
    recovery_copy: &Path,
    after_snapshot: &Path,
    channels: usize,
    sample_rate: u32,
    samples: &[f32],
) -> Result<(), AtomicWriteFailure> {
    write_wav_atomically_with(
        target,
        recovery_copy,
        after_snapshot,
        channels,
        sample_rate,
        samples,
        &RealAtomicEditIo,
    )
}

fn write_wav_atomically_with(
    target: &Path,
    recovery_copy: &Path,
    after_snapshot: &Path,
    channels: usize,
    sample_rate: u32,
    samples: &[f32],
    edit_io: &impl AtomicEditIo,
) -> Result<(), AtomicWriteFailure> {
    let parent = target.parent().ok_or_else(|| {
        AtomicWriteFailure::ordinary(format!(
            "Failed to stage edited WAV {}: target has no parent directory",
            target.display()
        ))
    })?;
    let target_permissions = fs::metadata(target)
        .map_err(|err| {
            AtomicWriteFailure::ordinary(format!("Failed to inspect original WAV: {err}"))
        })?
        .permissions();
    let staged = tempfile::Builder::new()
        .prefix(".wavecrate-edit-")
        .suffix(".wav")
        .tempfile_in(parent)
        .map_err(|err| {
            AtomicWriteFailure::ordinary(format!("Failed to stage edited WAV: {err}"))
        })?;
    let staged_file = staged.as_file().try_clone().map_err(|err| {
        AtomicWriteFailure::ordinary(format!("Failed to prepare edited WAV: {err}"))
    })?;
    let spec = hound::WavSpec {
        channels: channels as u16,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::new(staged_file, spec).map_err(|err| {
        AtomicWriteFailure::ordinary(format!("Failed to write edited WAV header: {err}"))
    })?;
    for (sample_index, sample) in samples.iter().enumerate() {
        edit_io.before_sample_write(sample_index).map_err(|err| {
            AtomicWriteFailure::ordinary(format!("Failed to write edited WAV sample: {err}"))
        })?;
        writer.write_sample(*sample).map_err(|err| {
            AtomicWriteFailure::ordinary(format!("Failed to write edited WAV sample: {err}"))
        })?;
    }
    edit_io.before_finalize().map_err(|err| {
        AtomicWriteFailure::ordinary(format!("Failed to finalize edited WAV: {err}"))
    })?;
    writer.finalize().map_err(|err| {
        AtomicWriteFailure::ordinary(format!("Failed to finalize edited WAV: {err}"))
    })?;
    fs::set_permissions(staged.path(), target_permissions).map_err(|err| {
        AtomicWriteFailure::ordinary(format!("Failed to preserve WAV permissions: {err}"))
    })?;
    staged
        .as_file()
        .sync_all()
        .map_err(|err| AtomicWriteFailure::ordinary(format!("Failed to sync edited WAV: {err}")))?;
    fs::copy(staged.path(), after_snapshot).map_err(|err| {
        AtomicWriteFailure::ordinary(format!("Failed to snapshot edited audio file: {err}"))
    })?;
    fs::File::open(after_snapshot)
        .and_then(|snapshot| snapshot.sync_all())
        .map_err(|err| {
            AtomicWriteFailure::ordinary(format!("Failed to sync edited audio snapshot: {err}"))
        })?;
    let staged_path = staged.into_temp_path();

    if let Err(replace_error) = edit_io.replace(&staged_path, target, ReplacePhase::Commit) {
        return restore_after_failed_commit(
            target,
            recovery_copy,
            parent,
            edit_io,
            format!("Failed to replace original WAV: {replace_error}"),
        );
    }
    if let Err(sync_error) = edit_io.sync_parent(parent) {
        return restore_after_failed_commit(
            target,
            recovery_copy,
            parent,
            edit_io,
            format!("Failed to durably commit edited WAV: {sync_error}"),
        );
    }
    Ok(())
}

fn restore_after_failed_commit(
    target: &Path,
    recovery_copy: &Path,
    parent: &Path,
    edit_io: &impl AtomicEditIo,
    commit_error: String,
) -> Result<(), AtomicWriteFailure> {
    match restore_recovery_copy(target, recovery_copy, parent, edit_io) {
        Ok(()) => Err(AtomicWriteFailure::ordinary(format!(
            "{commit_error}; the original audio was restored"
        ))),
        Err(restore_error) => Err(AtomicWriteFailure::recovery_required(format!(
            "{commit_error}; failed to restore the original audio: {restore_error}; recovery copy retained at {}",
            recovery_copy.display()
        ))),
    }
}

fn restore_recovery_copy(
    target: &Path,
    recovery_copy: &Path,
    parent: &Path,
    edit_io: &impl AtomicEditIo,
) -> io::Result<()> {
    let mut staged = tempfile::Builder::new()
        .prefix(".wavecrate-restore-")
        .suffix(".wav")
        .tempfile_in(parent)?;
    let mut recovery = fs::File::open(recovery_copy)?;
    io::copy(&mut recovery, staged.as_file_mut())?;
    fs::set_permissions(staged.path(), fs::metadata(recovery_copy)?.permissions())?;
    staged.as_file().sync_all()?;
    let staged_path = staged.into_temp_path();
    edit_io.replace(&staged_path, target, ReplacePhase::Restore)?;
    edit_io.sync_parent(parent)
}

#[cfg(not(target_os = "windows"))]
fn replace_file(staged: &Path, target: &Path) -> io::Result<()> {
    fs::rename(staged, target)
}

#[cfg(target_os = "windows")]
fn replace_file(staged: &Path, target: &Path) -> io::Result<()> {
    use windows::{
        Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        },
        core::PCWSTR,
    };

    let staged = wide_path(staged);
    let target = wide_path(target);
    let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
    unsafe { MoveFileExW(PCWSTR(staged.as_ptr()), PCWSTR(target.as_ptr()), flags) }
        .map_err(io::Error::other)
}

#[cfg(target_os = "windows")]
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn sync_parent_dir(parent: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(parent)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = parent;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
