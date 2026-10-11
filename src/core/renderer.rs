use crate::core::effect_plugin::evaluate_effects;
use crate::core::timeline::{
    Composition, Layer, LayerType, ShapeFillType, ShapeType, TrackMatteMode,
};
use half::f16;

use std::sync::Arc;

mod cache;
mod cache_identity;
mod helpers;
mod mask_raster;
mod media_textures;
mod ram_preview;
mod render_encoder;
mod render_pass;
mod resources;
mod setup;
mod text_raster;
mod track_matte;
mod uniform_builder;
use cache::VideoFrameKey;
pub(crate) use cache::{RenderKey, TextTextureCache, TextTextureKey, VideoFrameCache};
pub(crate) use cache_identity::{
    composition_cache_key, composition_media_revision, media_file_revision,
};
use helpers::*;
pub(crate) use mask_raster::*;
pub(crate) use text_raster::{bake_text_stroke, TextRasterParams};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 0.5],
        tex_coords: [0.0, 0.0],
    }, // Top-Left
    Vertex {
        position: [-0.5, -0.5],
        tex_coords: [0.0, 1.0],
    }, // Bottom-Left
    Vertex {
        position: [0.5, -0.5],
        tex_coords: [1.0, 1.0],
    }, // Bottom-Right
    Vertex {
        position: [0.5, 0.5],
        tex_coords: [1.0, 0.0],
    }, // Top-Right
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalsUniform {
    viewport_size: [f32; 2],
    exposure_ev: f32,
    lut_mode: u32,
    shadow_enabled: u32,
    shadow_strength: f32,
}

/// Compile-time proof that LayerUniform is non-zero-sized, embedded directly
/// into bind-group setup — no runtime unwrap needed anywhere.
const LAYER_UNIFORM_SIZE: std::num::NonZeroU64 =
    std::num::NonZeroU64::new(std::mem::size_of::<LayerUniform>() as u64)
        .expect("LayerUniform holds f32 fields and can never be zero-sized");
const LAYER_UNIFORM_STRIDE: usize = (std::mem::size_of::<LayerUniform>() + 255) & !255;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LayerUniform {
    transform_matrix: [[f32; 4]; 4],
    color: [f32; 4],
    opacity: f32,
    layer_type: u32,
    shape_type: u32,

    effect_tint_enabled: u32,
    effect_tint_color: [f32; 4],
    effect_tint_intensity: f32,
    effect_blur_enabled: u32,
    effect_blur_radius: f32,

    effect_shadow_enabled: u32,
    effect_shadow_color: [f32; 4],
    effect_shadow_opacity: f32,
    effect_shadow_direction: f32,
    effect_shadow_distance: f32,
    effect_shadow_softness: f32,

    effect_ca_enabled: u32,
    effect_ca_shift_r: f32,
    effect_ca_shift_b: f32,
    effect_ca_edge_falloff: f32,

    effect_vignette_enabled: u32,
    effect_vignette_intensity: f32,
    effect_vignette_roundness: f32,
    effect_vignette_feather: f32,
    effect_vignette_color: [f32; 4],
    blend_mode: u32,

    // Levels Adjustment
    levels_enabled: u32,
    levels_in_black: f32,
    levels_in_white: f32,
    levels_gamma: f32,
    levels_out_black: f32,
    levels_out_white: f32,

    // Hue / Saturation
    huesat_enabled: u32,
    huesat_hue: f32,
    huesat_sat: f32,
    huesat_light: f32,

    // Glow / Bloom
    glow_enabled: u32,
    glow_threshold: f32,
    glow_radius: f32,
    glow_intensity: f32,
    glow_color: [f32; 4],

    // Physical Film Grain
    grain_enabled: u32,
    grain_intensity: f32,
    grain_size: f32,

    // Track Matte System
    track_matte_mode: u32,

    // Shape params: x = polygon/star point count, y = rectangle corner radius (px)
    shape_params: [f32; 4],

    // Mesh Warp / Corner Pin
    meshwarp_enabled: u32,
    corner_top_left: [f32; 2],
    corner_top_right: [f32; 2],
    corner_bottom_left: [f32; 2],
    corner_bottom_right: [f32; 2],

    // Motion Blur
    motionblur_enabled: u32,
    motionblur_shutter: f32,
    motionblur_velocity_x: f32,
    motionblur_velocity_y: f32,
    motionblur_samples: u32,

    // Lens Flare
    flare_enabled: u32,
    flare_pos_x: f32,
    flare_pos_y: f32,
    flare_intensity: f32,
    flare_threshold: f32,
    flare_color: [f32; 4],

    // Per-layer GPU mask flags (coverage baked CPU-side; see rasterize_layer_masks)
    mask_enabled: u32,
    mask_mode: u32,
    mask_inverted: u32,
    mask_feather: f32,

    // TrimPaths (angular trim on shape SDF)
    trim_start: f32,
    trim_end: f32,
    trim_offset: f32,
    _pad_trim: f32,

    // Shape fill gradient (0=solid, 1=linear, 2=radial)
    fill_type: u32,
    grad_start_x: f32,
    grad_start_y: f32,
    grad_end_x: f32,
    grad_end_y: f32,
    grad_color1_r: f32,
    grad_color1_g: f32,
    grad_color1_b: f32,
    grad_color1_a: f32,
    grad_color2_r: f32,
    grad_color2_g: f32,
    grad_color2_b: f32,
    grad_color2_a: f32,
    grad_center_x: f32,
    grad_center_y: f32,
    grad_radius: f32,
    _grad_pad: f32,

    // Layer Styles
    ls_stroke_width: f32,
    ls_stroke_r: f32,
    ls_stroke_g: f32,
    ls_stroke_b: f32,
    ls_color_overlay_r: f32,
    ls_color_overlay_g: f32,
    ls_color_overlay_b: f32,
    ls_color_overlay_a: f32,
    ls_gradient_start_x: f32,
    ls_gradient_start_y: f32,
    ls_gradient_end_x: f32,
    ls_gradient_end_y: f32,
    ls_gradient_color1_r: f32,
    ls_gradient_color1_g: f32,
    ls_gradient_color1_b: f32,
    ls_gradient_color1_a: f32,
    ls_gradient_color2_r: f32,
    ls_gradient_color2_g: f32,
    ls_gradient_color2_b: f32,
    ls_gradient_color2_a: f32,
    ls_inner_shadow_offset_x: f32,
    ls_inner_shadow_offset_y: f32,
    ls_inner_shadow_size: f32,
    ls_inner_shadow_opacity: f32,
    ls_inner_shadow_r: f32,
    ls_inner_shadow_g: f32,
    ls_inner_shadow_b: f32,
    ls_bevel_size: f32,
    ls_bevel_angle: f32,
    ls_bevel_strength: f32,
    ls_bevel_light_r: f32,
    ls_bevel_light_g: f32,
    ls_bevel_light_b: f32,
    ls_style_flags: u32,
    _ls_pad1: f32,
    _ls_pad2: f32,
    _ls_pad3: f32,

    // 3D Extrusion
    extrusion_depth: f32,
    bevel_depth: f32,

    // ── GPU Real-time VFX Shader Extensions ──
    // Chromatic Aberration
    chromatic_enabled: u32,
    chromatic_amount: f32,
    chromatic_angle: f32,
    _pad_chromatic: f32,

    // Vignette
    vignette_enabled: u32,
    vignette_amount: f32,
    vignette_midpoint: f32,
    vignette_feather: f32,

    // Invert & Posterize
    invert_enabled: u32,
    posterize_enabled: u32,
    posterize_levels: f32,
    threshold_level: f32,

    // Tint
    tint_enabled: u32,
    tint_amount: f32,
    _pad_tint1: f32,
    _pad_tint2: f32,
    tint_black: [f32; 4],
    tint_white: [f32; 4],

    // CRT Scanlines
    crt_enabled: u32,
    crt_scanline_count: f32,
    crt_scanline_intensity: f32,
    crt_curvature: f32,

    // ── GPU Simulation Effects ──
    sim_enabled: u32,
    sim_type: u32,
    sim_p1: f32,
    sim_p2: f32,
    sim_p3: f32,
    sim_p4: f32,
    sim_p5: f32,
    sim_p6: f32,

    _padding_align: [f32; 4],
    _ls_pad4: f32,
    _ls_pad5: f32,
}

// ─── GPU Layer Mask Rasterization (CPU-baked coverage → group(3) texture) ──

/// One evaluated mask shape, already scaled into output pixel space.
#[derive(Debug, Clone)]
pub(crate) struct MaskShape {
    pub poly: Vec<[f32; 2]>,
    pub mode: crate::core::mask::MaskMode,
    pub opacity: f32,
    pub inverted: bool,
    /// Feather radius in output pixels (0 = hard edge).
    pub feather: f32,
}

/// Approximate byte budget for cached Rgba16Float source textures.
pub const MAX_VIDEO_FRAME_TEXTURE_CACHE_BYTES: usize = VideoFrameCache::DEFAULT_MAX_BYTES;
/// FIFO cap for cached text-layer textures: each entry can be large
/// (font_size × ~10 rows), so long editing sessions must not grow it forever.
pub const MAX_TEXT_TEXTURES: usize = 64;

#[allow(dead_code)]
pub struct WgpuRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    render_pipeline: wgpu::RenderPipeline,
    render_pipeline_screen: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,

    layer_buffer: wgpu::Buffer,
    layer_bind_group_layout: wgpu::BindGroupLayout,
    layer_bind_group: wgpu::BindGroup,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    dummy_texture_bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,

    // Combined mask+shadow+matte bind group (group 3, bindings 0-5)
    combined_bind_group_layout: wgpu::BindGroupLayout,
    combined_bind_group: wgpu::BindGroup,
    dummy_mask_view: wgpu::TextureView,
    dummy_shadow_view: wgpu::TextureView,
    dummy_matte_view: wgpu::TextureView,

    // Shared layer-mask resources
    mask_texture: Option<wgpu::Texture>,
    mask_view: Option<wgpu::TextureView>,
    mask_size: (u32, u32),

    // Shadow density map: CPU-built, GPU-sampled for preview parity
    shadow_texture: Option<wgpu::Texture>,
    shadow_view: Option<wgpu::TextureView>,
    shadow_size: (u32, u32),
    shadow_active: bool,
    /// Content-addressed cache so static masks skip EDT re-raster during
    /// playback of other layers.
    mask_raster_cache: std::cell::RefCell<MaskRasterCache>,

    // Target offscreen texture
    pub target_texture: Option<wgpu::Texture>,
    pub target_view: Option<wgpu::TextureView>,
    pub target_size: (u32, u32),

    // Snapshot target offscreen texture
    pub snapshot_texture: Option<wgpu::Texture>,
    pub snapshot_view: Option<wgpu::TextureView>,

    // Dirty-checking: skip re-render when inputs are unchanged.
    // Keyed by (version, frame, exposure bits, lut, target size) per target type.
    last_main_key: Option<RenderKey>,
    last_snapshot_key: Option<RenderKey>,

    // ── RAM preview ring ──
    // Pre-rendered frames for smooth playback, produced by render_ram_preview.
    // Invalidated wholesale when the project version changes.
    ram_ring: Vec<(u32, Option<(wgpu::Texture, wgpu::TextureView)>)>,
    ram_ring_version: u64,
    /// Index into ram_ring used while rendering a pre-pass frame.
    ram_render_idx: usize,

    /// Optional cap on preview render width (px). When set, large compositions
    /// are rendered at a downscaled resolution — the viewport samples the
    /// texture at display size anyway, so this is visually near-free and can
    /// cut fill-rate by 4-16x on 4K comps.
    preview_max_width: Option<u32>,
    /// WgpuRenderer is a preview-only target; final/export paths use the
    /// software renderer and never consult layer proxy media.
    preview_proxies: bool,

    // GPU text rendering: cache of CPU-rasterized text textures keyed by (layer_id, text, font_size)
    text_texture_cache: std::cell::RefCell<TextTextureCache>,
    video_frame_cache: std::cell::RefCell<VideoFrameCache>,
    video_cache_version: std::cell::Cell<u64>,
}

/// The layer uniform buffer is allocated for this many entries.  Keeping the
/// limit named prevents repeater expansion and draw submission from drifting
/// apart when the buffer layout changes.
const MAX_GPU_LAYERS: usize = 256;

impl WgpuRenderer {
    /// Current layer uniform buffer size in bytes. Grows past the initial
    /// `MAX_GPU_LAYERS` allocation when oversized compositions render.
    pub fn layer_buffer_size(&self) -> u64 {
        self.layer_buffer.size()
    }

    /// Pre-renders `from..=to` into the RAM preview ring. Called incrementally
    /// (a few frames per UI frame) so the UI stays responsive during the pass.
    ///
    /// `total_count` is the final ring size for this pre-pass; it is allocated on
    /// the first call and reused afterwards. Frames already in the ring are
    /// skipped, so repeated calls with advancing ranges are cheap.
    /// Whether shadow sampling is currently enabled (map uploaded).
    /// Caps preview render width (px). `None` renders at composition resolution.
    pub fn set_preview_max_width(&mut self, cap: Option<u32>) {
        if self.preview_max_width != cap {
            self.preview_max_width = cap;
            // Resolution change must invalidate the dirty-check keys
            self.last_main_key = None;
            self.last_snapshot_key = None;
        }
    }

    /// Renders the given composition at the specified frame, returning the texture view.
    pub fn render(
        &mut self,
        comp: &Composition,
        frame: u32,
        exposure_ev: f32,
        lut_mode: u32,
    ) -> (&wgpu::TextureView, bool) {
        let recreated = self.render_internal(comp, frame, exposure_ev, lut_mode, false);
        if self.target_view.is_none() {
            log::error!("[WgpuRenderer] render(): target view missing; using fallback view");
            self.dummy_view_or_create(false);
        }
        (
            self.target_view
                .as_ref()
                .expect("fallback view just created"),
            recreated,
        )
    }

    /// Renders the given composition at the specified frame to the snapshot target, returning the snapshot texture view.
    pub fn render_snapshot_frame(
        &mut self,
        comp: &Composition,
        frame: u32,
        exposure_ev: f32,
        lut_mode: u32,
    ) -> (&wgpu::TextureView, bool) {
        let recreated = self.render_internal(comp, frame, exposure_ev, lut_mode, true);
        if self.snapshot_view.is_none() {
            log::error!("[WgpuRenderer] render_snapshot_frame(): snapshot view missing; using fallback view");
            self.dummy_view_or_create(true);
        }
        (
            self.snapshot_view
                .as_ref()
                .expect("fallback view just created"),
            recreated,
        )
    }

    /// Last-resort 1x1 fallback view so a missing target can never panic the UI thread.
    fn dummy_view_or_create(&mut self, snapshot: bool) -> &wgpu::TextureView {
        let slot = if snapshot {
            &mut self.snapshot_view
        } else {
            &mut self.target_view
        };
        if slot.is_none() {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Fallback 1x1 Target"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            *slot = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        }
        // Invariant: the slot was just populated above (or already held a view).
        slot.as_ref()
            .expect("fallback target view was just created")
    }
}

/// Helper to align dynamic uniform buffer byte offsets against WGPU hardware limits.
/// Dynamically adapts to device.limits().min_uniform_buffer_offset_alignment (e.g. 64B, 256B, 512B).
#[allow(dead_code)]
pub fn align_uniform_buffer_offset(offset: u64, alignment: u32) -> u64 {
    let align = alignment as u64;
    if align == 0 {
        return offset;
    }
    (offset + align - 1) & !(align - 1)
}

#[cfg(test)]
#[path = "renderer/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "renderer/video_cache_tests.rs"]
mod video_cache_tests;

// ── GPU Compositing Pipeline (future) ──
// The full GPU compositing pipeline would:
// 1. Upload each layer's rasterized content as a separate texture
// 2. Use the composite.wgsl compute shader to blend all layers on GPU
// 3. Apply effects as fragment shader passes between blends
// 4. Output directly to the display texture without CPU roundtrip
//
// Current status: active layers are composited via per-layer GPU draws
// with matte bind groups on the GPU. The CPU path (software_renderer)
// remains the reference implementation for export/fallback.
