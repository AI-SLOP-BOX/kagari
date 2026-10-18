use eframe::egui::{self, Key};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutId {
    CommandPalette,
    TogglePlayback,
    Undo,
    Redo,
    Save,
    NewComposition,
    OpenRenderQueue,
    EasyEase,
    DuplicateLayer,
    SelectionTool,
    HandTool,
    ZoomTool,
    RotationTool,
    AnchorTool,
    RectangleTool,
    PenTool,
    CameraTool,
    TextTool,
    PuppetPinTool,
    PreviousKeyframe,
    NextKeyframe,
    PlayForward,
    WorkAreaStart,
    WorkAreaEnd,
    LayerInPoint,
    LayerOutPoint,
    RevealOpacity,
    RevealPosition,
    RevealScale,
    RevealRotation,
    RevealAnchorPoint,
}

#[derive(Clone, Copy, Debug)]
pub struct ShortcutDef {
    pub id: ShortcutId,
    key: Key,
    command: bool,
    shift: bool,
    alt: bool,
    pub description: &'static str,
}

pub const SHORTCUT_REGISTRY: &[ShortcutDef] = &[
    ShortcutDef {
        id: ShortcutId::CommandPalette,
        key: Key::K,
        command: true,
        shift: false,
        alt: false,
        description: "Open Command Palette",
    },
    ShortcutDef {
        id: ShortcutId::TogglePlayback,
        key: Key::Space,
        command: false,
        shift: false,
        alt: false,
        description: "Play / Stop RAM Preview",
    },
    ShortcutDef {
        id: ShortcutId::Undo,
        key: Key::Z,
        command: true,
        shift: false,
        alt: false,
        description: "Undo",
    },
    ShortcutDef {
        id: ShortcutId::Redo,
        key: Key::Z,
        command: true,
        shift: true,
        alt: false,
        description: "Redo",
    },
    ShortcutDef {
        id: ShortcutId::Save,
        key: Key::S,
        command: true,
        shift: false,
        alt: false,
        description: "Save Project",
    },
    ShortcutDef {
        id: ShortcutId::NewComposition,
        key: Key::N,
        command: true,
        shift: false,
        alt: false,
        description: "New Composition",
    },
    ShortcutDef {
        id: ShortcutId::OpenRenderQueue,
        key: Key::M,
        command: true,
        shift: false,
        alt: false,
        description: "Open Render Queue / Export",
    },
    ShortcutDef {
        id: ShortcutId::EasyEase,
        key: Key::F9,
        command: false,
        shift: false,
        alt: false,
        description: "Apply Easy Ease to selected keyframes",
    },
    ShortcutDef {
        id: ShortcutId::DuplicateLayer,
        key: Key::D,
        command: true,
        shift: false,
        alt: false,
        description: "Duplicate Selected Layer",
    },
    ShortcutDef {
        id: ShortcutId::SelectionTool,
        key: Key::V,
        command: false,
        shift: false,
        alt: false,
        description: "Selection Tool",
    },
    ShortcutDef {
        id: ShortcutId::HandTool,
        key: Key::H,
        command: false,
        shift: false,
        alt: false,
        description: "Hand / Pan Tool",
    },
    ShortcutDef {
        id: ShortcutId::ZoomTool,
        key: Key::Z,
        command: false,
        shift: false,
        alt: false,
        description: "Zoom Tool",
    },
    ShortcutDef {
        id: ShortcutId::RotationTool,
        key: Key::W,
        command: false,
        shift: false,
        alt: false,
        description: "Rotation Tool",
    },
    ShortcutDef {
        id: ShortcutId::AnchorTool,
        key: Key::Y,
        command: false,
        shift: false,
        alt: false,
        description: "Pan Behind / Anchor Point Tool",
    },
    ShortcutDef {
        id: ShortcutId::RectangleTool,
        key: Key::Q,
        command: false,
        shift: false,
        alt: false,
        description: "Shape Tool",
    },
    ShortcutDef {
        id: ShortcutId::PenTool,
        key: Key::G,
        command: false,
        shift: false,
        alt: false,
        description: "Pen / Vector Path Tool",
    },
    ShortcutDef {
        id: ShortcutId::CameraTool,
        key: Key::C,
        command: false,
        shift: false,
        alt: false,
        description: "3D Camera Tool",
    },
    ShortcutDef {
        id: ShortcutId::TextTool,
        key: Key::T,
        command: true,
        shift: false,
        alt: false,
        description: "Text Tool",
    },
    ShortcutDef {
        id: ShortcutId::PuppetPinTool,
        key: Key::P,
        command: true,
        shift: false,
        alt: false,
        description: "Puppet Pin Tool",
    },
    ShortcutDef {
        id: ShortcutId::PreviousKeyframe,
        key: Key::J,
        command: false,
        shift: false,
        alt: false,
        description: "Jump to Previous Keyframe",
    },
    ShortcutDef {
        id: ShortcutId::NextKeyframe,
        key: Key::K,
        command: false,
        shift: false,
        alt: false,
        description: "Jump to Next Keyframe",
    },
    ShortcutDef {
        id: ShortcutId::PlayForward,
        key: Key::L,
        command: false,
        shift: false,
        alt: false,
        description: "Play Forward (press again to increase speed)",
    },
    ShortcutDef {
        id: ShortcutId::WorkAreaStart,
        key: Key::B,
        command: false,
        shift: false,
        alt: false,
        description: "Set Work Area Start",
    },
    ShortcutDef {
        id: ShortcutId::WorkAreaEnd,
        key: Key::N,
        command: false,
        shift: false,
        alt: false,
        description: "Set Work Area End",
    },
    ShortcutDef {
        id: ShortcutId::LayerInPoint,
        key: Key::I,
        command: false,
        shift: false,
        alt: false,
        description: "Jump to Layer In-Point",
    },
    ShortcutDef {
        id: ShortcutId::LayerOutPoint,
        key: Key::O,
        command: false,
        shift: false,
        alt: false,
        description: "Jump to Layer Out-Point",
    },
    ShortcutDef {
        id: ShortcutId::RevealPosition,
        key: Key::P,
        command: false,
        shift: false,
        alt: false,
        description: "Select Position Property",
    },
    ShortcutDef {
        id: ShortcutId::RevealScale,
        key: Key::S,
        command: false,
        shift: false,
        alt: false,
        description: "Select Scale Property",
    },
    ShortcutDef {
        id: ShortcutId::RevealRotation,
        key: Key::R,
        command: false,
        shift: false,
        alt: false,
        description: "Select Rotation Property",
    },
    ShortcutDef {
        id: ShortcutId::RevealOpacity,
        key: Key::T,
        command: false,
        shift: false,
        alt: false,
        description: "Select Opacity Property",
    },
    ShortcutDef {
        id: ShortcutId::RevealAnchorPoint,
        key: Key::A,
        command: false,
        shift: false,
        alt: false,
        description: "Reveal Anchor Point Property",
    },
];

pub const ADDITIONAL_SHORTCUTS: &[(&str, &str)] = &[
    ("Tab / Shift+Tab", "Cycle through selected layers"),
    ("Numpad 0", "Start RAM preview"),
    ("Cmd+Y", "Create Solid Layer"),
    ("Cmd+Alt+Shift+T", "Create Text Layer"),
    ("Cmd+Alt+Shift+Y", "Create Null Layer"),
    ("Cmd+Alt+Y", "Create Adjustment Layer"),
    ("Cmd+Shift+C", "Pre-compose selected layers"),
    ("Cmd+Shift+K", "Toggle Motion Sketch"),
    (
        "Shift+Z",
        "Fit selected layers in Viewer, or fit composition",
    ),
    ("Cmd+0", "Fit composition in Viewer"),
    ("Cmd+Alt+←", "Return to previous composition"),
    ("Cmd+Alt+F", "Fit selected layer to composition"),
    ("Cmd+Shift+D", "Split selected layer at playhead"),
    ("Cmd+Shift+E", "Open Composition Settings"),
    ("Alt+M", "Add or remove layer marker"),
    ("M", "Add or remove composition marker"),
    ("Shift+;", "Go to next composition marker"),
    ("Cmd+;", "Go to previous composition marker"),
    (
        "U",
        "Reveal keyframed properties; press twice for modified properties",
    ),
    (
        "Arrow Keys",
        "Nudge selected layer (Shift for larger steps)",
    ),
    ("Page Up / Page Down", "Step one frame backward / forward"),
    ("Home / End", "Go to first / last frame"),
    ("= / -", "Zoom timeline in / out"),
    ("Escape", "Deselect current selection"),
    ("F2", "Rename selected layer"),
    ("Delete / Backspace", "Delete selected layers or keyframes"),
];

pub const SHORTCUT_GESTURES: &[(&str, &str)] = &[
    ("Double-click layer", "Open inline numeric editor"),
    ("Shift + drag", "Constrain rotation or movement"),
    ("Ctrl + drag ruler", "Set work area in / out"),
    ("Right-click audio layer", "Add or remove audio fade"),
    (
        "Shift + drag keyframe row",
        "Select keyframes with a marquee",
    ),
    ("Right-click keyframe", "Open keyframe actions"),
    ("Alt + drag layer bar", "Slip edit layer timing"),
    ("Drag corner handle", "Scale selected layer"),
];

pub fn shortcut_registry() -> &'static [ShortcutDef] {
    SHORTCUT_REGISTRY
}

impl ShortcutDef {
    fn matches(self, input: &egui::InputState, id: ShortcutId) -> bool {
        self.id == id
            && input.key_pressed(self.key)
            && if self.command {
                input.modifiers.command || input.modifiers.ctrl
            } else {
                !input.modifiers.command && !input.modifiers.ctrl
            }
            && input.modifiers.shift == self.shift
            && input.modifiers.alt == self.alt
    }
}

pub fn shortcut_pressed(input: &egui::InputState, id: ShortcutId) -> bool {
    SHORTCUT_REGISTRY
        .iter()
        .copied()
        .any(|shortcut| shortcut.matches(input, id))
}

pub fn display_shortcut(id: ShortcutId) -> String {
    let shortcut = SHORTCUT_REGISTRY
        .iter()
        .find(|shortcut| shortcut.id == id)
        .expect("every displayed shortcut must be registered");
    let key = match shortcut.key {
        Key::Space => "Space",
        Key::Num0 => "0",
        Key::ArrowLeft => "←",
        Key::ArrowRight => "→",
        key => {
            return format_shortcut(
                &format!("{key:?}"),
                shortcut.command,
                shortcut.shift,
                shortcut.alt,
            );
        }
    };
    format_shortcut(key, shortcut.command, shortcut.shift, shortcut.alt)
}

pub fn draw_shortcuts_reference(ui: &mut egui::Ui) {
    egui::Grid::new("shortcuts_registry_grid")
        .striped(true)
        .spacing([20.0, 6.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Shortcut").strong());
            ui.label(egui::RichText::new("Action").strong());
            ui.end_row();
            for shortcut in SHORTCUT_REGISTRY {
                ui.label(
                    egui::RichText::new(display_shortcut(shortcut.id))
                        .monospace()
                        .color(crate::ui::theme::colors::ACCENT_INFORMATION),
                );
                ui.label(shortcut.description);
                ui.end_row();
            }
            for (binding, description) in ADDITIONAL_SHORTCUTS {
                ui.label(
                    egui::RichText::new(format_platform_shortcut_hint(binding))
                        .monospace()
                        .color(crate::ui::theme::colors::ACCENT_INFORMATION),
                );
                ui.label(*description);
                ui.end_row();
            }
            ui.end_row();
            ui.label(egui::RichText::new("Gesture").strong());
            ui.label(egui::RichText::new("Action").strong());
            ui.end_row();
            for (gesture, action) in SHORTCUT_GESTURES {
                ui.label(egui::RichText::new(*gesture).monospace());
                ui.label(*action);
                ui.end_row();
            }
        });
}

/// Return platform-native command modifier label ("Cmd" on macOS, "Ctrl" on Windows/Linux)
pub fn cmd_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}

/// Return platform-native option modifier label ("Option" on macOS, "Alt" on Windows/Linux)
pub fn option_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "Option"
    } else {
        "Alt"
    }
}

/// Format a keyboard shortcut string dynamically based on host OS target platform.
pub fn format_shortcut(key: &str, cmd: bool, shift: bool, alt: bool) -> String {
    let mut parts = Vec::new();
    if cmd {
        parts.push(cmd_name());
    }
    if alt {
        parts.push(option_name());
    }
    if shift {
        parts.push("Shift");
    }
    parts.push(key);
    parts.join("+")
}

pub fn format_platform_shortcut_hint(hint: &str) -> String {
    format_platform_shortcut_hint_for(hint, cmd_name(), option_name())
}

pub fn format_platform_shortcut_hint_for(hint: &str, command: &str, option: &str) -> String {
    hint.replace("Cmd", command)
        .replace("Option", option)
        .replace("Opt", option)
        .replace("Alt", option)
}
