//! OpenEXR reader — lowest-common-denominator import path for Kagari VFX.
//!
//! ## Why not OpenImageIO / OpenColorIO?
//!
//! This workspace does not vendor or bind those libraries, and vendoring a C++ toolset
//! would add native-build and dependency weight we do not need on the production line.
//! Instead we reuse the Rust `image` crate, which already ships complete EXR + TIFF
//! decoders in pure Rust (`image`'s `exr`/`tiff` features are enabled in `Cargo.toml`).
//! That gives us:
//!
//! - Scanline/ZIP compression and HALF/FLOAT samples decode correctly on the host.
//! - No native dependencies, no loader cache, no asset symlinks.
//! - A stable API for both EXR and TIFF import.
//!
//! ## Scope / limitations
//!
//! - `read_exr` consumes non-deep, RGB or RGBA EXR content. EXR containers
//!   with anything else (depth-only layers, Cryptomatte, deep data, tiled TIFFs)
//!   are rejected rather than silently dropping samples.
//! - `read_image_any` additionally accepts the common integer formats that
//!   `image` decodes (PNG / JPEG / TIFF in 8-bit and 16-bit, grayscale or
//!   RGB(A)). Integer samples are normalized to `0.0..=1.0`; grayscale is
//!   expanded to R=G=B. No color-space conversion (e.g. sRGB → linear) is applied.
//! - HALF samples are widened to `f32` on import; every returned channel is
//!   [`ExrSampleType::Float`](crate::core::exr_writer::ExrSampleType::Float).
//! - Channel metadata outside RGB(RGBA) is discarded on import. To preserve arbitrary
//!   channel names, route that file through `exr_writer` separately.
//! - Large multi-gigabyte EXRs are decoded into memory; treat this as an import
//!   utility, not a streaming pipeline component.
//!
//! Non-EXR formats (`.tif` / `.tiff` / `.png` / `.jpg` …) must go through
//! [`read_image_any`], which decodes through `image`'s codec set.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use image::{DynamicImage, ImageBuffer, Rgb, Rgba};

use crate::core::exr_writer::{ExrChannel, ExrCompression, ExrImage};

const EXR_MAGIC_LE: [u8; 4] = *b"\x76\x2F\x31\x01"; // 0x01312F76, little-endian

/// Read an OpenEXR file into Kagari's domain [`ExrImage`](crate::core::exr_writer::ExrImage).
///
/// Only non-deep, RGB or RGBA EXR content is accepted. Anything else
/// (depth-only EXRs, Cryptomatte, missing R/G/B) returns `Err` with a descriptive message.
pub fn read_exr(path: impl AsRef<Path>) -> Result<ExrImage, String> {
    let path = path.as_ref();

    validate_exr_magic(path)?;

    let img =
        image::open(path).map_err(|e| format!("EXR open failed for {}: {e}", path.display()))?;

    dynamic_to_exr_image(img, path)
}

/// Read any image format `image` can decode: EXR, TIFF, PNG, JPEG, …
///
/// Unlike [`read_exr`] this also accepts the ordinary integer formats —
/// 8-bit and 16-bit grayscale / RGB / RGBA (as produced by PNG, JPEG, TIFF…).
/// Integer samples are normalized to `0.0..=1.0` and grayscale is expanded to
/// R=G=B so downstream consumers always receive RGB(A) float planes. Formats
/// outside that set return `Err`; nothing is silently converted or dropped.
/// Useful for the "import images" panel.
pub fn read_image_any(path: impl AsRef<Path>) -> Result<ExrImage, String> {
    let path = path.as_ref();

    let img = image::open(path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;

    dynamic_to_exr_image(img, path)
}

/// Shared tail of `read_exr` / `read_image_any`: de-interleave the decoded
/// buffer into per-channel planes (`ExrChannel` holds one sample per pixel).
/// Float buffers are used as-is; integer buffers are normalized to `0.0..=1.0`
/// and grayscale is expanded to R=G=B.
fn dynamic_to_exr_image(img: DynamicImage, path: &Path) -> Result<ExrImage, String> {
    let width = img.width();
    let height = img.height();
    let n = (width as usize) * (height as usize);

    let channels = match &img {
        DynamicImage::ImageRgb32F(buf) => rgb_channels(buf),
        DynamicImage::ImageRgba32F(buf) => rgba_channels(buf),
        DynamicImage::ImageLuma8(buf) => planes_from_u8(buf.as_raw(), 1, n),
        DynamicImage::ImageLumaA8(buf) => planes_from_u8(buf.as_raw(), 2, n),
        DynamicImage::ImageRgb8(buf) => planes_from_u8(buf.as_raw(), 3, n),
        DynamicImage::ImageRgba8(buf) => planes_from_u8(buf.as_raw(), 4, n),
        DynamicImage::ImageLuma16(buf) => planes_from_u16(buf.as_raw(), 1, n),
        DynamicImage::ImageLumaA16(buf) => planes_from_u16(buf.as_raw(), 2, n),
        DynamicImage::ImageRgb16(buf) => planes_from_u16(buf.as_raw(), 3, n),
        DynamicImage::ImageRgba16(buf) => planes_from_u16(buf.as_raw(), 4, n),
        other => {
            return Err(format!(
                "image {} has unsupported pixel format for import: {:?}",
                path.display(),
                other.color()
            ))
        }
    };

    Ok(ExrImage {
        width,
        height,
        channels,
        compression: ExrCompression::None,
    })
}

fn validate_exr_magic(path: &Path) -> Result<(), String> {
    let mut file =
        File::open(path).map_err(|e| format!("cannot open EXR {}: {e}", path.display()))?;

    let mut header = [0u8; 4];
    file.read_exact(&mut header)
        .map_err(|e| format!("cannot read EXR header {}: {e}", path.display()))?;

    if header != EXR_MAGIC_LE {
        return Err(format!(
            "{} is not an OpenEXR file (bad magic bytes 0x{:08X})",
            path.display(),
            u32::from_le_bytes(header) & 0x00FFFFFF
        ));
    }

    Ok(())
}

/// De-interleave an RGBA f32 buffer into per-channel planes, listed in
/// alphabetical order (`A`, `B`, `G`, `R`) like an OpenEXR channel list.
fn rgba_channels(img: &ImageBuffer<Rgba<f32>, Vec<f32>>) -> Vec<ExrChannel> {
    let n = (img.width() as usize) * (img.height() as usize);
    let mut a = Vec::with_capacity(n);
    let mut b = Vec::with_capacity(n);
    let mut g = Vec::with_capacity(n);
    let mut r = Vec::with_capacity(n);
    for px in img.pixels() {
        r.push(px.0[0]);
        g.push(px.0[1]);
        b.push(px.0[2]);
        a.push(px.0[3]);
    }
    vec![
        ExrChannel::new("A", a),
        ExrChannel::new("B", b),
        ExrChannel::new("G", g),
        ExrChannel::new("R", r),
    ]
}

/// De-interleave an RGB f32 buffer into per-channel planes, listed in
/// alphabetical order (`B`, `G`, `R`) like an OpenEXR channel list.
fn rgb_channels(img: &ImageBuffer<Rgb<f32>, Vec<f32>>) -> Vec<ExrChannel> {
    let n = (img.width() as usize) * (img.height() as usize);
    let mut b = Vec::with_capacity(n);
    let mut g = Vec::with_capacity(n);
    let mut r = Vec::with_capacity(n);
    for px in img.pixels() {
        r.push(px.0[0]);
        g.push(px.0[1]);
        b.push(px.0[2]);
    }
    vec![
        ExrChannel::new("B", b),
        ExrChannel::new("G", g),
        ExrChannel::new("R", r),
    ]
}

/// Pack interleaved 8-bit samples (1=gray, 2=gray+alpha, 3=RGB, 4=RGBA) into
/// normalized `f32` planes. Grayscale is expanded to R=G=B.
fn planes_from_u8(raw: &[u8], components: usize, n: usize) -> Vec<ExrChannel> {
    debug_assert_eq!(raw.len(), n * components);
    let scale = 1.0 / 255.0;
    let mut r = Vec::with_capacity(n);
    let mut g = Vec::with_capacity(n);
    let mut b = Vec::with_capacity(n);
    let mut a = Vec::with_capacity(if components >= 2 { n } else { 0 });
    for px in raw.chunks_exact(components) {
        match components {
            1 => {
                let luma = px[0] as f32 * scale;
                r.push(luma);
                g.push(luma);
                b.push(luma);
            }
            2 => {
                let luma = px[0] as f32 * scale;
                r.push(luma);
                g.push(luma);
                b.push(luma);
                a.push(px[1] as f32 * scale);
            }
            3 => {
                r.push(px[0] as f32 * scale);
                g.push(px[1] as f32 * scale);
                b.push(px[2] as f32 * scale);
            }
            4 => {
                r.push(px[0] as f32 * scale);
                g.push(px[1] as f32 * scale);
                b.push(px[2] as f32 * scale);
                a.push(px[3] as f32 * scale);
            }
            _ => unreachable!("planes_from_u8 only handles 1..=4 components"),
        }
    }
    assemble_planes(r, g, b, a, components >= 2)
}

/// Same as [`planes_from_u8`] for 16-bit samples (normalized by 65535).
fn planes_from_u16(raw: &[u16], components: usize, n: usize) -> Vec<ExrChannel> {
    debug_assert_eq!(raw.len(), n * components);
    let scale = 1.0 / 65_535.0;
    let mut r = Vec::with_capacity(n);
    let mut g = Vec::with_capacity(n);
    let mut b = Vec::with_capacity(n);
    let mut a = Vec::with_capacity(if components >= 2 { n } else { 0 });
    for px in raw.chunks_exact(components) {
        match components {
            1 => {
                let luma = px[0] as f32 * scale;
                r.push(luma);
                g.push(luma);
                b.push(luma);
            }
            2 => {
                let luma = px[0] as f32 * scale;
                r.push(luma);
                g.push(luma);
                b.push(luma);
                a.push(px[1] as f32 * scale);
            }
            3 => {
                r.push(px[0] as f32 * scale);
                g.push(px[1] as f32 * scale);
                b.push(px[2] as f32 * scale);
            }
            4 => {
                r.push(px[0] as f32 * scale);
                g.push(px[1] as f32 * scale);
                b.push(px[2] as f32 * scale);
                a.push(px[3] as f32 * scale);
            }
            _ => unreachable!("planes_from_u16 only handles 1..=4 components"),
        }
    }
    assemble_planes(r, g, b, a, components >= 2)
}

/// Order the planes alphabetically like an OpenEXR channel list, dropping the
/// alpha plane when the source had none.
fn assemble_planes(r: Vec<f32>, g: Vec<f32>, b: Vec<f32>, a: Vec<f32>, has_alpha: bool) -> Vec<ExrChannel> {
    if has_alpha {
        vec![
            ExrChannel::new("A", a),
            ExrChannel::new("B", b),
            ExrChannel::new("G", g),
            ExrChannel::new("R", r),
        ]
    } else {
        vec![
            ExrChannel::new("B", b),
            ExrChannel::new("G", g),
            ExrChannel::new("R", r),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interleaved_rgba(width: u32, height: u32) -> Vec<f32> {
        let mut rgba = vec![0.0f32; (width as usize) * (height as usize) * 4];
        for (i, v) in rgba.iter_mut().enumerate() {
            *v = (i % 4) as f32;
        }
        rgba
    }

    /// Split interleaved RGBA into alphabetical planes `A,B,G,R` for comparison.
    fn expected_planes(rgba: &[f32]) -> Vec<Vec<f32>> {
        let n = rgba.len() / 4;
        let mut a = Vec::with_capacity(n);
        let mut b = Vec::with_capacity(n);
        let mut g = Vec::with_capacity(n);
        let mut r = Vec::with_capacity(n);
        for chunk in rgba.as_chunks::<4>().0 {
            r.push(chunk[0]);
            g.push(chunk[1]);
            b.push(chunk[2]);
            a.push(chunk[3]);
        }
        vec![a, b, g, r]
    }

    #[test]
    fn roundtrip_rgba() {
        let width = 3;
        let height = 2;
        let n = (width * height) as usize;
        let rgba = interleaved_rgba(width, height);
        let file = std::env::temp_dir().join(format!(
            "kagari_exr_roundtrip_rgba_{}.exr",
            std::process::id()
        ));

        let image = ExrImage::rgba_f32(width, height, &rgba);
        crate::core::exr_writer::write_exr_file(&file, &image).expect("write exr");

        let loaded = read_exr(&file).expect("read");
        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels.len(), 4);
        let names: Vec<&str> = loaded.channels.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["A", "B", "G", "R"]);
        for (ch, plane) in loaded.channels.iter().zip(expected_planes(&rgba)) {
            ch.validate(n).expect("validate");
            assert_eq!(ch.data, plane);
        }
        std::fs::remove_file(&file).ok();
    }

    #[test]
    fn roundtrip_rgb() {
        let width = 2;
        let height = 2;
        let n = (width * height) as usize;
        let rgba = interleaved_rgba(width, height);
        let file = std::env::temp_dir().join(format!(
            "kagari_exr_roundtrip_rgb_{}.exr",
            std::process::id()
        ));

        // Build a genuine 3-channel image (no alpha) with per-channel planes.
        let mut b = Vec::with_capacity(n);
        let mut g = Vec::with_capacity(n);
        let mut r = Vec::with_capacity(n);
        for chunk in rgba.as_chunks::<4>().0 {
            r.push(chunk[0]);
            g.push(chunk[1]);
            b.push(chunk[2]);
        }
        let image = ExrImage {
            channels: vec![
                ExrChannel::new("R", r.clone()),
                ExrChannel::new("G", g.clone()),
                ExrChannel::new("B", b.clone()),
            ],
            width,
            height,
            compression: ExrCompression::None,
        };
        crate::core::exr_writer::write_exr_file(&file, &image).expect("write exr");

        let loaded = read_exr(&file).expect("read");
        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels.len(), 3);
        let names: Vec<&str> = loaded.channels.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["B", "G", "R"]);
        for (ch, plane) in loaded.channels.iter().zip([b, g, r]) {
            ch.validate(n).expect("validate");
            assert_eq!(ch.data, plane);
        }
        std::fs::remove_file(&file).ok();
    }

    #[test]
    fn errors_on_non_exr() {
        let tmp =
            std::env::temp_dir().join(format!("kagari_exr_not_exr_{}.txt", std::process::id()));
        std::fs::write(&tmp, b"hello world").unwrap();
        let err = read_exr(&tmp).unwrap_err();
        assert!(err.contains("not an OpenEXR file"));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn errors_on_missing() {
        let err = read_exr("/nonexistent/path/for_test_exr_12345.exr").unwrap_err();
        assert!(err.contains("cannot open EXR"));
    }

    #[test]
    fn read_image_any_normalizes_8bit_rgba_png() {
        let file = std::env::temp_dir().join(format!(
            "kagari_read_any_rgba8_{}.png",
            std::process::id()
        ));
        let pixels = vec![
            255, 0, 128, 64, // px 0
            0, 255, 64, 128, // px 1
            10, 20, 30, 40, // px 2
            200, 100, 50, 255, // px 3
        ];
        let img = image::RgbaImage::from_raw(2, 2, pixels).expect("raw rgba");
        image::DynamicImage::ImageRgba8(img)
            .save(&file)
            .expect("save 8-bit png");

        let loaded = read_image_any(&file).expect("8-bit RGBA PNG must import");
        assert_eq!((loaded.width, loaded.height), (2, 2));
        let names: Vec<&str> = loaded.channels.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["A", "B", "G", "R"]);
        let by_name = |name: &str| {
            loaded
                .channels
                .iter()
                .find(|c| c.name == name)
                .unwrap_or_else(|| panic!("missing channel {name}"))
                .data
                .clone()
        };
        assert_eq!(by_name("R"), [255.0, 0.0, 10.0, 200.0].map(|v: f32| v / 255.0));
        assert_eq!(by_name("G"), [0.0, 255.0, 20.0, 100.0].map(|v: f32| v / 255.0));
        assert_eq!(by_name("B"), [128.0, 64.0, 30.0, 50.0].map(|v: f32| v / 255.0));
        assert_eq!(by_name("A"), [64.0, 128.0, 40.0, 255.0].map(|v: f32| v / 255.0));
        std::fs::remove_file(&file).ok();
    }

    #[test]
    fn read_image_any_expands_8bit_grayscale_to_rgb() {
        let file = std::env::temp_dir().join(format!(
            "kagari_read_any_gray8_{}.png",
            std::process::id()
        ));
        let img = image::GrayImage::from_raw(2, 1, vec![0, 255]).expect("raw gray");
        image::DynamicImage::ImageLuma8(img)
            .save(&file)
            .expect("save 8-bit gray png");

        let loaded = read_image_any(&file).expect("8-bit grayscale PNG must import");
        assert_eq!((loaded.width, loaded.height), (2, 1));
        let names: Vec<&str> = loaded.channels.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["B", "G", "R"]);
        for channel in &loaded.channels {
            assert_eq!(channel.data, vec![0.0, 1.0], "channel {}", channel.name);
        }
        std::fs::remove_file(&file).ok();
    }

    #[test]
    fn read_image_any_reads_16bit_png() {
        let file = std::env::temp_dir().join(format!(
            "kagari_read_any_gray16_{}.png",
            std::process::id()
        ));
        let img = image::ImageBuffer::<image::Luma<u16>, Vec<u16>>::from_raw(3, 1, vec![
            0, 32_768, 65_535,
        ])
        .expect("raw gray16");
        image::DynamicImage::ImageLuma16(img)
            .save(&file)
            .expect("save 16-bit png");

        let loaded = read_image_any(&file).expect("16-bit PNG must import");
        let r = &loaded
            .channels
            .iter()
            .find(|c| c.name == "R")
            .expect("R channel")
            .data;
        assert_eq!(r.len(), 3);
        assert_eq!(r[0], 0.0);
        assert_eq!(r[1], 32_768.0f32 / 65_535.0);
        assert_eq!(r[2], 1.0);
        std::fs::remove_file(&file).ok();
    }

    #[test]
    fn read_image_any_rejects_non_image_files() {
        let tmp = std::env::temp_dir().join(format!(
            "kagari_read_any_not_image_{}.txt",
            std::process::id()
        ));
        std::fs::write(&tmp, b"this is not an image").unwrap();
        let err = read_image_any(&tmp).unwrap_err();
        assert!(err.contains("failed to open"));
        std::fs::remove_file(&tmp).ok();
    }
}
