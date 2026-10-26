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
    let to_buf_local = |lp: [f32; 2]| -> [f32; 2] {
        let wx = pt.cx + (lp[0] * pt.cos_r - lp[1] * pt.sin_r) * pt.scale[0] / 100.0;
        let wy = pt.cy + (lp[0] * pt.sin_r + lp[1] * pt.cos_r) * pt.scale[1] / 100.0;
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
        let half = stroke.size * 0.5;
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
        let stroke_min = stroke.points.iter().fold([f32::MAX, f32::MAX], |acc, p| {
            [acc[0].min(p[0]), acc[1].min(p[1])]
        });
        let stroke_max = stroke.points.iter().fold([f32::MIN, f32::MIN], |acc, p| {
            [acc[0].max(p[0]), acc[1].max(p[1])]
        });
        let sb_min = to_buf_local(stroke_min);
        let sb_max = to_buf_local(stroke_max);
        let s_min_x = sb_min[0] - stroke.size * 0.5;
        let s_min_y = sb_min[1] - stroke.size * 0.5;
        let s_max_x = sb_max[0] + stroke.size * 0.5;
        let s_max_y = sb_max[1] + stroke.size * 0.5;
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
            let source_buf = layer_buf.to_vec();
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
                stroke.size.max(1.0),
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
                stroke.size.max(1.0),
                stroke.hardness,
                stroke.opacity,
                stroke.flow,
            );
        }
    }
}
