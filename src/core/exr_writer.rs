//! OpenEXR scanline writer — half/float pixel types, ZIP compression, arbitrary channel lists.
//!
//! Implements the OpenEXR container directly (no external EXR crate) so VFX interchange
//! stays dependency-light:
//! - `NONE` and `ZIP`/`ZIPS` compression, the latter via `flate2` zlib streams
//! - `HALF` (f16) and `FLOAT` (f32) channel sample types
//! - multi-pass output: `R,G,B,A` plus auxiliary passes (`Z` depth, normals, Cryptomatte
//!   ids, motion vectors) as extra channels in one file
//!
//! Channels are emitted in alphabetical order as required by the spec, interleaved
//! per-channel across a scanline block. ZIP applies the OpenEXR byte reorder plus delta
//! predictor before deflating, so readers decode identical pixels to `NONE`.

use std::io::{self, Seek, SeekFrom, Write};

#[cfg(feature = "cli")]
use flate2::{write::ZlibEncoder, Compression};

const EXR_MAGIC: u32 = 0x0131_2f76;
const EXR_VERSION: u32 = 2;
const PIXEL_HALF: i32 = 1;
const PIXEL_FLOAT: i32 = 2;
const COMP_NONE: u8 = 0;
const COMP_ZIPS: u8 = 2;
const COMP_ZIP: u8 = 3;
const LINE_INCREASING_Y: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExrCompression {
    None,
    Zips,
    Zip,
}

impl ExrCompression {
    fn code(self) -> u8 {
        match self {
            ExrCompression::None => COMP_NONE,
            ExrCompression::Zips => COMP_ZIPS,
            ExrCompression::Zip => COMP_ZIP,
        }
    }

    fn scanlines_per_block(self) -> u32 {
        match self {
            ExrCompression::None | ExrCompression::Zips => 1,
            ExrCompression::Zip => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExrSampleType {
    Half,
    Float,
}

impl ExrSampleType {
    fn code(self) -> i32 {
        match self {
            ExrSampleType::Half => PIXEL_HALF,
            ExrSampleType::Float => PIXEL_FLOAT,
        }
    }
}

/// One named channel holding one sample per pixel, in row-major order.
#[derive(Debug, Clone)]
pub struct ExrChannel {
    pub name: String,
    pub sample_type: ExrSampleType,
    pub data: Vec<f32>,
}

impl ExrChannel {
    pub fn new(name: impl Into<String>, data: Vec<f32>) -> Self {
        Self {
            name: name.into(),
            sample_type: ExrSampleType::Float,
            data,
        }
    }

    pub fn half(name: impl Into<String>, data: Vec<f32>) -> Self {
        Self {
            name: name.into(),
            sample_type: ExrSampleType::Half,
            data,
        }
    }

    fn validate(&self, expected: usize) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("EXR channel name must not be empty".to_string());
        }
        if self.name.contains('\0') {
            return Err(format!("EXR channel name contains NUL: {:?}", self.name));
        }
        if self.data.iter().any(|v| !v.is_finite()) {
            return Err(format!("EXR channel {:?} holds non-finite data", self.name));
        }
        if self.data.len() != expected {
            return Err(format!(
                "EXR channel {:?} has {} samples, expected {}",
                self.name,
                self.data.len(),
                expected
            ));
        }
        Ok(())
    }
}

/// An OpenEXR scanline image: one or more channels sharing a data window.
#[derive(Debug, Clone)]
pub struct ExrImage {
    pub width: u32,
    pub height: u32,
    pub channels: Vec<ExrChannel>,
    pub compression: ExrCompression,
}

impl ExrImage {
    /// Beauty pass from interleaved linear `R,G,B,A` floats.
    pub fn rgba_f32(width: u32, height: u32, rgba: &[f32]) -> Self {
        let n = (width as usize) * (height as usize);
        let mut r = vec![0.0f32; n];
        let mut g = vec![0.0f32; n];
        let mut b = vec![0.0f32; n];
        let mut a = vec![0.0f32; n];
        for (i, chunk) in rgba.as_chunks::<4>().0.iter().take(n).enumerate() {
            r[i] = chunk[0];
            g[i] = chunk[1];
            b[i] = chunk[2];
            a[i] = chunk[3];
        }
        Self {
            width,
            height,
            channels: vec![
                ExrChannel::new("A", a),
                ExrChannel::new("B", b),
                ExrChannel::new("G", g),
                ExrChannel::new("R", r),
            ],
            compression: ExrCompression::Zip,
        }
    }

    fn sorted_channels(&self) -> Vec<&ExrChannel> {
        let mut v: Vec<&ExrChannel> = self.channels.iter().collect();
        v.sort_by(|x, y| x.name.cmp(&y.name));
        v
    }

    /// Validate dimensions, channel names, sample counts and finiteness.
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err("EXR image must be non-empty".to_string());
        }
        if self.channels.is_empty() {
            return Err("EXR image needs at least one channel".to_string());
        }
        let expected = (self.width as usize) * (self.height as usize);
        let mut seen: Vec<&str> = Vec::with_capacity(self.channels.len());
        for ch in &self.channels {
            ch.validate(expected)?;
            if seen.contains(&ch.name.as_str()) {
                return Err(format!("duplicate EXR channel {:?}", ch.name));
            }
            seen.push(&ch.name);
        }
        Ok(())
    }
}

/// Write an [`ExrImage`] to any seekable sink as an OpenEXR scanline file.
pub fn write_exr<W: Write + Seek>(sink: &mut W, image: &ExrImage) -> Result<(), String> {
    image.validate()?;
    let channels = image.sorted_channels();
    let width = image.width;
    let height = image.height;
    let spb = image.compression.scanlines_per_block();
    let blocks = height.div_ceil(spb);

    sink.write_all(&EXR_MAGIC.to_le_bytes())
        .map_err(|e| e.to_string())?;
    sink.write_all(&EXR_VERSION.to_le_bytes())
        .map_err(|e| e.to_string())?;

    write_attribute(sink, "channels", "chlist", &chlist_bytes(&channels))?;
    write_attribute(
        sink,
        "compression",
        "compression",
        &[image.compression.code()],
    )?;
    write_attribute(
        sink,
        "dataWindow",
        "box2i",
        &box2i_bytes(0, 0, width - 1, height - 1),
    )?;
    write_attribute(
        sink,
        "displayWindow",
        "box2i",
        &box2i_bytes(0, 0, width - 1, height - 1),
    )?;
    write_attribute(sink, "lineOrder", "lineOrder", &[LINE_INCREASING_Y])?;
    write_attribute(sink, "pixelAspectRatio", "float", &1.0f32.to_le_bytes())?;
    write_attribute(
        sink,
        "screenWindowCenter",
        "v2f",
        &[0.0f32.to_le_bytes(), 0.0f32.to_le_bytes()].concat(),
    )?;
    write_attribute(sink, "screenWindowWidth", "float", &1.0f32.to_le_bytes())?;
    sink.write_all(&[0u8]).map_err(|e| e.to_string())?;

    let table_pos = sink.stream_position().map_err(|e| e.to_string())?;
    sink.write_all(&vec![0u8; blocks as usize * 8])
        .map_err(|e| e.to_string())?;

    let mut offsets = Vec::with_capacity(blocks as usize);
    for block in 0..blocks {
        let y0 = block * spb;
        let rows = spb.min(height - y0);
        let raw = pack_scanline_block(&channels, width, y0, rows);
        let payload = match image.compression {
            ExrCompression::None => raw,
            ExrCompression::Zips | ExrCompression::Zip => zip_deflate(&raw)?,
        };
        offsets.push(sink.stream_position().map_err(|e| e.to_string())?);
        sink.write_all(&(y0 as i32).to_le_bytes())
            .map_err(|e| e.to_string())?;
        sink.write_all(&(payload.len() as i32).to_le_bytes())
            .map_err(|e| e.to_string())?;
        sink.write_all(&payload).map_err(|e| e.to_string())?;
    }

    sink.seek(SeekFrom::Start(table_pos))
        .map_err(|e| e.to_string())?;
    for off in offsets {
        sink.write_all(&off.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    sink.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// Write an [`ExrImage`] to a file on disk.
pub fn write_exr_file(path: impl AsRef<std::path::Path>, image: &ExrImage) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    write_exr(&mut io::BufWriter::new(file), image)
}

fn write_attribute<W: Write>(
    sink: &mut W,
    name: &str,
    type_name: &str,
    value: &[u8],
) -> Result<(), String> {
    sink.write_all(name.as_bytes()).map_err(|e| e.to_string())?;
    sink.write_all(&[0u8]).map_err(|e| e.to_string())?;
    sink.write_all(type_name.as_bytes())
        .map_err(|e| e.to_string())?;
    sink.write_all(&[0u8]).map_err(|e| e.to_string())?;
    sink.write_all(&(value.len() as i32).to_le_bytes())
        .map_err(|e| e.to_string())?;
    sink.write_all(value).map_err(|e| e.to_string())?;
    Ok(())
}

fn chlist_bytes(channels: &[&ExrChannel]) -> Vec<u8> {
    let mut out = Vec::new();
    for ch in channels {
        out.extend_from_slice(ch.name.as_bytes());
        out.push(0u8);
        out.extend_from_slice(&ch.sample_type.code().to_le_bytes());
        out.push(0u8);
        out.extend_from_slice(&[0u8, 0u8, 0u8]);
        out.extend_from_slice(&1i32.to_le_bytes());
        out.extend_from_slice(&1i32.to_le_bytes());
    }
    out.push(0u8);
    out
}

fn box2i_bytes(x_min: u32, y_min: u32, x_max: u32, y_max: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&(x_min as i32).to_le_bytes());
    out.extend_from_slice(&(y_min as i32).to_le_bytes());
    out.extend_from_slice(&(x_max as i32).to_le_bytes());
    out.extend_from_slice(&(y_max as i32).to_le_bytes());
    out
}

fn pack_scanline_block(channels: &[&ExrChannel], width: u32, y0: u32, rows: u32) -> Vec<u8> {
    let w = width as usize;
    let mut out = Vec::with_capacity(w * rows as usize * channels.len() * 4);
    for ch in channels {
        for row in 0..rows as usize {
            let base = (y0 as usize + row) * w;
            for &v in &ch.data[base..base + w] {
                match ch.sample_type {
                    ExrSampleType::Half => {
                        out.extend_from_slice(&half::f16::from_f32(v).to_bits().to_le_bytes());
                    }
                    ExrSampleType::Float => out.extend_from_slice(&v.to_le_bytes()),
                }
            }
        }
    }
    out
}

/// Apply the OpenEXR ZIP filter and deflate into a zlib stream.
fn zip_deflate(raw: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(feature = "cli")]
    {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&zip_reorder_predict(raw))
            .map_err(|e| e.to_string())?;
        enc.finish().map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "cli"))]
    {
        let _ = raw;
        Err("EXR ZIP compression requires the cli feature".to_string())
    }
}

/// OpenEXR ZIP filter: de-interleave even/odd bytes, then apply the delta predictor.
fn zip_reorder_predict(raw: &[u8]) -> Vec<u8> {
    if raw.is_empty() {
        return Vec::new();
    }
    let n = raw.len();
    let mut tmp = vec![0u8; n];
    let mut src = 0usize;
    let mut t1 = 0usize;
    let mut t2 = n.div_ceil(2);
    while src < n {
        tmp[t1] = raw[src];
        t1 += 1;
        src += 1;
        if src >= n {
            break;
        }
        tmp[t2] = raw[src];
        t2 += 1;
        src += 1;
    }
    let mut prev = tmp[0];
    for slot in tmp.iter_mut().skip(1) {
        let cur = *slot;
        *slot = cur.wrapping_sub(prev).wrapping_add(128);
        prev = cur;
    }
    tmp
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct Parsed {
        names: Vec<String>,
        types: Vec<ExrSampleType>,
        compression: ExrCompression,
        window: (u32, u32, u32, u32),
        blocks: usize,
        offsets: Vec<u64>,
        chunks: Vec<(i32, Vec<u8>)>,
    }

    fn u32_at(b: &[u8], at: &mut usize) -> u32 {
        let v = u32::from_le_bytes([b[*at], b[*at + 1], b[*at + 2], b[*at + 3]]);
        *at += 4;
        v
    }

    fn i32_at(b: &[u8], at: &mut usize) -> i32 {
        u32_at(b, at) as i32
    }

    fn cstr_at(b: &[u8], at: &mut usize) -> String {
        let start = *at;
        while b[*at] != 0 {
            *at += 1;
        }
        let s = String::from_utf8(b[start..*at].to_vec()).expect("utf8");
        *at += 1;
        s
    }

    fn parse(b: &[u8]) -> Parsed {
        let mut at = 0usize;
        assert_eq!(u32_at(b, &mut at), EXR_MAGIC, "magic");
        assert_eq!(u32_at(b, &mut at), EXR_VERSION, "version");

        let mut names = Vec::new();
        let mut types = Vec::new();
        let mut compression = ExrCompression::None;
        let mut window = (0u32, 0u32, 0u32, 0u32);

        loop {
            let name = cstr_at(b, &mut at);
            if name.is_empty() {
                break;
            }
            let type_name = cstr_at(b, &mut at);
            let size = i32_at(b, &mut at) as usize;
            let body = &b[at..at + size];
            at += size;
            match (name.as_str(), type_name.as_str()) {
                ("channels", "chlist") => {
                    let mut c = 0usize;
                    loop {
                        let cn = cstr_at(body, &mut c);
                        if cn.is_empty() {
                            break;
                        }
                        let pt = i32_at(body, &mut c);
                        assert_eq!(body[c], 0, "pLinear must be 0");
                        c += 4;
                        let xs = i32_at(body, &mut c);
                        let ys = i32_at(body, &mut c);
                        assert_eq!((xs, ys), (1, 1), "sampling must be 1,1");
                        types.push(match pt {
                            PIXEL_HALF => ExrSampleType::Half,
                            PIXEL_FLOAT => ExrSampleType::Float,
                            other => panic!("unexpected pixel type {other}"),
                        });
                        names.push(cn);
                    }
                }
                ("compression", "compression") => {
                    compression = match body[0] {
                        COMP_NONE => ExrCompression::None,
                        COMP_ZIPS => ExrCompression::Zips,
                        COMP_ZIP => ExrCompression::Zip,
                        other => panic!("unexpected compression {other}"),
                    };
                }
                ("dataWindow", "box2i") => {
                    let mut c = 0usize;
                    let x0 = i32_at(body, &mut c) as u32;
                    let y0 = i32_at(body, &mut c) as u32;
                    let x1 = i32_at(body, &mut c) as u32;
                    let y1 = i32_at(body, &mut c) as u32;
                    window = (x0, y0, x1, y1);
                }
                _ => {}
            }
        }

        let blocks = (window.3 - window.1 + 1).div_ceil(compression.scanlines_per_block()) as usize;
        let mut offsets = Vec::with_capacity(blocks);
        for _ in 0..blocks {
            let raw: [u8; 8] = b[at..at + 8].try_into().expect("8 bytes");
            offsets.push(u64::from_le_bytes(raw));
            at += 8;
        }
        let mut chunks = Vec::new();
        for &off in &offsets {
            let mut c = off as usize;
            let y = i32_at(b, &mut c);
            let len = i32_at(b, &mut c) as usize;
            chunks.push((y, b[c..c + len].to_vec()));
        }
        Parsed {
            names,
            types,
            compression,
            window,
            blocks,
            offsets,
            chunks,
        }
    }

    fn inflate_unpredict(payload: &[u8]) -> Vec<u8> {
        let mut dec = flate2::read::ZlibDecoder::new(payload);
        let mut filtered = Vec::new();
        std::io::Read::read_to_end(&mut dec, &mut filtered).expect("inflate");
        let n = filtered.len();
        assert!(n > 0, "empty filtered block");
        let mut deint = Vec::with_capacity(n);
        let mut prev = filtered[0];
        deint.push(prev);
        for &slot in filtered.iter().skip(1) {
            prev = slot.wrapping_sub(128).wrapping_add(prev);
            deint.push(prev);
        }
        let mut out = vec![0u8; n];
        let mut src = 0usize;
        let mut t1 = 0usize;
        let mut t2 = n.div_ceil(2);
        while src < n {
            out[src] = deint[t1];
            t1 += 1;
            src += 1;
            if src >= n {
                break;
            }
            out[src] = deint[t2];
            t2 += 1;
            src += 1;
        }
        out
    }

    fn decode(parsed: &Parsed) -> Vec<Vec<f32>> {
        let width = (parsed.window.2 + 1) as usize;
        let height = parsed.window.3 + 1;
        let spb = parsed.compression.scanlines_per_block() as usize;
        let mut out: Vec<Vec<f32>> = parsed.names.iter().map(|_| Vec::new()).collect();
        for (y, payload) in &parsed.chunks {
            let rows = (spb as u32).min(height - *y as u32) as usize;
            let raw = match parsed.compression {
                ExrCompression::None => payload.clone(),
                _ => inflate_unpredict(payload),
            };
            let mut c = 0usize;
            for (ci, st) in parsed.types.iter().enumerate() {
                for _ in 0..(rows * width) {
                    match st {
                        ExrSampleType::Float => {
                            let bits = u32_at(&raw, &mut c);
                            out[ci].push(f32::from_bits(bits));
                        }
                        ExrSampleType::Half => {
                            let bits = u16::from_le_bytes([raw[c], raw[c + 1]]);
                            c += 2;
                            out[ci].push(half::f16::from_bits(bits).to_f32());
                        }
                    }
                }
            }
        }
        out
    }

    fn zlib_of(raw: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut enc, raw).expect("write");
        enc.finish().expect("finish")
    }

    fn ramp(w: u32, h: u32) -> Vec<f32> {
        let n = (w * h) as usize;
        let mut v = Vec::with_capacity(n * 4);
        for i in 0..n {
            let x = (i % w as usize) as f32;
            let y = (i / w as usize) as f32;
            v.push(x * 0.25 - 1.0);
            v.push(y * 0.5 + 2.5);
            v.push(if i % 3 == 0 { 8.0 } else { 0.002 });
            v.push((i % 251) as f32 / 255.0);
        }
        v
    }

    fn write_to_vec(img: &ExrImage) -> Vec<u8> {
        let mut sink = Cursor::new(Vec::new());
        write_exr(&mut sink, img).expect("write");
        sink.into_inner()
    }

    #[test]
    fn test_header_channels_sorted_and_windows_correct() {
        let bytes = write_to_vec(&ExrImage::rgba_f32(3, 2, &ramp(3, 2)));
        let p = parse(&bytes);
        assert_eq!(p.names, vec!["A", "B", "G", "R"], "must be alphabetical");
        assert_eq!(p.window, (0, 0, 2, 1));
        assert_eq!(p.blocks, 1, "2 rows fit in one 16-row block");
        assert_eq!(p.compression, ExrCompression::Zip);
        assert!(p.offsets.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn test_none_compression_roundtrip_is_bit_exact() {
        let (w, h) = (3u32, 2u32);
        let data = ramp(w, h);
        let mut img = ExrImage::rgba_f32(w, h, &data);
        img.compression = ExrCompression::None;
        let bytes = write_to_vec(&img);
        let ch = decode(&parse(&bytes));
        assert_eq!(ch.len(), 4);
        for px in 0..(w * h) as usize {
            let got = [ch[3][px], ch[2][px], ch[1][px], ch[0][px]];
            let want = [
                data[px * 4],
                data[px * 4 + 1],
                data[px * 4 + 2],
                data[px * 4 + 3],
            ];
            assert_eq!(got, want, "pixel {px} must survive exactly");
        }
    }

    #[test]
    fn test_zip_matches_none_and_preserves_overbright() {
        let (w, h) = (4u32, 2u32);
        let data = ramp(w, h);
        let mut plain = ExrImage::rgba_f32(w, h, &data);
        plain.compression = ExrCompression::None;
        let mut zipped = plain.clone();
        zipped.compression = ExrCompression::Zip;

        let (ab, bb) = (write_to_vec(&plain), write_to_vec(&zipped));
        let (pa, pb) = (parse(&ab), parse(&bb));
        let (ca, cb) = (decode(&pa), decode(&pb));
        for i in 0..4 {
            assert_eq!(ca[i], cb[i], "ZIP must equal NONE for channel {i}");
        }
        assert!(ca[2].iter().any(|v| *v > 1.0), "overbright G must survive");
        assert!(ca[3].iter().any(|v| *v < 0.0), "negative R must survive");
    }

    #[test]
    fn test_zip_predictor_is_invertible() {
        let raw: Vec<u8> = (0..257usize).map(|i| (i * 37 % 256) as u8).collect();
        let filtered = zip_reorder_predict(&raw);
        assert_ne!(filtered, raw, "predictor must transform the bytes");
        let back = inflate_unpredict(&zlib_of(&filtered));
        assert_eq!(back, raw, "predictor must be invertible");
    }

    #[test]
    fn test_multiblock_offsets_and_row_labels() {
        let (w, h) = (2u32, 40u32);
        let mut img = ExrImage::rgba_f32(w, h, &ramp(w, h));
        img.compression = ExrCompression::Zip;
        let bytes = write_to_vec(&img);
        let p = parse(&bytes);
        assert_eq!(p.blocks, 3, "40 rows / 16 per block");
        assert!(p.offsets.iter().all(|&o| (o as usize) < bytes.len()));
        for (i, (y, _)) in p.chunks.iter().enumerate() {
            assert_eq!(*y as u32, (i as u32) * 16, "block {i} row label");
        }
        assert_eq!(decode(&p)[0].len(), (w * h) as usize, "all rows present");
    }

    #[test]
    fn test_multipass_with_depth_channel() {
        let (w, h) = (2u32, 2u32);
        let mut img = ExrImage::rgba_f32(w, h, &ramp(w, h));
        img.compression = ExrCompression::None;
        img.channels
            .push(ExrChannel::new("Z", vec![10.0, 500.0, 2500.0, 42.5]));
        let bytes = write_to_vec(&img);
        let p = parse(&bytes);
        assert_eq!(p.names, vec!["A", "B", "G", "R", "Z"]);
        assert_eq!(decode(&p)[4], vec![10.0, 500.0, 2500.0, 42.5]);
    }

    #[test]
    fn test_half_channel_is_f16_quantized() {
        let img = ExrImage {
            width: 1,
            height: 1,
            channels: vec![ExrChannel::half("Y", vec![0.5])],
            compression: ExrCompression::None,
        };
        let bytes = write_to_vec(&img);
        let p = parse(&bytes);
        assert_eq!(p.types, vec![ExrSampleType::Half]);
        assert_eq!(decode(&p)[0], vec![0.5]);
    }

    #[test]
    fn test_validation_rejects_malformed_images() {
        let ok = ExrImage::rgba_f32(1, 1, &[1.0, 1.0, 1.0, 1.0]);
        assert!(ok.validate().is_ok());

        let mut nan = ok.clone();
        nan.channels[0].data[0] = f32::NAN;
        assert!(nan.validate().is_err(), "non-finite must be rejected");

        let mut short = ok.clone();
        short.channels[0].data.clear();
        assert!(short.validate().is_err(), "sample count must be checked");

        let mut dup = ok.clone();
        dup.channels[1].name = "A".to_string();
        assert!(dup.validate().is_err(), "duplicates must be rejected");

        let mut nul = ok.clone();
        nul.channels[0].name = "A\0B".to_string();
        assert!(nul.validate().is_err(), "NUL in name must be rejected");

        let mut empty = ok.clone();
        empty.channels.clear();
        assert!(empty.validate().is_err(), "no channels must be rejected");

        let mut zero = ok.clone();
        zero.height = 0;
        assert!(zero.validate().is_err(), "zero height must be rejected");
    }

    #[test]
    fn test_write_to_file_roundtrip() {
        let (w, h) = (3u32, 3u32);
        let data = ramp(w, h);
        let mut img = ExrImage::rgba_f32(w, h, &data);
        img.compression = ExrCompression::Zip;
        let path =
            std::env::temp_dir().join(format!("kagari_exr_{}_{}.exr", std::process::id(), line!()));
        write_exr_file(&path, &img).expect("file write");
        let bytes = std::fs::read(&path).expect("read back");
        let _ = std::fs::remove_file(&path);
        let p = parse(&bytes);
        assert_eq!(p.names, vec!["A", "B", "G", "R"]);
        assert_eq!(decode(&p)[3][4], data[16], "centre pixel R survives");
    }
}
