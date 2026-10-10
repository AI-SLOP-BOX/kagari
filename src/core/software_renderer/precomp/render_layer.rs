use super::content::{rasterize_layer_content, LayerTransformBounds};
use super::paint::{apply_paint_strokes, PaintTransform};
use super::raster::RasterizationRegion;
use crate::core::timeline::{Composition, Layer, LayerType};

pub struct PrecompRenderContext<'a> {
    pub comp: &'a Composition,
    pub width: u32,
    pub height: u32,
}

pub struct LayerRenderParams {
    pub layer_idx: usize,
    pub frame: u32,
    pub source_frame: u32,
    pub pos: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
    pub opacity: f32,
}

pub fn render_precomp_layers_inner(
    _comp: &Composition,
    precomp_comp: &Composition,
    frame: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let size = super::super::rgba_buffer_size(width, height).unwrap_or(0);
    if size == 0 {
        return Vec::new();
    }

    let mut buffer = vec![0u8; size];
    for p in (0..size).step_by(4) {
        buffer[p] = 0;
        buffer[p + 1] = 0;
        buffer[p + 2] = 0;
        buffer[p + 3] = 0;
    }

    let has_solo = precomp_comp
        .layers
        .iter()
        .any(|l| l.is_active(frame) && l.solo);

    let ctx = PrecompRenderContext {
        comp: precomp_comp,
        width,
        height,
    };

    for (layer_idx, layer) in precomp_comp.layers.iter().enumerate() {
        if !layer.is_active(frame) || !layer.visible || layer.is_guide_layer {
            continue;
        }
        if has_solo && !layer.solo {
            continue;
        }

        let source_frame = layer.effective_render_frame(frame, precomp_comp.fps);
        let (pos, scale, rotation, opacity) = precomp_comp.resolve_world_transform(layer, frame);
        let l_opacity = (opacity / 100.0).clamp(0.0, 1.0);
        if l_opacity < 0.001 {
            continue;
        }

        if matches!(layer.layer_type, LayerType::AdjustmentLayer) {
            apply_adjustment_layer(&ctx, layer, layer_idx, source_frame, l_opacity, &mut buffer);
            continue;
        }

        let params = LayerRenderParams {
            layer_idx,
            frame,
            source_frame,
            pos,
            scale,
            rotation,
            opacity: l_opacity,
        };

        render_single_precomp_layer(&ctx, layer, &params, &mut buffer);
    }

    buffer
}

fn apply_adjustment_layer(
    ctx: &PrecompRenderContext,
    layer: &Layer,
    layer_idx: usize,
    source_frame: u32,
    l_opacity: f32,
    buffer: &mut [u8],
) {
    if !layer.effects.is_empty() && l_opacity > 0.003 {
        let mut adjusted = buffer.to_vec();
        crate::core::cpu_effects::apply_layer_effects(
            Some(ctx.comp),
            Some(layer_idx),
            &mut adjusted,
            ctx.width,
            ctx.height,
            &layer.effects,
            source_frame,
            ctx.comp.fps,
        );
        for i in (0..buffer.len()).step_by(4) {
            for c in 0..3 {
                buffer[i + c] = (buffer[i + c] as f32 * (1.0 - l_opacity)
                    + adjusted[i + c] as f32 * l_opacity)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
}

fn render_single_precomp_layer(
    ctx: &PrecompRenderContext,
    layer: &Layer,
    p: &LayerRenderParams,
    buffer: &mut [u8],
) {
    let (base_w, base_h) = match &layer.layer_type {
        LayerType::Solid { .. } | LayerType::PreComp { .. } => {
            (ctx.comp.width as f32, ctx.comp.height as f32)
        }
        LayerType::Text { .. } => (ctx.comp.width as f32, ctx.comp.height as f32),
        LayerType::Shape { .. } => (ctx.comp.width as f32, ctx.comp.width as f32),
        LayerType::Model3D { .. } => (ctx.comp.width as f32, ctx.comp.height as f32),
        LayerType::Image { .. } | LayerType::Video { .. } => {
            (ctx.comp.width as f32, ctx.comp.height as f32)
        }
        _ => return,
    };

    let w = (p.scale[0].abs() / 100.0) * base_w;
    let h = (p.scale[1].abs() / 100.0) * base_h;

    let base_color = match &layer.layer_type {
        LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
        LayerType::Shape { color, .. } => *color,
        LayerType::Image { .. } => [0.2, 0.6, 0.9, 1.0],
        LayerType::Video { .. } => [1.0, 1.0, 1.0, 1.0],
        LayerType::Model3D { .. } => [0.72, 0.78, 0.9, 1.0],
        LayerType::PreComp { .. } => [1.0, 1.0, 1.0, 1.0],
        _ => return,
    };

    let rad = p.rotation.to_radians();
    let (cos_r, sin_r) = rad.sin_cos();
    let cx = p.pos[0];
    let cy = p.pos[1];
    let bounds_x = w * 0.5;
    let bounds_y = h * 0.5;

    let ext_x = (bounds_x.abs() + 2.0) * 1.5;
    let ext_y = (bounds_y.abs() + 2.0) * 1.5;
    let min_x = ((cx - ext_x).max(0.0) as u32).min(ctx.width);
    let max_x = ((cx + ext_x).max(0.0) as u32).min(ctx.width);
    let min_y = ((cy - ext_y).max(0.0) as u32).min(ctx.height);
    let max_y = ((cy + ext_y).max(0.0) as u32).min(ctx.height);
    let bw = max_x.saturating_sub(min_x);
    let bh = max_y.saturating_sub(min_y);
    if bw == 0 || bh == 0 {
        return;
    }

    let buf_size = (bw * bh * 4) as usize;
    let mut layer_buf = vec![0u8; buf_size];
    let raster_region = RasterizationRegion {
        min_x,
        min_y,
        max_x,
        max_y,
        buffer_width: bw,
        center_x: cx,
        center_y: cy,
        cos_rotation: cos_r,
        sin_rotation: sin_r,
        bounds_x,
        bounds_y,
    };

    let tb = LayerTransformBounds {
        bw,
        bh,
        min_x,
        min_y,
        max_x,
        max_y,
        cx,
        cy,
        bounds_x,
        bounds_y,
        cos_r,
        sin_r,
    };

    let rp = super::content::RasterizeParams {
        comp: ctx.comp,
        layer,
        frame: p.frame,
        source_frame: p.source_frame,
        base_color,
        raster_region,
        comp_size: (ctx.width, ctx.height),
    };

    rasterize_layer_content(&rp, &tb, &mut layer_buf);

    if !layer.paint_strokes.is_empty() {
        let pt = PaintTransform {
            scale: p.scale,
            cos_r,
            sin_r,
            cx,
            cy,
            min_x,
            min_y,
        };
        apply_paint_strokes(layer, p.source_frame, &pt, bw, bh, &mut layer_buf);
    }

    if !layer.puppet_pins.is_empty() {
        let to_buf = |pt: [f32; 2]| -> [f32; 2] { [pt[0] - min_x as f32, pt[1] - min_y as f32] };
        let pins: Vec<([f32; 2], [f32; 2])> = layer
            .puppet_pins
            .iter()
            .map(|pin| {
                let s = to_buf(pin.comp_source);
                let d = to_buf(pin.position.evaluate(p.source_frame));
                (s, d)
            })
            .collect();
        crate::core::puppet_warp::warp_layer_buf_mesh(&mut layer_buf, bw, bh, &pins);
    }

    crate::core::cpu_effects::apply_layer_effects(
        Some(ctx.comp),
        Some(p.layer_idx),
        &mut layer_buf,
        bw,
        bh,
        &layer.effects,
        p.source_frame,
        ctx.comp.fps,
    );

    composite_layer_buffer(&layer_buf, &tb, ctx.width, p.opacity, buffer);
}

fn composite_layer_buffer(
    layer_buf: &[u8],
    tb: &LayerTransformBounds,
    width: u32,
    l_opacity: f32,
    buffer: &mut [u8],
) {
    for ly in 0..tb.bh {
        for lx in 0..tb.bw {
            let lidx = ((ly * tb.bw + lx) * 4) as usize;
            let src_a = layer_buf[lidx + 3] as f32 / 255.0 * l_opacity;
            if src_a <= 0.001 {
                continue;
            }
            let px = tb.min_x + lx;
            let py = tb.min_y + ly;
            let idx = ((py * width + px) * 4) as usize;
            if idx + 3 >= buffer.len() {
                continue;
            }
            let src_linear = crate::core::color::Rgbaf::from_rgba8(
                layer_buf[lidx],
                layer_buf[lidx + 1],
                layer_buf[lidx + 2],
                255,
            );
            let src_lin =
                crate::core::color::Rgbaf::new(src_linear.r, src_linear.g, src_linear.b, src_a);
            let dst_linear = crate::core::color::Rgbaf::from_rgba8(
                buffer[idx],
                buffer[idx + 1],
                buffer[idx + 2],
                buffer[idx + 3],
            );
            let out = src_lin.over(dst_linear);
            let out_rgba = out.to_rgba8();
            buffer[idx] = out_rgba[0];
            buffer[idx + 1] = out_rgba[1];
            buffer[idx + 2] = out_rgba[2];
            buffer[idx + 3] = out_rgba[3];
        }
    }
}
