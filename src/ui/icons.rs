use eframe::egui;

pub const SVG_HOME: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 10.5 12 3l9 7.5"/><path d="M5 9.5V21h14V9.5"/><path d="M9 21v-6h6v6"/></svg>"#;
pub const SVG_FOLDER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6.5h7l2 2h9v10.5H3z"/></svg>"#;
pub const SVG_LAYERS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linejoin="round"><path d="m12 3 9 5-9 5-9-5 9-5Z"/><path d="m3 12 9 5 9-5"/><path d="m3 16 9 5 9-5"/></svg>"#;
pub const SVG_SETTINGS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>"#;
pub const SVG_HELP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M9.5 9a2.5 2.5 0 1 1 4.1 1.9c-1 .8-1.6 1.1-1.6 2.6"/><path d="M12 17h.01"/></svg>"#;
pub const SVG_BOOK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M4 5.5A2.5 2.5 0 0 1 6.5 3H11v17H6.5A2.5 2.5 0 0 0 4 22z"/><path d="M20 5.5A2.5 2.5 0 0 0 17.5 3H13v17h4.5A2.5 2.5 0 0 1 20 22z"/></svg>"#;
pub const SVG_STAR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linejoin="round"><path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-2.9-5.6 2.9 1.1-6.2L3 9.6l6.2-.9L12 3Z"/></svg>"#;
pub const SVG_KEYING: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><rect width="80" height="48" rx="3" fill="#44505d"/><path d="M0 0h10v8H0zM20 0h10v8H20zM10 8h10v8H10zM0 16h10v8H0zM20 16h10v8H20zM10 24h10v8H10zM0 32h10v8H0zM20 32h10v8H20zM10 40h10v8H10z" fill="#65727f"/><rect x="34" width="46" height="48" fill="#52d27b"/><circle cx="40" cy="16" r="7" fill="#d8e0e8" opacity=".9"/><path d="M29 43c1-10 5-16 11-16s10 6 11 16Z" fill="#d8e0e8" opacity=".9"/></svg>"##;
pub const SVG_TRACKING: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48" fill="none" stroke="#9aaeca" stroke-width="1.5"><rect x="15" y="6" width="50" height="36" stroke-dasharray="4 3"/><circle cx="40" cy="24" r="8"/><path d="M40 12v24M28 24h24"/></svg>"##;
pub const SVG_CLEAN_PLATE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48" fill="none"><path d="m20 26 13-16 37 6-13 16Z" fill="#7f94ad" opacity=".5"/><path d="m16 34 13-16 37 6-13 16Z" fill="#9fb0c4" opacity=".64"/><path d="m12 42 13-16 37 6-13 16Z" fill="#c3cedb" opacity=".78"/></svg>"##;
pub const SVG_PARTICLES: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48" fill="#a6b9d8"><circle cx="10" cy="33" r="1.2"/><circle cx="15" cy="22" r="1.8"/><circle cx="21" cy="12" r="1.1"/><circle cx="24" cy="29" r="2"/><circle cx="29" cy="19" r="1.3"/><circle cx="34" cy="38" r="1.7"/><circle cx="37" cy="26" r="1"/><circle cx="41" cy="11" r="1.9"/><circle cx="44" cy="32" r="1.1"/><circle cx="49" cy="18" r="1.5"/><circle cx="53" cy="39" r="1.2"/><circle cx="56" cy="27" r="2.1"/><circle cx="61" cy="13" r="1"/><circle cx="65" cy="34" r="1.7"/><circle cx="71" cy="21" r="1.4"/><circle cx="74" cy="39" r="1"/><circle cx="18" cy="42" r=".9"/><circle cx="31" cy="9" r=".8"/><circle cx="47" cy="7" r=".9"/><circle cx="68" cy="29" r=".8"/></svg>"##;
pub const SVG_TITLE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48" fill="none" stroke="#a6b9d8" stroke-width="1.4"><path d="M13 10v7M13 10h7M67 10v7M67 10h-7M13 38v-7M13 38h7M67 38v-7M67 38h-7"/><text x="40" y="34" text-anchor="middle" font-family="serif" font-size="32" fill="#a6b9d8" stroke="none">T</text></svg>"##;
pub const SVG_SCREEN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48" fill="none"><circle cx="34" cy="24" r="16" fill="#b6c5d9" opacity=".75" stroke="#dce5f0" stroke-width="1"/><circle cx="48" cy="24" r="16" fill="#8091aa" opacity=".75" stroke="#dce5f0" stroke-width="1"/></svg>"##;
pub const SVG_TEMPLATE_CINEMATIC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><defs><radialGradient id="flare" cx="50%" cy="50%" r="50%"><stop offset="0" stop-color="#fff6d7"/><stop offset=".18" stop-color="#ffc36e" stop-opacity=".95"/><stop offset="1" stop-color="#f28a35" stop-opacity="0"/></radialGradient></defs><rect width="80" height="48" rx="3" fill="#151e2a"/><circle cx="40" cy="24" r="18" fill="url(#flare)"/><path d="M8 40 68 8M40 4v40M19 3l42 42M19 45 61 3" stroke="#f3b36b" stroke-width=".7" opacity=".6"/><circle cx="40" cy="24" r="2" fill="#fff8df"/></svg>"##;
pub const SVG_TEMPLATE_LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><rect width="80" height="48" rx="3" fill="#10283d"/><circle cx="40" cy="24" r="12" fill="none" stroke="#73d9e8" stroke-width="1.6"/><path d="M40 10v5M40 33v5M26 24h5M49 24h5" stroke="#73d9e8" stroke-width="1" opacity=".75"/><path d="M40 12 45 24 40 36 35 24Z" fill="#73d9e8" opacity=".9"/><path d="M40 16 42 24 40 30 38 24Z" fill="#10283d"/></svg>"##;
pub const SVG_TEMPLATE_HUD: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><rect width="80" height="48" rx="3" fill="#10202d"/><circle cx="40" cy="24" r="13" fill="none" stroke="#5ccce0"/><circle cx="40" cy="24" r="6" fill="none" stroke="#5ccce0"/><path d="M8 12h18M54 12h18M8 36h18M54 36h18M40 5v10M40 33v10" stroke="#5ccce0"/></svg>"##;
pub const SVG_TEMPLATE_PRODUCT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><defs><linearGradient id="product-body" x1="0" x2="1" y1="0" y2="1"><stop offset="0" stop-color="#f1f3f6"/><stop offset="1" stop-color="#9ca8b6"/></linearGradient></defs><rect width="80" height="48" rx="3" fill="#bdc7d2"/><ellipse cx="42" cy="39" rx="24" ry="4" fill="#69788a" opacity=".45"/><path d="m24 31 11-17 24 5-9 17Z" fill="url(#product-body)"/><path d="m35 14 8-6 24 6-8 5Z" fill="#e7ebef"/><path d="m59 19 8-5v17l-9 5Z" fill="#7b8999"/><path d="m35 20 16 3-5 10-16-3Z" fill="#c5ced8" opacity=".8"/></svg>"##;
pub const SVG_TEMPLATE_BREAKDOWN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><rect width="80" height="48" rx="3" fill="#34465b"/><path d="m0 41 19-18 12 8 17-20 32 30Z" fill="#9aaec3"/><path d="m0 48 25-17 13 7 14-12 28 15Z" fill="#60758e"/></svg>"##;
pub const SVG_TEMPLATE_SOCIAL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><rect width="80" height="48" rx="3" fill="#273d65"/><rect x="34" y="6" width="13" height="36" rx="2" fill="#e879b5"/><text x="40.5" y="29" text-anchor="middle" font-family="sans-serif" font-size="8" fill="white">9:16</text></svg>"##;
pub const SVG_TEMPLATE_TEXT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><rect width="80" height="48" rx="3" fill="#162d45"/><text x="40" y="34" text-anchor="middle" font-family="sans-serif" font-size="27" font-weight="600" fill="#75b7ed">Aa</text></svg>"##;
pub const SVG_TEMPLATE_CHECKER: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><rect width="80" height="48" rx="3" fill="#aab4c0"/><path d="M0 0h20v12H0zM40 0h20v12H40zM20 12h20v12H20zM60 12h20v12H60zM0 24h20v12H0zM40 24h20v12H40zM20 36h20v12H20zM60 36h20v12H60z" fill="#d5dbe2"/></svg>"##;
pub const SVG_ASSET_LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 48"><rect width="80" height="48" rx="3" fill="#03070b"/><circle cx="40" cy="24" r="10" fill="none" stroke="#edf2f8" stroke-width="1.2"/><path d="M40 14 44 24 40 34 36 24Z" fill="#edf2f8"/><path d="M40 18 41.8 24 40 30 38.2 24Z" fill="#03070b"/><path d="M29 35 40 40 51 35" fill="none" stroke="#edf2f8" stroke-width="1" opacity=".8"/></svg>"##;
pub const SVG_FILE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M5 2.75h8.7L19 8.05v13.2H5z"/><path d="M13.7 2.75v5.5H19"/></svg>"#;
pub const SVG_FILE_PLUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M5 2h9l5 5v15H5z"/><path d="M14 2v6h5M12 12v6M9 15h6"/></svg>"#;
pub const SVG_DOCUMENT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M6 2.75h8l4 4v14.5H6z"/><path d="M14 2.75v4h4M9 11h6M9 15h6M9 19h3.5"/></svg>"#;
pub const SVG_POWER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"><path d="M12 2v10"/><path d="M7.1 5.2a9 9 0 1 0 9.8 0"/></svg>"#;
pub const SVG_PLAY_CIRCLE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="m10 8 6 4-6 4z" fill="white" stroke="none"/></svg>"#;
pub const SVG_CHAT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M20 11.5a7.5 7.5 0 0 1-8 7.5 8.5 8.5 0 0 1-4-.9L4 20l1.5-3.2A7.2 7.2 0 0 1 4 12a7.5 7.5 0 0 1 8-7.5 7.5 7.5 0 0 1 8 7z"/><path d="M8 12h.01M12 12h.01M16 12h.01"/></svg>"#;
pub const SVG_OPEN_FOLDER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linejoin="round"><path d="M3 6h7l2 2h9l-2 11H3z"/><path d="M3 6V4h7l2 2"/></svg>"#;
pub const SVG_COMPOSITION: &str = SVG_LAYERS;
pub const SVG_IMPORT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12M7 10l5 5 5-5"/><path d="M4 20h16"/></svg>"#;
pub const SVG_MORE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><circle cx="5" cy="12" r="1.7"/><circle cx="12" cy="12" r="1.7"/><circle cx="19" cy="12" r="1.7"/></svg>"#;
pub const SVG_COMP: &str = SVG_COMPOSITION;
pub const SVG_RESOLUTION: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8"><rect x="3" y="5" width="18" height="14" rx="1"/><path d="M7 9h10M7 13h6"/></svg>"#;
pub const SVG_FRAME: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8"><rect x="4" y="4" width="16" height="16" rx="1"/><path d="M8 1v3M16 1v3M8 20v3M16 20v-3M1 8h3M1 16h3M20 8h3M20 16h-3"/></svg>"#;
pub const SVG_CLOCK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg>"#;
pub const SVG_PROFILE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.5"><circle cx="12" cy="12" r="10" fill="#9aa8bf" stroke="none"/><circle cx="12" cy="9" r="3" fill="#1a2533" stroke="none"/><path d="M6.5 19c1.5-4 9.5-4 11 0" fill="#1a2533" stroke="none"/></svg>"##;
pub const SVG_WINDOW_MINIMIZE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.6" stroke-linecap="round"><path d="M5 12h14"/></svg>"#;
pub const SVG_WINDOW_MAXIMIZE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.6" stroke-linejoin="round"><rect x="5" y="5" width="14" height="14" rx="1"/></svg>"#;
pub const SVG_WINDOW_CLOSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.6" stroke-linecap="round"><path d="m7 7 10 10M17 7 7 17"/></svg>"#;
pub const SVG_ARROW_RIGHT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12h15M13 6l6 6-6 6"/></svg>"#;
pub const SVG_CHEVRON_DOWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
pub const SVG_CHEVRON_RIGHT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m9 6 6 6-6 6"/></svg>"#;
pub const SVG_GRID: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.6"><rect x="4" y="4" width="6" height="6"/><rect x="14" y="4" width="6" height="6"/><rect x="4" y="14" width="6" height="6"/><rect x="14" y="14" width="6" height="6"/></svg>"#;
pub const SVG_SORT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round"><path d="M7 5v14M4 8l3-3 3 3M17 19V5M14 16l3 3 3-3"/></svg>"#;
pub const SVG_CPU: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="6" y="6" width="12" height="12" rx="1"/><rect x="9" y="9" width="6" height="6"/><path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4"/></svg>"#;
pub const SVG_PALETTE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round"><path d="M12 3a9 9 0 0 0 0 18h1.5a2 2 0 0 0 0-4H12a2 2 0 0 1 0-4h2.5A6.5 6.5 0 0 0 12 3Z"/><circle cx="7.5" cy="10" r=".8" fill="white"/><circle cx="10" cy="7" r=".8" fill="white"/><circle cx="14" cy="7" r=".8" fill="white"/></svg>"#;
pub const SVG_KEYBOARD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linejoin="round"><rect x="2" y="5" width="20" height="14" rx="2"/><path d="M5 9h1M9 9h1M13 9h1M17 9h1M5 13h1M9 13h6M17 13h1M6 16h12"/></svg>"#;
pub const SVG_SEARCH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"><circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/></svg>"#;
pub const SVG_FILTER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 5h18l-7 8v5l-4 2v-7L3 5Z"/></svg>"#;
pub const SVG_PLUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>"#;

pub const SVG_EYE_OPEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>"#;
pub const SVG_EYE_CLOSED: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="gray" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>"#;

pub const SVG_LOCK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>"#;
pub const SVG_UNLOCK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="gray" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/></svg>"#;

#[allow(dead_code)]
pub const SVG_PLAY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><polygon points="5 3 19 12 5 21 5 3"/></svg>"#;
#[allow(dead_code)]
pub const SVG_STEP_BACK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M6 5v14"/><path d="m18 6-8 6 8 6Z"/></svg>"#;
#[allow(dead_code)]
pub const SVG_STEP_FORWARD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M18 5v14"/><path d="m6 6 8 6-8 6Z"/></svg>"#;
#[allow(dead_code)]
pub const SVG_JUMP_BACK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><rect x="3" y="4" width="2" height="16" rx="1"/><path d="M19 5 10 12l9 7Z"/><path d="M12 5 3 12l9 7Z"/></svg>"#;
#[allow(dead_code)]
pub const SVG_JUMP_FORWARD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><rect x="19" y="4" width="2" height="16" rx="1"/><path d="m5 5 9 7-9 7Z"/><path d="m12 5 9 7-9 7Z"/></svg>"#;
#[allow(dead_code)]
pub const SVG_PAUSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>"#;
#[allow(dead_code)]
pub const SVG_MARKER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="orange"><path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z"/><line x1="4" y1="22" x2="4" y2="15" stroke="orange" stroke-width="2"/></svg>"#;
#[allow(dead_code)]
pub const SVG_KEYFRAME: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="cyan"><polygon points="12 2 22 12 12 22 2 12 12 2"/></svg>"#;
#[allow(dead_code)]
pub const SVG_AUDIO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="deepskyblue" stroke-width="2"><path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07"/></svg>"#;
#[allow(dead_code)]
pub const SVG_GPU: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="15" x2="23" y2="15"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="15" x2="4" y2="15"/></svg>"#;

// ── AE Tool SVG Icons ──
#[allow(dead_code)]
pub const SVG_TOOL_SELECT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><path d="M3 3l7 18 3-7 7-3L3 3z"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_HAND: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M18 11V6a2 2 0 0 0-4 0v5M14 10V4a2 2 0 0 0-4 0v6M10 10.5V2.5a2 2 0 0 0-4 0v11M6 14v-1.5a2 2 0 0 0-4 0V16a8 8 0 0 0 16 0v-5a2 2 0 0 0-4 0"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_ZOOM: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="11" y1="8" x2="11" y2="14"/><line x1="8" y1="11" x2="14" y2="11"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_CAMERA: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/><circle cx="12" cy="13" r="4"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_ROTATE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_SHAPE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_PEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M12 19l7-7 3 3-7 7-3-3z"/><path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z"/><path d="M2 2l7.5 7.5"/><circle cx="11" cy="11" r="2"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_TEXT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><path d="M5 4h14v3h-5.5v13h-3V7H5V4z"/></svg>"#;

// ── AE Switch SVG Icons ──
#[allow(dead_code)]
pub const SVG_SWITCH_3D: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="deepskyblue" stroke-width="2"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>"#;
#[allow(dead_code)]
pub const SVG_SWITCH_SOLO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="gold" stroke-width="2"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="4"/><line x1="12" y1="2" x2="12" y2="4"/><line x1="12" y1="20" x2="12" y2="22"/><line x1="2" y1="12" x2="4" y2="12"/><line x1="20" y1="12" x2="22" y2="12"/></svg>"#;
#[allow(dead_code)]
pub const SVG_SWITCH_GRAPH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="cyan" stroke-width="2"><path d="M3 3v18h18"/><path d="M19 9l-5 5-4-4-3 3"/></svg>"#;

/// Setup egui_extras image loaders (supports SVG, PNG, GIF, etc.)
pub fn init_image_loaders(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
}

/// Render an SVG string directly as an egui Image widget.
/// SVG bytes are borrowed statically; only the cache URI is allocated per call.
fn svg_uri(name: &str, svg: &str) -> String {
    format!("bytes://{name}-{}.svg", egui::Id::new(svg).value())
}

pub fn render_svg_bytes(
    ui: &mut egui::Ui,
    name: &str,
    svg_str: &'static str,
    size: egui::Vec2,
    tint: egui::Color32,
) -> egui::Response {
    ui.add(
        egui::Image::new(egui::ImageSource::Bytes {
            uri: std::borrow::Cow::Owned(svg_uri(name, svg_str)),
            bytes: egui::load::Bytes::Static(svg_str.as_bytes()),
        })
        .fit_to_exact_size(size)
        .tint(tint),
    )
}

// ── Additional Tool Icons ──
#[allow(dead_code)]
pub const SVG_TOOL_ANCHOR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="12" cy="5" r="2"/><circle cx="5" cy="19" r="2"/><circle cx="19" cy="19" r="2"/><path d="M12 7v4M12 11L6 17M12 11l6 6"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_BRUSH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M9.06 11.9l8.07-8.06a2.85 2.85 0 1 1 4.03 4.03l-8.06 8.08"/><path d="M7.07 14.94c-1.66 0-3 1.35-3 3.02 0 1.33-2.5 1.52-2 2.02 1.08 1.1 2.49 2.02 4 2.02 2.2 0 4-1.8 4-4.04a3.01 3.01 0 0 0-3-3.02z"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_STAMP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M5 22h14"/><path d="M19.27 13.73A2.5 2.5 0 0 0 17.5 13h-11A2.5 2.5 0 0 0 4 15.5V17a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-1.5c0-.66-.26-1.3-.73-1.77z" transform="translate(0 -1)"/><path d="M14 13V8.5C14 7 15 7 15 5a3 3 0 0 0-6 0c0 2 1 2 1 3.5V13"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_ERASER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M20 20H7L3 16a1.41 1.41 0 0 1 0-2L13 4a1.41 1.41 0 0 1 2 0l6 6a1.41 1.41 0 0 1 0 2l-9 9"/><line x1="8.5" y1="8.5" x2="15.5" y2="15.5"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_ROTO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><circle cx="12" cy="18" r="3"/><path d="M8.5 7.5L10.5 15M15.5 7.5L13.5 15M9 6h6"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_PUPPET: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="12" cy="4" r="2" fill="white"/><path d="M12 6v6M12 12l-5 6M12 12l5 6M6 20h12"/></svg>"#;
#[allow(dead_code)]
pub const SVG_RENDER_QUEUE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><rect x="2" y="4" width="20" height="16" rx="2"/><polygon points="10 8 16 12 10 16 10 8" fill="white" stroke="none"/></svg>"#;
#[allow(dead_code)]
pub const SVG_SNAP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M3 3v18M21 3v18"/><path d="M7 12h10"/><path d="M7 12l3-3M7 12l3 3M17 12l-3-3M17 12l-3 3"/></svg>"#;

// ── Professional Motion Graphics & VFX SVG Icons ──
#[allow(dead_code)]
pub const SVG_SHY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="cyan" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M8 15s1.5 2 4 2 4-2 4-2"/><line x1="9" y1="9" x2="9.01" y2="9"/><line x1="15" y1="9" x2="15.01" y2="9"/></svg>"#;

#[allow(dead_code)]
pub const SVG_COLLAPSE_TRANSFORM: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="orange" stroke-width="2"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/></svg>"#;

#[allow(dead_code)]
pub const SVG_QUALITY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="lightgreen" stroke-width="2"><line x1="4" y1="20" x2="20" y2="4"/><line x1="4" y1="14" x2="14" y2="4"/><line x1="10" y1="20" x2="20" y2="10"/></svg>"#;

#[allow(dead_code)]
pub const SVG_MOTION_BLUR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="deepskyblue" stroke-width="2"><circle cx="8" cy="12" r="6" opacity="0.4"/><circle cx="12" cy="12" r="6" opacity="0.7"/><circle cx="16" cy="12" r="6"/></svg>"#;

#[allow(dead_code)]
pub const SVG_TRIM_PATHS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="magenta" stroke-width="2"><circle cx="12" cy="12" r="9" stroke-dasharray="14 6"/><circle cx="12" cy="3" r="2" fill="magenta"/></svg>"#;

#[allow(dead_code)]
pub const SVG_EASING_BEZIER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="gold" stroke-width="2"><path d="M3 21C8 21 8 3 21 3"/><circle cx="3" cy="21" r="2" fill="gold"/><circle cx="21" cy="3" r="2" fill="gold"/></svg>"#;

#[allow(dead_code)]
pub const SVG_GRAPH_EDITOR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="cyan" stroke-width="2"><path d="M3 3v18h18"/><path d="M3 18C9 18 11 6 21 6"/><circle cx="11" cy="12" r="1.5" fill="cyan"/></svg>"#;

#[allow(dead_code)]
pub const SVG_3D_CUBE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="deepskyblue" stroke-width="2"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>"#;

#[allow(dead_code)]
pub const SVG_EXPRESSION: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="pink" stroke-width="2"><path d="M4 19l4-14M14 8h6M17 5v6M9 12h5"/></svg>"#;

#[allow(dead_code)]
pub const SVG_LIGHT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="yellow" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>"#;

#[allow(dead_code)]
pub const SVG_CAMERA: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="lightskyblue" stroke-width="2"><path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/><circle cx="12" cy="13" r="4"/></svg>"#;

#[allow(dead_code)]
pub const SVG_TEXT_TO_SHAPES: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M4 7V4h10v3M9 4v11"/><polygon points="17 13 22 17 17 21 12 17" fill="cyan" stroke="none"/></svg>"#;

#[allow(dead_code)]
pub const SVG_HOT_RELOAD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="lime" stroke-width="2"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/></svg>"#;

#[allow(dead_code)]
pub const SVG_BRAIN_AI: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="violet" stroke-width="2"><path d="M12 2a4 4 0 0 0-4 4v1a3 3 0 0 0-3 3v2a3 3 0 0 0 2 2.82V16a4 4 0 0 0 4 4h2a4 4 0 0 0 4-4v-1.18A3 3 0 0 0 19 12v-2a3 3 0 0 0-3-3V6a4 4 0 0 0-4-4z"/><path d="M9 12h6M12 9v6"/></svg>"#;

#[allow(dead_code)]
pub const SVG_EXPORT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M4 15.5v4a1.75 1.75 0 0 0 1.75 1.75h12.5A1.75 1.75 0 0 0 20 19.5v-4"/><path d="M12 16V3M7.5 7.5 12 3l4.5 4.5"/></svg>"#;
pub const SVG_EFFECTS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round"><path d="M3 6h18M3 12h18M3 18h18"/><circle cx="8" cy="6" r="2" fill="#0d161d"/><circle cx="16" cy="12" r="2" fill="#0d161d"/><circle cx="10" cy="18" r="2" fill="#0d161d"/></svg>"##;

/// The application logo mark: a stylized composition frame with a playhead.
/// Drawn procedurally so it stays crisp at any size and adapts to the theme.
pub fn draw_logo(ui: &mut egui::Ui, size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    let accent = egui::Color32::from_rgb(0, 160, 240);
    let purple = egui::Color32::from_rgb(150, 80, 220);

    // Rounded frame
    p.rect_stroke(rect.shrink(1.0), 3.0, egui::Stroke::new(1.6_f32, accent));
    // Diagonal split suggesting motion
    let tl = rect.left_top();
    let br = rect.right_bottom();
    let mid = egui::pos2(rect.center().x, rect.center().y);
    p.line_segment([tl, mid], egui::Stroke::new(1.2_f32, purple));
    p.line_segment([mid, br], egui::Stroke::new(1.2_f32, purple));
    // Playhead diamond at center
    let c = rect.center();
    let s = size * 0.14;
    let pts = [
        egui::pos2(c.x, c.y - s),
        egui::pos2(c.x + s, c.y),
        egui::pos2(c.x, c.y + s),
        egui::pos2(c.x - s, c.y),
    ];
    p.add(egui::Shape::convex_polygon(
        pts.to_vec(),
        accent,
        egui::Stroke::NONE,
    ));
    resp
}

/// Renders an SVG at an arbitrary position (used by icon-button painting).
pub fn render_svg_at(
    ui: &mut egui::Ui,
    name: String,
    svg_str: &'static str,
    size: egui::Vec2,
    tint: egui::Color32,
    pos: egui::Pos2,
) -> egui::Response {
    ui.put(
        egui::Rect::from_min_size(pos, size),
        egui::Image::new(egui::ImageSource::Bytes {
            uri: std::borrow::Cow::Owned(svg_uri(&name, svg_str)),
            bytes: egui::load::Bytes::Static(svg_str.as_bytes()),
        })
        .fit_to_exact_size(size)
        .tint(tint),
    )
}

// ── Phosphor glyph helpers (registered via theme::configure_fonts) ──

/// Crisp icon glyph for a layer type, used in timeline rows & panels.
pub fn layer_icon(lt: &crate::core::timeline::LayerType) -> &'static str {
    use crate::core::timeline::LayerType;
    use egui_phosphor::regular as p;
    match lt {
        LayerType::Video { .. } => p::FILM_STRIP,
        LayerType::Image { .. } => p::IMAGE,
        LayerType::Audio { .. } => p::WAVEFORM,
        LayerType::Text { .. } => p::TEXT_T,
        LayerType::Shape { .. } => p::POLYGON,
        LayerType::Solid { .. } => p::SQUARE,
        LayerType::Null => p::CIRCLE,
        LayerType::PreComp { .. } => p::PACKAGE,
        LayerType::AdjustmentLayer => p::CIRCLE_HALF,
        LayerType::Particle { .. } => p::SPARKLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_icons_decode_and_state_variants_do_not_share_cache_entries() {
        let ctx = egui::Context::default();
        init_image_loaders(&ctx);
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_svg_bytes(
                    ui,
                    "visibility",
                    SVG_EYE_OPEN,
                    egui::vec2(18.0, 18.0),
                    egui::Color32::WHITE,
                );
                render_svg_bytes(
                    ui,
                    "visibility",
                    SVG_EYE_CLOSED,
                    egui::vec2(18.0, 18.0),
                    egui::Color32::WHITE,
                );
                render_svg_at(
                    ui,
                    "hand".into(),
                    SVG_TOOL_HAND,
                    egui::vec2(24.0, 24.0),
                    egui::Color32::WHITE,
                    egui::pos2(100.0, 100.0),
                );
            });
        });
        assert_ne!(
            svg_uri("visibility", SVG_EYE_OPEN),
            svg_uri("visibility", SVG_EYE_CLOSED)
        );
        for (name, svg) in [
            ("visibility", SVG_EYE_OPEN),
            ("visibility", SVG_EYE_CLOSED),
            ("hand", SVG_TOOL_HAND),
        ] {
            assert!(matches!(
                ctx.try_load_image(&svg_uri(name, svg), egui::load::SizeHint::Size(24, 24)),
                Ok(egui::load::ImagePoll::Ready { .. })
            ));
        }
    }
}
