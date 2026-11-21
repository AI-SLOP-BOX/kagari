//! Psyop / Foundry Cryptomatte ID-Matte Extraction (Nuke / Blender / Houdini / Maya AOV parity).
//!
//! Implements the open Cryptomatte specification:
//! - MurmurHash3-32 name hashing -> IEEE-754 `f32` ID encoding (exponent-clamped per spec)
//! - Multi-rank `(id, coverage)` pair accumulation across `Crypto00`, `Crypto01`, `Crypto02`, ...
//! - Anti-aliased fractional matte extraction for any selected object/material/asset list.

use std::collections::HashMap;

/// MurmurHash3-32 + Cryptomatte `f32` encoding per the open Cryptomatte specification.
pub fn cryptomatte_id_float(name: &str) -> f32 {
    let h = murmur3_32(name.as_bytes(), 0);
    // Exponent manipulation per Cryptomatte spec so the hash is always a finite non-denorm f32
    let exp = (h >> 23) & 0xFF;
    let mantissa = h & 0x7F_FFFF;
    let adjusted = if exp == 0 || exp == 0xFF {
        ((exp ^ 0x55) << 23) | mantissa | (h & 0x8000_0000)
    } else {
        h
    };
    f32::from_bits(adjusted)
}

fn murmur3_32(data: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xCC9E_2D51;
    const C2: u32 = 0x1B87_3593;
    let mut h = seed;
    let chunks = data.chunks_exact(4);
    let rem = chunks.remainder();

    for chunk in chunks {
        let mut k = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        k = k.wrapping_mul(C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(C2);
        h ^= k;
        h = h.rotate_left(13);
        h = h.wrapping_mul(5).wrapping_add(0xE654_6B64);
    }

    let mut k1 = 0u32;
    for (i, &b) in rem.iter().enumerate() {
        k1 |= (b as u32) << (i * 8);
    }
    if !rem.is_empty() {
        k1 = k1.wrapping_mul(C1);
        k1 = k1.rotate_left(15);
        k1 = k1.wrapping_mul(C2);
        h ^= k1;
    }

    h ^= data.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^ (h >> 16)
}

/// Parses a Cryptomatte JSON manifest (`{ "object_name": "deadbeef", ... }`) into `name -> f32_id`.
pub fn parse_cryptomatte_manifest(manifest_json: &str) -> Result<HashMap<String, f32>, String> {
    let map: HashMap<String, String> =
        serde_json::from_str(manifest_json).map_err(|e| format!("invalid manifest JSON: {e}"))?;
    let mut out = HashMap::with_capacity(map.len());
    for (name, hex_str) in map {
        if let Ok(bits) = u32::from_str_radix(hex_str.trim_start_matches("0x"), 16) {
            out.insert(name, f32::from_bits(bits));
        } else {
            out.insert(name.clone(), cryptomatte_id_float(&name));
        }
    }
    Ok(out)
}

/// Extracts an anti-aliased `[0..1]` matte for `selected_ids` across one or more
/// `CryptoXX` RGBA32F rank layers (each containing `(id0, cov0, id1, cov1)` per pixel).
pub fn extract_cryptomatte_matte(
    rank_layers_rgba: &[&[f32]],
    width: u32,
    height: u32,
    selected_ids: &[f32],
) -> Option<Vec<f32>> {
    let count = (width as usize).checked_mul(height as usize)?;
    if count == 0 || rank_layers_rgba.is_empty() {
        return None;
    }
    for &layer in rank_layers_rgba {
        if layer.len() != count * 4 {
            return None;
        }
    }

    let selected_bits: Vec<u32> = selected_ids.iter().map(|id| id.to_bits()).collect();
    let mut matte = vec![0.0_f32; count];

    for idx in 0..count {
        let mut cov_sum = 0.0_f32;
        for &layer in rank_layers_rgba {
            let base = idx * 4;
            for pair in 0..2 {
                let id_bits = layer[base + pair * 2].to_bits();
                let cov = layer[base + pair * 2 + 1];
                if cov > 0.0 && selected_bits.contains(&id_bits) {
                    cov_sum += cov;
                }
            }
        }
        matte[idx] = cov_sum.clamp(0.0, 1.0);
    }

    Some(matte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cryptomatte_extracts_antialiased_coverage_across_ranks() {
        let hero_id = cryptomatte_id_float("HeroCar");
        let bg_id = cryptomatte_id_float("Road");
        let rim_id = cryptomatte_id_float("Wheel");

        assert_ne!(hero_id.to_bits(), bg_id.to_bits());
        assert!(hero_id.is_finite());

        // 2x1 pixels:
        // Pixel 0: Rank0 has (HeroCar 0.6, Road 0.4), Rank1 has (Wheel 0.0, 0.0)
        // Pixel 1: Rank0 has (Road 0.75, Wheel 0.25), Rank1 has (HeroCar 0.15, 0.0)
        let rank0 = [
            hero_id, 0.6, bg_id, 0.4,
            bg_id, 0.75, rim_id, 0.25,
        ];
        let rank1 = [
            0.0, 0.0, 0.0, 0.0,
            hero_id, 0.15, 0.0, 0.0,
        ];

        let matte = extract_cryptomatte_matte(&[&rank0, &rank1], 2, 1, &[hero_id, rim_id])
            .expect("matte");

        assert!((matte[0] - 0.6).abs() < 1e-5);
        assert!((matte[1] - 0.40).abs() < 1e-5); // 0.25 (Wheel) + 0.15 (HeroCar)
    }
}
