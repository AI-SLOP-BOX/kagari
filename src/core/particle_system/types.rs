use crate::core::particle_forces::LifeCurve;
use serde::{Deserialize, Serialize};

/// Emitter shape type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EmitterShape {
    #[default]
    Point = 0,
    Box = 1,
    Circle = 2,
    Line = 3,
    /// Emission from a thin circle at `emitter_size[0]` diameter.
    Ring = 4,
}

/// Opacity fade curve over particle lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FadeCurve {
    #[default]
    Linear = 0,
    EaseIn = 1,
    EaseOut = 2,
}

impl FadeCurve {
    /// Map normalized lifetime progress t (0..1) to fade factor (0..1).
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            FadeCurve::Linear => 1.0 - t,
            FadeCurve::EaseIn => 1.0 - t * t,
            FadeCurve::EaseOut => (1.0 - t) * (1.0 - t),
        }
    }
}

/// Particle emitter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleEmitter {
    /// Particles per second
    pub rate: f32,
    /// Max particles alive at once
    pub max_particles: u32,
    /// Particle lifetime in seconds
    pub lifetime: f32,
    /// Random lifetime variation (0..1)
    pub lifetime_variance: f32,
    /// Initial speed (pixels/sec)
    pub speed: f32,
    /// Random speed variation (0..1)
    pub speed_variance: f32,
    /// Spread angle in degrees (0 = one direction, 360 = all directions)
    pub spread_degrees: f32,
    /// Emitter shape
    pub shape: EmitterShape,
    /// Box emitter size [width, height]
    pub emitter_size: [f32; 2],
    /// Gravity [x, y] (pixels/sec^2)
    pub gravity: [f32; 2],
    /// Wind [x, y] (pixels/sec)
    pub wind: [f32; 2],
    /// Turbulence strength
    pub turbulence: f32,
    /// Start color [r, g, b, a]
    pub color_start: [f32; 4],
    /// End color [r, g, b, a]
    pub color_end: [f32; 4],
    /// Start size (pixels)
    pub size_start: f32,
    /// End size (pixels)
    pub size_end: f32,
    /// Start opacity (0..1)
    pub opacity_start: f32,
    /// End opacity (0..1)
    pub opacity_end: f32,
    /// Rotation speed (degrees/sec)
    #[serde(default)]
    pub rotation_speed: f32,
    /// Initial rotation (degrees)
    #[serde(default)]
    pub rotation_start: f32,
    /// Random rotation speed variation (0..1)
    #[serde(default)]
    pub rotation_speed_variance: f32,
    /// Opacity fade curve over lifetime
    #[serde(default)]
    pub fade_curve: FadeCurve,
    /// Blend mode: 0=Normal, 1=Add, 2=Screen
    pub blend_mode: u32,
    /// Gravity multiplier sampled over normalized particle lifetime.
    #[serde(default)]
    pub gravity_curve: LifeCurve,
    /// Peak sinusoidal wind gust strength (px/s^2), 0 = steady wind only.
    #[serde(default)]
    pub wind_gust_strength: f32,
    /// Wind gust frequency in Hz.
    #[serde(default)]
    pub wind_gust_frequency: f32,
    /// Exponential air drag coefficient (1/s).
    #[serde(default)]
    pub drag: f32,
    /// Enable axis-aligned boundary collisions.
    #[serde(default)]
    pub collision_enabled: bool,
    /// Collision bounds [min_x, min_y, max_x, max_y].
    #[serde(default)]
    pub collision_bounds: [f32; 4],
    /// Bounce energy retention on collision (0..1).
    #[serde(default)]
    pub restitution: f32,
    /// Tangential velocity retention on collision (0..1).
    #[serde(default)]
    pub surface_friction: f32,
    /// Particle-vs-particle soft-sphere collisions (O(n²) per update step).
    #[serde(default)]
    pub particle_collisions: bool,
    /// Contact diameter for particle-vs-particle collisions (px).
    #[serde(default = "default_particle_diameter")]
    pub particle_diameter: f32,
    /// Trail length: number of previous positions to store (0 = no trail)
    #[serde(default)]
    pub trail_length: u8,
    /// Trail taper: alpha multiplier per trail step (0..1, 1 = no fade)
    #[serde(default = "default_trail_taper")]
    pub trail_taper: f32,
    /// Vortex tangential acceleration around `vortex_center` (px/s^2, signed:
    /// positive spins clockwise in screen space, 0 = off)
    #[serde(default)]
    pub vortex_strength: f32,
    /// Vortex center [x, y]
    #[serde(default)]
    pub vortex_center: [f32; 2],
    /// Attraction (+) / repulsion (-) toward `attract_center` (px/s^2)
    #[serde(default)]
    pub attract_strength: f32,
    /// Attractor point [x, y]
    #[serde(default)]
    pub attract_center: [f32; 2],
    /// Enable per-particle Z depth for camera-projected rendering
    #[serde(default)]
    pub depth_enabled: bool,
    /// Spawn Z range [min, max] offsets from the layer plane (world units)
    #[serde(default)]
    pub depth_range: [f32; 2],
    /// Child particles emitted radially when a particle dies (0 = off)
    #[serde(default)]
    pub death_spawn_count: u32,
    /// Child initial speed = parent emitter speed × this (0..1 typical)
    #[serde(default = "default_death_speed_scale")]
    pub death_spawn_speed_scale: f32,
    /// Child lifetime = emitter lifetime × this
    #[serde(default = "default_death_life_scale")]
    pub death_spawn_life_scale: f32,
}

fn default_death_speed_scale() -> f32 {
    0.5
}
fn default_death_life_scale() -> f32 {
    0.5
}
fn default_particle_diameter() -> f32 {
    8.0
}
fn default_trail_taper() -> f32 {
    0.7
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            rate: 50.0,
            max_particles: 1000,
            lifetime: 2.0,
            lifetime_variance: 0.2,
            speed: 200.0,
            speed_variance: 0.3,
            spread_degrees: 360.0,
            shape: EmitterShape::Point,
            emitter_size: [100.0, 100.0],
            gravity: [0.0, 300.0],
            wind: [0.0, 0.0],
            turbulence: 0.0,
            color_start: [1.0, 0.8, 0.2, 1.0],
            color_end: [1.0, 0.1, 0.0, 0.0],
            size_start: 8.0,
            size_end: 2.0,
            opacity_start: 1.0,
            opacity_end: 0.0,
            rotation_speed: 0.0,
            rotation_start: 0.0,
            rotation_speed_variance: 0.0,
            fade_curve: FadeCurve::Linear,
            blend_mode: 1, // Add
            gravity_curve: LifeCurve::default(),
            wind_gust_strength: 0.0,
            wind_gust_frequency: 0.5,
            drag: 0.0,
            collision_enabled: false,
            collision_bounds: [0.0, 0.0, 1920.0, 1080.0],
            restitution: 0.5,
            surface_friction: 0.9,
            particle_collisions: false,
            particle_diameter: 8.0,
            trail_length: 0,
            trail_taper: 0.7,
            vortex_strength: 0.0,
            vortex_center: [0.0, 0.0],
            attract_strength: 0.0,
            attract_center: [0.0, 0.0],
            death_spawn_count: 0,
            death_spawn_speed_scale: 0.5,
            death_spawn_life_scale: 0.5,
            depth_enabled: false,
            depth_range: [0.0, 0.0],
        }
    }
}

/// State of a single alive particle.
#[derive(Debug, Clone)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub rotation: f32,
    /// Per-particle angular velocity (degrees/sec), set at emission
    pub angular_velocity: f32,
    /// Trail history: last N positions (newest first), ring buffer
    pub trail: [(f32, f32); 8],
    pub trail_len: u8,
    /// Depth offset (world Z) for camera-projected rendering
    pub z: f32,
}

/// Camera model for projected particle rendering.
#[derive(Debug, Clone, Copy)]
pub struct CameraProjection {
    pub cam_x: f32,
    pub cam_y: f32,
    pub cam_z: f32,
    /// Focal length in pixels derived from vertical FOV and output height
    pub focal: f32,
    pub cos_rz: f32,
    pub sin_rz: f32,
}

impl CameraProjection {
    /// World (x,y,z) -> screen (sx, sy, size-scale). None when behind camera.
    pub fn project(&self, x: f32, y: f32, z: f32) -> Option<(f32, f32, f32)> {
        let dz = z - self.cam_z;
        if dz <= 0.1 {
            return None;
        }
        let s = self.focal / dz;
        let rx = x - self.cam_x;
        let ry = y - self.cam_y;
        let sx = self.cam_x + (rx * self.cos_rz - ry * self.sin_rz) * s;
        let sy = self.cam_y - (rx * self.sin_rz + ry * self.cos_rz) * s;
        // Size scales relative to the z=0 plane; guard degenerate on-plane cams
        let ref_dz = (-self.cam_z).abs().max(100.0);
        Some((sx, sy, (ref_dz / dz).max(0.01)))
    }
}
