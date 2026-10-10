/// 5x7 Bitmapped font glyphs for '0'-'9' and ':'
const GLYPH_WIDTH: usize = 5;
const GLYPH_HEIGHT: usize = 7;

static GLYPHS: [[u8; 7]; 11] = [
    // '0'
    [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ],
    // '1'
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ],
    // '2'
    [
        0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
    ],
    // '3'
    [
        0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
    ],
    // '4'
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ],
    // '5'
    [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ],
    // '6'
    [
        0b01110, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b01110,
    ],
    // '7'
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ],
    // '8'
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ],
    // '9'
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
    ],
    // ':'
    [
        0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
    ],
];

fn get_glyph_row(ch: char, row: usize) -> u8 {
    let idx = match ch {
        '0'..='9' => (ch as usize) - ('0' as usize),
        ':' => 10,
        _ => return 0,
    };
    if row < GLYPH_HEIGHT {
        GLYPHS[idx][row]
    } else {
        0
    }
}

/// Parameters for [`apply_timecode_burn_in`].
#[derive(Debug, Clone, Copy)]
pub struct TimecodeParams {
    pub frame: u32,
    pub fps: u32,
    /// X, Y offset in pixels.
    pub position: [f32; 2],
    /// Scale multiplier (1.0 = standard, 2.0 = large, etc.).
    pub scale: f32,
    /// Foreground text colour RGBA `[0..1]`.
    pub text_color: [f32; 4],
    /// Background box opacity `[0..1]`.
    pub bg_opacity: f32,
}

impl Default for TimecodeParams {
    fn default() -> Self {
        Self {
            frame: 0,
            fps: 30,
            position: [20.0, 20.0],
            scale: 2.0,
            text_color: [1.0, 1.0, 1.0, 1.0],
            bg_opacity: 0.6,
        }
    }
}

/// Burn SMPTE timecode (HH:MM:SS:FF) into the pixel buffer.
pub fn apply_timecode_burn_in(pixels: &mut [u8], width: u32, height: u32, p: &TimecodeParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let fps = p.fps.max(1);
    let total_sec = p.frame / fps;
    let frames = p.frame % fps;
    let sec = total_sec % 60;
    let min = (total_sec / 60) % 60;
    let hrs = total_sec / 3600;

    let text = format!("{:02}:{:02}:{:02}:{:02}", hrs, min, sec, frames);
    let scale = p.scale.max(1.0);
    let char_w = (GLYPH_WIDTH as f32 * scale).round() as u32;
    let char_h = (GLYPH_HEIGHT as f32 * scale).round() as u32;
    let spacing = (2.0 * scale).round() as u32;

    let total_w = text.len() as u32 * (char_w + spacing) + spacing * 2;
    let total_h = char_h + spacing * 2;

    let start_x = p.position[0].max(0.0) as u32;
    let start_y = p.position[1].max(0.0) as u32;

    // Draw background box
    if p.bg_opacity > 0.001 {
        let box_x0 = start_x;
        let box_y0 = start_y;
        let box_x1 = (box_x0 + total_w).min(width);
        let box_y1 = (box_y0 + total_h).min(height);
        let bg_alpha = p.bg_opacity.clamp(0.0, 1.0);

        for y in box_y0..box_y1 {
            for x in box_x0..box_x1 {
                let idx = ((y * width + x) * 4) as usize;
                for c in 0..3 {
                    pixels[idx + c] = (pixels[idx + c] as f32 * (1.0 - bg_alpha)).round() as u8;
                }
                pixels[idx + 3] = (pixels[idx + 3] as f32 + bg_alpha * 255.0).min(255.0) as u8;
            }
        }
    }

    // Draw text characters
    let mut cur_x = start_x + spacing;
    let cur_y = start_y + spacing;

    for ch in text.chars() {
        for row in 0..GLYPH_HEIGHT {
            let row_bits = get_glyph_row(ch, row);
            let py_start = cur_y + (row as f32 * scale) as u32;
            let py_end = (cur_y + ((row + 1) as f32 * scale) as u32).min(height);

            for col in 0..GLYPH_WIDTH {
                if (row_bits & (1 << (4 - col))) != 0 {
                    let px_start = cur_x + (col as f32 * scale) as u32;
                    let px_end = (cur_x + ((col + 1) as f32 * scale) as u32).min(width);

                    for y in py_start..py_end {
                        for x in px_start..px_end {
                            let idx = ((y * width + x) * 4) as usize;
                            for c in 0..3 {
                                pixels[idx + c] = (p.text_color[c] * 255.0).clamp(0.0, 255.0) as u8;
                            }
                            pixels[idx + 3] = (p.text_color[3] * 255.0).clamp(0.0, 255.0) as u8;
                        }
                    }
                }
            }
        }
        cur_x += char_w + spacing;
    }
}
