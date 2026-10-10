use eframe::egui;

pub const COLUMN_HEADER_HEIGHT: f32 = 28.0;
pub const LAYER_ROW_HEIGHT: f32 = 30.0;
pub const PROPERTY_ROW_HEIGHT: f32 = 18.0;
pub const LAYER_NAME_MIN_WIDTH: f32 = 40.0;
pub const LAYER_NAME_MAX_WIDTH: f32 = 220.0;
pub const LAYER_SWITCHES_WIDTH: f32 = 82.0;
pub const LAYER_HIDDEN_SWITCHES_RESERVE: f32 = 112.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineColumns {
    pub total_width: f32,
    pub left_width: f32,
    pub time_width: f32,
    pub gap_width: f32,
    pub min_left_width: f32,
    pub max_left_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineRowRects {
    pub left: egui::Rect,
    pub time: egui::Rect,
}

pub fn allocate_columns<L, T>(
    ui: &mut egui::Ui,
    columns: TimelineColumns,
    height: f32,
    left_contents: impl FnOnce(&mut egui::Ui) -> L,
    time_contents: impl FnOnce(&mut egui::Ui) -> T,
) -> (
    egui::InnerResponse<L>,
    egui::InnerResponse<T>,
    TimelineRowRects,
) {
    let left = ui.allocate_ui(egui::vec2(columns.left_width, height), left_contents);
    let time = ui.allocate_ui(egui::vec2(columns.time_width, height), time_contents);
    let rects = TimelineRowRects {
        left: left.response.rect,
        time: time.response.rect,
    };
    (left, time, rects)
}

impl TimelineColumns {
    pub fn new(total_width: f32, requested_left_width: f32, gap_width: f32) -> Self {
        let total_width = if total_width.is_finite() {
            total_width.max(0.0)
        } else {
            0.0
        };
        if total_width == 0.0 {
            return Self {
                total_width,
                left_width: 0.0,
                time_width: 0.0,
                gap_width: 0.0,
                min_left_width: 0.0,
                max_left_width: 0.0,
            };
        }

        let gap_width = if gap_width.is_finite() {
            gap_width.max(0.0).min(total_width)
        } else {
            0.0
        };
        let content_width = (total_width - gap_width).max(0.0);
        let min_time_width = if content_width >= 400.0 {
            180.0
        } else {
            content_width * 0.35
        };
        let max_left_width = (content_width - min_time_width).clamp(0.0, 280.0);
        let min_left_width = 180.0_f32.min(max_left_width);
        let requested_left_width = if requested_left_width.is_finite() {
            requested_left_width
        } else {
            min_left_width
        };
        let left_width = requested_left_width.clamp(min_left_width, max_left_width);
        Self {
            total_width,
            left_width,
            time_width: (content_width - left_width).max(0.0),
            gap_width,
            min_left_width,
            max_left_width,
        }
    }

    pub fn layer_name_width(remaining_width: f32, show_switches: bool) -> f32 {
        let remaining_width = if remaining_width.is_finite() {
            remaining_width.max(0.0)
        } else {
            0.0
        };
        let reserved_width = if show_switches {
            LAYER_SWITCHES_WIDTH
        } else {
            LAYER_HIDDEN_SWITCHES_RESERVE
        };
        let available_name_width = (remaining_width - reserved_width).max(0.0);
        if available_name_width < LAYER_NAME_MIN_WIDTH {
            0.0
        } else {
            available_name_width.min(LAYER_NAME_MAX_WIDTH)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TimelineColumns, LAYER_NAME_MAX_WIDTH};

    #[test]
    fn timeline_columns_fit_narrow_and_wide_panels() {
        for width in [0.0, 120.0, 260.0, 399.0, 400.0, 1280.0] {
            let columns = TimelineColumns::new(width, 900.0, 4.0);
            assert!(columns.left_width >= 0.0);
            assert!(columns.time_width >= 0.0);
            assert!(
                (columns.left_width + columns.time_width + columns.gap_width - width).abs() < 0.01
            );
            assert!(columns.time_width <= width);
        }
        let narrow = TimelineColumns::new(300.0, 260.0, 4.0);
        assert!(narrow.time_width >= 103.5);
        let wide = TimelineColumns::new(1280.0, 320.0, 4.0);
        assert_eq!(wide.left_width, 280.0);
        assert_eq!(wide.time_width, 996.0);
    }

    #[test]
    fn invalid_widths_cannot_poison_column_geometry() {
        let columns = TimelineColumns::new(f32::NAN, f32::INFINITY, f32::NAN);
        assert_eq!(columns.total_width, 0.0);
        assert_eq!(columns.left_width, 0.0);
        assert_eq!(columns.time_width, 0.0);
    }

    #[test]
    fn layer_name_column_uses_shared_switch_reservations_and_clamps() {
        assert_eq!(TimelineColumns::layer_name_width(300.0, true), 218.0);
        assert_eq!(TimelineColumns::layer_name_width(300.0, false), 188.0);
        assert_eq!(TimelineColumns::layer_name_width(60.0, true), 0.0);
        assert_eq!(
            TimelineColumns::layer_name_width(600.0, true),
            LAYER_NAME_MAX_WIDTH
        );
        assert_eq!(TimelineColumns::layer_name_width(f32::NAN, true), 0.0);
    }
}
