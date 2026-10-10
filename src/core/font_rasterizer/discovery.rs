use std::path::PathBuf;

/// System font directory paths per platform.
pub fn system_font_paths(family_name: &str) -> Vec<PathBuf> {
    let lower = family_name.to_lowercase().replace(' ', "");
    let _candidates = [
        format!("{}.ttf", lower),
        format!("{}.otf", lower),
        format!("{}.ttc", lower),
        format!("{}.TTF", lower),
        format!("{}.OTF", lower),
    ];

    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from(format!("/Library/Fonts/{}.ttf", family_name)));
        paths.push(PathBuf::from(format!(
            "/System/Library/Fonts/{}.ttf",
            family_name
        )));
        paths.push(PathBuf::from(format!(
            "/System/Library/Fonts/Supplemental/{}.ttf",
            family_name
        )));
        // TrueType Collections (e.g. /System/Library/Fonts/Helvetica.ttc)
        paths.push(PathBuf::from(format!(
            "/System/Library/Fonts/{}.ttc",
            family_name
        )));
        paths.push(PathBuf::from(format!(
            "/System/Library/Fonts/Supplemental/{}.ttc",
            family_name
        )));
        // Also check ~/Library/Fonts
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(PathBuf::from(home).join(format!("Library/Fonts/{}.ttf", family_name)));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(font_dir) = std::env::var_os("WINDIR") {
            paths.push(PathBuf::from(font_dir).join(format!("Fonts/{}.ttf", family_name)));
        }
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from(format!(
            "/usr/share/fonts/truetype/{}/{}.ttf",
            lower, family_name
        )));
        paths.push(PathBuf::from(format!(
            "/usr/share/fonts/truetype/dejavu/{}.ttf",
            family_name
        )));
        paths.push(PathBuf::from(format!(
            "/usr/share/fonts/truetype/ubuntu/{}.ttf",
            family_name
        )));
        paths.push(PathBuf::from(format!(
            "/usr/share/fonts/truetype/noto/{}.ttf",
            family_name
        )));
        paths.push(PathBuf::from(format!(
            "/usr/share/fonts/truetype/liberation/{}.ttf",
            family_name
        )));
        paths.push(PathBuf::from(format!(
            "/usr/share/fonts/truetype/{}.ttf",
            lower
        )));
        paths.push(PathBuf::from(format!(
            "/usr/share/fonts/{}/{}.ttf",
            lower, family_name
        )));
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            paths.push(home_path.join(format!(".local/share/fonts/{}.ttf", family_name)));
            paths.push(home_path.join(format!(".fonts/{}.ttf", family_name)));
        }
    }

    paths
}

/// Dynamically discover installed system fonts by scanning font directories.
/// Returns a Vec of (family_name, file_path) pairs.
pub fn discover_system_fonts() -> Vec<(String, String)> {
    let mut fonts = Vec::new();

    #[cfg(target_os = "macos")]
    {
        let dirs = [
            "/Library/Fonts",
            "/System/Library/Fonts",
            "/System/Library/Fonts/Supplemental",
        ];
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            let user_dirs = [home.join("Library/Fonts")];
            for dir in &user_dirs {
                if dir.is_dir() {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                            if matches!(ext, "ttf" | "otf" | "ttc") {
                                let name = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown")
                                    .to_string();
                                fonts.push((name, path.to_string_lossy().to_string()));
                            }
                        }
                    }
                }
            }
        }
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(ext, "ttf" | "otf" | "ttc") {
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        fonts.push((name, path.to_string_lossy().to_string()));
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let dirs = ["/usr/share/fonts", "/usr/local/share/fonts"];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Ok(sub_entries) = std::fs::read_dir(&path) {
                            for sub_entry in sub_entries.flatten() {
                                let sub_path = sub_entry.path();
                                let ext =
                                    sub_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                if matches!(ext, "ttf" | "otf") {
                                    let name = sub_path
                                        .file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("Unknown")
                                        .to_string();
                                    fonts.push((name, sub_path.to_string_lossy().to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            let fonts_dir = PathBuf::from(windir).join("Fonts");
            if let Ok(entries) = std::fs::read_dir(&fonts_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(ext, "ttf" | "otf" | "ttc") {
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        fonts.push((name, path.to_string_lossy().to_string()));
                    }
                }
            }
        }
    }

    fonts
}
