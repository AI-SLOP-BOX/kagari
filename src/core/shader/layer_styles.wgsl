fn ls_stroke_edge_distance(alpha: f32) -> f32 {
    let dx = abs(dpdx(alpha));
    let dy = abs(dpdy(alpha));
    let edge_width = max(dx + dy, 0.001);
    return clamp((1.0 - alpha) / edge_width, 0.0, 1.0);
}

fn ls_apply_stroke(color: vec4<f32>, alpha: f32, stroke_color: vec3<f32>, width: f32) -> vec4<f32> {
    if width <= 0.0 || alpha <= 0.0 { return color; }
    let dist = ls_stroke_edge_distance(alpha);
    let stroke_alpha = 1.0 - smoothstep(0.0, width * 0.01, dist);
    let out_rgb = mix(color.rgb, stroke_color, stroke_alpha);
    let out_a = max(color.a, stroke_alpha);
    return vec4<f32>(out_rgb, out_a);
}

fn ls_apply_color_overlay(color: vec4<f32>, overlay: vec4<f32>) -> vec4<f32> {
    let a = overlay.a * color.a;
    return vec4<f32>(mix(color.rgb, overlay.rgb, overlay.a), a);
}

fn ls_apply_gradient_overlay(color: vec4<f32>, pos: vec2<f32>, start: vec2<f32>, end: vec2<f32>, c1: vec4<f32>, c2: vec4<f32>) -> vec4<f32> {
    let d = end - start;
    let len_sq = dot(d, d);
    var t: f32;
    if len_sq < 0.001 {
        t = 0.5;
    } else {
        t = clamp(dot(pos - start, d) / len_sq, 0.0, 1.0);
    }
    let grad_color = mix(c1, c2, t);
    let a = grad_color.a * color.a;
    return vec4<f32>(mix(color.rgb, grad_color.rgb, grad_color.a), a);
}

fn ls_apply_inner_shadow(color: vec4<f32>, alpha: f32, offset: vec2<f32>, size: f32, shadow_color: vec3<f32>, opacity: f32) -> vec4<f32> {
    if alpha > 0.99 || size <= 0.0 || opacity <= 0.0 { return color; }
    let edge_dist = 1.0 - alpha;
    let shadow_falloff = 1.0 - smoothstep(0.0, size * 0.01, edge_dist);
    let offset_len = length(offset);
    let shadow_strength = shadow_falloff * opacity * smoothstep(0.0, offset_len + 0.001, edge_dist * 10.0);
    let shadow = vec4<f32>(shadow_color, shadow_strength);
    let out_a = max(color.a, shadow.a * color.a);
    let out_rgb = mix(color.rgb, shadow.rgb, shadow.a);
    return vec4<f32>(out_rgb, out_a);
}

fn ls_apply_bevel(color: vec4<f32>, alpha: f32, size: f32, angle: f32, strength: f32, light_color: vec3<f32>, dark_color: vec3<f32>) -> vec4<f32> {
    if alpha <= 0.01 || size <= 0.0 { return color; }
    let normal_x = dpdx(alpha);
    let normal_y = dpdy(alpha);
    let edge_width = max(abs(normal_x) + abs(normal_y), 0.001);
    let bevel_t = clamp((1.0 - alpha) / (size * 0.01 * 4.0 + 0.001), 0.0, 1.0);
    let rad = angle * 0.01745329251;
    let light_dir = vec2<f32>(cos(rad), sin(rad));
    let edge_normal = normalize(vec2<f32>(normal_x, normal_y) + vec2<f32>(0.0001));
    let ndotl = dot(edge_normal, light_dir);
    let highlight = max(ndotl, 0.0) * strength * 0.01;
    let shadow = max(-ndotl, 0.0) * strength * 0.01;
    let highlight_mask = bevel_t * highlight;
    let shadow_mask = bevel_t * shadow;
    let out_rgb = color.rgb + light_color * highlight_mask - dark_color * shadow_mask;
    return vec4<f32>(clamp(out_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), color.a);
}

@fragment
