mod part1;
mod part2;
mod part3;

use crate::core::timeline::{Effect, EffectType};
use std::sync::OnceLock;

#[allow(dead_code)]
pub struct EffectPreset {
    pub name: &'static str,
    pub button_label: &'static str,
    pub search_key: &'static str,
    pub id_prefix: &'static str,
    pub create_fn: fn(idx: usize) -> Effect,
}

static ALL_PRESETS: OnceLock<Vec<EffectPreset>> = OnceLock::new();

pub fn get_all_effect_presets() -> &'static [EffectPreset] {
    ALL_PRESETS.get_or_init(|| {
        let mut all = part1::get_presets_part1();
        all.extend(part2::get_presets_part2());
        all.extend(part3::get_presets_part3());
        all
    })
}

/// Factory-fresh default `EffectType` for the same variant as `effect_type`.
///
/// Used by the Effect Controls "Reset" action: the first registry preset with
/// a matching variant discriminant supplies the defaults. Returns `None` when
/// no preset covers the variant (e.g. script-defined effects).
pub fn fresh_default_for(effect_type: &EffectType) -> Option<EffectType> {
    let want = std::mem::discriminant(effect_type);
    get_all_effect_presets()
        .iter()
        .map(|p| (p.create_fn)(0))
        .find(|e| std::mem::discriminant(&e.effect_type) == want)
        .map(|e| e.effect_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registry_preset_resolves_a_fresh_default() {
        let presets = get_all_effect_presets();
        assert!(!presets.is_empty());
        for p in presets {
            let effect = (p.create_fn)(0);
            let fresh = fresh_default_for(&effect.effect_type);
            assert!(
                fresh.is_some(),
                "Reset has no factory defaults for '{}'",
                p.name
            );
            assert_eq!(
                std::mem::discriminant(&fresh.unwrap()),
                std::mem::discriminant(&effect.effect_type),
                "Reset changed the variant for '{}'",
                p.name
            );
        }
    }
}
