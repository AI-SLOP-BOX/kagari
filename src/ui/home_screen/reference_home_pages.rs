use super::*;

#[path = "reference_home_pages/components.rs"]
mod components;
pub(in crate::ui::home_screen) use components::*;

#[path = "reference_home_pages/landing.rs"]
mod landing;
pub(in crate::ui::home_screen) use landing::draw_reference_home;
#[path = "reference_home_pages/continue_page.rs"]
mod continue_page;
pub(in crate::ui::home_screen) use continue_page::draw_reference_continue_body;
#[path = "reference_home_pages/projects.rs"]
mod projects;
pub(in crate::ui::home_screen) use projects::draw_reference_project_row;
#[path = "reference_home_pages/templates.rs"]
mod templates;
pub(in crate::ui::home_screen) use templates::draw_reference_templates;
