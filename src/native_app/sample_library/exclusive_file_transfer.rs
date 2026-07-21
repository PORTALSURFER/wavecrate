use std::{
    fs::{self, File},
    io::{self, ErrorKind, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub(super) struct CommittedFile {
    path: PathBuf,
    owned_file: Arc<File>,
}

impl CommittedFile {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn remove_if_owned(&self) -> io::Result<bool> {
        if !self.still_owned()? {
            return Ok(false);
        }
        fs::remove_file(&self.path)?;
        Ok(true)
    }

    pub(super) fn move_back_if_owned(&self, source: &Path) -> io::Result<bool> {
        let mut input = self.owned_file.try_clone()?;
        input.seek(SeekFrom::Start(0))?;
        copy_open_file_no_replace(
            &mut input,
            self.owned_file.metadata()?.permissions(),
            source,
        )?;
        let _ = self.remove_if_owned();
        Ok(true)
    }

    fn still_owned(&self) -> io::Result<bool> {
        let actual = match File::open(&self.path) {
            Ok(actual) => actual,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
        same_file_handles(&self.owned_file, &actual)
    }
}

pub(super) fn copy_file_no_replace(source: &Path, destination: &Path) -> io::Result<CommittedFile> {
    let staged = stage_file_copy(source, destination)?;
    publish_staged_file(staged, destination)
}

pub(super) fn copy_file_to_unique_destination(
    source: &Path,
    first_candidate: &Path,
) -> io::Result<CommittedFile> {
    copy_file_to_unique_destination_with(source, first_candidate, |_, _| {})
}

pub(super) fn move_file_no_replace(source: &Path, destination: &Path) -> io::Result<CommittedFile> {
    let source_file = File::open(source)?;
    match rename_no_replace(source, destination) {
        Ok(()) => Ok(committed_file(destination, source_file)),
        Err(error) => move_file_after_rename_error(source, destination, error),
    }
}

fn move_file_after_rename_error(
    source: &Path,
    destination: &Path,
    rename_error: io::Error,
) -> io::Result<CommittedFile> {
    if rename_requires_copy_fallback(&rename_error) {
        let committed = copy_file_no_replace(source, destination)?;
        if let Err(remove_error) = fs::remove_file(source) {
            return Err(io::Error::new(
                remove_error.kind(),
                format!(
                    "copied to {} without replacing another file, but failed to remove the source: {remove_error}; the completed copy was preserved",
                    destination.display()
                ),
            ));
        }
        Ok(committed)
    } else {
        Err(rename_error)
    }
}

pub(super) fn move_file_to_unique_destination(
    source: &Path,
    first_candidate: &Path,
) -> io::Result<CommittedFile> {
    for index in 0..10_000 {
        let candidate = unique_copy_candidate(first_candidate, index);
        match move_file_no_replace(source, &candidate) {
            Ok(committed) => return Ok(committed),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        ErrorKind::AlreadyExists,
        "could not find an available destination name",
    ))
}

pub(super) fn unique_copy_candidate(first_candidate: &Path, index: usize) -> PathBuf {
    if index == 0 {
        return first_candidate.to_path_buf();
    }
    let parent = first_candidate.parent().unwrap_or_else(|| Path::new(""));
    let stem = first_candidate
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("sample"));
    let extension = first_candidate
        .extension()
        .map(|extension| extension.to_string_lossy().to_string());
    let file_name = match extension {
        Some(extension) => format!("{stem}_copy{index:03}.{extension}"),
        None => format!("{stem}_copy{index:03}"),
    };
    parent.join(file_name)
}

fn stage_file_copy(source: &Path, destination: &Path) -> io::Result<tempfile::NamedTempFile> {
    let parent = destination.parent().unwrap_or_else(|| Path::new(""));
    let mut input = File::open(source)?;
    let permissions = input.metadata()?.permissions();
    stage_open_file_copy(&mut input, permissions, parent)
}

fn stage_open_file_copy(
    input: &mut File,
    permissions: fs::Permissions,
    parent: &Path,
) -> io::Result<tempfile::NamedTempFile> {
    let mut staged = tempfile::Builder::new()
        .prefix(".wavecrate-transfer-")
        .suffix(".tmp")
        .tempfile_in(parent)?;
    io::copy(input, staged.as_file_mut())?;
    staged.as_file().set_permissions(permissions)?;
    staged.as_file().sync_all()?;
    Ok(staged)
}

fn copy_open_file_no_replace(
    input: &mut File,
    permissions: fs::Permissions,
    destination: &Path,
) -> io::Result<CommittedFile> {
    let parent = destination.parent().unwrap_or_else(|| Path::new(""));
    publish_staged_file(
        stage_open_file_copy(input, permissions, parent)?,
        destination,
    )
}

fn publish_staged_file(
    staged: tempfile::NamedTempFile,
    destination: &Path,
) -> io::Result<CommittedFile> {
    let file = staged
        .persist_noclobber(destination)
        .map_err(|error| error.error)?;
    Ok(committed_file(destination, file))
}

pub(super) fn copy_file_to_unique_destination_with(
    source: &Path,
    first_candidate: &Path,
    mut before_publish: impl FnMut(usize, &Path),
) -> io::Result<CommittedFile> {
    let mut staged = stage_file_copy(source, first_candidate)?;
    for index in 0..10_000 {
        let candidate = unique_copy_candidate(first_candidate, index);
        before_publish(index, &candidate);
        match staged.persist_noclobber(&candidate) {
            Ok(file) => return Ok(committed_file(&candidate, file)),
            Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {
                staged = error.file;
            }
            Err(error) => return Err(error.error),
        }
    }
    Err(io::Error::new(
        ErrorKind::AlreadyExists,
        "could not find an available destination name",
    ))
}

fn committed_file(path: &Path, owned_file: File) -> CommittedFile {
    CommittedFile {
        path: path.to_path_buf(),
        owned_file: Arc::new(owned_file),
    }
}

#[cfg(unix)]
fn same_file_handles(expected: &File, actual: &File) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let expected = expected.metadata()?;
    let actual = actual.metadata()?;
    Ok(expected.dev() == actual.dev() && expected.ino() == actual.ino())
}

#[cfg(windows)]
fn same_file_handles(expected: &File, actual: &File) -> io::Result<bool> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle},
    };

    fn identity(file: &File) -> io::Result<(u32, u64, u64)> {
        let mut information = BY_HANDLE_FILE_INFORMATION::default();
        unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }
            .map_err(io::Error::other)?;
        let index =
            (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
        let created = (u64::from(information.ftCreationTime.dwHighDateTime) << 32)
            | u64::from(information.ftCreationTime.dwLowDateTime);
        Ok((information.dwVolumeSerialNumber, index, created))
    }

    Ok(identity(expected)? == identity(actual)?)
}

#[cfg(not(any(unix, windows)))]
fn same_file_handles(_expected: &File, _actual: &File) -> io::Result<bool> {
    Ok(false)
}

fn rename_requires_copy_fallback(error: &io::Error) -> bool {
    error.kind() == ErrorKind::CrossesDevices
        || error.kind() == ErrorKind::Unsupported
        || cfg!(any(target_os = "linux", target_os = "android"))
            && matches!(
                error.raw_os_error(),
                Some(libc::ENOSYS | libc::EINVAL | libc::EOPNOTSUPP)
            )
}

#[cfg(target_os = "windows")]
fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::{
            Foundation::{ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS, ERROR_NOT_SAME_DEVICE},
            Storage::FileSystem::{MOVE_FILE_FLAGS, MoveFileExW},
        },
        core::{HRESULT, PCWSTR},
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVE_FILE_FLAGS(0),
        )
    }
    .map_err(|error| {
        let code = error.code();
        if code == HRESULT::from_win32(ERROR_ALREADY_EXISTS.0)
            || code == HRESULT::from_win32(ERROR_FILE_EXISTS.0)
        {
            io::Error::from(ErrorKind::AlreadyExists)
        } else if code == HRESULT::from_win32(ERROR_NOT_SAME_DEVICE.0) {
            io::Error::from(ErrorKind::CrossesDevices)
        } else {
            io::Error::other(error)
        }
    })
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "android"))]
fn path_to_c_string(path: &Path) -> io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;

    std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            ErrorKind::InvalidInput,
            "file transfer path contains an interior NUL byte",
        )
    })
}

#[cfg(target_os = "macos")]
fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    let source = path_to_c_string(source)?;
    let destination = path_to_c_string(destination)?;
    let result =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    let source = path_to_c_string(source)?;
    let destination = path_to_c_string(destination)?;
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "android"
)))]
fn rename_no_replace(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        ErrorKind::Unsupported,
        "atomic no-replace rename is unavailable on this platform",
    ))
}

#[cfg(test)]
mod tests;
