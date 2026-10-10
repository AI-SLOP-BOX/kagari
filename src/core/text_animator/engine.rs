use super::types::{CharacterTransform, RangeSelector, SelectorShape};

pub struct TextAnimatorEngine;

impl TextAnimatorEngine {
    /// Deterministic pseudo-random value in [0, 1) for a character index.
    fn hash_random(idx: usize, total_chars: usize) -> f32 {
        let mut h = (idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= total_chars as u64;
        h ^= h >> 33;
        h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
        h ^= h >> 33;
        ((h >> 11) as f32 / (1u64 << 53) as f32).fract()
    }

    /// Remap a character index for `random_order` selection (deterministic shuffle).
    fn random_order_index(idx: usize, total_chars: usize) -> usize {
        // Multiplicative permutation: bijective when modulus is coprime with multiplier.
        ((idx as u64 + 1).wrapping_mul(2654435761) % total_chars as u64) as usize
    }

    fn in_char_range(char_idx: usize, selector: &RangeSelector) -> bool {
        let i = char_idx as i32;
        if selector.char_start >= 0 && i < selector.char_start {
            return false;
        }
        if selector.char_end >= 0 && i > selector.char_end {
            return false;
        }
        true
    }

    /// Evaluate a per-character Rhai expression for the expression selector.
    fn eval_expression_amount(
        expr_src: &str,
        char_idx: usize,
        total_chars: usize,
        time: f32,
    ) -> f32 {
        let v = crate::core::expression_engine::eval_expression_f64(
            expr_src,
            &[
                ("index", char_idx as f64),
                ("textIndex", (char_idx + 1) as f64), // AE standard: 1-based character index
                ("textTotal", total_chars as f64),    // AE standard: total character count
                ("total", total_chars as f64),
                ("time", time as f64),
                ("value", 1.0),
            ],
        );
        (v as f32).clamp(0.0, 1.0)
    }

    pub fn compute_amount(
        char_idx: usize,
        total_chars: usize,
        selector: &RangeSelector,
        time: f32,
    ) -> f32 {
        if total_chars == 0 || !Self::in_char_range(char_idx, selector) {
            return 0.0;
        }

        // Expression selector: evaluate per-character Rhai expression.
        if selector.shape == SelectorShape::Expression {
            if let Some(ref expr_src) = selector.expression {
                return Self::eval_expression_amount(expr_src, char_idx, total_chars, time);
            }
            return 0.0;
        }

        // For Wobble/Random shapes the amount is index-driven, not range-driven.
        match selector.shape {
            SelectorShape::Wobble => {
                return (char_idx as f32 * 1.7 + std::f32::consts::PI * 0.5).sin() * 0.5 + 0.5;
            }
            SelectorShape::Random => {
                return Self::hash_random(
                    if selector.random_order {
                        Self::random_order_index(char_idx, total_chars)
                    } else {
                        char_idx
                    },
                    total_chars,
                );
            }
            _ => {}
        }

        let sel_idx = if selector.random_order {
            Self::random_order_index(char_idx, total_chars)
        } else {
            char_idx
        };

        let char_pct = (sel_idx as f32 / total_chars as f32) * 100.0;
        let effective_start = (selector.start + selector.offset).clamp(0.0, 100.0);
        let effective_end = (selector.end + selector.offset).clamp(0.0, 100.0);

        if effective_start >= effective_end {
            return 0.0;
        }

        if char_pct < effective_start || char_pct > effective_end {
            return 0.0;
        }

        let norm_t = (char_pct - effective_start) / (effective_end - effective_start);

        match selector.shape {
            SelectorShape::Square => 1.0,
            SelectorShape::RampUp => norm_t,
            SelectorShape::RampDown => 1.0 - norm_t,
            SelectorShape::Triangle => {
                if norm_t < 0.5 {
                    norm_t * 2.0
                } else {
                    (1.0 - norm_t) * 2.0
                }
            }
            SelectorShape::Round => (norm_t * std::f32::consts::PI).sin(),
            SelectorShape::Smooth => norm_t * norm_t * (3.0 - 2.0 * norm_t),
            SelectorShape::Expression => unreachable!("handled above"),
            SelectorShape::Wobble | SelectorShape::Random => 1.0,
        }
    }

    /// Evaluate per-character transforms for a text string given target animator property offsets.
    #[allow(clippy::too_many_arguments)]
    pub fn eval_character_transforms(
        text: &str,
        selector: &RangeSelector,
        target_position: [f32; 2],
        target_scale: [f32; 2],
        target_opacity: f32,
        target_tracking: f32,
        target_rotation: f32,
        time: f32,
    ) -> Vec<CharacterTransform> {
        Self::eval_character_transforms_extended(
            text,
            selector,
            target_position,
            target_scale,
            target_opacity,
            target_tracking,
            target_rotation,
            0.0,
            false,
            time,
        )
    }

    /// Extended evaluation with per-character blur amount and optional cumulative
    /// (AE-style) tracking, where each character's tracking offset includes the
    /// accumulated offsets of all preceding characters in the selection.
    #[allow(clippy::too_many_arguments)]
    pub fn eval_character_transforms_extended(
        text: &str,
        selector: &RangeSelector,
        target_position: [f32; 2],
        target_scale: [f32; 2],
        target_opacity: f32,
        target_tracking: f32,
        target_rotation: f32,
        target_blur: f32,
        cumulative_tracking: bool,
        time: f32,
    ) -> Vec<CharacterTransform> {
        let total_chars = text.chars().count();
        let amounts: Vec<f32> = (0..total_chars)
            .map(|idx| Self::compute_amount(idx, total_chars, selector, time))
            .collect();
        Self::eval_with_amounts(
            amounts,
            target_position,
            target_scale,
            target_opacity,
            target_tracking,
            target_rotation,
            target_blur,
            cumulative_tracking,
        )
    }

    /// Core interpolation from precomputed per-character selector amounts
    /// (each in 0..1) to transforms. Public so unit-based selectors
    /// (words/lines) can drive the same math.
    #[allow(clippy::too_many_arguments)]
    pub fn eval_with_amounts(
        amounts: Vec<f32>,
        target_position: [f32; 2],
        target_scale: [f32; 2],
        target_opacity: f32,
        target_tracking: f32,
        target_rotation: f32,
        target_blur: f32,
        cumulative_tracking: bool,
    ) -> Vec<CharacterTransform> {
        let total_chars = amounts.len();
        let mut transforms = Vec::with_capacity(total_chars);
        let mut tracking_accum = 0.0;

        for amount in amounts.iter().copied() {
            let pos_x = target_position[0] * amount;
            let pos_y = target_position[1] * amount;
            let scale_x = 1.0 + (target_scale[0] - 1.0) * amount;
            let scale_y = 1.0 + (target_scale[1] - 1.0) * amount;
            let opa = 1.0 - (1.0 - target_opacity) * amount;
            let track = target_tracking * amount
                + if cumulative_tracking {
                    tracking_accum
                } else {
                    0.0
                };
            tracking_accum += target_tracking * amount;
            let rot = target_rotation * amount;
            let blur = target_blur.max(0.0) * amount;

            transforms.push(CharacterTransform {
                position_offset: [pos_x, pos_y],
                scale_multiplier: [scale_x, scale_y],
                opacity_multiplier: opa,
                tracking_offset: track,
                rotation_deg: rot,
                blur,
            });
        }

        transforms
    }
}
