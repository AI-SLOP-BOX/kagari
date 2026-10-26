use crate::core::keyframe::{InterpolationType, Keyframe};

const MAX_BAKED_LOOP_KEYS: u64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    Cycle,
    PingPong,
}

impl LoopMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cycle => "cycle",
            Self::PingPong => "pingpong",
        }
    }
}

pub fn loop_keyframes(
    in_frame: u32,
    out_frame: u32,
    composition_duration: u32,
    mode: LoopMode,
) -> Result<Vec<Keyframe<f32>>, String> {
    let source_duration = u64::from(out_frame.saturating_sub(in_frame).max(1));
    let remaining = u64::from(composition_duration.saturating_sub(in_frame));
    if remaining == 0 {
        return Err("The selected layer starts after the composition ends.".to_owned());
    }
    if source_duration == 1 {
        return Ok(vec![Keyframe::new(
            in_frame,
            0.0,
            InterpolationType::Linear,
        )]);
    }
    let mode_multiplier = if mode == LoopMode::Cycle { 1 } else { 2 };
    let step = source_duration * mode_multiplier;
    let iterations = remaining.div_ceil(step);
    let keys_per_iteration = if mode == LoopMode::Cycle { 2 } else { 4 };
    let key_count = iterations * keys_per_iteration;
    if key_count > MAX_BAKED_LOOP_KEYS {
        return Err(format!(
            "Loop would create {key_count} keyframes; the limit is {MAX_BAKED_LOOP_KEYS}. Increase the source duration or shorten the composition."
        ));
    }

    let source_end = (source_duration - 1) as f32;
    let mut keyframes = Vec::with_capacity(key_count as usize);
    let mut time = u64::from(in_frame);
    for _ in 0..iterations {
        let push = |frame: u64, value: f32, keyframes: &mut Vec<Keyframe<f32>>| {
            keyframes.push(Keyframe::new(
                frame.min(u64::from(u32::MAX)) as u32,
                value,
                InterpolationType::Linear,
            ));
        };
        push(time, 0.0, &mut keyframes);
        push(time + source_duration - 1, source_end, &mut keyframes);
        if mode == LoopMode::PingPong {
            let reverse_start = time + source_duration;
            push(reverse_start, source_end, &mut keyframes);
            push(reverse_start + source_duration - 1, 0.0, &mut keyframes);
        }
        time += step;
    }
    Ok(keyframes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_keyframes_rejects_unbounded_expansion_before_allocating() {
        let error = loop_keyframes(0, 2, u32::MAX, LoopMode::Cycle).unwrap_err();
        assert!(error.contains("limit is"));
    }

    #[test]
    fn one_frame_loop_near_float_precision_limit_terminates() {
        let keys = loop_keyframes(1 << 24, (1 << 24) + 1, (1 << 24) + 3, LoopMode::Cycle).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].frame, 1 << 24);
        assert_eq!(keys[0].value, 0.0);
    }
}
