use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn ffmpeg_command() -> Command {
    command_for("ffmpeg")
}

pub(crate) fn ffprobe_command() -> Command {
    command_for("ffprobe")
}

fn command_for(name: &str) -> Command {
    Command::new(resolve_executable(name).unwrap_or_else(|| name.into()))
}

fn resolve_executable(name: &str) -> Option<PathBuf> {
    let path_entries = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    if let Some(path) = path_entries
        .iter()
        .map(|directory| directory.join(name))
        .find(|path| is_executable_file(path))
    {
        return Some(path);
    }

    #[cfg(target_os = "macos")]
    {
        let mut directories = vec![
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/local/bin"),
            PathBuf::from("/nix/var/nix/profiles/default/bin"),
        ];
        if let Some(home) = env::var_os("HOME") {
            let home = PathBuf::from(home);
            directories.push(home.join(".nix-profile/bin"));
            directories.push(home.join(".local/bin"));
        }
        return directories
            .iter()
            .map(|directory| directory.join(name))
            .find(|path| is_executable_file(path));
    }

    #[allow(unreachable_code)]
    None
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}
