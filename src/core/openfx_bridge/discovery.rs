//! Discovery and filesystem scanning for OpenFX plugins.

use std::path::{Path, PathBuf};

/// A discovered OpenFX plugin bundle on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct OfxPluginDescriptor {
    /// Display name (bundle dir name without .bundle)
    pub name: String,
    /// Path to the binary inside Contents/*/
    pub binary_path: PathBuf,
    /// Bundle root path
    pub bundle_root: PathBuf,
}

/// Standard OFX plugin search locations for the current platform
/// (plus `$OFX_PLUGIN_PATH`, matching the OFX spec's host requirements).
pub fn ofx_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(custom) = std::env::var("OFX_PLUGIN_PATH") {
        paths.push(PathBuf::from(custom));
    }
    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/Library/OFX/Plugins"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        paths.push(PathBuf::from("/usr/local/lib/OFX/Plugins"));
        paths.push(PathBuf::from("/opt/OFX/Plugins"));
    }
    #[cfg(windows)]
    {
        if let Some(pf) = std::env::var("ProgramFiles").ok() {
            paths.push(
                PathBuf::from(pf)
                    .join("Common Files")
                    .join("OFX")
                    .join("Plugins"),
            );
        }
    }
    paths.into_iter().filter(|p| p.is_dir()).collect()
}

/// Scan a directory tree (max depth 3) for `*.bundle` directories containing
/// a binary under `Contents/<platform>/` or directly under `Contents/`.
pub fn discover_ofx_plugins(root: &Path) -> Vec<OfxPluginDescriptor> {
    let mut found = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else {
        return found;
    };
    let platform_dir = if cfg!(target_os = "macos") {
        "MacOS"
    } else if cfg!(windows) {
        "Win64"
    } else {
        "Linux-x86-64"
    };
    'bundle: for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.to_string_lossy().ends_with(".bundle") {
            continue;
        }
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let contents = path.join("Contents");
        // Preferred: Contents/<Platform>/<name>.ofx
        for candidate in [contents.join(platform_dir), contents.clone()] {
            if let Ok(bin_rd) = std::fs::read_dir(&candidate) {
                for b in bin_rd.flatten() {
                    let bp = b.path();
                    if bp.extension().is_some_and(|e| e == "ofx") {
                        found.push(OfxPluginDescriptor {
                            name,
                            binary_path: bp,
                            bundle_root: path.clone(),
                        });
                        continue 'bundle;
                    }
                }
            }
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Discover plugins across all standard search locations.
pub fn discover_all_ofx_plugins() -> Vec<OfxPluginDescriptor> {
    let mut all = Vec::new();
    for root in ofx_search_paths() {
        all.extend(discover_ofx_plugins(&root));
    }
    all.sort_by(|a, b| a.name.cmp(&b.name));
    all.dedup_by(|a, b| a.binary_path == b.binary_path);
    all
}
