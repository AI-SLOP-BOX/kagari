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

#[cfg(feature = "cli")]
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
            #[cfg(feature = "cli")]
            _ => inflate_unpredict(payload),
            #[cfg(not(feature = "cli"))]
            _ => panic!("compressed EXR decoding requires the cli feature"),
        };
        let mut c = 0usize;
        // Scanline-major chunk layout: for each row, each channel's samples.
        for _ in 0..rows {
            for (ci, st) in parsed.types.iter().enumerate() {
                for _ in 0..width {
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
    }
    out
}

#[cfg(feature = "cli")]
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
fn test_pixel_type_codes_match_openexr_spec() {
    // OpenEXR file format spec, "Pixel Types": UINT=0, HALF=1, FLOAT=2.
    // The roundtrip tests parse with these same constants, so a wrong value
    // there is self-consistent — assert against the literal spec numbers.
    assert_eq!(PIXEL_HALF, 1);
    assert_eq!(PIXEL_FLOAT, 2);
    assert_eq!(PIXEL_HALF, ExrSampleType::Half.code());
    assert_eq!(PIXEL_FLOAT, ExrSampleType::Float.code());

    let mut img = ExrImage::rgba_f32(1, 1, &[0.5, 0.25, 0.125, 1.0]);
    img.compression = ExrCompression::None;
    let bytes = write_to_vec(&img);
    let p = parse(&bytes);
    assert_eq!(p.types, vec![ExrSampleType::Float; 4]);
}

#[test]
fn test_header_channels_sorted_and_windows_correct() {
    let mut img = ExrImage::rgba_f32(3, 2, &ramp(3, 2));
    img.compression = ExrCompression::None;
    let bytes = write_to_vec(&img);
    let p = parse(&bytes);
    assert_eq!(p.names, vec!["A", "B", "G", "R"], "must be alphabetical");
    assert_eq!(p.window, (0, 0, 2, 1));
    assert_eq!(p.blocks, 2, "NONE packs 1 row per block");
    assert_eq!(p.compression, ExrCompression::None);
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

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
#[test]
fn test_zip_predictor_is_invertible() {
    let raw: Vec<u8> = (0..257usize).map(|i| (i * 37 % 256) as u8).collect();
    let filtered = zip_reorder_predict(&raw);
    assert_ne!(filtered, raw, "predictor must transform the bytes");
    let back = inflate_unpredict(&zlib_of(&filtered));
    assert_eq!(back, raw, "predictor must be invertible");
}

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
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
