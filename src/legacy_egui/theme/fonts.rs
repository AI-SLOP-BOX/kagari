use super::layout;
use eframe::egui;

/// Preserve the bundled font metrics so mixed Japanese and Latin runs share one baseline.
fn japanese_ui_font(data: Vec<u8>) -> egui::FontData {
    egui::FontData::from_owned(data)
}

pub(crate) fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Load system fonts for professional appearance
    #[cfg(target_os = "macos")]
    {
        let sf_pro_paths = [
            "/System/Library/Fonts/SFNS.ttf",
            "/Library/Fonts/SF-Pro-Display-Regular.otf",
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/SFCompact.ttf",
        ];
        let menlo_paths = [
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/Menlo-Regular.ttc",
        ];
        let mut jp_font_paths = [
            "/System/Library/Fonts/ヒラギノ角ゴシック W4.ttc",
            "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/Library/Fonts/LINESeedJP_OTF_Rg.otf",
            "/Library/Fonts/LINESeedJP_OTF_Rg.ttf",
        ]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
        if let Some(home) = std::env::var_os("HOME") {
            let user_fonts = std::path::PathBuf::from(home).join("Library/Fonts");
            jp_font_paths.extend(
                [
                    "LINESeedJP_OTF_Rg.otf",
                    "LINESeedJP_OTF_Rg.ttf",
                    "NotoSansCJKjp-Regular.otf",
                    "NotoSansJP-Regular.ttf",
                ]
                .map(|name| user_fonts.join(name)),
            );
        }
        let inter_paths = [
            "/Library/Fonts/Inter.ttf",
            "/System/Library/Fonts/Supplemental/Inter.ttf",
            "/Library/Fonts/Inter-Regular.ttf",
        ];

        // Try to load Inter for Latin UI (primary)
        let mut loaded_inter = false;
        for path in &inter_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("Inter".to_string(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "Inter".to_string());
                loaded_inter = true;
                break;
            }
        }
        // Fallback to SF Pro if Inter not available
        if !loaded_inter {
            for path in &sf_pro_paths {
                if let Ok(data) = std::fs::read(path) {
                    fonts
                        .font_data
                        .insert("SFPro".to_string(), egui::FontData::from_owned(data));
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, "SFPro".to_string());
                    loaded_inter = true;
                    break;
                }
            }
        }
        if !loaded_inter {
            log::info!("Using egui default proportional font (system fonts not found)");
        }

        // Load monospace font (Menlo)
        let mut loaded_mono = false;
        for path in &menlo_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("Menlo".to_string(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "Menlo".to_string());
                loaded_mono = true;
                break;
            }
        }
        if !loaded_mono {
            log::info!("Using egui default monospace font (Menlo not found)");
        }

        // Load Japanese font (Hiragino / LINE Seed JP fallback)
        for path in &jp_font_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("LineSeedJP".to_string(), japanese_ui_font(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("LineSeedJP".to_string());
                break;
            }
        }
    }

    // Load Linux system fonts (DejaVu Sans, Ubuntu, Noto Sans, Liberation)
    #[cfg(target_os = "linux")]
    {
        let linux_prop_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ];
        let mut linux_jp_paths = [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
            "/usr/share/fonts/truetype/noto/NotoSansJP-Regular.ttf",
        ]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
        let jp_font_names = [
            "NotoSansCJK-Regular.ttc",
            "NotoSansCJKjp-Regular.otf",
            "NotoSansJP-Regular.ttf",
            "LINESeedJP_OTF_Rg.otf",
        ];
        if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
            let user_fonts = std::path::PathBuf::from(data_home).join("fonts");
            linux_jp_paths.extend(jp_font_names.map(|name| user_fonts.join(name)));
        }
        if let Some(home) = std::env::var_os("HOME") {
            let home = std::path::PathBuf::from(home);
            for font_dir in [home.join(".local/share/fonts"), home.join(".fonts")] {
                linux_jp_paths.extend(jp_font_names.map(|name| font_dir.join(name)));
            }
        }
        let linux_mono_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        ];

        for path in &linux_prop_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("LinuxSystemFont".to_string(), japanese_ui_font(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "LinuxSystemFont".to_string());
                break;
            }
        }
        for path in &linux_mono_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("LinuxMonoFont".to_string(), japanese_ui_font(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "LinuxMonoFont".to_string());
                break;
            }
        }
        for path in &linux_jp_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "LinuxJapaneseFont".to_string(),
                    egui::FontData::from_owned(data),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("LinuxJapaneseFont".to_string());
                break;
            }
        }
    }

    // Load Windows system fonts (Segoe UI, Consolas)
    #[cfg(target_os = "windows")]
    {
        let win_prop_paths = [
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];
        let win_mono_paths = ["C:\\Windows\\Fonts\\consola.ttf"];
        let mut win_jp_paths = [
            "C:\\Windows\\Fonts\\YuGothR.ttc",
            "C:\\Windows\\Fonts\\YuGothM.ttc",
            "C:\\Windows\\Fonts\\msgothic.ttc",
            "C:\\Windows\\Fonts\\NotoSansCJKjp-Regular.otf",
            "C:\\Windows\\Fonts\\NotoSansJP-Regular.otf",
        ]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
        if let Some(user_font_dir) = std::env::var_os("LOCALAPPDATA") {
            let user_font_dir = std::path::PathBuf::from(user_font_dir)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts");
            win_jp_paths.extend([
                user_font_dir.join("NotoSansCJKjp-Regular.otf"),
                user_font_dir.join("NotoSansJP-Regular.otf"),
            ]);
        }

        for path in &win_prop_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("SegoeUI".to_string(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "SegoeUI".to_string());
                break;
            }
        }
        for path in &win_mono_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("Consolas".to_string(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "Consolas".to_string());
                break;
            }
        }
        for path in &win_jp_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "WindowsJapaneseFont".to_string(),
                    egui::FontData::from_owned(data),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("WindowsJapaneseFont".to_string());
                break;
            }
        }
    }

    // Keep the product UI visually consistent even when the host OS does not
    // provide Inter. The bundled variable font contains the requested UI weights.
    if let Some(data) = crate::ui::embedded_assets::bytes("assets/fonts/Inter.ttf") {
        fonts.font_data.insert(
            "KagariInter".to_string(),
            egui::FontData::from_owned(data.to_vec()),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "KagariInter".to_string());
    }

    if let Some(data) = crate::ui::embedded_assets::bytes("assets/fonts/NotoSansJP-Regular.otf") {
        fonts.font_data.insert(
            "KagariJapanese".to_string(),
            japanese_ui_font(data.to_vec()),
        );
        let proportional = fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default();
        proportional.insert(0, "KagariJapanese".to_string());
    }

    // Phosphor icon glyphs (used across panels for crisp vector icons)
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    ctx.set_fonts(fonts);

    ctx.style_mut(|style| {
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(layout::FONT_SIZE_SMALL, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(layout::FONT_SIZE_HEADING, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Monospace),
        );
    });
}
