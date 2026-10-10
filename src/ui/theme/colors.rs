use eframe::egui::Color32;

// ── Background Layers (darkest → lightest) ──
/// Deep application background (#121418 - ultra-dense studio base).
pub const BG_DEEPEST: Color32 = Color32::from_rgb(18, 20, 24);
/// Standard panel surface (#1A1D24 - sleek modern panel).
pub const BG_DARKEST: Color32 = Color32::from_rgb(26, 29, 36);
pub const BG_DARK: Color32 = Color32::from_rgb(26, 29, 36);
/// Raised surface for rows and controls (#222731).
pub const BG_MID: Color32 = Color32::from_rgb(34, 39, 49);
pub const BG_PANEL: Color32 = Color32::from_rgb(34, 39, 49);
/// Deep input surface (#161820).
pub const BG_SURFACE: Color32 = Color32::from_rgb(22, 24, 32);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(40, 46, 58);
pub const BG_PANEL_ALT: Color32 = Color32::from_rgb(26, 29, 36);
pub const BG_PANEL_DEEP: Color32 = Color32::from_rgb(18, 20, 24);
pub const BG_PANEL_DARK: Color32 = Color32::from_rgb(26, 29, 36);
pub const BG_PANEL_BASE: Color32 = Color32::from_rgb(26, 29, 36);
pub const BG_PANEL_RAISED: Color32 = Color32::from_rgb(34, 39, 49);
pub const BG_EXTREME: Color32 = Color32::from_rgb(20, 22, 28);

// ── Interactive States ──
pub const BG_HOVER: Color32 = Color32::from_rgb(31, 36, 40);
pub const BG_ACTIVE: Color32 = Color32::from_rgb(52, 34, 27);
pub const BG_PRESSED: Color32 = Color32::from_rgb(66, 40, 29);
pub const BG_SELECTION: Color32 = Color32::from_rgb(52, 34, 27);
pub const BG_SELECTION_HOVER: Color32 = Color32::from_rgb(66, 44, 35);

// ── Accent Colors (Restrained Production Palette) ──
/// Saturated blue retained for domain graphics that specifically encode blue.
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(14, 120, 220);
/// Compatibility token for interface accents, aligned with Kagari's warm brand color.
pub const ACCENT_CYAN: Color32 = ACCENT_BRAND;
/// Muted amber - timecode, warnings, keyframes.
pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(213, 168, 90);
/// Muted green - success, solo states.
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(105, 185, 157);
/// Red - error, mute. Kept readable; errors must pop.
pub const ACCENT_RED: Color32 = Color32::from_rgb(214, 107, 107);
/// Warning amber. Reserve this token for warning states.
pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(245, 155, 0);
pub const ACCENT_BRAND: Color32 = Color32::from_rgb(255, 107, 22);
pub const ACCENT_BRAND_HOVER: Color32 = Color32::from_rgb(255, 145, 50);
pub const ACCENT_SELECTION: Color32 = ACCENT_BRAND;
pub const ACCENT_TOGGLE: Color32 = ACCENT_SELECTION;
pub const ACCENT_WORKSPACE: Color32 = ACCENT_SELECTION;
pub const ACCENT_PRIMARY_ACTION: Color32 = ACCENT_SELECTION;
pub const ACCENT_WARNING: Color32 = Color32::from_rgb(213, 168, 90);
pub const ACCENT_PROGRESS: Color32 = ACCENT_SELECTION;
pub const ACCENT_INFORMATION: Color32 = ACCENT_SELECTION;
pub const ACCENT_BEAT_MARKER: Color32 = ACCENT_YELLOW;
/// Purple - expressions, advanced.
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(155, 110, 230);
pub const ACCENT_BLUE_LIGHT: Color32 = Color32::from_rgb(90, 160, 255);
pub const ACCENT_ACTION: Color32 = ACCENT_PRIMARY_ACTION;
pub const ACCENT_ACTION_HOVER: Color32 = ACCENT_BRAND_HOVER;
pub const ACCENT_ACTION_STROKE: Color32 = ACCENT_SELECTION;
pub const METER_BAD: Color32 = Color32::from_rgb(255, 82, 78);
pub const METER_WARNING: Color32 = Color32::from_rgb(255, 190, 45);
pub const METER_GOOD: Color32 = Color32::from_rgb(42, 211, 86);
pub const OVERLAY_SURFACE: Color32 = Color32::from_rgba_premultiplied(18, 22, 30, 225);
/// Dark glassmorphism background tint with subtle translucency.
pub const GLASS_SURFACE: Color32 = Color32::from_rgba_premultiplied(22, 25, 33, 220);
pub const GLASS_BORDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 28);

// ── Borders (crisp 1px with modern cool-slate tone) ──
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 43, 54);
pub const BORDER_MEDIUM: Color32 = Color32::from_rgb(48, 55, 69);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(64, 73, 92);
pub const BORDER_ACTIVE: Color32 = ACCENT_SELECTION;
pub const BORDER_FOCUS: Color32 = BORDER_ACTIVE;

// ── Typography Colors ──
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(225, 228, 230);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(158, 166, 172);
pub const TEXT_SECONDARY_BRIGHT: Color32 = Color32::from_rgb(193, 199, 203);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(112, 120, 125);
pub const TEXT_ACCENT: Color32 = ACCENT_SELECTION;
pub const TEXT_ON_ACCENT: Color32 = Color32::from_rgb(255, 255, 255);

pub const TUTORIAL_HINT_BACKGROUND: Color32 = Color32::from_rgb(25, 29, 33);
pub const TUTORIAL_COMPLETE_BACKGROUND: Color32 = Color32::from_rgb(32, 58, 56);

// ── Layer Label Colors (AE standard) ──
pub const LABEL_RED: Color32 = Color32::from_rgb(255, 60, 60);
pub const LABEL_ORANGE: Color32 = Color32::from_rgb(255, 160, 40);
pub const LABEL_YELLOW: Color32 = Color32::from_rgb(255, 230, 50);
pub const LABEL_GREEN: Color32 = Color32::from_rgb(80, 220, 100);
pub const LABEL_CYAN: Color32 = Color32::from_rgb(50, 210, 255);
pub const LABEL_BLUE: Color32 = Color32::from_rgb(50, 140, 255);
pub const LABEL_PURPLE: Color32 = Color32::from_rgb(160, 110, 255);
pub const LABEL_MAGENTA: Color32 = Color32::from_rgb(230, 80, 200);

// ── Viewport Overlay Colors (desaturated; overlays must not glow) ──
pub const GRID_LINE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 28);
pub const MOTION_PATH: Color32 = crate::ui::theme::colors::ACCENT_CYAN;
pub const KEYFRAME_DOT: Color32 = Color32::from_rgb(205, 170, 95);
pub const GUIDE_LINE: Color32 = crate::ui::theme::colors::ACCENT_CYAN;
pub const HUD_BG: Color32 = Color32::from_rgba_premultiplied(14, 20, 30, 220);
pub const HUD_STROKE: Color32 = crate::ui::theme::colors::ACCENT_CYAN;
pub const HUD_TEXT: Color32 = Color32::from_rgb(175, 195, 215);
pub const HUD_STATUS_TEXT: Color32 = Color32::from_rgb(165, 185, 205);
pub const FPS_GOOD: Color32 = Color32::from_rgb(75, 155, 115);
pub const FPS_BAD: Color32 = Color32::from_rgb(220, 110, 95);

// ── 3D Gizmo Colors ──
pub const GIZMO_X: Color32 = Color32::from_rgb(235, 70, 70);
pub const GIZMO_Y: Color32 = Color32::from_rgb(60, 210, 80);
pub const GIZMO_Z: Color32 = Color32::from_rgb(60, 140, 245);
pub const BBOX_STROKE: Color32 = Color32::from_rgb(70, 135, 190);
pub const HANDLE_NORMAL: Color32 = Color32::WHITE;
pub const HANDLE_HOVER_FILL: Color32 = Color32::from_rgb(215, 185, 110);
pub const HANDLE_HOVER_STROKE: Color32 = Color32::from_rgb(225, 110, 60);
pub const CENTER_DOT: Color32 = Color32::from_rgb(210, 175, 85);
pub const CENTER_HOVER_RING: Color32 = Color32::from_rgb(70, 135, 190);

// ── Timeline Overlay Colors ──
pub const TIMELINE_PLAYHEAD: Color32 = ACCENT_BRAND;
pub const TIMELINE_KEYFRAME: Color32 = Color32::from_rgb(205, 165, 90);
pub const TIMELINE_CACHE: Color32 = Color32::from_rgb(176, 105, 66);
pub const TIMELINE_WAVEFORM: Color32 = Color32::from_rgb(176, 105, 66);
pub const TIMELINE_SELECTION: Color32 = Color32::from_rgba_premultiplied(38, 16, 3, 38);
