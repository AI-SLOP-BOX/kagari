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
const PIXEL_HALF: i32 = 1; // OpenEXR spec: UINT=0, HALF=1, FLOAT=2
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

    pub(crate) fn validate(&self, expected: usize) -> Result<(), String> {
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

/// Pack one scanline chunk. OpenEXR layout is scanline-major: for each row in
/// the block, every channel's samples for that row, channels in header order.
fn pack_scanline_block(channels: &[&ExrChannel], width: u32, y0: u32, rows: u32) -> Vec<u8> {
    let w = width as usize;
    let mut out = Vec::with_capacity(w * rows as usize * channels.len() * 4);
    for row in 0..rows as usize {
        let base = (y0 as usize + row) * w;
        for ch in channels {
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
#[cfg(feature = "cli")]
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
#[path = "exr_writer/tests.rs"]
mod tests;
