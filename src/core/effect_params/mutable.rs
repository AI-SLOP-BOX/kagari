mod extended;
mod primary;

use super::types::ParamRef;
use crate::core::timeline::EffectType;

impl EffectType {
    /// All keyframeable parameters of this effect, in display order.
    pub fn animatable_params(&mut self) -> Vec<(&'static str, ParamRef<'_>)> {
        let mut out = Vec::new();
        if primary::is_primary_effect(self) {
            primary::populate_primary_params(self, &mut out);
        } else {
            extended::populate_extended_params(self, &mut out);
        }
        out
    }
}
