use super::*;

/// Places the activity table columns against the same three equal tracks as
/// the project cards, so labels do not drift toward arbitrary percentages.
fn activity_column_anchors(left: f32, right: f32, density: f32) -> (f32, f32) {
    let width = (right - left).max(1.0);
    let gap = 12.0 * density;
    let track = ((width - gap * 2.0) / 3.0).max(1.0);
    let second_track_center = left + track + gap + track * 0.5;
    let third_track_center = left + 2.0 * (track + gap) + track * 0.5;
    (
        second_track_center + 4.0 * density,
        third_track_center + 14.0 * density,
    )
}

#[path = "landing_pages/nav.rs"]
mod nav;
pub(in crate::ui::home_screen) use nav::landing_nav_row;
#[path = "landing_pages/sidebar.rs"]
mod sidebar;
pub(in crate::ui::home_screen) use sidebar::draw_landing_sidebar;
#[path = "landing_pages/button.rs"]
mod button;
pub(in crate::ui::home_screen) use button::landing_button_rect;
#[path = "landing_pages/recent.rs"]
mod recent;
pub(in crate::ui::home_screen) use recent::{
    draw_landing_recent_projects, draw_landing_recent_work,
};
#[path = "landing_pages/home.rs"]
mod home;
pub(in crate::ui::home_screen) use home::draw_landing_home;
