use super::*;

mod scene;

mod world_transform;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMarker {
    pub frame: u32,
    pub label: String,
    pub color: [f32; 3],
}

// ─── 3D Light System ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LightType {
    Ambient,
    Point,
    Spot {
        cone_angle_deg: f32,
        cone_feather_pct: f32,
    },
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Light3D {
    pub id: String,
    pub name: String,
    pub light_type: LightType,
    pub color: [f32; 4],
    pub intensity: f32,
    pub position: Animatable<[f32; 3]>,
    pub casts_shadows: bool,
    /// Shadow opacity cast by this light (0–100, AE Shadow Darkness). Default 60.
    #[serde(default = "default_shadow_darkness")]
    pub shadow_darkness: f32,
    /// Distance falloff exponent (0 = none, 1 = linear, 2 = inverse-square). Default 1.
    #[serde(default)]
    pub falloff: f32,
    /// Maximum influence radius (0 = unlimited). Default 0.
    #[serde(default)]
    pub max_radius: f32,
}

impl Default for Light3D {
    fn default() -> Self {
        Self {
            id: "light_main".to_string(),
            name: "Key Light".to_string(),
            light_type: LightType::Point,
            color: [1.0, 1.0, 1.0, 1.0],
            intensity: 100.0,
            position: Animatable::new_constant([960.0, 540.0, -500.0]),
            casts_shadows: true,
            shadow_darkness: 60.0,
            falloff: 1.0,
            max_radius: 0.0,
        }
    }
}

fn default_shutter_phase() -> f32 {
    -90.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composition {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_frames: u32,
    pub layers: Vec<Layer>,

    pub motion_blur_shutter_angle: f32,
    #[serde(default = "default_shutter_phase")]
    pub motion_blur_shutter_phase: f32,
    pub background_color: [f32; 4],
    pub active_camera: Camera3D,
    /// Additional scene cameras for multi-camera switching. The camera with
    /// `active == true` wins; empty list falls back to `active_camera`.
    #[serde(default)]
    pub cameras: Vec<Camera3D>,
    pub lights: Vec<Light3D>,
    pub markers: Vec<TimelineMarker>,

    /// Sub-compositions for PreComp nesting (keyed by comp id).
    #[serde(default)]
    pub sub_compositions: Vec<Composition>,
    /// Blend layers in linear light (AE "Blend Colors Using 1.0 Gamma").
    /// Off = classic gamma-space compositing (legacy look).
    #[serde(default)]
    pub blend_linear: bool,
    /// Triangular-PDF dither on final 8-bit output (kills gradient banding).
    /// Off for legacy projects/tests to preserve byte-exact renders.
    #[serde(default)]
    pub dither_output: bool,
    /// Composition color depth: 8bpc integer, 16bpc float, or 32bpc scene-linear float.
    #[serde(default)]
    pub bit_depth: crate::core::color_science::BitDepth,
    /// Composition working color space (Rec.709, Display P3, Rec.2020, Linear sRGB, ACEScg).
    #[serde(default)]
    pub working_color_space: crate::core::color_science::WorkingColorSpace,
    /// Composition-level proxy settings for preview speed.
    #[serde(default)]
    pub comp_proxy: crate::core::proxy::CompProxy,
}

impl Composition {
    pub fn new(
        id: String,
        name: String,
        width: u32,
        height: u32,
        fps: u32,
        duration_frames: u32,
    ) -> Self {
        Self {
            id,
            name,
            width,
            height,
            fps,
            duration_frames,
            layers: Vec::new(),
            motion_blur_shutter_angle: 180.0,
            motion_blur_shutter_phase: -90.0,
            background_color: [0.05, 0.05, 0.08, 1.0],
            active_camera: Camera3D::default(),
            cameras: Vec::new(),
            lights: vec![Light3D::default()],
            markers: Vec::new(),
            sub_compositions: Vec::new(),

            blend_linear: false,

            dither_output: false,
            bit_depth: crate::core::color_science::BitDepth::EightBit,
            working_color_space: crate::core::color_science::WorkingColorSpace::Rec709,
            comp_proxy: crate::core::proxy::CompProxy::default(),
        }
    }

    pub fn next_layer_id(&self, prefix: &str) -> String {
        let mut index = self.layers.len();
        loop {
            let candidate = format!("{prefix}_{index}");
            if !self.layers.iter().any(|layer| layer.id == candidate) {
                return candidate;
            }
            index = index.saturating_add(1);
        }
    }

    pub fn next_camera_id(&self) -> String {
        let mut index = self.cameras.len().saturating_add(1);
        loop {
            let candidate = format!("camera_{index}");
            if candidate != self.active_camera.id
                && !self.cameras.iter().any(|camera| camera.id == candidate)
            {
                return candidate;
            }
            index = index.saturating_add(1);
        }
    }

    pub fn next_light_id(&self) -> String {
        let mut index = self.lights.len().saturating_add(1);
        loop {
            let candidate = format!("light_{index}");
            if !self.lights.iter().any(|light| light.id == candidate) {
                return candidate;
            }
            index = index.saturating_add(1);
        }
    }

    /// Remove a timeline layer and the scene object owned by that layer.
    /// Ordinary layers are removed without any additional cleanup.
    pub fn remove_layer_at(&mut self, index: usize) -> Option<Layer> {
        let layer = self.layers.get(index)?.clone();
        self.layers.remove(index);

        match layer.scene_object.as_ref() {
            Some(SceneObjectRef::Camera { id }) => {
                let still_referenced = self.layers.iter().any(|other| {
                    matches!(
                        other.scene_object.as_ref(),
                        Some(SceneObjectRef::Camera { id: other_id }) if other_id == id
                    )
                });
                if !still_referenced {
                    if let Some(camera_index) =
                        self.cameras.iter().position(|camera| camera.id == *id)
                    {
                        let was_active = self.cameras[camera_index].active;
                        self.cameras.remove(camera_index);
                        if was_active {
                            self.set_active_camera(None);
                        }
                    }
                }
            }
            Some(SceneObjectRef::Light { id }) => {
                let still_referenced = self.layers.iter().any(|other| {
                    matches!(
                        other.scene_object.as_ref(),
                        Some(SceneObjectRef::Light { id: other_id }) if other_id == id
                    )
                });
                if !still_referenced {
                    self.lights.retain(|light| light.id != *id);
                }
            }
            None => {}
        }

        Some(layer)
    }

    /// Resizes the composition to new dimensions and automatically remaps layer positions based on Constraints.
    pub fn resize_and_remap(&mut self, new_w: u32, new_h: u32, old_w: u32, old_h: u32) {
        self.width = new_w;
        self.height = new_h;
        for layer in &mut self.layers {
            let current_p = layer.transform.position.evaluate(0);
            let remapped = layer.constraints.remap_position(
                current_p,
                old_w as f32,
                old_h as f32,
                new_w as f32,
                new_h as f32,
            );
            layer.transform.position = Animatable::new_constant(remapped);
        }
    }

    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Set parent layer safely, returning false if a cycle would be introduced.
    pub fn set_layer_parent(&mut self, layer_id: &str, parent_id: Option<String>) -> bool {
        if let Some(ref pid) = parent_id {
            if pid == layer_id {
                return false;
            }
            let mut curr = pid.clone();
            let mut visited = std::collections::HashSet::new();
            visited.insert(layer_id.to_string());
            while let Some(parent_layer) = self.layers.iter().find(|l| l.id == curr) {
                if visited.contains(&parent_layer.id) {
                    return false; // Cycle detected!
                }
                visited.insert(parent_layer.id.clone());
                if let Some(ref next_pid) = parent_layer.parent_id {
                    curr = next_pid.clone();
                } else {
                    break;
                }
            }
        }

        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == layer_id) {
            layer.parent_id = parent_id;
            true
        } else {
            false
        }
    }

    /// Validate and sanitize parent-child links across all layers on deserialization or project load.
    /// Breaks any invalid circular parent dependencies to guarantee panic-free rendering.
    pub fn sanitize_parent_cycles(&mut self) -> usize {
        let id_map = self.build_layer_id_index_map();
        let mut visited = std::collections::HashSet::new();
        let mut to_clear = Vec::new();

        for i in 0..self.layers.len() {
            let layer_id = self.layers[i].id.clone();
            if let Some(ref pid) = self.layers[i].parent_id {
                visited.clear();
                visited.insert(layer_id.clone());

                let mut curr = pid.clone();

                while let Some(&idx) = id_map.get(curr.as_str()) {
                    let parent_layer = &self.layers[idx];
                    if !visited.insert(parent_layer.id.clone()) {
                        to_clear.push(i);
                        break;
                    }
                    if let Some(ref next_pid) = parent_layer.parent_id {
                        curr = next_pid.clone();
                    } else {
                        break;
                    }
                }
            }
        }

        let broken_count = to_clear.len();
        for idx in to_clear {
            log::warn!(
                "Broken circular parent reference detected on layer idx {}. Clearing parent_id.",
                idx
            );
            self.layers[idx].parent_id = None;
        }
        broken_count
    }

    /// Evenly distribute selected layers spatially along horizontal (X) or vertical (Y) axes.
    pub fn distribute_selected_layers(
        &mut self,
        selected_indices: &[usize],
        horizontal: bool,
        frame: u32,
    ) {
        if selected_indices.len() < 3 {
            return;
        }
        let fps = self.fps;

        let mut layers_info: Vec<(usize, f32)> = selected_indices
            .iter()
            .copied()
            .filter(|&idx| idx < self.layers.len())
            .map(|idx| {
                let pos = self.layers[idx].transform.eval_position(frame, fps);
                let coord = if horizontal { pos[0] } else { pos[1] };
                (idx, coord)
            })
            .collect();

        if layers_info.len() < 3 {
            return;
        }

        layers_info.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let (Some(min_pos), Some(max_pos)) = (
            layers_info.first().map(|e| e.1),
            layers_info.last().map(|e| e.1),
        ) else {
            return;
        };
        let count = layers_info.len();

        // Guard: if all layers are at the same coordinate, distribution is a no-op
        if (max_pos - min_pos).abs() < 0.001 {
            return;
        }

        let step = (max_pos - min_pos) / (count - 1) as f32;

        for (i, &(layer_idx, _)) in layers_info.iter().enumerate() {
            let target_coord = min_pos + step * i as f32;
            let current_pos = self.layers[layer_idx].transform.eval_position(frame, fps);
            let new_pos = if horizontal {
                [target_coord, current_pos[1]]
            } else {
                [current_pos[0], target_coord]
            };
            self.layers[layer_idx]
                .transform
                .position
                .set_value_at_frame(frame, new_pos);
        }
    }

    /// Pre-compose selected layers into a new nested Sub-Composition (AE Parity).
    /// Returns the newly created Sub-Composition if successful.
    pub fn precompose_layers(
        &mut self,
        layer_ids: &[String],
        new_comp_id: String,
        new_comp_name: String,
        mode: PrecompAttributesMode,
    ) -> Option<Composition> {
        if layer_ids.is_empty() {
            return None;
        }

        // Find lowest index of selected layers in parent comp to place the new precomp layer at that spot
        let mut min_idx = usize::MAX;
        let mut extracted_layers = Vec::new();

        let mut remaining_layers = Vec::new();
        for (i, layer) in self.layers.drain(..).enumerate() {
            if layer_ids.iter().any(|id| id == &layer.id) {
                min_idx = min_idx.min(i);
                extracted_layers.push(layer);
            } else {
                remaining_layers.push(layer);
            }
        }
        self.layers = remaining_layers;

        if extracted_layers.is_empty() {
            return None;
        }

        let mut new_sub_comp = Composition::new(
            new_comp_id.clone(),
            new_comp_name.clone(),
            self.width,
            self.height,
            self.fps,
            self.duration_frames,
        );

        let mut precomp_layer = Layer::new(
            format!("layer_precomp_{}", new_comp_id),
            new_comp_name,
            LayerType::PreComp {
                comp_id: new_comp_id.clone(),
            },
            self.duration_frames,
        );
        precomp_layer.transform.position =
            Animatable::new_constant([self.width as f32 * 0.5, self.height as f32 * 0.5]);

        match mode {
            PrecompAttributesMode::MoveToNewComp => {
                // Move all layers and attributes into sub-comp
                new_sub_comp.layers = extracted_layers;
            }
            PrecompAttributesMode::LeaveInParent => {
                // Single-layer only: move source layer into sub-comp with default transform,
                // and keep original transform & effects on the precomp layer in the parent.
                if let Some(mut single_layer) = extracted_layers.into_iter().next() {
                    precomp_layer.transform = single_layer.transform.clone();
                    precomp_layer.effects = std::mem::take(&mut single_layer.effects);
                    precomp_layer.masks = std::mem::take(&mut single_layer.masks);
                    single_layer.transform = Transform2D::default();
                    new_sub_comp.layers = vec![single_layer];
                }
            }
        }

        let moved_layers = new_sub_comp.layers.clone();
        self.move_scene_objects_to(&mut new_sub_comp, &moved_layers);

        let insert_idx = min_idx.min(self.layers.len());
        self.layers.insert(insert_idx, precomp_layer);
        self.sub_compositions.push(new_sub_comp.clone());

        Some(new_sub_comp)
    }
}
