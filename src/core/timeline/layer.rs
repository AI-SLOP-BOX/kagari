use super::*;

// ─── 3D Material Options (AE-style material properties) ─────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialOptions {
    /// Ambient light contribution (0.0–1.0). Default 0.3.
    pub ambient: f32,
    /// Diffuse (Lambertian) reflection intensity (0.0–1.0). Default 0.8.
    pub diffuse: f32,
    /// Specular highlight intensity (0.0–1.0). Default 0.5.
    pub specular: f32,
    /// Specular highlight sharpness / exponent (1–256). Higher = tighter highlight.
    pub specular_exponent: f32,
    /// Emissive self-illumination (0.0–1.0). Default 0.0.
    pub emission: f32,
    /// Metalness for PBR-like shading (0.0 = dielectric, 1.0 = metal). Default 0.0.
    pub metalness: f32,
    /// Whether this layer casts shadows from shadow-casting lights (AE material option).
    #[serde(default = "default_true")]
    pub cast_shadows: bool,
    /// Whether this layer receives shadows cast by other layers.
    #[serde(default = "default_true")]
    pub accepts_shadows: bool,
    /// Whether this layer is illuminated by 3D lights.
    #[serde(default = "default_true")]
    pub accepts_lights: bool,
    /// Light transmission through layer (0.0–100.0). Default 0.0.
    #[serde(default)]
    pub light_transmission: f32,
}

impl Default for MaterialOptions {
    fn default() -> Self {
        Self {
            ambient: 0.3,
            diffuse: 0.8,
            specular: 0.5,
            specular_exponent: 32.0,
            emission: 0.0,
            metalness: 0.0,
            cast_shadows: true,
            accepts_shadows: true,
            accepts_lights: true,
            light_transmission: 0.0,
        }
    }
}

// ─── Text Formatting ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFormatting {
    pub font_family: String,
    pub tracking: f32,
    pub leading: f32,
    pub stroke_color: Option<[f32; 4]>,
    pub stroke_width: f32,
    /// Paragraph alignment: 0=Left, 1=Center, 2=Right, 3=Justify
    #[serde(default)]
    pub alignment: u32,
    /// Text box width for line wrapping (0 = no wrapping)
    #[serde(default)]
    pub box_width: f32,
    /// Text box height for vertical overflow (0 = no limit)
    #[serde(default)]
    pub box_height: f32,
    /// Synthetic weight used when the selected family has no bold face.
    #[serde(default)]
    pub faux_bold: bool,
    /// Synthetic slant used when the selected family has no italic face.
    #[serde(default)]
    pub faux_italic: bool,
    /// Render alphabetic characters as uppercase without changing the source text.
    #[serde(default)]
    pub all_caps: bool,
    /// Render alphabetic characters as compact uppercase glyphs.
    #[serde(default)]
    pub small_caps: bool,
}

impl Default for TextFormatting {
    fn default() -> Self {
        Self {
            font_family: "Inter".to_string(),
            tracking: 0.0,
            leading: 1.2,
            stroke_color: None,
            stroke_width: 1.0,
            alignment: 0,
            box_width: 0.0,
            box_height: 0.0,
            faux_bold: false,
            faux_italic: false,
            all_caps: false,
            small_caps: false,
        }
    }
}

// ─── Layer ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub layer_type: LayerType,
    pub in_frame: u32,
    pub out_frame: u32,
    pub transform: Transform2D,
    pub effects: Vec<Effect>,
    pub visible: bool,
    pub locked: bool,

    pub parent_id: Option<String>,
    pub track_matte: TrackMatteMode,
    /// Explicit target layer for universal track matting (AE CC 2023+).
    /// When None, falls back to the adjacent layer above for backward compatibility.
    #[serde(default)]
    pub track_matte_layer_id: Option<String>,
    pub solo: bool,
    pub motion_blur: bool,
    pub label: LabelColor,
    pub time_remap: Option<Animatable<f32>>,

    pub trackers: Vec<TrackerPoint>,

    pub is_3d: bool,
    #[serde(default)]
    pub scene_object: Option<SceneObjectRef>,
    pub transform_3d: Transform3D,

    #[serde(default)]
    pub material: MaterialOptions,

    // ── AE Blend Mode ──
    pub blend_mode: BlendMode,

    pub is_adjustment_layer: bool,
    pub is_guide_layer: bool,
    pub is_shy: bool,
    pub effects_enabled: bool,
    pub is_collapsed: bool,

    // ── AE Masking System ──
    pub masks: Vec<crate::core::mask::Mask>,
    #[serde(default)]
    pub puppet_pins: Vec<PuppetPin>,
    #[serde(default)]
    pub paint_strokes: Vec<PaintStroke>,
    /// Persistent Roto Brush source strokes. The generated matte remains an
    /// editable result, while these strokes let the user refine it after a
    /// project reload instead of losing the authoring input in egui state.
    #[serde(default)]
    pub roto_brush_strokes: Vec<crate::core::roto_brush_engine::RotoStroke>,

    /// Blend adjacent source frames when video speed or time-remap lands
    /// between frames. Disabled by default for legacy project parity.
    #[serde(default)]
    pub frame_blending: bool,

    // ── AE Layer Markers (per-layer comment flags) ──
    #[serde(default)]
    pub markers: Vec<TimelineMarker>,

    // ── AE Auto-Orient (rotation follows motion path) ──
    #[serde(default)]
    pub auto_orient: crate::core::auto_orient::AutoOrientMode,

    // ── Posterize Time (stop-motion frame quantization) ──
    #[serde(default)]
    pub posterize_time: Option<crate::core::posterize_time::PosterizeTimeSettings>,

    // ── AE Layer Style System ──
    pub style: LayerStyle,

    // ── Text Formatting System ──
    pub text_formatting: Option<TextFormatting>,

    // ── Text Animator (per-character animation) ──
    #[serde(default)]
    pub text_animator: Option<crate::core::text_animator::TextAnimatorSettings>,
    pub text_animator_stack: Option<AnimatorStack>,

    // ── AE Transparency System ──
    #[serde(default)]
    pub preserve_transparency: bool,

    // ── Trim Paths Animator (Shape path trimming) ──
    #[serde(default)]
    pub trim_paths: Option<TrimPaths>,

    // ── Shape Repeater (shape duplication) ──
    #[serde(default)]
    pub shape_repeater: Option<crate::core::shape_repeater::ShapeRepeaterOptions>,

    // ── Layer Constraints System (Pinning) ──
    #[serde(default)]
    pub constraints: crate::core::layer_constraints::LayerConstraints,

    // ── Essential Properties (Master Properties) ──
    #[serde(default)]
    pub essential_properties: Vec<crate::core::essential_properties::EssentialProperty>,

    // ── Proxy (low-res preview) ──
    #[serde(default)]
    pub proxy: crate::core::proxy::LayerProxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PaintStrokeMode {
    #[default]
    Brush,
    CloneStamp,
}

/// A brush stroke painted onto a layer. Points live in layer-local space
/// (origin = the layer's rest center), so strokes follow transforms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintStroke {
    pub color: [f32; 4],
    pub size: f32,
    #[serde(default)]
    pub mode: PaintStrokeMode,
    #[serde(default)]
    pub clone_offset: [f32; 2],
    #[serde(default = "default_paint_hardness")]
    pub hardness: f32,
    #[serde(default = "default_paint_opacity")]
    pub opacity: f32,
    #[serde(default = "default_paint_flow")]
    pub flow: f32,
    pub points: Vec<[f32; 2]>,
    /// First frame the stroke is visible.
    #[serde(default)]
    pub start_frame: u32,
    /// Last frame it is visible; 0 = until the layer's out-point.
    #[serde(default)]
    pub end_frame: u32,
}

fn default_paint_hardness() -> f32 {
    0.8
}

fn default_paint_opacity() -> f32 {
    1.0
}

fn default_paint_flow() -> f32 {
    1.0
}

/// A puppet-tool deformation pin. `comp_source` is the rest position in
/// composition space where the pin was placed; `position` (animatable) is
/// its current comp-space location — the delta drives the mesh warp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PuppetPin {
    pub id: String,
    pub name: String,
    pub comp_source: [f32; 2],
    pub position: Animatable<[f32; 2]>,
}

impl PuppetPin {
    pub fn new(id: String, name: String, source: [f32; 2]) -> Self {
        Self {
            id,
            name,
            comp_source: source,
            position: Animatable::new_constant(source),
        }
    }
}

impl Layer {
    pub fn new(id: String, name: String, layer_type: LayerType, duration_frames: u32) -> Self {
        let is_adj = matches!(layer_type, LayerType::AdjustmentLayer);
        Self {
            id,
            name,
            layer_type,
            in_frame: 0,
            out_frame: duration_frames,
            transform: Transform2D::default(),
            effects: Vec::new(),
            visible: true,
            locked: false,
            parent_id: None,
            track_matte: TrackMatteMode::None,
            track_matte_layer_id: None,
            solo: false,
            motion_blur: false,
            label: LabelColor::Red,
            time_remap: None,
            trackers: Vec::new(),
            is_3d: false,
            scene_object: None,
            transform_3d: Transform3D::default(),
            material: MaterialOptions::default(),
            blend_mode: BlendMode::Normal,
            is_adjustment_layer: is_adj,
            is_guide_layer: false,
            is_shy: false,
            effects_enabled: true,
            is_collapsed: false,
            masks: Vec::new(),
            puppet_pins: Vec::new(),
            paint_strokes: Vec::new(),
            roto_brush_strokes: Vec::new(),
            frame_blending: false,
            markers: Vec::new(),
            auto_orient: crate::core::auto_orient::AutoOrientMode::Off,
            posterize_time: None,
            style: LayerStyle::default(),
            text_formatting: None,
            text_animator: None,
            text_animator_stack: None,
            preserve_transparency: false,
            trim_paths: None,
            shape_repeater: None,
            constraints: crate::core::layer_constraints::LayerConstraints::default(),
            essential_properties: vec![],
            proxy: crate::core::proxy::LayerProxy::default(),
        }
    }

    pub fn new_null(id: String, name: String, duration_frames: u32) -> Self {
        let mut l = Self::new(id, name, LayerType::Null, duration_frames);
        l.label = LabelColor::Red;
        l
    }

    pub fn new_adjustment(id: String, name: String, duration_frames: u32) -> Self {
        let mut l = Self::new(id, name, LayerType::AdjustmentLayer, duration_frames);
        l.label = LabelColor::Lavender;
        l
    }

    /// Computes the accurate, unscaled bounding box dimensions (width, height) of the layer based on its LayerType.
    pub fn bounding_size(&self) -> [f32; 2] {
        match &self.layer_type {
            LayerType::Solid { color: _ } => [1920.0, 1080.0],
            LayerType::Text {
                text,
                font_size,
                tracking,
                ..
            } => {
                let char_count = text.chars().count().max(1) as f32;
                let fs = *font_size as f32;
                let approx_w = char_count * fs * 0.6 + (char_count - 1.0) * tracking;
                let approx_h = fs * 1.2;
                [approx_w.max(10.0), approx_h.max(10.0)]
            }
            _ => [100.0, 100.0],
        }
    }

    pub fn is_active(&self, frame: u32) -> bool {
        self.visible && frame >= self.in_frame && frame <= self.out_frame
    }

    pub fn duration_frames(&self) -> u32 {
        self.out_frame.saturating_sub(self.in_frame)
    }

    pub fn remap_frame(&self, frame: u32) -> u32 {
        self.remap_frame_f32(frame).round().max(0.0) as u32
    }

    /// Return the source frame used by render-time layer evaluation.
    ///
    /// Time remapping is applied first, then optional posterize-time sampling
    /// quantizes that source frame. Keeping this on Layer prevents the CPU,
    /// GPU, precomp and shadow paths from drifting apart.
    pub fn effective_render_frame(&self, frame: u32, fps: u32) -> u32 {
        let remapped = self.remap_frame(frame);
        match &self.posterize_time {
            Some(settings) if settings.enabled => {
                crate::core::posterize_time::quantize_frame_posterize(remapped, fps, settings)
            }
            _ => remapped,
        }
    }

    /// Evaluate time remapping without losing the fractional source frame.
    /// Video rasterization uses this for optional frame blending.
    pub fn remap_frame_f32(&self, frame: u32) -> f32 {
        match &self.time_remap {
            Some(anim) => anim.evaluate(frame).max(0.0),
            None => frame as f32,
        }
    }

    /// AE "Enable Time Remapping": identity map over the layer span, giving the
    /// user editable keyframes. No-op if remapping is already enabled.
    pub fn enable_time_remapping(&mut self) {
        if self.time_remap.is_some() {
            return;
        }
        let in_f = self.in_frame;
        let out_f = self.out_frame;
        let linear = crate::core::keyframe::InterpolationType::Linear;
        self.time_remap = Some(Animatable::Animated(vec![
            crate::core::keyframe::Keyframe::new(in_f, in_f as f32, linear),
            crate::core::keyframe::Keyframe::new(out_f, out_f as f32, linear),
        ]));
    }

    /// AE "Time Stretch": scale the layer span by `factor` and keep source
    /// playback consistent via a linear remap (source advances at `factor`).
    /// factor > 1 slows the source down; factor < 1 speeds it up.
    pub fn time_stretch(&mut self, factor: f32) {
        if factor <= 0.001 {
            return;
        }
        let in_f = self.in_frame;
        let duration = self.duration_frames() as f32;
        let new_duration = ((duration * factor).round() as u32).max(1);
        self.out_frame = in_f.saturating_add(new_duration);

        let end_source = in_f as f32 + new_duration as f32 / factor;
        let linear = crate::core::keyframe::InterpolationType::Linear;
        self.time_remap = Some(Animatable::Animated(vec![
            crate::core::keyframe::Keyframe::new(in_f, in_f as f32, linear),
            crate::core::keyframe::Keyframe::new(
                in_f.saturating_add(new_duration),
                end_source,
                linear,
            ),
        ]));
    }

    /// AE "Time-Reverse Layer": play the layer's source backwards.
    pub fn time_reverse(&mut self) {
        let in_f = self.in_frame;
        let out_f = self.out_frame;
        let linear = crate::core::keyframe::InterpolationType::Linear;
        self.time_remap = Some(Animatable::Animated(vec![
            crate::core::keyframe::Keyframe::new(in_f, out_f as f32, linear),
            crate::core::keyframe::Keyframe::new(out_f, in_f as f32, linear),
        ]));
    }

    /// AE "Freeze Frame": hold one source frame across the whole span.
    pub fn freeze_at(&mut self, source_frame: u32) {
        self.time_remap = Some(Animatable::new_constant(source_frame as f32));
    }

    /// Remove time remapping so the source plays at comp time again.
    pub fn clear_time_remap(&mut self) {
        self.time_remap = None;
    }

    /// AE "Easy Ease" (F9): apply smooth bezier interpolation to every
    /// transform keyframe. Properties driven by expressions are untouched.
    pub fn easy_ease_transform(&mut self) {
        let t = &mut self.transform;
        if t.position_expression.is_none() {
            t.position.easy_ease();
        }
        if t.scale_expression.is_none() {
            t.scale.easy_ease();
        }
        if t.rotation_expression.is_none() {
            t.rotation.easy_ease();
        }
        if t.opacity_expression.is_none() {
            t.opacity.easy_ease();
        }
        if t.anchor_point_expression.is_none() {
            t.anchor_point.easy_ease();
        }
    }
}
