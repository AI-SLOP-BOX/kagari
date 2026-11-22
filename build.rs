use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn collect(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files);
        } else if path.is_file() {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
}

fn generate_embedded_assets(root: &Path, output: &Path) {
    let assets = root.join("assets");
    println!("cargo:rerun-if-changed={}", assets.display());
    let mut files = Vec::new();
    collect(&assets, &assets, &mut files);
    files.sort();

    let mut generated = String::from("pub const EMBEDDED_PATHS: &[&str] = &[\n");
    for relative in &files {
        let key = format!("assets/{}", relative.to_string_lossy().replace('\\', "/"));
        println!("cargo:rerun-if-changed={}", assets.join(relative).display());
        writeln!(generated, "    {:?},", key).unwrap();
    }
    generated
        .push_str("];\n\npub fn bytes(path: &str) -> Option<&'static [u8]> {\n    match path {\n");
    for relative in files {
        let key = format!("assets/{}", relative.to_string_lossy().replace('\\', "/"));
        let source = assets.join(&relative);
        writeln!(
            generated,
            "        {:?} => Some(include_bytes!({:?}).as_slice()),",
            key, source
        )
        .unwrap();
    }
    generated.push_str("        _ => None,\n    }\n}\n");

    std::fs::write(output.join("embedded_assets.rs"), generated).expect("write embedded assets");
}

struct LocaleCatalog {
    code: String,
    label: String,
    keywords: String,
    command: String,
    order: i64,
    entries: BTreeMap<String, String>,
}

fn escape_ame_str(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn load_locales(locales_dir: &Path) -> Vec<LocaleCatalog> {
    println!("cargo:rerun-if-changed={}", locales_dir.display());
    let mut paths = Vec::new();
    if let Ok(entries) = std::fs::read_dir(locales_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                paths.push(path);
            }
        }
    }
    paths.sort();

    let mut locales = Vec::new();
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        let code = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("locale file stem")
            .to_owned();
        let raw = std::fs::read_to_string(&path).expect("read locale json");
        let parsed: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()));
        let obj = parsed
            .as_object()
            .unwrap_or_else(|| panic!("{} must be a JSON object", path.display()));

        let meta = obj.get("_meta").and_then(|v| v.as_object());
        let label = meta
            .and_then(|m| m.get("label"))
            .and_then(|v| v.as_str())
            .unwrap_or(&code)
            .to_owned();
        let keywords = meta
            .and_then(|m| m.get("keywords"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let command = meta
            .and_then(|m| m.get("command"))
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("Language: {label}"));
        let order = meta
            .and_then(|m| m.get("order"))
            .and_then(|v| v.as_i64())
            .unwrap_or(100);

        let mut entries = BTreeMap::new();
        for (key, value) in obj {
            if key.starts_with('_') {
                continue;
            }
            let text = value
                .as_str()
                .unwrap_or_else(|| panic!("key `{key}` in {} must be a string", path.display()));
            entries.insert(key.clone(), text.to_owned());
        }

        locales.push(LocaleCatalog {
            code,
            label,
            keywords,
            command,
            order,
            entries,
        });
    }

    locales.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.code.cmp(&b.code)));
    locales
}

fn generate_i18n(root: &Path, output: &Path) {
    let locales_dir = root.join("src/ui/locales");
    let locales = load_locales(&locales_dir);
    assert!(
        locales.iter().any(|l| l.code == "en"),
        "src/ui/locales/en.json must exist as the fallback locale"
    );

    // 1. i18n_strings.ame
    let mut strings_ame = String::from("translations(locale: ui_locale, fallback: \"en\") {\n");
    for locale in &locales {
        writeln!(strings_ame, "  locale {} {{", escape_ame_str(&locale.code)).unwrap();
        for (key, value) in &locale.entries {
            writeln!(strings_ame, "    {key}: {};", escape_ame_str(value)).unwrap();
        }
        strings_ame.push_str("  }\n");
    }
    strings_ame.push_str("}\n");
    std::fs::write(output.join("i18n_strings.ame"), strings_ame).expect("write i18n_strings.ame");

    // 2. i18n_locale_select.ame
    let mut select_ame = String::from(
        "component LocaleSelect() {\n  Select(ui_locale, action: setting_locale_changed) {\n",
    );
    for locale in &locales {
        writeln!(
            select_ame,
            "    Option({}, {})",
            escape_ame_str(&locale.code),
            escape_ame_str(&locale.label)
        )
        .unwrap();
    }
    select_ame.push_str("  }\n}\n");
    std::fs::write(output.join("i18n_locale_select.ame"), select_ame)
        .expect("write i18n_locale_select.ame");

    // 3. i18n_locales.rs
    let mut locales_rs = String::from(
        "#[allow(dead_code)]\npub struct LocaleMeta {\n    pub code: &'static str,\n    pub label: &'static str,\n    pub keywords: &'static str,\n}\n\n#[allow(dead_code)]\npub const SUPPORTED_LOCALES: &[LocaleMeta] = &[\n",
    );
    let mut all_keywords = Vec::new();
    for locale in &locales {
        writeln!(
            locales_rs,
            "    LocaleMeta {{ code: {:?}, label: {:?}, keywords: {:?} }},",
            locale.code, locale.label, locale.keywords
        )
        .unwrap();
        all_keywords.push(locale.code.clone());
        if !locale.keywords.is_empty() {
            all_keywords.push(locale.keywords.clone());
        }
    }
    writeln!(
        locales_rs,
        "];\n\n#[allow(dead_code)]\npub const LOCALE_SEARCH_KEYWORDS: &str = {:?};\n\n#[allow(dead_code)]\npub fn is_supported_locale(code: &str) -> bool {{\n    SUPPORTED_LOCALES.iter().any(|locale| locale.code == code)\n}}",
        all_keywords.join(" ")
    )
    .unwrap();
    std::fs::write(output.join("i18n_locales.rs"), locales_rs).expect("write i18n_locales.rs");

    // 4. i18n_kagari_studio.ame
    let studio_path = root.join("src/ui/ame/kagari_studio.ame");
    println!("cargo:rerun-if-changed={}", studio_path.display());
    let studio_src = std::fs::read_to_string(&studio_path).expect("read kagari_studio.ame");
    let start_marker = "// <generated-locale-commands>";
    let end_marker = "// </generated-locale-commands>";
    let start_idx = studio_src
        .find(start_marker)
        .expect("missing // <generated-locale-commands> in kagari_studio.ame");
    let end_idx = studio_src
        .find(end_marker)
        .expect("missing // </generated-locale-commands> in kagari_studio.ame");
    assert!(end_idx > start_idx, "invalid locale commands markers");

    let mut commands = String::new();
    for locale in &locales {
        writeln!(
            commands,
            "              Command({}, {})",
            escape_ame_str(&format!("switch_locale_{}", locale.code)),
            escape_ame_str(&locale.command)
        )
        .unwrap();
    }
    let generated_studio = format!(
        "{}{}\n{}              {}",
        &studio_src[..start_idx],
        start_marker,
        commands,
        &studio_src[end_idx..]
    );
    std::fs::write(output.join("i18n_kagari_studio.ame"), generated_studio)
        .expect("write i18n_kagari_studio.ame");
}

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("out dir"));
    generate_embedded_assets(&root, &output);
    generate_i18n(&root, &output);
}
