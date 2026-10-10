use crate::KagariApp;
use eframe::egui;
use std::sync::Arc;

mod style {
    pub const LABEL_FONT_SIZE: f32 = 10.0;
    pub const SAMPLE_RADIUS: f32 = 0.8;
    pub const CARD_MIN_HEIGHT: f32 = 56.0;
    pub const CARD_COMPACT_BOTTOM_INSET: f32 = 8.0;
    pub const CARD_BOTTOM_INSET: f32 = 12.0;
    pub const PLOT_INSET: f32 = 8.0;
    pub const PLOT_CORNER_RADIUS: f32 = 2.0;
    pub const PLOT_LABEL_INSET: f32 = 4.0;
    pub const GRID_MINOR_STROKE: f32 = 0.5;
    pub const GRID_MAJOR_STROKE: f32 = 0.6;
}

#[derive(Clone)]
pub struct ScopePixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<Vec<u8>>,
}

impl ScopePixels {
    pub fn is_empty(&self) -> bool {
        self.rgba.is_empty()
    }
}

#[derive(Clone)]
struct ScopeCache {
    composition_id: String,
    frame: u32,
    generation: u64,
    pixels: ScopePixels,
}

#[derive(Clone, PartialEq, Eq)]
struct ScopeKey {
    composition_id: String,
    frame: u32,
    generation: u64,
}

#[derive(Clone)]
struct ScopeJob {
    key: ScopeKey,
    receiver: Arc<std::sync::Mutex<std::sync::mpsc::Receiver<ScopePixels>>>,
}

pub fn frame_pixels(app: &KagariApp, ctx: &egui::Context) -> Option<ScopePixels> {
    let comp = app.history.current().active_composition();
    let key = ScopeKey {
        composition_id: comp.id.clone(),
        frame: app.playback.current_frame,
        generation: app.history.generation(),
    };
    let cache_id = egui::Id::new("color_workspace_scope_frame");
    if let Some(cache) = ctx.data(|data| data.get_temp::<ScopeCache>(cache_id)) {
        if cache.composition_id == key.composition_id
            && cache.frame == key.frame
            && cache.generation == key.generation
        {
            return Some(cache.pixels);
        }
    }

    let job_id = egui::Id::new("color_workspace_scope_job");
    if let Some(job) = ctx.data(|data| data.get_temp::<ScopeJob>(job_id)) {
        if let Ok(receiver) = job.receiver.lock() {
            match receiver.try_recv() {
                Ok(pixels) => {
                    ctx.data_mut(|data| {
                        data.remove::<ScopeJob>(job_id);
                        data.insert_temp(
                            cache_id,
                            ScopeCache {
                                composition_id: job.key.composition_id.clone(),
                                frame: job.key.frame,
                                generation: job.key.generation,
                                pixels: pixels.clone(),
                            },
                        );
                    });
                    if job.key == key {
                        return Some(pixels);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(50));
                    return ctx
                        .data(|data| data.get_temp::<ScopeCache>(cache_id))
                        .map(|cache| cache.pixels);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    ctx.data_mut(|data| data.remove::<ScopeJob>(job_id));
                }
            }
        }
    }

    let composition = comp.clone();
    let frame = key.frame;
    let job_key = key.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let repaint = ctx.clone();
    if std::thread::Builder::new()
        .name("kagari-color-scopes".into())
        .spawn(move || {
            let width = 192;
            let height =
                ((width * composition.height.max(1)) / composition.width.max(1)).clamp(32, 144);
            let rgba = crate::core::software_renderer::render_frame_to_pixels(
                &composition,
                frame,
                width,
                height,
                0.0,
                0,
            );
            let _ = sender.send(ScopePixels {
                width,
                height,
                rgba: Arc::new(rgba),
            });
            repaint.request_repaint();
        })
        .is_ok()
    {
        ctx.data_mut(|data| {
            data.insert_temp(
                job_id,
                ScopeJob {
                    key: job_key,
                    receiver: Arc::new(std::sync::Mutex::new(receiver)),
                },
            )
        });
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(50));
    ctx.data(|data| data.get_temp::<ScopeCache>(cache_id))
        .map(|cache| cache.pixels)
}

pub fn draw_parade(ui: &mut egui::Ui, pixels: &ScopePixels, compact: bool) {
    draw_scope_card(ui, "RGB Parade", compact, |painter, rect| {
        draw_grid(&painter, rect);
        let rgba = pixels.rgba.as_slice();
        let total = (pixels.width as usize).saturating_mul(pixels.height as usize);
        let stride = (total / 4000).max(1);
        let channel_colors = [
            egui::Color32::from_rgba_unmultiplied(238, 91, 91, 95),
            egui::Color32::from_rgba_unmultiplied(89, 211, 133, 95),
            egui::Color32::from_rgba_unmultiplied(93, 145, 244, 95),
        ];
        for pixel_index in (0..total).step_by(stride) {
            let offset = pixel_index * 4;
            let x = (pixel_index % pixels.width as usize) as f32 / pixels.width as f32;
            if rgba.get(offset + 3).copied().unwrap_or(0) < 8 {
                continue;
            }
            for channel in 0..3 {
                let intensity = rgba[offset + channel] as f32 / 255.0;
                let px = rect.left() + (channel as f32 + x) * rect.width() / 3.0;
                let py = rect.bottom() - intensity * rect.height();
                painter.circle_filled(
                    egui::pos2(px, py),
                    style::SAMPLE_RADIUS,
                    channel_colors[channel],
                );
            }
        }
        for (index, label) in ["R", "G", "B"].into_iter().enumerate() {
            let x = rect.left() + rect.width() * (index as f32 + 0.5) / 3.0;
            painter.text(
                egui::pos2(x, rect.bottom() + 2.0),
                egui::Align2::CENTER_TOP,
                label,
                egui::FontId::proportional(style::LABEL_FONT_SIZE),
                channel_colors[index],
            );
        }
    });
}

pub fn draw_vectorscope(ui: &mut egui::Ui, pixels: &ScopePixels, compact: bool) {
    draw_scope_card(ui, "Vectorscope", compact, |painter, rect| {
        crate::ui::vectorscope::draw_scope(painter, rect, pixels.rgba.as_slice());
    });
}

pub fn draw_histogram(ui: &mut egui::Ui, pixels: &ScopePixels, compact: bool) {
    draw_scope_card(ui, "RGB Histogram", compact, |painter, rect| {
        draw_grid(&painter, rect);
        let mut bins = [[0u32; 64]; 3];
        for px in pixels.rgba.as_slice().as_chunks::<4>().0 {
            if px[3] < 8 {
                continue;
            }
            for channel in 0..3 {
                bins[channel][px[channel] as usize * 64 / 256] += 1;
            }
        }
        let max = bins.iter().flatten().copied().max().unwrap_or(1).max(1) as f32;
        let colors = [
            egui::Color32::from_rgba_unmultiplied(238, 91, 91, 110),
            egui::Color32::from_rgba_unmultiplied(89, 211, 133, 110),
            egui::Color32::from_rgba_unmultiplied(93, 145, 244, 110),
        ];
        let bin_width = rect.width() / 64.0;
        for channel in 0..3 {
            for (bin, count) in bins[channel].iter().enumerate() {
                let height = (*count as f32 / max).sqrt() * rect.height();
                let bar = egui::Rect::from_min_max(
                    egui::pos2(rect.left() + bin as f32 * bin_width, rect.bottom() - height),
                    egui::pos2(rect.left() + (bin + 1) as f32 * bin_width, rect.bottom()),
                );
                painter.rect_filled(bar, 0.0, colors[channel]);
            }
        }
    });
}

pub fn draw_waveform(ui: &mut egui::Ui, pixels: &ScopePixels, compact: bool) {
    draw_scope_card(ui, "Luma Waveform", compact, |painter, rect| {
        draw_grid(&painter, rect);
        let rgba = pixels.rgba.as_slice();
        let total = (pixels.width as usize).saturating_mul(pixels.height as usize);
        let stride = (total / 6000).max(1);
        for pixel_index in (0..total).step_by(stride) {
            let offset = pixel_index * 4;
            if rgba.get(offset + 3).copied().unwrap_or(0) < 8 {
                continue;
            }
            let x = (pixel_index % pixels.width as usize) as f32 / pixels.width as f32;
            let r = rgba[offset] as f32;
            let g = rgba[offset + 1] as f32;
            let b = rgba[offset + 2] as f32;
            let luma = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0;
            let pos = egui::pos2(
                rect.left() + x * rect.width(),
                rect.bottom() - luma * rect.height(),
            );
            painter.circle_filled(
                pos,
                style::SAMPLE_RADIUS,
                egui::Color32::from_rgba_unmultiplied(86, 194, 205, 85),
            );
        }
    });
}

fn draw_scope_card(
    ui: &mut egui::Ui,
    title: &str,
    compact: bool,
    draw: impl FnOnce(egui::Painter, egui::Rect),
) {
    crate::ui::kui::group(ui, title, |ui| {
        let available = ui.available_size();
        let bottom_inset = if compact {
            style::CARD_COMPACT_BOTTOM_INSET
        } else {
            style::CARD_BOTTOM_INSET
        };
        let height = (available.y - bottom_inset).max(style::CARD_MIN_HEIGHT);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(available.x.max(style::CARD_MIN_HEIGHT), height),
            egui::Sense::hover(),
        );
        let plot = rect.shrink(style::PLOT_INSET);
        let painter = ui.painter().with_clip_rect(plot);
        painter.rect_filled(
            plot,
            style::PLOT_CORNER_RADIUS,
            crate::ui::theme::colors::BG_DEEPEST,
        );
        draw(
            painter,
            plot.shrink2(egui::vec2(0.0, style::PLOT_LABEL_INSET)),
        );
    });
}

fn draw_grid(painter: &egui::Painter, rect: egui::Rect) {
    for step in 1..4 {
        let y = rect.top() + rect.height() * step as f32 / 4.0;
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(
                style::GRID_MAJOR_STROKE,
                crate::ui::theme::colors::GRID_LINE,
            ),
        );
    }
    for step in 1..8 {
        let x = rect.left() + rect.width() * step as f32 / 8.0;
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(
                style::GRID_MINOR_STROKE,
                crate::ui::theme::colors::GRID_LINE,
            ),
        );
    }
}
