mod extended;
mod primary;

use super::types::ParamRefRef;
use crate::core::timeline::EffectType;

impl EffectType {
    pub fn animatable_params_ref(&self) -> Vec<(&'static str, ParamRefRef<'_>)> {
        let mut out = Vec::new();
        if !primary::populate_primary_params_ref(self, &mut out) {
            extended::populate_extended_params_ref(self, &mut out);
        }
        out
    }
}
