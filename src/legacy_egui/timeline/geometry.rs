use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineViewport {
    pub start_frame: u32,
    pub span: u32,
    pub rect: egui::Rect,
}

impl TimelineViewport {
    pub fn new(start_frame: u32, span: u32, rect: egui::Rect) -> Self {
        Self {
            start_frame,
            span: span.max(1),
            rect,
        }
    }

    pub fn end_frame(self) -> u32 {
        self.start_frame.saturating_add(self.span)
    }

    pub fn contains(self, frame: u32) -> bool {
        (self.start_frame..=self.end_frame()).contains(&frame)
    }

    pub fn frame_to_x(self, frame: u32) -> f32 {
        frame_to_x(frame, self.start_frame, self.span, self.rect)
    }

    pub fn x_to_frame(self, x: f32) -> u32 {
        x_to_frame(x, self.start_frame, self.span, self.rect)
    }
}

pub fn visible_range(total_frames: u32, zoom: f32, view_start: u32) -> (u32, u32) {
    let zoom_span = (total_frames as f32 / zoom.max(0.01)).max(10.0) as u32;
    (view_start, zoom_span)
}

pub fn frame_to_x(frame: u32, start_frame: u32, zoom_span: u32, rect: egui::Rect) -> f32 {
    let norm = (frame.saturating_sub(start_frame)) as f32 / zoom_span.max(1) as f32;
    rect.left() + norm * rect.width()
}

pub fn x_to_frame(px: f32, start_frame: u32, zoom_span: u32, rect: egui::Rect) -> u32 {
    if !px.is_finite() || !rect.width().is_finite() || rect.width() <= 0.0 {
        return start_frame;
    }
    let norm = ((px - rect.left()) / rect.width()).clamp(0.0, 1.0);
    start_frame.saturating_add((norm * zoom_span as f32).round() as u32)
}

pub fn clamp_view_start(
    current_frame: u32,
    zoom_span: u32,
    total_frames: u32,
    view_start: u32,
) -> u32 {
    if current_frame < view_start || current_frame >= view_start.saturating_add(zoom_span) {
        current_frame
            .saturating_sub(zoom_span / 2)
            .min(total_frames.saturating_sub(zoom_span))
    } else {
        view_start
    }
}
