use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecompAttributesMode {
    /// Leaves transforms, effects, and masks on the new precomp layer in the current comp. (Single layer only)
    LeaveInParent,
    /// Moves all layer transforms, effects, and masks inside the new sub-composition.
    MoveToNewComp,
}

// ─── Project Item & Asset Management ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectItemType {
    Composition {
        comp_idx: usize,
    },
    Image {
        path: String,
        width: u32,
        height: u32,
    },
    Model3D {
        path: String,
    },
    Video {
        path: String,
        duration_sec: f32,
    },
    Audio {
        path: String,
        duration_sec: f32,
    },
    Solid {
        color: [f32; 4],
    },
    Folder {
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub id: String,
    pub name: String,
    pub item_type: ProjectItemType,
    /// Containing folder (id of the Folder item), None = project root.
    #[serde(default)]
    pub parent_folder: Option<String>,
}

impl ProjectItem {
    pub fn new(id: impl Into<String>, name: impl Into<String>, item_type: ProjectItemType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            item_type,
            parent_folder: None,
        }
    }

    /// Filesystem path for file-backed footage, if any.
    pub fn media_path(&self) -> Option<&str> {
        match &self.item_type {
            ProjectItemType::Image { path, .. }
            | ProjectItemType::Model3D { path }
            | ProjectItemType::Video { path, .. }
            | ProjectItemType::Audio { path, .. } => Some(path),
            _ => None,
        }
    }

    /// True when the item references a file that no longer exists on disk.
    pub fn is_media_missing(&self) -> bool {
        match self.media_path() {
            Some(p) => !std::path::Path::new(p).is_file(),
            None => false,
        }
    }
}

// ─── Project ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub compositions: Vec<Composition>,
    #[serde(default)]
    pub active_composition_idx: usize,
    #[serde(default)]
    pub assets: Vec<ProjectItem>,
    /// Render engine preference persisted with the project:
    /// GPU compute effects (blurs) run when true AND a compatible adapter exists.
    #[serde(default)]
    pub use_gpu_compute: bool,
}

impl Default for Project {
    fn default() -> Self {
        let mut comp = Composition::new(
            "comp_main".to_string(),
            "Main Comp 1".to_string(),
            1920,
            1080,
            30,
            300,
        );

        // Background Solid
        let mut bg_layer = Layer::new(
            "layer_bg".to_string(),
            "Background".to_string(),
            LayerType::Solid {
                color: [0.08, 0.08, 0.12, 1.0],
            },
            300,
        );
        bg_layer.transform.scale = Animatable::new_constant([1920.0, 1080.0]);
        bg_layer.transform.position = Animatable::new_constant([960.0, 540.0]);
        bg_layer.label = LabelColor::Blue;
        comp.add_layer(bg_layer);

        // Main Title Text
        let mut text_layer = Layer::new(
            "layer_text".to_string(),
            "Main Title".to_string(),
            LayerType::new_text("Kagari VFX", 64, [1.0, 1.0, 1.0, 1.0]),
            300,
        );
        text_layer.transform.position = Animatable::new_constant([960.0, 500.0]);
        text_layer.label = LabelColor::Aqua;
        comp.add_layer(text_layer);

        // Null Object controller
        let mut null_ctrl = Layer::new_null(
            "layer_null_ctrl".to_string(),
            "Controller [NULL]".to_string(),
            300,
        );
        null_ctrl.transform.position = Animatable::new_constant([960.0, 540.0]);
        null_ctrl.label = LabelColor::Peach;
        comp.add_layer(null_ctrl);

        let default_assets = vec![
            ProjectItem::new(
                "item_comp1",
                "Main Comp 1",
                ProjectItemType::Composition { comp_idx: 0 },
            ),
            ProjectItem::new(
                "item_bg_solid",
                "Dark Solid Background",
                ProjectItemType::Solid {
                    color: [0.08, 0.08, 0.12, 1.0],
                },
            ),
            ProjectItem::new(
                "item_logo",
                "kagari_logo.webp",
                ProjectItemType::Image {
                    path: "assets/kagari_logo.webp".to_string(),
                    width: 1254,
                    height: 1254,
                },
            ),
            ProjectItem::new(
                "item_solids_folder",
                "Solids",
                ProjectItemType::Folder {
                    name: "Solids".to_string(),
                },
            ),
        ];

        Self {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: default_assets,
            use_gpu_compute: false,
        }
    }
}

impl Project {
    pub fn next_composition_id(&self, prefix: &str) -> String {
        fn contains_id(composition: &Composition, candidate: &str) -> bool {
            composition.id == candidate
                || composition
                    .sub_compositions
                    .iter()
                    .any(|child| contains_id(child, candidate))
        }

        let mut suffix = self.compositions.len();
        loop {
            let candidate = format!("{prefix}_{suffix}");
            if !self
                .compositions
                .iter()
                .any(|composition| contains_id(composition, &candidate))
            {
                return candidate;
            }
            suffix = suffix.saturating_add(1);
        }
    }

    pub fn next_project_item_id(&self, prefix: &str) -> String {
        let mut suffix = self.assets.len();
        loop {
            let candidate = format!("{prefix}_{suffix}");
            if !self.assets.iter().any(|asset| asset.id == candidate) {
                return candidate;
            }
            suffix = suffix.saturating_add(1);
        }
    }

    /// A clean editing session with a safe, empty composition for UI code that
    /// expects an active composition to exist.
    pub fn empty_for_new_project() -> Self {
        Self {
            compositions: vec![Composition::new(
                "comp_empty_start".to_string(),
                "Untitled".to_string(),
                1920,
                1080,
                30,
                300,
            )],
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: false,
        }
    }

    pub fn has_pristine_start_composition(&self) -> bool {
        self.compositions.len() == 1
            && self.compositions[0].id == "comp_empty_start"
            && self.compositions[0].layers.is_empty()
            && self.assets.is_empty()
    }

    pub fn active_composition(&self) -> &Composition {
        let idx = self
            .active_composition_idx
            .min(self.compositions.len().saturating_sub(1));
        &self.compositions[idx]
    }

    pub fn active_composition_mut(&mut self) -> &mut Composition {
        let idx = self
            .active_composition_idx
            .min(self.compositions.len().saturating_sub(1));
        &mut self.compositions[idx]
    }

    /// Collect all unique external asset file paths used across compositions and project items (Collect Files).
    pub fn collect_dependencies(&self) -> Vec<String> {
        let mut deps = std::collections::BTreeSet::new();
        // Project items
        for item in &self.assets {
            match &item.item_type {
                ProjectItemType::Image { path, .. }
                | ProjectItemType::Model3D { path }
                | ProjectItemType::Video { path, .. }
                | ProjectItemType::Audio { path, .. }
                    if !path.is_empty() =>
                {
                    deps.insert(path.clone());
                }
                _ => {}
            }
        }
        // Layers across all compositions (including sub_compositions)
        let mut visit_comp = |c: &Composition| {
            for l in &c.layers {
                match &l.layer_type {
                    LayerType::Image { path, .. } | LayerType::Audio { path, .. } => {
                        if !path.is_empty() {
                            deps.insert(path.clone());
                        }
                    }
                    LayerType::Model3D { path } => {
                        if !path.is_empty() {
                            deps.insert(path.clone());
                        }
                    }
                    LayerType::Video {
                        source, frames_dir, ..
                    } => {
                        if !source.is_empty() {
                            deps.insert(source.clone());
                        }
                        if !frames_dir.is_empty() {
                            deps.insert(frames_dir.clone());
                        }
                    }
                    _ => {}
                }
            }
        };
        for comp in &self.compositions {
            visit_comp(comp);
            for sub in &comp.sub_compositions {
                visit_comp(sub);
            }
        }
        deps.into_iter().collect()
    }

    /// Automatically resolves missing external footage paths by scanning candidate directories
    /// (e.g. `./`, `./Footage/`, `./Assets/`, `./Media/`) around the saved project file location.
    pub fn resolve_relative_footage_paths(&mut self, project_file_dir: &std::path::Path) -> usize {
        let mut relinked_count = 0usize;
        let candidate_subdirs = [
            "", "Footage", "footage", "Assets", "assets", "Media", "media", "audio", "Audio",
        ];

        let find_file = |raw_path: &str| -> Option<String> {
            let p = std::path::Path::new(raw_path);
            if p.is_file() {
                return None;
            }
            let filename = p.file_name()?;
            for sub in &candidate_subdirs {
                let candidate = if sub.is_empty() {
                    project_file_dir.join(filename)
                } else {
                    project_file_dir.join(sub).join(filename)
                };
                if candidate.is_file() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
            None
        };

        // Relink project assets
        for asset in &mut self.assets {
            match &mut asset.item_type {
                ProjectItemType::Image { path, .. } => {
                    if let Some(new_p) = find_file(path) {
                        *path = new_p;
                        relinked_count += 1;
                    }
                }
                ProjectItemType::Model3D { path } => {
                    if let Some(new_p) = find_file(path) {
                        *path = new_p;
                        relinked_count += 1;
                    }
                }
                ProjectItemType::Video { path, .. } => {
                    if let Some(new_p) = find_file(path) {
                        *path = new_p;
                        relinked_count += 1;
                    }
                }
                ProjectItemType::Audio { path, .. } => {
                    if let Some(new_p) = find_file(path) {
                        *path = new_p;
                        relinked_count += 1;
                    }
                }
                _ => {}
            }
        }

        fn relink_comp(
            comp: &mut Composition,
            find_file: &dyn Fn(&str) -> Option<String>,
            relinked_count: &mut usize,
        ) {
            for layer in &mut comp.layers {
                match &mut layer.layer_type {
                    LayerType::Image { path } => {
                        if let Some(new_p) = find_file(path) {
                            *path = new_p;
                            *relinked_count += 1;
                        }
                    }
                    LayerType::Model3D { path } => {
                        if let Some(new_p) = find_file(path) {
                            *path = new_p;
                            *relinked_count += 1;
                        }
                    }
                    LayerType::Video {
                        source,
                        frames_dir,
                        frame_count,
                        audio_wav,
                        ..
                    } => {
                        if let Some(new_p) = find_file(source) {
                            *source = new_p;
                            // The old extraction belongs to the missing source. Force the
                            // relink pass to decode the newly found movie before rendering.
                            frames_dir.clear();
                            *frame_count = 0;
                            *audio_wav = None;
                            *relinked_count += 1;
                        }
                    }
                    LayerType::Audio { path, .. } => {
                        if let Some(new_p) = find_file(path) {
                            *path = new_p;
                            *relinked_count += 1;
                        }
                    }
                    _ => {}
                }
            }
            for sub in &mut comp.sub_compositions {
                relink_comp(sub, find_file, relinked_count);
            }
        }

        // Relink layers across the full precomp tree, not only the top-level comps.
        for comp in &mut self.compositions {
            relink_comp(comp, &find_file, &mut relinked_count);
        }

        relinked_count
    }
}

// ──────────────── Keyframe & Timeline Assistant Tools ────────────────

/// Rove Across Time (AE Parity):
/// Automatically spaces intermediate keyframes in time according to the spatial distance
/// between their 2D positions, keeping the overall motion velocity constant between the first and last keyframe.
pub fn rove_across_time(keyframes: &mut [crate::core::keyframe::Keyframe<[f32; 2]>]) {
    if keyframes.len() < 3 {
        return;
    }
    let first_frame = keyframes.first().unwrap().frame as f32;
    let last_frame = keyframes.last().unwrap().frame as f32;
    let total_time_span = last_frame - first_frame;
    if total_time_span <= 0.0 {
        return;
    }

    // Compute segment distances
    let mut distances = Vec::with_capacity(keyframes.len() - 1);
    let mut total_distance = 0.0f32;
    for i in 0..keyframes.len() - 1 {
        let p0 = keyframes[i].value;
        let p1 = keyframes[i + 1].value;
        let dist = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2)).sqrt();
        distances.push(dist);
        total_distance += dist;
    }

    if total_distance <= 0.001 {
        return;
    }

    // Distribute intermediate keyframes
    let mut accumulated = 0.0f32;
    for i in 1..keyframes.len() - 1 {
        accumulated += distances[i - 1];
        let ratio = accumulated / total_distance;
        keyframes[i].frame = (first_frame + ratio * total_time_span).round() as u32;
    }
}

/// Sequence Layers (AE Parity):
/// Arranges multiple layers end-to-end in time with optional overlap and opacity crossfade.
pub fn sequence_layers(layers: &mut [Layer], overlap_frames: u32, crossfade: bool) {
    if layers.is_empty() {
        return;
    }
    let mut cur_in = layers[0].in_frame;
    for layer in layers.iter_mut() {
        let duration = layer.out_frame.saturating_sub(layer.in_frame).max(1);
        layer.in_frame = cur_in;
        layer.out_frame = cur_in + duration;

        if crossfade && overlap_frames > 0 {
            // Build crossfade opacity keyframes
            let fade = overlap_frames.min(duration / 2);
            let kfs = vec![
                crate::core::keyframe::Keyframe::new(
                    cur_in,
                    0.0,
                    crate::core::keyframe::InterpolationType::Linear,
                ),
                crate::core::keyframe::Keyframe::new(
                    cur_in + fade,
                    100.0,
                    crate::core::keyframe::InterpolationType::Linear,
                ),
                crate::core::keyframe::Keyframe::new(
                    layer.out_frame.saturating_sub(fade),
                    100.0,
                    crate::core::keyframe::InterpolationType::Linear,
                ),
                crate::core::keyframe::Keyframe::new(
                    layer.out_frame,
                    0.0,
                    crate::core::keyframe::InterpolationType::Linear,
                ),
            ];
            layer.transform.opacity = Animatable::new_animated(kfs);
        }

        cur_in = (layer.out_frame.saturating_sub(overlap_frames)).max(cur_in);
    }
}

/// Export timeline markers formatted as YouTube / Video Chapters string.
/// Format: `00:00 Intro\n01:23 Section Title\n...`
pub fn export_youtube_chapters(markers: &[TimelineMarker], fps: u32) -> String {
    let fps = fps.max(1);
    let mut sorted_markers = markers.to_vec();
    sorted_markers.sort_by_key(|m| m.frame);

    let mut lines = Vec::new();
    // Ensure 00:00 exists if first marker isn't at frame 0
    if sorted_markers.first().map(|m| m.frame).unwrap_or(1) > 0 {
        lines.push("00:00 Intro".to_string());
    }

    for m in &sorted_markers {
        let total_sec = m.frame / fps;
        let sec = total_sec % 60;
        let min = (total_sec / 60) % 60;
        let hrs = total_sec / 3600;
        let comment = if m.label.is_empty() {
            format!("Chapter {}", lines.len() + 1)
        } else {
            m.label.clone()
        };

        let time_str = if hrs > 0 {
            format!("{:02}:{:02}:{:02}", hrs, min, sec)
        } else {
            format!("{:02}:{:02}", min, sec)
        };
        lines.push(format!("{} {}", time_str, comment));
    }
    lines.join("\n")
}
