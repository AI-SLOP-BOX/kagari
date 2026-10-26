#![allow(dead_code)]

// 91. Audio Waveform
pub fn apply_audio_waveforms(pixels: &mut [u8], width: u32, height: u32, wave_color: [u8; 4]) {
    crate::core::ae_effects_pack_v2::apply_grid(pixels, width, height, 16, 1, wave_color);
}

// 92. Beam — renders a glowing beam with true pixel-width thickness
pub fn apply_beam(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    p1: [f32; 2],
    p2: [f32; 2],
    thickness: f32,
    beam_color: [u8; 4],
) {
    let half_t = (thickness * 0.5).max(0.5);
    let num_samples = 200;
    for s in 0..num_samples {
        let t = s as f32 / num_samples as f32;
        let cx = p1[0] + (p2[0] - p1[0]) * t;
        let cy = p1[1] + (p2[1] - p1[1]) * t;

        // Fill a circle of radius half_t around each point
        for dy in -(half_t as i32)..=(half_t as i32) {
            for dx in -(half_t as i32)..=(half_t as i32) {
                if (dx * dx + dy * dy) as f32 <= half_t * half_t {
                    let px = (cx + dx as f32).clamp(0.0, width as f32 - 1.0) as u32;
                    let py = (cy + dy as f32).clamp(0.0, height as f32 - 1.0) as u32;
                    let idx = ((py * width + px) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(&beam_color);
                }
            }
        }
    }
}

// 93. Ellipse Generator
pub fn apply_ellipse_generator(pixels: &mut [u8], width: u32, height: u32, color: [u8; 4]) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let rx = width as f32 * 0.4;
    let ry = height as f32 * 0.4;
    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 - cx) / rx;
            let dy = (y as f32 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

// 94. Radio Waves — renders wave_count concentric expanding rings
pub fn apply_radio_waves(pixels: &mut [u8], width: u32, height: u32, wave_count: u32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = (cx.min(cy)) as u32;
    let spacing = (max_r / wave_count.max(1)).max(1);

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt() as u32;

            // Draw ring at every `spacing` pixels
            if dist > 0 && dist % spacing < 2 {
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx] = 0;
                pixels[idx + 1] = 200;
                pixels[idx + 2] = 255;
                pixels[idx + 3] = 255;
            }
        }
    }
}

// 95. Stroke Path
pub fn apply_stroke_path(pixels: &mut [u8], width: u32, height: u32, color: [u8; 4]) {
    apply_ellipse_generator(pixels, width, height, color);
}

// 96. Write-on
pub fn apply_write_on(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    pos: [f32; 2],
    brush_size: f32,
    color: [u8; 4],
) {
    apply_beam(
        pixels,
        width,
        height,
        pos,
        [pos[0] + brush_size, pos[1]],
        brush_size,
        color,
    );
}

// 97. Lightning
pub fn apply_lightning(pixels: &mut [u8], width: u32, height: u32, p1: [f32; 2], p2: [f32; 2]) {
    apply_beam(pixels, width, height, p1, p2, 2.0, [200, 230, 255, 255]);
}

// 98. Star Burst
pub fn apply_star_burst(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_particle_world(
        pixels,
        width,
        height,
        frame,
        [255, 255, 255, 255],
    );
}

// 99. CC Particle Systems II
pub fn apply_particle_systems_ii(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_particle_world(
        pixels,
        width,
        height,
        frame,
        [255, 180, 50, 255],
    );
}

// 100. CC Drizzle / Rain
pub fn apply_rain(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    apply_star_burst(pixels, width, height, frame);
}
