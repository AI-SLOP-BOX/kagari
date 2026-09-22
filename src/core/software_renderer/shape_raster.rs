//! Shape layer rasterization (SDF fills, trims, repeaters).

use super::mask::compute_combined_mask_coverage;
use super::raster::RasterCtx;
use crate::core::sdf::{rasterize_shape_sdf, rasterize_shape_sdf_with_rotation, sdf_boolean_op};
use crate::core::timeline::LayerType;

/// Rasterize a shape layer into its local buffer (plus pre-effect mask).
pub(crate) fn rasterize_shape_layer(ctx: RasterCtx<'_>) {
    let RasterCtx {
        layer,
        effective_frame,
        masks,
        min_x,
        min_y,
        bw,
        bh,
        cx,
        cy,
        bounds_x,
        bounds_y,
        base_color,
        layer_buf,
        ..
    } = ctx;
    let LayerType::Shape {
        shape_type,
        stroke_color,
        stroke_width,
        fill_type,
        extrusion_depth,
        ..
    } = &layer.layer_type
    else {
        return;
    };
    let sc = *stroke_color;
    let sw = *stroke_width;
    let ft = fill_type;
    // Check for MergePaths effect to enable boolean operations
    let merge_op = layer.effects.iter().find_map(|e| {
        if let crate::core::timeline::EffectType::MergePaths { operation } = &e.effect_type {
            if e.enabled {
                Some(operation.evaluate(effective_frame) as u32)
            } else {
                None
            }
        } else {
            None
        }
    });
    if merge_op.is_some() {
        // MergePaths: render shape SDF into a second buffer, then combine with boolean
        let merge_op_val = merge_op.unwrap_or(0);
        let mut second_buf = vec![0u8; (bw * bh * 4) as usize];
        // Render a copy shifted by 20% for visual demonstration of boolean op
        let shift_x = bounds_x * 0.3;
        let shift_y = bounds_y * 0.2;
        rasterize_shape_sdf(
            &mut second_buf,
            bw,
            bh,
            min_x,
            min_y,
            cx + shift_x,
            cy + shift_y,
            bounds_x,
            bounds_y,
            base_color,
            ft,
            sc,
            sw,
            1.0,
            shape_type,
            effective_frame,
            layer.trim_paths.as_ref(),
        );
        // Also render primary shape
        rasterize_shape_sdf(
            &mut *layer_buf,
            bw,
            bh,
            min_x,
            min_y,
            cx,
            cy,
            bounds_x,
            bounds_y,
            base_color,
            ft,
            sc,
            sw,
            1.0,
            shape_type,
            effective_frame,
            layer.trim_paths.as_ref(),
        );
        // Combine using boolean SDF: modify layer_buf alpha based on second_buf
        for i in (3..layer_buf.len()).step_by(4) {
            let a1 = layer_buf[i] as f32 / 255.0;
            let a2 = second_buf[i] as f32 / 255.0;
            let combined = sdf_boolean_op(merge_op_val, a1, a2);
            layer_buf[i] = (combined.clamp(0.0, 1.0) * 255.0) as u8;
        }
    } else if let Some(repeater) = &layer.shape_repeater {
        // Repeater: render shape multiple times with transforms
        let instances = crate::core::shape_repeater::evaluate_shape_repeater_at_frame(
            repeater,
            effective_frame,
        );
        for instance in &instances {
            // Apply repeater transform to center position
            let m = &instance.transform_matrix;
            let rx = cx * m[0][0] + cy * m[0][1] + m[0][2];
            let ry = cx * m[1][0] + cy * m[1][1] + m[1][2];
            let copy_bounds_x = bounds_x * (m[0][0].hypot(m[1][0]).max(0.0001));
            let copy_bounds_y = bounds_y * (m[0][1].hypot(m[1][1]).max(0.0001));
            let mut copy_color = base_color;
            copy_color[3] *= instance.opacity;
            rasterize_shape_sdf_with_rotation(
                &mut *layer_buf,
                bw,
                bh,
                min_x,
                min_y,
                rx,
                ry,
                copy_bounds_x,
                copy_bounds_y,
                copy_color,
                ft,
                sc,
                sw,
                1.0,
                shape_type,
                effective_frame,
                layer.trim_paths.as_ref(),
                m[1][0].atan2(m[0][0]).to_degrees(),
            );
        }
    } else {
        if *extrusion_depth > 0.1 {
            let steps = (*extrusion_depth as usize).min(64);
            let side_col = [
                base_color[0] * 0.65,
                base_color[1] * 0.65,
                base_color[2] * 0.65,
                base_color[3],
            ];
            // Render back-to-front extrusion slices with isometric/perspective depth
            for step in (1..=steps).rev() {
                let offset_d = step as f32;
                rasterize_shape_sdf(
                    &mut *layer_buf,
                    bw,
                    bh,
                    min_x,
                    min_y,
                    cx + offset_d * 0.5,
                    cy + offset_d * 0.5,
                    bounds_x,
                    bounds_y,
                    side_col,
                    ft,
                    sc,
                    sw,
                    1.0,
                    shape_type,
                    effective_frame,
                    layer.trim_paths.as_ref(),
                );
            }
        }
        rasterize_shape_sdf(
            &mut *layer_buf,
            bw,
            bh,
            min_x,
            min_y,
            cx,
            cy,
            bounds_x,
            bounds_y,
            base_color,
            ft,
            sc,
            sw,
            1.0,
            shape_type,
            effective_frame,
            layer.trim_paths.as_ref(),
        );
        // Pre-effect mask (isolate): shape rasterization has no
        // baked mask, so multiply it here before effects run.
        // Solid/Text/Image bake coverage during their own raster;
        // all types get a post-effect re-mask below (contain).
        if !masks.is_empty() {
            for ly in 0..bh {
                for lx in 0..bw {
                    let lidx = ((ly * bw + lx) * 4) as usize;
                    if lidx + 3 >= layer_buf.len() || layer_buf[lidx + 3] == 0 {
                        continue;
                    }
                    let cov = compute_combined_mask_coverage(
                        (min_x + lx) as f32,
                        (min_y + ly) as f32,
                        masks,
                    );
                    layer_buf[lidx + 3] = (layer_buf[lidx + 3] as f32 * cov) as u8;
                }
            }
        }
    }
}
