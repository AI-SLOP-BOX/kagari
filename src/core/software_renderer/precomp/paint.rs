use crate::core::timeline::Layer;

pub struct PaintTransform {
    pub scale: [f32; 2],
    pub cos_r: f32,
    pub sin_r: f32,
    pub cx: f32,
    pub cy: f32,
    pub min_x: u32,
    pub min_y: u32,
}

pub fn apply_paint_strokes(
    layer: &Layer,
    source_frame: u32,
    pt: &PaintTransform,
    bw: u32,
    bh: u32,
    layer_buf: &mut [u8],
) {
    let scale_x = pt.scale[0] / 100.0;
    let scale_y = pt.scale[1] / 100.0;
    let brush_scale = pt.scale[0].abs().max(pt.scale[1].abs()) / 100.0;
    let to_buf_local = |lp: [f32; 2]| -> [f32; 2] {
        // Layer-local points are scaled on their own axes, then rotated into
        // composition space. Scaling the already-rotated coordinates distorts
        // the transform whenever a non-uniformly scaled layer is rotated.
        let x = lp[0] * scale_x;
        let y = lp[1] * scale_y;
        let wx = pt.cx + x * pt.cos_r - y * pt.sin_r;
        let wy = pt.cy + x * pt.sin_r + y * pt.cos_r;
        [wx - pt.min_x as f32, wy - pt.min_y as f32]
    };
    let mut dirty_min_x = bw as f32;
    let mut dirty_min_y = bh as f32;
    let mut dirty_max_x = 0.0f32;
    let mut dirty_max_y = 0.0f32;
    for stroke in &layer.paint_strokes {
        let end_f = if stroke.end_frame == 0 {
            layer.out_frame
        } else {
            stroke.end_frame
        };
        if source_frame < stroke.start_frame || source_frame > end_f {
            continue;
        }
        let half = stroke.size * brush_scale * 0.5;
        for &p in &stroke.points {
            let bp = to_buf_local(p);
            dirty_min_x = dirty_min_x.min(bp[0] - half);
            dirty_min_y = dirty_min_y.min(bp[1] - half);
            dirty_max_x = dirty_max_x.max(bp[0] + half);
            dirty_max_y = dirty_max_y.max(bp[1] + half);
        }
    }
    let dr_x0 = dirty_min_x.floor().max(0.0) as u32;
    let dr_y0 = dirty_min_y.floor().max(0.0) as u32;
    let dr_x1 = dirty_max_x.ceil().min(bw as f32) as u32;
    let dr_y1 = dirty_max_y.ceil().min(bh as f32) as u32;
    for stroke in &layer.paint_strokes {
        let end_f = if stroke.end_frame == 0 {
            layer.out_frame
        } else {
            stroke.end_frame
        };
        if source_frame < stroke.start_frame || source_frame > end_f {
            continue;
        }
        let brush_radius = stroke.size * brush_scale * 0.5;
        let (stroke_min, stroke_max) = stroke.points.iter().map(|&p| to_buf_local(p)).fold(
            ([f32::MAX, f32::MAX], [f32::MIN, f32::MIN]),
            |(min, max), p| {
                (
                    [min[0].min(p[0]), min[1].min(p[1])],
                    [max[0].max(p[0]), max[1].max(p[1])],
                )
            },
        );
        let s_min_x = stroke_min[0] - brush_radius;
        let s_min_y = stroke_min[1] - brush_radius;
        let s_max_x = stroke_max[0] + brush_radius;
        let s_max_y = stroke_max[1] + brush_radius;
        if s_max_x < dr_x0 as f32
            || s_min_x > dr_x1 as f32
            || s_max_y < dr_y0 as f32
            || s_min_y > dr_y1 as f32
        {
            continue;
        }
        let buf_pts: Vec<[f32; 2]> = stroke.points.iter().map(|&p| to_buf_local(p)).collect();
        let col = stroke.color;
        if stroke.mode == crate::core::timeline::PaintStrokeMode::CloneStamp {
            let Some(source_buf) = super::super::try_clone_buffer(layer_buf) else {
                log::error!(
                    "[Renderer] Could not allocate clone-stamp source for layer {}",
                    layer.id
                );
                return;
            };
            let source_pts: Vec<[f32; 2]> = stroke
                .points
                .iter()
                .map(|&p| {
                    to_buf_local([p[0] + stroke.clone_offset[0], p[1] + stroke.clone_offset[1]])
                })
                .collect();
            crate::core::paint::draw_clone_stroke_with_brush(
                layer_buf,
                &source_buf,
                bw,
                bh,
                &buf_pts,
                &source_pts,
                (stroke.size * brush_scale).max(1.0),
                stroke.hardness,
                stroke.opacity,
                stroke.flow,
            );
        } else {
            crate::core::paint::draw_stroke_with_brush(
                layer_buf,
                bw,
                bh,
                &buf_pts,
                col,
                (stroke.size * brush_scale).max(1.0),
                stroke.hardness,
                stroke.opacity,
                stroke.flow,
            );
        }
    }
}
