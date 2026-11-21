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

#[cfg(feature = "exr-compression")]
use flate2::{write::ZlibEncoder, Compression};

const EXR_MAGIC: u32 = 0x0131_2f76;
const EXR_VERSION: u32 = 2;
const PIXEL_HALF: i32 = 1; // OpenEXR spec: UINT=0, HALF=1, FLOAT=2
const PIXEL_FLOAT: i32 = 2;
const COMP_NONE: u8 = 0;
const COMP_ZIPS: u8 = 2;
const COMP_ZIP: u8 = 3;
const LINE_INCREASING_Y: u8 = 0;
static TEMP_FILE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn create_temporary_exr(
    path: &std::path::Path,
) -> Result<(std::path::PathBuf, std::fs::File), String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| "EXR output path must include a filename".to_owned())?
        .to_string_lossy();
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));

    for _ in 0..100 {
        let temporary = parent.join(format!(
            ".{file_name}.kagari-tmp-{}-{}",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&temporary) {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("failed to create temporary EXR: {error}")),
        }
    }
    Err("could not reserve a unique temporary EXR after 100 attempts".to_owned())
}

fn restore_exr_destination_permissions(
    temporary: &std::fs::File,
    destination: &std::path::Path,
) -> Result<(), String> {
    #[cfg(unix)]
    match std::fs::metadata(destination) {
        Ok(metadata) => temporary
            .set_permissions(metadata.permissions())
            .map_err(|error| format!("failed to preserve EXR output permissions: {error}")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to inspect EXR output permissions: {error}")),
    }
    #[cfg(not(unix))]
    {
        let _ = (temporary, destination);
        Ok(())
    }
}

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
        if self.data.len() != expected {
            return Err(format!(
                "EXR channel {:?} has {} samples, expected {}",
                self.name,
                self.data.len(),
                expected
            ));
        }
        if self.data.iter().any(|v| !v.is_finite()) {
            return Err(format!("EXR channel {:?} holds non-finite data", self.name));
        }
        if self.sample_type == ExrSampleType::Half
            && self
                .data
                .iter()
                .any(|value| !half::f16::from_f32(*value).is_finite())
        {
            return Err(format!(
                "EXR HALF channel {:?} contains values outside the finite HALF range",
                self.name
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
    pub fn rgba_f32(width: u32, height: u32, rgba: &[f32]) -> Result<Self, String> {
        if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err("EXR dimensions must be within the signed 32-bit data-window range".into());
        }
        let pixels = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| "EXR dimensions overflow this platform".to_owned())?;
        let expected_values = pixels
            .checked_mul(4)
            .ok_or_else(|| "EXR RGBA sample count overflows this platform".to_owned())?;
        if rgba.len() != expected_values {
            return Err(format!(
                "EXR RGBA data has {} samples, expected {expected_values} for {width}x{height}",
                rgba.len()
            ));
        }

        let mut r = Vec::new();
        let mut g = Vec::new();
        let mut b = Vec::new();
        let mut a = Vec::new();
        for channel in [&mut r, &mut g, &mut b, &mut a] {
            channel
                .try_reserve_exact(pixels)
                .map_err(|error| format!("Could not allocate EXR channel data: {error}"))?;
        }
        for chunk in rgba.as_chunks::<4>().0 {
            r.push(chunk[0]);
            g.push(chunk[1]);
            b.push(chunk[2]);
            a.push(chunk[3]);
        }

        Ok(Self {
            width,
            height,
            channels: vec![
                ExrChannel::new("A", a),
                ExrChannel::new("B", b),
                ExrChannel::new("G", g),
                ExrChannel::new("R", r),
            ],
            compression: ExrCompression::Zip,
        })
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
        if self.width > i32::MAX as u32 || self.height > i32::MAX as u32 {
            return Err("EXR dimensions exceed the signed 32-bit data-window range".to_string());
        }
        if self.channels.is_empty() {
            return Err("EXR image needs at least one channel".to_string());
        }
        let expected = (self.width as usize)
            .checked_mul(self.height as usize)
            .ok_or_else(|| "EXR dimensions overflow this platform".to_owned())?;
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
    let bytes_per_pixel = channels.iter().try_fold(0usize, |total, channel| {
        total.checked_add(match channel.sample_type {
            ExrSampleType::Half => 2usize,
            ExrSampleType::Float => 4usize,
        })
    });
    let max_block_bytes = bytes_per_pixel
        .and_then(|bytes| bytes.checked_mul(width as usize))
        .and_then(|bytes| bytes.checked_mul(spb.min(height) as usize))
        .ok_or_else(|| "EXR scanline block size overflows this platform".to_owned())?;
    if max_block_bytes > i32::MAX as usize {
        return Err(format!(
            "EXR scanline block is too large for the file format ({max_block_bytes} bytes; maximum {})",
            i32::MAX
        ));
    }
    let table_bytes = (blocks as usize)
        .checked_mul(8)
        .ok_or_else(|| "EXR chunk offset table size overflows this platform".to_owned())?;

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
    const ZEROES: [u8; 4096] = [0; 4096];
    let mut remaining_table_bytes = table_bytes;
    while remaining_table_bytes > 0 {
        let count = remaining_table_bytes.min(ZEROES.len());
        sink.write_all(&ZEROES[..count])
            .map_err(|e| e.to_string())?;
        remaining_table_bytes -= count;
    }

    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(blocks as usize)
        .map_err(|error| format!("could not allocate EXR chunk offsets: {error}"))?;
    for block in 0..blocks {
        let y0 = block * spb;
        let rows = spb.min(height - y0);
        let raw = pack_scanline_block(&channels, width, y0, rows)?;
        let payload = match image.compression {
            ExrCompression::None => raw,
            ExrCompression::Zips | ExrCompression::Zip => zip_deflate(&raw)?,
        };
        let payload_size = i32::try_from(payload.len()).map_err(|_| {
            "EXR compressed scanline block exceeds the signed 32-bit size limit".to_owned()
        })?;
        offsets.push(sink.stream_position().map_err(|e| e.to_string())?);
        sink.write_all(&(y0 as i32).to_le_bytes())
            .map_err(|e| e.to_string())?;
        sink.write_all(&payload_size.to_le_bytes())
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
    let path = path.as_ref();
    image.validate()?;
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let (temporary, file) = create_temporary_exr(path)?;

    let mut owns_temporary = true;
    let result = (|| {
        let mut writer = io::BufWriter::new(file);
        write_exr(&mut writer, image)?;
        writer
            .flush()
            .map_err(|error| format!("failed to flush temporary EXR: {error}"))?;
        restore_exr_destination_permissions(writer.get_ref(), path)?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|error| format!("failed to sync temporary EXR: {error}"))?;
        drop(writer);
        crate::core::atomic_file::replace_file(&temporary, path)
            .map_err(|error| format!("failed to atomically replace EXR output: {error}"))?;
        owns_temporary = false;
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();

    match result {
        Ok(identity) => Ok(identity),
        Err(error) if owns_temporary => match std::fs::remove_file(&temporary) {
            Ok(()) => Err(error),
            Err(cleanup_error) if cleanup_error.kind() == io::ErrorKind::NotFound => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}; failed to remove temporary EXR {}: {cleanup_error}",
                temporary.display()
            )),
        },
        Err(error) => Err(error),
    }
}

/// Write an EXR file only if the target does not already exist.
pub fn write_exr_file_new(
    path: impl AsRef<std::path::Path>,
    image: &ExrImage,
) -> Result<(), String> {
    write_exr_file_new_with_identity(path, image).map(|_| ())
}

/// Write a new EXR file and return the identity of the published file.
pub fn write_exr_file_new_with_identity(
    path: impl AsRef<std::path::Path>,
    image: &ExrImage,
) -> Result<crate::core::ffmpeg_export::FileIdentity, String> {
    let path = path.as_ref();
    image.validate()?;
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let (temporary, file) = create_temporary_exr(path)?;

    let mut owns_temporary = true;
    let result = (|| {
        let mut writer = io::BufWriter::new(file);
        write_exr(&mut writer, image)?;
        writer
            .flush()
            .map_err(|error| format!("failed to flush temporary EXR: {error}"))?;
        restore_exr_destination_permissions(writer.get_ref(), path)?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|error| format!("failed to sync temporary EXR: {error}"))?;
        drop(writer);

        let identity = crate::core::ffmpeg_export::publish_file_no_replace(&temporary, path)
            .map_err(|error| format!("failed to create EXR output without overwriting: {error}"))?;
        if let Err(error) = std::fs::remove_file(&temporary) {
            if error.kind() != io::ErrorKind::NotFound {
                log::warn!(
                    "EXR output was published at {}, but its staging link {} could not be removed: {error}",
                    path.display(),
                    temporary.display()
                );
            }
        }
        owns_temporary = false;
        if let Ok(directory) = std::fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(identity)
    })();

    match result {
        Ok(identity) => Ok(identity),
        Err(error) if owns_temporary => match std::fs::remove_file(&temporary) {
            Ok(()) => Err(error),
            Err(cleanup_error) if cleanup_error.kind() == io::ErrorKind::NotFound => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}; failed to remove temporary EXR {}: {cleanup_error}",
                temporary.display()
            )),
        },
        Err(error) => Err(error),
    }
}

fn write_attribute<W: Write>(
    sink: &mut W,
    name: &str,
    type_name: &str,
    value: &[u8],
) -> Result<(), String> {
    let value_len = i32::try_from(value.len())
        .map_err(|_| format!("EXR attribute {name:?} exceeds the signed 32-bit size limit"))?;
    sink.write_all(name.as_bytes()).map_err(|e| e.to_string())?;
    sink.write_all(&[0u8]).map_err(|e| e.to_string())?;
    sink.write_all(type_name.as_bytes())
        .map_err(|e| e.to_string())?;
    sink.write_all(&[0u8]).map_err(|e| e.to_string())?;
    sink.write_all(&value_len.to_le_bytes())
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
fn pack_scanline_block(
    channels: &[&ExrChannel],
    width: u32,
    y0: u32,
    rows: u32,
) -> Result<Vec<u8>, String> {
    let w = width as usize;
    let bytes_per_pixel = channels.iter().try_fold(0usize, |total, channel| {
        total.checked_add(match channel.sample_type {
            ExrSampleType::Half => 2usize,
            ExrSampleType::Float => 4usize,
        })
    });
    let capacity = bytes_per_pixel
        .and_then(|bytes| bytes.checked_mul(w))
        .and_then(|bytes| bytes.checked_mul(rows as usize))
        .ok_or_else(|| "EXR scanline block size overflows this platform".to_owned())?;
    let mut out = Vec::new();
    out.try_reserve_exact(capacity)
        .map_err(|error| format!("could not allocate EXR scanline block: {error}"))?;
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
    Ok(out)
}

/// Apply the OpenEXR ZIP filter and deflate into a zlib stream.
fn zip_deflate(raw: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(feature = "exr-compression")]
    {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&zip_reorder_predict(raw))
            .map_err(|e| e.to_string())?;
        enc.finish().map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "exr-compression"))]
    {
        let _ = raw;
        Err("EXR ZIP compression requires the exr-compression feature".to_string())
    }
}

/// OpenEXR ZIP filter: de-interleave even/odd bytes, then apply the delta predictor.
#[cfg(feature = "exr-compression")]
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
