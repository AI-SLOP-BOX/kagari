use super::*;

const LANDING_CARD_GAP: f32 = 24.0;

#[path = "landing_pages/nav.rs"]
mod nav;
pub(in crate::ui::home_screen) use nav::landing_nav_row;
#[path = "landing_pages/sidebar.rs"]
mod sidebar;
pub(in crate::ui::home_screen) use sidebar::draw_landing_sidebar;
#[path = "landing_pages/button.rs"]
mod button;
pub(in crate::ui::home_screen) use button::landing_button_rect;
#[path = "landing_pages/hero.rs"]
mod hero;
pub(in crate::ui::home_screen) use hero::draw_landing_hero;
#[path = "landing_pages/home.rs"]
mod home;
pub(in crate::ui::home_screen) use home::draw_landing_home;
