//! Pure-Rust Adobe Photoshop (`.psd`) Layered Importer.
//!
//! Parses PSD v1 (`8BPS`) containers including:
//! - Canvas dimensions and 8-bit RGB/RGBA layers
//! - Per-layer bounding rect (`top, left, bottom, right`), opacity, visibility, blend key, and name
//! - Raw (compression=0) and PackBits RLE (compression=1) channel pixel decoding

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsdLayer {
    pub name: String,
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub opacity: u8,
    pub visible: bool,
    pub blend_key: [u8; 4],
    pub rgba8: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PsdDocument {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<PsdLayer>,
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.remaining() < n {
            return Err("unexpected EOF in PSD".to_string());
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        let b = self.read_bytes(2)?;
        Ok(i16::from_be_bytes([b[0], b[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        let b = self.read_bytes(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn skip(&mut self, n: usize) -> Result<(), String> {
        if self.remaining() < n {
            return Err("unexpected EOF while skipping PSD section".to_string());
        }
        self.pos += n;
        Ok(())
    }
}

/// Parses an Adobe Photoshop `.psd` byte stream into [`PsdDocument`] with per-layer RGBA8 buffers.
pub fn parse_psd(bytes: &[u8]) -> Result<PsdDocument, String> {
    let mut c = Cursor::new(bytes);
    if c.read_bytes(4)? != b"8BPS" {
        return Err("not a valid PSD file (missing 8BPS)".to_string());
    }
    let version = c.read_u16()?;
    if version != 1 {
        return Err(format!("unsupported PSD version {version} (expected 1)"));
    }
    c.skip(6)?; // reserved
    let _channels = c.read_u16()?;
    let doc_height = c.read_u32()?;
    let doc_width = c.read_u32()?;
    let depth = c.read_u16()?;
    let color_mode = c.read_u16()?;
    if depth != 8 || color_mode != 3 {
        return Err(format!(
            "unsupported PSD format: depth={depth}, color_mode={color_mode} (expected 8-bit RGB)"
        ));
    }

    // Color Mode Data + Image Resources
    let cmd_len = c.read_u32()? as usize;
    c.skip(cmd_len)?;
    let res_len = c.read_u32()? as usize;
    c.skip(res_len)?;

    // Layer and Mask Information
    let lmi_len = c.read_u32()? as usize;
    if lmi_len == 0 {
        return Ok(PsdDocument {
            width: doc_width,
            height: doc_height,
            layers: Vec::new(),
        });
    }
    let layer_info_len = c.read_u32()? as usize;
    if layer_info_len == 0 {
        return Ok(PsdDocument {
            width: doc_width,
            height: doc_height,
            layers: Vec::new(),
        });
    }
    let raw_count = c.read_i16()?;
    let layer_count = raw_count.unsigned_abs() as usize;

    struct RawLayerMeta {
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        channels: Vec<(i16, usize)>,
        blend_key: [u8; 4],
        opacity: u8,
        visible: bool,
        name: String,
    }

    let mut metas = Vec::with_capacity(layer_count);
    for _ in 0..layer_count {
        let top = c.read_i32()?;
        let left = c.read_i32()?;
        let bottom = c.read_i32()?;
        let right = c.read_i32()?;
        let width = (right - left).max(0) as u32;
        let height = (bottom - top).max(0) as u32;

        let ch_count = c.read_u16()? as usize;
        let mut channels = Vec::with_capacity(ch_count);
        for _ in 0..ch_count {
            let ch_id = c.read_i16()?;
            let ch_len = c.read_u32()? as usize;
            channels.push((ch_id, ch_len));
        }

        if c.read_bytes(4)? != b"8BIM" {
            return Err("invalid PSD layer blend signature".to_string());
        }
        let bk = c.read_bytes(4)?;
        let blend_key = [bk[0], bk[1], bk[2], bk[3]];
        let opacity = c.read_u8()?;
        let _clipping = c.read_u8()?;
        let flags = c.read_u8()?;
        let visible = (flags & 0x02) == 0;
        let _filler = c.read_u8()?;

        let extra_len = c.read_u32()? as usize;
        let extra_end = c.pos + extra_len;
        let mask_len = c.read_u32()? as usize;
        c.skip(mask_len)?;
        let ranges_len = c.read_u32()? as usize;
        c.skip(ranges_len)?;

        // Pascal string padded to multiple of 4 bytes
        let name_len = c.read_u8()? as usize;
        let name_bytes = c.read_bytes(name_len)?;
        let name = String::from_utf8_lossy(name_bytes).into_owned();
        let pad = (4 - ((1 + name_len) % 4)) % 4;
        c.skip(pad)?;

        if c.pos < extra_end {
            c.skip(extra_end - c.pos)?;
        }

        metas.push(RawLayerMeta {
            left,
            top,
            width,
            height,
            channels,
            blend_key,
            opacity,
            visible,
            name,
        });
    }

    let mut layers = Vec::with_capacity(metas.len());
    for meta in metas {
        let px_count = (meta.width as usize).saturating_mul(meta.height as usize);
        let mut rgba8 = vec![255u8; px_count * 4];
        for (i, px) in rgba8.chunks_exact_mut(4).enumerate() {
            let _ = i;
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
            px[3] = 255;
        }

        for &(ch_id, ch_len) in &meta.channels {
            let ch_bytes = c.read_bytes(ch_len)?;
            if px_count == 0 || ch_bytes.len() < 2 {
                continue;
            }
            let comp = u16::from_be_bytes([ch_bytes[0], ch_bytes[1]]);
            let decoded = match comp {
                0 => ch_bytes[2..].to_vec(),
                1 => decode_packbits_channel(&ch_bytes[2..], meta.width as usize, meta.height as usize)?,
                other => return Err(format!("unsupported PSD channel compression {other}")),
            };
            let target_ch = match ch_id {
                0 => Some(0usize),
                1 => Some(1usize),
                2 => Some(2usize),
                -1 => Some(3usize),
                _ => None,
            };
            if let Some(tc) = target_ch {
                for (i, &val) in decoded.iter().take(px_count).enumerate() {
                    rgba8[i * 4 + tc] = val;
                }
            }
        }

        layers.push(PsdLayer {
            name: meta.name,
            left: meta.left,
            top: meta.top,
            width: meta.width,
            height: meta.height,
            opacity: meta.opacity,
            visible: meta.visible,
            blend_key: meta.blend_key,
            rgba8,
        });
    }

    Ok(PsdDocument {
        width: doc_width,
        height: doc_height,
        layers,
    })
}

fn decode_packbits_channel(data: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    let counts_bytes = height * 2;
    if data.len() < counts_bytes {
        return Err("truncated PSD PackBits row table".to_string());
    }
    let mut pos = counts_bytes;
    let mut out = Vec::with_capacity(width * height);
    for row in 0..height {
        let row_len = u16::from_be_bytes([data[row * 2], data[row * 2 + 1]]) as usize;
        if pos + row_len > data.len() {
            return Err("truncated PSD PackBits scanline".to_string());
        }
        let scan = &data[pos..pos + row_len];
        pos += row_len;

        let mut sp = 0usize;
        let row_start = out.len();
        while sp < scan.len() && out.len() - row_start < width {
            let n = scan[sp] as i8;
            sp += 1;
            if n >= 0 {
                let count = (n as usize) + 1;
                if sp + count > scan.len() {
                    return Err("truncated PackBits literal run".to_string());
                }
                out.extend_from_slice(&scan[sp..sp + count]);
                sp += count;
            } else if n != -128 {
                let count = (1 - n as i32) as usize;
                if sp >= scan.len() {
                    return Err("truncated PackBits repeat run".to_string());
                }
                let val = scan[sp];
                sp += 1;
                out.resize(out.len() + count, val);
            }
        }
        out.resize(row_start + width, 0);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_psd_decodes_layered_raw_and_packbits_rgba() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"8BPS");
        buf.extend_from_slice(&1u16.to_be_bytes()); // version
        buf.extend_from_slice(&[0u8; 6]);
        buf.extend_from_slice(&4u16.to_be_bytes()); // channels
        buf.extend_from_slice(&2u32.to_be_bytes()); // height
        buf.extend_from_slice(&2u32.to_be_bytes()); // width
        buf.extend_from_slice(&8u16.to_be_bytes()); // depth
        buf.extend_from_slice(&3u16.to_be_bytes()); // RGB
        buf.extend_from_slice(&0u32.to_be_bytes()); // color mode data len
        buf.extend_from_slice(&0u32.to_be_bytes()); // image resources len

        // Build Layer & Mask Info
        let mut lmi = Vec::new();
        lmi.extend_from_slice(&1i16.to_be_bytes()); // 1 layer
        lmi.extend_from_slice(&0i32.to_be_bytes()); // top
        lmi.extend_from_slice(&0i32.to_be_bytes()); // left
        lmi.extend_from_slice(&2i32.to_be_bytes()); // bottom
        lmi.extend_from_slice(&2i32.to_be_bytes()); // right
        lmi.extend_from_slice(&4u16.to_be_bytes()); // 4 channels

        // Channel payloads: R=raw(200), G=packbits(100), B=raw(50), A=raw(255)
        let ch_r: Vec<u8> = vec![0, 0, 200, 200, 200, 200];
        let ch_g: Vec<u8> = vec![
            0, 1, // PackBits
            0, 2, 0, 2, // row 0: 2 bytes, row 1: 2 bytes
            255, 100, // repeat 100 x2
            255, 100, // repeat 100 x2
        ];
        let ch_b: Vec<u8> = vec![0, 0, 50, 50, 50, 50];
        let ch_a: Vec<u8> = vec![0, 0, 255, 255, 255, 255];

        for (id, data) in [(0i16, &ch_r), (1, &ch_g), (2, &ch_b), (-1, &ch_a)] {
            lmi.extend_from_slice(&id.to_be_bytes());
            lmi.extend_from_slice(&(data.len() as u32).to_be_bytes());
        }
        lmi.extend_from_slice(b"8BIM");
        lmi.extend_from_slice(b"norm");
        lmi.push(255); // opacity
        lmi.push(0); // clipping
        lmi.push(0); // visible
        lmi.push(0); // filler

        // Extra data: mask(0) + ranges(0) + name("Hero", len=4 => [4, b'H',b'e',b'r',b'o', 0,0,0])
        let extra_len = 4 + 4 + 8;
        lmi.extend_from_slice(&(extra_len as u32).to_be_bytes());
        lmi.extend_from_slice(&0u32.to_be_bytes());
        lmi.extend_from_slice(&0u32.to_be_bytes());
        lmi.push(4);
        lmi.extend_from_slice(b"Hero");
        lmi.extend_from_slice(&[0u8; 3]);

        lmi.extend_from_slice(&ch_r);
        lmi.extend_from_slice(&ch_g);
        lmi.extend_from_slice(&ch_b);
        lmi.extend_from_slice(&ch_a);

        buf.extend_from_slice(&((lmi.len() + 4) as u32).to_be_bytes());
        buf.extend_from_slice(&(lmi.len() as u32).to_be_bytes());
        buf.extend_from_slice(&lmi);

        let doc = parse_psd(&buf).expect("parse psd");
        assert_eq!(doc.width, 2);
        assert_eq!(doc.height, 2);
        assert_eq!(doc.layers.len(), 1);
        assert_eq!(doc.layers[0].name, "Hero");
        assert!(doc.layers[0].visible);
        assert_eq!(&doc.layers[0].rgba8[0..4], &[200, 100, 50, 255]);
    }
}
