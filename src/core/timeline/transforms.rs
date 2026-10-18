use super::*;

// ─── Transform ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform2D {
    pub anchor_point: Animatable<[f32; 2]>,
    pub position: Animatable<[f32; 2]>,
    pub scale: Animatable<[f32; 2]>,
    pub rotation: Animatable<f32>,
    pub opacity: Animatable<f32>,

    #[serde(default)]
    pub anchor_point_expression: Option<Expression>,
    pub position_expression: Option<Expression>,
    pub rotation_expression: Option<Expression>,
    pub scale_expression: Option<Expression>,
    pub opacity_expression: Option<Expression>,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            anchor_point: Animatable::new_constant([0.0, 0.0]),
            position: Animatable::new_constant([0.0, 0.0]),
            scale: Animatable::new_constant([100.0, 100.0]),
            rotation: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            anchor_point_expression: None,
            position_expression: None,
            rotation_expression: None,
            scale_expression: None,
            opacity_expression: None,
        }
    }
}

/// Helper to map an extended frame number according to keyframe loop settings (LoopOut / PingPong).
pub fn remap_frame_for_loop(frame: u32, first_kf: u32, last_kf: u32, is_pingpong: bool) -> u32 {
    if first_kf >= last_kf || frame <= last_kf {
        return frame;
    }
    let span = last_kf - first_kf;
    if span == 0 {
        return first_kf;
    }
    let offset = frame - first_kf;
    let cycle_idx = offset / span;
    let rem = offset % span;

    if is_pingpong && (cycle_idx % 2 == 1) {
        last_kf - rem
    } else {
        first_kf + rem
    }
}

#[derive(Clone, Copy)]
enum LoopProp {
    Position,
    Scale,
    Rotation,
    Opacity,
}

impl Transform2D {
    fn remap_loop_frame(
        frame: u32,
        expression: &Option<Expression>,
        first_frame: u32,
        last_frame: u32,
    ) -> u32 {
        match expression {
            Some(Expression::LoopOut) => {
                remap_frame_for_loop(frame, first_frame, last_frame, false)
            }
            Some(Expression::PingPong) => {
                remap_frame_for_loop(frame, first_frame, last_frame, true)
            }
            _ => frame,
        }
    }

    pub fn eval_position(&self, frame: u32, fps: u32) -> [f32; 2] {
        let eval_frame = if let Some(kfs) = self.position.keyframes() {
            match (kfs.first(), kfs.last()) {
                (Some(first), Some(last)) => Self::remap_loop_frame(
                    frame,
                    &self.position_expression,
                    first.frame,
                    last.frame,
                ),
                _ => frame,
            }
        } else {
            frame
        };
        let base = self.position.evaluate(eval_frame);
        match &self.position_expression {
            Some(Expression::Raw(script)) if expression_engine::script_uses_loops(script) => {
                let loops = self.compute_loop_vals(frame, fps, LoopProp::Position);
                expression_engine::eval_v2_with_loops(script, base, frame, fps, loops)
            }
            Some(expr) => expr.evaluate_v2_with_property(base, eval_frame, fps, &self.position),
            None => base,
        }
    }

    /// Computes loopOut/loopIn reference values from this property's keyframes for Raw scripts.
    fn compute_loop_vals(
        &self,
        frame: u32,
        _fps: u32,
        prop: LoopProp,
    ) -> expression_engine::LoopVals {
        use crate::core::expression_engine::LoopVals;
        let mut vals = LoopVals::default();
        // Sample the animatable at one cycle past the last keyframe for each loop mode
        let mut compute = |kfs: &[crate::core::keyframe::Keyframe<[f32; 2]>],
                           sample: &dyn Fn(u32) -> [f32; 2]| {
            if let (Some(first), Some(last)) = (kfs.first(), kfs.last()) {
                let first = first.frame;
                let last = last.frame;
                let out_c = sample(remap_frame_for_loop(frame, first, last, false));
                let out_p = sample(remap_frame_for_loop(frame, first, last, true));
                vals.out_cycle = out_c[0];
                vals.out_pingpong = out_p[0];
                vals.in_cycle = out_c[1];
                vals.in_pingpong = out_p[1];
            }
        };
        match prop {
            LoopProp::Position => {
                if let Some(kfs) = self.position.keyframes() {
                    compute(kfs, &|f| self.position.evaluate(f));
                }
            }
            LoopProp::Scale => {
                if let Some(kfs) = self.scale.keyframes() {
                    compute(kfs, &|f| self.scale.evaluate(f));
                }
            }
            LoopProp::Rotation | LoopProp::Opacity => {
                // Scalar properties: expose the scalar in both X and Y slots
                type ScalarKfs<'a> = (
                    Option<&'a [crate::core::keyframe::Keyframe<f32>]>,
                    &'a dyn Fn(u32) -> f32,
                );
                let (kfs, anim): ScalarKfs = match prop {
                    LoopProp::Rotation => {
                        (self.rotation.keyframes(), &|f| self.rotation.evaluate(f))
                    }
                    _ => (self.opacity.keyframes(), &|f| self.opacity.evaluate(f)),
                };
                if let Some(kfs) = kfs {
                    if let (Some(first), Some(last)) = (kfs.first(), kfs.last()) {
                        let first = first.frame;
                        let last = last.frame;
                        let v2 = |f: u32| [anim(f), anim(f)];
                        vals.out_cycle = v2(remap_frame_for_loop(frame, first, last, false))[0];
                        vals.out_pingpong = v2(remap_frame_for_loop(frame, first, last, true))[0];
                        vals.in_cycle = vals.out_cycle;
                        vals.in_pingpong = vals.out_pingpong;
                    }
                }
            }
        }
        vals
    }

    pub fn eval_rotation(&self, frame: u32, fps: u32) -> f32 {
        let eval_frame = if let Some(kfs) = self.rotation.keyframes() {
            match (kfs.first(), kfs.last()) {
                (Some(first), Some(last)) => Self::remap_loop_frame(
                    frame,
                    &self.rotation_expression,
                    first.frame,
                    last.frame,
                ),
                _ => frame,
            }
        } else {
            frame
        };
        let base = self.rotation.evaluate(eval_frame);
        match &self.rotation_expression {
            Some(Expression::Raw(script)) if expression_engine::script_uses_loops(script) => {
                let loops = self.compute_loop_vals(frame, fps, LoopProp::Rotation);
                expression_engine::eval_f32_with_loops(script, base, frame, fps, loops)
            }
            Some(expr) => expr.evaluate_f32_with_property(base, eval_frame, fps, &self.rotation),
            None => base,
        }
    }

    pub fn eval_scale(&self, frame: u32, fps: u32) -> [f32; 2] {
        let eval_frame = if let Some(kfs) = self.scale.keyframes() {
            match (kfs.first(), kfs.last()) {
                (Some(first), Some(last)) => {
                    Self::remap_loop_frame(frame, &self.scale_expression, first.frame, last.frame)
                }
                _ => frame,
            }
        } else {
            frame
        };
        let base = self.scale.evaluate(eval_frame);
        match &self.scale_expression {
            Some(Expression::Raw(script)) if expression_engine::script_uses_loops(script) => {
                let loops = self.compute_loop_vals(frame, fps, LoopProp::Scale);
                expression_engine::eval_v2_with_loops(script, base, frame, fps, loops)
            }
            Some(expr) => expr.evaluate_v2_with_property(base, eval_frame, fps, &self.scale),
            None => base,
        }
    }

    pub fn eval_opacity(&self, frame: u32, fps: u32) -> f32 {
        let eval_frame = if let Some(kfs) = self.opacity.keyframes() {
            match (kfs.first(), kfs.last()) {
                (Some(first), Some(last)) => {
                    Self::remap_loop_frame(frame, &self.opacity_expression, first.frame, last.frame)
                }
                _ => frame,
            }
        } else {
            frame
        };
        let base = self.opacity.evaluate(eval_frame);
        match &self.opacity_expression {
            Some(Expression::Raw(script)) if expression_engine::script_uses_loops(script) => {
                let loops = self.compute_loop_vals(frame, fps, LoopProp::Opacity);
                expression_engine::eval_f32_with_loops(script, base, frame, fps, loops)
            }
            Some(expr) => expr.evaluate_f32_with_property(base, eval_frame, fps, &self.opacity),
            None => base,
        }
    }
}

// ─── 3D Space Extension (Camera Suite) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform3D {
    pub position: Animatable<[f32; 3]>,
    pub rotation: Animatable<[f32; 3]>,
    pub scale: Animatable<[f32; 3]>,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            position: Animatable::new_constant([0.0, 0.0, 0.0]),
            rotation: Animatable::new_constant([0.0, 0.0, 0.0]),
            scale: Animatable::new_constant([100.0, 100.0, 100.0]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera3D {
    /// Stable scene-object identity used by timeline camera layers.
    #[serde(default = "default_camera_id")]
    pub id: String,
    pub name: String,
    pub active: bool,
    pub fov_degrees: f32,
    pub focus_distance: f32,
    pub aperture: f32,
    /// Optional animated lens tracks. `None` preserves legacy constant-only projects.
    #[serde(default)]
    pub fov_animation: Option<Animatable<f32>>,
    #[serde(default)]
    pub focus_distance_animation: Option<Animatable<f32>>,
    #[serde(default)]
    pub aperture_animation: Option<Animatable<f32>>,
    #[serde(default)]
    pub dof_max_blur_animation: Option<Animatable<f32>>,
    pub transform: Transform3D,
    /// Depth of Field toggle
    #[serde(default)]
    pub dof_enabled: bool,
    #[serde(default)]
    pub dof_enabled_animation: Option<Animatable<f32>>,
    /// Maximum blur radius in pixels for out-of-focus areas (1–64)
    #[serde(default = "default_dof_max_blur")]
    pub dof_max_blur: f32,
    /// Iris shape: 0=circle, 3=triangle, 5=pentagon, 6=hexagon, 8=octagon
    #[serde(default)]
    pub dof_iris_sides: u32,
}

fn default_dof_max_blur() -> f32 {
    16.0
}

fn default_camera_id() -> String {
    "camera_active".to_string()
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            id: default_camera_id(),
            name: "Active Camera".to_string(),
            active: true,
            fov_degrees: 50.0,
            focus_distance: 1000.0,
            aperture: 2.8,
            fov_animation: None,
            focus_distance_animation: None,
            aperture_animation: None,
            dof_max_blur_animation: None,
            transform: Transform3D::default(),
            dof_enabled: false,
            dof_enabled_animation: None,
            dof_max_blur: 16.0,
            dof_iris_sides: 0,
        }
    }
}

/// A non-rendering timeline layer that exposes a composition camera or light
/// as a selectable scene object, while the actual scene settings remain in
/// the composition's camera/light collections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneObjectRef {
    Camera { id: String },
    Light { id: String },
}

impl Camera3D {
    pub fn set_fov_at(&mut self, frame: u32, value: f32) {
        let value = value.clamp(1.0, 179.0);
        let legacy = self.fov_degrees;
        self.fov_animation
            .get_or_insert_with(|| Animatable::new_constant(legacy))
            .set_keyframe(crate::core::keyframe::Keyframe::new(
                frame,
                value,
                crate::core::keyframe::InterpolationType::Linear,
            ));
        self.fov_degrees = value;
    }

    pub fn set_focus_distance_at(&mut self, frame: u32, value: f32) {
        let value = value.max(0.0);
        let legacy = self.focus_distance;
        self.focus_distance_animation
            .get_or_insert_with(|| Animatable::new_constant(legacy))
            .set_keyframe(crate::core::keyframe::Keyframe::new(
                frame,
                value,
                crate::core::keyframe::InterpolationType::Linear,
            ));
        self.focus_distance = value;
    }

    pub fn set_aperture_at(&mut self, frame: u32, value: f32) {
        let value = value.max(0.0);
        let legacy = self.aperture;
        self.aperture_animation
            .get_or_insert_with(|| Animatable::new_constant(legacy))
            .set_keyframe(crate::core::keyframe::Keyframe::new(
                frame,
                value,
                crate::core::keyframe::InterpolationType::Linear,
            ));
        self.aperture = value;
    }

    pub fn fov_at(&self, frame: u32) -> f32 {
        self.fov_animation
            .as_ref()
            .map(|a| a.evaluate(frame))
            .unwrap_or(self.fov_degrees)
    }

    pub fn focus_distance_at(&self, frame: u32) -> f32 {
        self.focus_distance_animation
            .as_ref()
            .map(|a| a.evaluate(frame))
            .unwrap_or(self.focus_distance)
    }

    pub fn aperture_at(&self, frame: u32) -> f32 {
        self.aperture_animation
            .as_ref()
            .map(|a| a.evaluate(frame))
            .unwrap_or(self.aperture)
    }

    pub fn dof_max_blur_at(&self, frame: u32) -> f32 {
        self.dof_max_blur_animation
            .as_ref()
            .map(|a| a.evaluate(frame))
            .unwrap_or(self.dof_max_blur)
    }

    pub fn dof_enabled_at(&self, frame: u32) -> bool {
        self.dof_enabled_animation
            .as_ref()
            .map(|a| a.evaluate(frame) >= 0.5)
            .unwrap_or(self.dof_enabled)
    }

    pub fn set_dof_enabled_at(&mut self, frame: u32, enabled: bool) {
        let legacy = if self.dof_enabled { 1.0 } else { 0.0 };
        self.dof_enabled_animation
            .get_or_insert_with(|| Animatable::new_constant(legacy))
            .set_keyframe(crate::core::keyframe::Keyframe::new(
                frame,
                if enabled { 1.0 } else { 0.0 },
                crate::core::keyframe::InterpolationType::Linear,
            ));
        self.dof_enabled = enabled;
    }

    pub fn set_dof_max_blur_at(&mut self, frame: u32, value: f32) {
        let value = value.clamp(1.0, 64.0);
        let legacy = self.dof_max_blur;
        self.dof_max_blur_animation
            .get_or_insert_with(|| Animatable::new_constant(legacy))
            .set_keyframe(crate::core::keyframe::Keyframe::new(
                frame,
                value,
                crate::core::keyframe::InterpolationType::Linear,
            ));
        self.dof_max_blur = value;
    }
}

/// Project a world-space point through the composition camera to normalized
/// screen coordinates ([0..1], origin top-left). Returns None when the point is
/// behind the camera. Mirrors the simplified camera model (translate + Z-rotate)
/// used by the renderers' `perspective_project_layer`.
pub fn project_point_to_screen(
    cam: &Camera3D,
    point: [f32; 3],
    screen_width: f32,
    screen_height: f32,
) -> Option<[f32; 2]> {
    project_point_to_screen_at_frame(cam, point, screen_width, screen_height, 0)
}

pub fn project_point_to_screen_at_frame(
    cam: &Camera3D,
    point: [f32; 3],
    screen_width: f32,
    screen_height: f32,
    frame: u32,
) -> Option<[f32; 2]> {
    if screen_width <= 0.0 || screen_height <= 0.0 {
        return None;
    }
    let cam_pos = cam.transform.position.evaluate(frame);
    let cam_rot = cam.transform.rotation.evaluate(frame);
    let to_rad = |d: f32| d * std::f32::consts::PI / 180.0;
    let cam_zr = to_rad(cam_rot[2]);
    let (ccrz, ssrz) = (cam_zr.cos(), cam_zr.sin());

    let dx = point[0] - cam_pos[0];
    let dy = point[1] - cam_pos[1];
    let dz = point[2] - cam_pos[2];
    let cx = dx * ccrz - dy * ssrz;
    let cy = dx * ssrz + dy * ccrz;
    let cz = dz;
    if cz <= 0.1 {
        return None;
    }

    let fov_rad = cam.fov_at(frame).max(1.0) * std::f32::consts::PI / 180.0;
    let focal = (screen_height * 0.5) / (fov_rad * 0.5).tan();
    let sx = screen_width * 0.5 + (cx * focal) / cz;
    let sy = screen_height * 0.5 - (cy * focal) / cz;
    Some([sx / screen_width, sy / screen_height])
}
