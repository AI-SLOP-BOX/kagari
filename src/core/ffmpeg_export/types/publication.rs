use super::replace_existing_file;
use super::{FileIdentity, TEMP_OUTPUT_SEQUENCE};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(unix)]
pub fn file_identity(file: &std::fs::File) -> std::io::Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok(FileIdentity(metadata.dev(), metadata.ino()))
}

#[cfg(windows)]
pub fn file_identity(file: &std::fs::File) -> std::io::Result<FileIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, info.as_mut_ptr()) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let info = unsafe { info.assume_init() };
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok(FileIdentity(info.dwVolumeSerialNumber, index))
}

#[cfg(not(any(unix, windows)))]
pub fn file_identity(_file: &std::fs::File) -> std::io::Result<FileIdentity> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "file identity is unavailable on this platform",
    ))
}

pub fn path_file_identity(path: &std::path::Path) -> std::io::Result<FileIdentity> {
    #[cfg(any(unix, windows))]
    {
        file_identity(&std::fs::File::open(path)?)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "file identity is unavailable on this platform",
        ))
    }
}

fn sync_completed_file(path: &std::path::Path) -> std::io::Result<FileIdentity> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;
    file.sync_all()?;
    file_identity(&file)
}

fn sync_parent_directory_best_effort(path: &std::path::Path) {
    #[cfg(unix)]
    {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

pub fn snapshot_overwrite_identity(
    path: &std::path::Path,
    overwrite_existing: bool,
) -> std::io::Result<Option<FileIdentity>> {
    if !overwrite_existing {
        return Ok(None);
    }
    match path_file_identity(path) {
        Ok(identity) => Ok(Some(identity)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn publish_export_output(
    temporary: &std::path::Path,
    destination: &std::path::Path,
    expected_identity: Option<FileIdentity>,
) -> std::io::Result<()> {
    match expected_identity {
        Some(expected) => {
            let current = path_file_identity(destination)?;
            if current != expected {
                return Err(std::io::Error::other(
                    "destination changed after overwrite confirmation; leaving it untouched",
                ));
            }
            sync_completed_file(temporary)?;
            replace_existing_file(temporary, destination)?;
            sync_parent_directory_best_effort(destination);
            Ok(())
        }
        None => publish_file_no_replace(temporary, destination).map(|_| ()),
    }
}

pub(crate) fn publish_export_output_cancelable(
    temporary: &std::path::Path,
    destination: &std::path::Path,
    expected_identity: Option<FileIdentity>,
    cancel_flag: &AtomicBool,
) -> std::io::Result<()> {
    let cancelled =
        || std::io::Error::new(std::io::ErrorKind::Interrupted, "export canceled by user");
    match expected_identity {
        Some(expected) => {
            if path_file_identity(destination)? != expected {
                return Err(std::io::Error::other(
                    "destination changed after overwrite confirmation; leaving it untouched",
                ));
            }
            sync_completed_file(temporary)?;
            if cancel_flag.load(Ordering::SeqCst) {
                return Err(cancelled());
            }
            if path_file_identity(destination)? != expected {
                return Err(std::io::Error::other(
                    "destination changed after overwrite confirmation; leaving it untouched",
                ));
            }
            if cancel_flag.load(Ordering::SeqCst) {
                return Err(cancelled());
            }
            replace_existing_file(temporary, destination)?;
            sync_parent_directory_best_effort(destination);
            Ok(())
        }
        None => publish_file_no_replace_cancelable(temporary, destination, cancel_flag).map(|_| ()),
    }
}

/// Publish without replacing an existing destination. Hard links are atomic
/// and cheap on supported filesystems; create_new plus a synced copy is the
/// fallback for filesystems that do not support hard links.
pub fn publish_file_no_replace(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<FileIdentity> {
    publish_file_no_replace_with(source, destination, |source, destination| {
        std::fs::hard_link(source, destination)
    })
}

fn publish_file_no_replace_cancelable(
    source: &std::path::Path,
    destination: &std::path::Path,
    cancel_flag: &AtomicBool,
) -> std::io::Result<FileIdentity> {
    let source_identity = sync_completed_file(source)?;
    if cancel_flag.load(Ordering::SeqCst) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "export canceled by user",
        ));
    }
    match std::fs::hard_link(source, destination) {
        Ok(()) => {
            sync_parent_directory_best_effort(destination);
            Ok(source_identity)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(error),
        Err(_) => stage_copy_and_publish(
            source,
            destination,
            |input, output| copy_cancelable(input, output, cancel_flag),
            |temporary, destination| {
                if cancel_flag.load(Ordering::SeqCst) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "export canceled by user",
                    ));
                }
                rename_no_replace(temporary, destination)
            },
        ),
    }
}

fn copy_cancelable(
    input: &mut std::fs::File,
    output: &mut std::fs::File,
    cancel_flag: &AtomicBool,
) -> std::io::Result<u64> {
    use std::io::{Read, Write};

    let mut buffer = [0u8; 256 * 1024];
    let mut copied = 0u64;
    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "export canceled by user",
            ));
        }
        let bytes = input.read(&mut buffer)?;
        if bytes == 0 {
            return Ok(copied);
        }
        output.write_all(&buffer[..bytes])?;
        copied = copied.saturating_add(bytes as u64);
    }
}

fn publish_file_no_replace_with(
    source: &std::path::Path,
    destination: &std::path::Path,
    link: impl FnOnce(&std::path::Path, &std::path::Path) -> std::io::Result<()>,
) -> std::io::Result<FileIdentity> {
    let source_identity = sync_completed_file(source)?;
    match link(source, destination) {
        Ok(()) => {
            sync_parent_directory_best_effort(destination);
            Ok(source_identity)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(error),
        Err(_) => stage_copy_and_publish(source, destination, std::io::copy, rename_no_replace),
    }
}

pub(crate) fn stage_copy_and_publish(
    source: &std::path::Path,
    destination: &std::path::Path,
    copy: impl FnOnce(&mut std::fs::File, &mut std::fs::File) -> std::io::Result<u64>,
    publish: impl FnOnce(&std::path::Path, &std::path::Path) -> std::io::Result<()>,
) -> std::io::Result<FileIdentity> {
    let mut input = std::fs::File::open(source)?;
    let parent = destination
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let file_name = destination.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "output path has no filename",
        )
    })?;
    let mut staged = None;
    for _ in 0..100 {
        let sequence = TEMP_OUTPUT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{}.kagari-publish-{}-{sequence}.tmp",
            file_name.to_string_lossy(),
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                staged = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let (temporary, mut output) = staged.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not reserve a staging file after 100 attempts",
        )
    })?;
    let copy_result = (|| -> std::io::Result<FileIdentity> {
        let identity = file_identity(&output)?;
        copy(&mut input, &mut output)?;
        output.sync_all()?;
        drop(output);
        publish(&temporary, destination)?;
        sync_parent_directory_best_effort(destination);
        Ok(identity)
    })();
    if copy_result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    copy_result
}

pub(crate) fn rename_no_replace(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
        let source = source
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let destination = destination
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        let source = std::ffi::CString::new(source.as_os_str().as_bytes()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "source path contains NUL")
        })?;
        let destination =
            std::ffi::CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "destination path contains NUL",
                )
            })?;
        let result = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                destination.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        unsafe extern "C" {
            fn renamex_np(from: *const i8, to: *const i8, flags: u32) -> i32;
        }
        let source = CString::new(source.as_os_str().as_bytes()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "source path contains NUL")
        })?;
        let destination = CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "destination path contains NUL",
            )
        })?;
        if unsafe { renamex_np(source.as_ptr(), destination.as_ptr(), 0x0000_0004) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = (source, destination);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "atomic no-replace publishing is unsupported on this platform",
        ))
    }
}
