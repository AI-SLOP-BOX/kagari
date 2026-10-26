//! Theme definition and styling constants for Kagari VFX.
//!
//! Provides the professional dark AE-inspired palette, typography, layout metrics,
//! and widget helper functions.

pub mod colors;
mod fonts;
pub mod helpers;
pub mod layout;
mod visuals;

pub use helpers::{
    ae_drag_value, draw_custom_tab, draw_label_chip, draw_prop_value, draw_section_header,
    draw_section_header_svg, draw_separator, panel_frame, side_panel_frame,
};
pub use visuals::configure_ae_theme;
