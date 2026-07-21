use std::{fs::File, io, path::Path};

#[cfg(unix)]
pub(super) fn same_file_handles(expected: &File, actual: &File) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let expected = expected.metadata()?;
    let actual = actual.metadata()?;
    Ok(expected.dev() == actual.dev() && expected.ino() == actual.ino())
}

#[cfg(windows)]
pub(super) fn same_file_handles(expected: &File, actual: &File) -> io::Result<bool> {
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
pub(super) fn same_file_handles(_expected: &File, _actual: &File) -> io::Result<bool> {
    Ok(false)
}

#[cfg(target_os = "windows")]
pub(super) fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
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
            io::Error::from(io::ErrorKind::AlreadyExists)
        } else if code == HRESULT::from_win32(ERROR_NOT_SAME_DEVICE.0) {
            io::Error::from(io::ErrorKind::CrossesDevices)
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
            io::ErrorKind::InvalidInput,
            "file transfer path contains an interior NUL byte",
        )
    })
}

#[cfg(target_os = "macos")]
pub(super) fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
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
pub(super) fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
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
pub(super) fn rename_no_replace(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic no-replace rename is unavailable on this platform",
    ))
}
