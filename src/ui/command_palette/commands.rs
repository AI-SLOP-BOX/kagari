use crate::KagariApp;
use super::advanced;
use super::general;

pub struct PaletteCommand {
    pub name: &'static str,
    pub category: &'static str,
    pub shortcut_hint: &'static str,
    pub action: Box<dyn Fn(&mut KagariApp) + Send + Sync>,
}

pub fn get_all_commands() -> Vec<PaletteCommand> {
    let mut list = general::get_general_commands();
    list.extend(advanced::get_advanced_commands());
    list
}
