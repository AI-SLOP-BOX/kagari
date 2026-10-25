use eframe::egui;

#[derive(Clone, Copy)]
pub(super) enum KeyframeMenuAction {
    Linear,
    EasyEase,
    ToggleHold,
    Reverse,
    Delete,
    EaseIn,
    EaseOut,
    Overshoot,
    Bounce,
    Elastic,
}

pub(super) fn attach(
    response: &egui::Response,
    property_key: &str,
    frame: u32,
    commands: &mut Vec<(String, u32, KeyframeMenuAction)>,
) {
    response.context_menu(|ui| {
        ui.set_min_width(210.0);
        let items = [
            ("Linear Interpolation", KeyframeMenuAction::Linear),
            ("Easy Ease (F9)", KeyframeMenuAction::EasyEase),
            ("Ease In (Shift+F9)", KeyframeMenuAction::EaseIn),
            ("Ease Out (Ctrl+Shift+F9)", KeyframeMenuAction::EaseOut),
            ("Overshoot / Spring", KeyframeMenuAction::Overshoot),
            ("Bounce", KeyframeMenuAction::Bounce),
            ("Elastic", KeyframeMenuAction::Elastic),
            ("⬛ Toggle Hold Keyframe", KeyframeMenuAction::ToggleHold),
        ];
        for (label, action) in items {
            if crate::ui::kui::menu_item(ui, label).clicked() {
                commands.push((property_key.to_owned(), frame, action));
                ui.close_menu();
            }
        }
        ui.separator();
        for (label, action) in [
            ("⇄ Time-Reverse Keyframes", KeyframeMenuAction::Reverse),
            ("Delete Keyframe (Del)", KeyframeMenuAction::Delete),
        ] {
            if crate::ui::kui::menu_item(ui, label).clicked() {
                commands.push((property_key.to_owned(), frame, action));
                ui.close_menu();
            }
        }
    });
}
