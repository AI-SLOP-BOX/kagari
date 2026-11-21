use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Replace a file only after the complete contents have been written and synced.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let target = match std::fs::symlink_metadata(path) {
        Ok(_) => std::fs::canonicalize(path)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => path.to_path_buf(),
        Err(error) => return Err(error),
    };
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "File path has no filename"))?;

    for _ in 0..100 {
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary: PathBuf = parent.join(format!(
            ".{}.tmp.{}.{}",
            file_name.to_string_lossy(),
            std::process::id(),
            sequence
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = match options.open(&temporary) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let result = (|| {
            file.write_all(contents)?;
            // Keep the staging file private under the process umask while its
            // contents are incomplete. Apply the destination mode only once
            // all bytes have been written, immediately before syncing it.
            match std::fs::metadata(&target) {
                Ok(metadata) => file.set_permissions(metadata.permissions())?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            file.sync_all()?;
            drop(file);
            replace_file(&temporary, &target)?;
            if let Ok(directory) = std::fs::File::open(parent) {
                let _ = directory.sync_all();
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        return result;
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Could not reserve a unique temporary file after 100 attempts",
    ))
}

/// Atomically replace an existing destination with a fully written staging file.
pub fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    crate::core::ffmpeg_export::types::replace_existing_file(source, destination)
}

/// Replace a destination and return the identity of the published file.
pub fn replace_file_with_identity(
    source: &Path,
    destination: &Path,
) -> io::Result<crate::core::ffmpeg_export::types::FileIdentity> {
    let identity = crate::core::ffmpeg_export::types::path_file_identity(source)?;
    replace_file(source, destination)?;
    Ok(identity)
}

/// Remove a path only while it still refers to the file previously published.
/// Returns `false` when the path is absent or has been replaced.
pub fn remove_file_if_identity(
    path: &Path,
    expected: crate::core::ffmpeg_export::types::FileIdentity,
) -> io::Result<bool> {
    let actual = match crate::core::ffmpeg_export::types::path_file_identity(path) {
        Ok(identity) => identity,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if actual != expected {
        return Ok(false);
    }
    std::fs::remove_file(path)?;
    Ok(true)
}
