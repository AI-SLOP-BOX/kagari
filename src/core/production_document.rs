//! Versionable cross-domain document joining the VFX project and audio clock.

use crate::core::audio_types::{AudioCorrectionSettings, MixerChannel};
use crate::core::automation_binding::{AutomationBinding, AutomationCurve, ProductionClock};
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, EffectType, LayerType, Project, ProjectItemType};
use crate::core::unified_time::{FrameRate, TempoMap};
use serde::{Deserialize, Serialize};

mod bindings;
use std::collections::HashMap;

mod validation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDocumentSettings {
    pub sample_rate: u32,
    pub master_gain: f32,
    pub channels: Vec<MixerChannel>,
    #[serde(default)]
    pub correction: AudioCorrectionSettings,
}

impl Default for AudioDocumentSettings {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            master_gain: 1.0,
            channels: Vec::new(),
            correction: AudioCorrectionSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionDocument {
    pub schema_version: u32,
    pub project: Project,
    #[serde(default)]
    pub color_lut_path: Option<String>,
    #[serde(default)]
    pub audio: AudioDocumentSettings,
    #[serde(default)]
    pub tempo: TempoMap,
    #[serde(default)]
    pub bindings: Vec<AutomationBinding>,
}

impl ProductionDocument {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;
    pub const MAX_AUDIO_CHANNELS: usize = 4096;
    pub const MAX_BINDINGS: usize = 8192;
    pub const MAX_COMPOSITION_DIMENSION: u32 = 16_384;
    pub const MAX_ASSETS: usize = 100_000;
    pub const MAX_COMPOSITIONS: usize = 10_000;
    pub const MAX_LAYERS_PER_COMPOSITION: usize = 100_000;
    pub const MAX_SUB_COMPOSITIONS_PER_COMPOSITION: usize = 10_000;
    pub const MAX_CAMERAS_PER_COMPOSITION: usize = 10_000;
    pub const MAX_LIGHTS_PER_COMPOSITION: usize = 10_000;
    pub const MAX_MARKERS_PER_COMPOSITION: usize = 100_000;
    pub const MAX_EFFECTS_PER_LAYER: usize = 10_000;
    pub const MAX_MASKS_PER_LAYER: usize = 10_000;
    pub const MAX_MASK_VERTICES_PER_LAYER: usize = 262_144;
    pub const MAX_COMPOSITION_FRAMES: u32 = 10_000_000;
    pub const MAX_ASSET_DURATION_SEC: f32 = 10_000_000.0;
    pub const MAX_ASSET_STRING_LENGTH: usize = 4_096;

    pub fn new(project: Project) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            project,
            color_lut_path: None,
            audio: AudioDocumentSettings::default(),
            tempo: TempoMap::new(120.0),
            bindings: Vec::new(),
        }
    }

    pub fn clock(&self) -> ProductionClock {
        ProductionClock {
            tempo: self.tempo.clone(),
            sample_rate: self.audio.sample_rate,
        }
    }

    /// Adds a Logic Pro-compatible absolute-sample automation lane and binds
    /// it to a VFX target without exposing sample units to the VFX graph.
    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        let json = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        crate::core::project_migration::validate_project_json_size(&json)?;
        Ok(json)
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.len() > crate::core::project_migration::MAX_PROJECT_JSON_BYTES {
            return Err(format!(
                "Production document exceeds the {} MiB limit",
                crate::core::project_migration::MAX_PROJECT_JSON_BYTES / (1024 * 1024)
            ));
        }
        let document: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        document.validate()?;
        Ok(document)
    }

    pub fn save_atomic(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let json = self.to_json()?;
        crate::core::atomic_file::write_atomic(path.as_ref(), json.as_bytes())
            .map_err(|error| format!("failed to atomically save production document: {error}"))
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let json = crate::core::project_migration::read_bounded_text_file(
            path.as_ref(),
            crate::core::project_migration::MAX_PROJECT_JSON_BYTES,
        )
        .map_err(|error| format!("failed to read production document: {error}"))?;
        Self::from_json(&json)
    }

    /// Upgrade a legacy Project JSON into the unified production document.
    /// Audio, tempo, and binding data receive safe defaults until the caller
    /// supplies domain-specific values.
    pub fn from_legacy_project_json(json: &str) -> Result<Self, String> {
        let project = crate::core::project_migration::load_project_migrated(json)?;
        let document = Self::new(project);
        document.validate()?;
        Ok(document)
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn project_mut(&mut self) -> &mut Project {
        &mut self.project
    }

    /// Validate the bounded legacy single-composition project format using
    /// the same layer, media, animation, and precomposition rules as a full
    /// production document.
    pub(crate) fn validate_standalone_composition(composition: &Composition) -> Result<(), String> {
        validation::validate_standalone_composition(composition)
    }
}

#[cfg(test)]
#[path = "production_document/tests.rs"]
mod tests;
