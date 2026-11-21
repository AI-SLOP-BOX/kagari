use super::*;

#[derive(Clone, Copy)]
pub(super) enum TimelineClipDragMode {
    Move,
    TrimStart,
    TrimEnd,
    Keyframe,
}

#[derive(Clone, Copy)]
pub(super) enum TimelineWorkAreaDragMode {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TimelineView {
    All,
    Animation,
    CurrentKeyframes,
    Audio,
    Media,
    Text,
    Shapes,
    ThreeD,
    Cameras,
    Lights,
    Markers,
    Graph,
    Flowchart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PropertiesSection {
    Transform,
    Content,
    Composite,
    Advanced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GraphChannel {
    PositionX,
    PositionY,
    AnchorX,
    AnchorY,
    ScaleX,
    ScaleY,
    Rotation,
    Opacity,
    Position3DX,
    Position3DY,
    Position3DZ,
    Rotation3DX,
    Rotation3DY,
    Rotation3DZ,
    Scale3DX,
    Scale3DY,
    Scale3DZ,
    AudioVolume,
    TimeRemap,
    CameraFov,
    CameraFocus,
    CameraAperture,
    CameraDofBlur,
    CameraDofEnabled,
    LightIntensity,
    MaskFeather,
    MaskOpacity,
    MaskExpansion,
    ShapeWidth,
    ShapeHeight,
    ShapeCornerRadius,
    ShapePoints,
    ShapeInnerRadius,
    ShapeOuterRadius,
    ShapeSides,
    ShapeRadius,
    EffectParameter,
    EffectColorRed,
    EffectColorGreen,
    EffectColorBlue,
    EffectColorAlpha,
}

pub(super) fn action_edits_selected_layer(action: &str) -> bool {
    pub(super) const PREFIXES: &[&str] = &[
        "position_",
        "anchor_",
        "scale_",
        "rotation_",
        "opacity_",
        "material_",
        "toggle_material_",
        "text_",
        "shape_",
        "toggle_shape_",
        "selected_camera_",
        "selected_light_",
        "toggle_selected_camera_",
        "toggle_selected_light_",
        "solid_color_",
        "mask_",
        "toggle_mask_",
        "cycle_mask_",
        "effect_parameter_",
        "effect_color_",
        "emitter_",
        "audio_volume_",
        "video_speed_",
        "video_posterize_",
        "time_remap_",
        "cycle_layer_",
        "auto_orient_",
        "graph_add_",
        "keyframe_",
        "graph_cycle_",
        "text_animator_",
    ];
    if PREFIXES.iter().any(|prefix| action.starts_with(prefix)) {
        return true;
    }
    matches!(
        action,
        "cycle_selected_light_type"
            | "select_light_type"
            | "select_layer_proxy_resolution"
            | "select_mask_mode"
            | "paste_effect_to_layer"
            | "duplicate_selected_effect"
            | "cycle_shape_fill_type"
            | "select_shape_fill_type"
            | "select_composition_bit_depth"
            | "select_composition_working_space"
            | "toggle_video_time_remap"
            | "toggle_video_posterize"
            | "toggle_frame_blending"
            | "toggle_preserve_transparency"
            | "toggle_guide_layer"
            | "toggle_shy_layer"
            | "toggle_layer_effects"
            | "toggle_selected_layer_motion_blur"
            | "toggle_precomp_collapse_transforms"
            | "toggle_layer_style"
            | "effect_parameter_slider_changed"
            | "text_animator_parameter_slider_changed"
            | "layer_style_parameter_changed"
            | "layer_style_parameter_slider_changed"
            | "reset_color_wheels"
            | "cycle_constraint_horizontal"
            | "select_constraint_horizontal"
            | "cycle_constraint_vertical"
            | "select_constraint_vertical"
            | "select_text_animator_shape"
            | "layer_in_frame_changed"
            | "layer_out_frame_changed"
            | "layer_in_to_playhead"
            | "layer_out_to_playhead"
            | "cycle_auto_orient"
            | "select_auto_orient"
            | "select_layer_label_color"
            | "select_track_matte_source"
            | "clear_track_matte_source"
            | "add_layer_marker"
            | "remove_layer_marker"
            | "layer_marker_label_changed"
            | "layer_marker_frame_changed"
            | "add_keyframe"
            | "graph_remove_keyframe"
            | "ease_selected_layer"
            | "split_selected_layer_at_playhead"
            | "set_selected_layer_in_to_playhead"
            | "set_selected_layer_out_to_playhead"
            | "parent_selected_layer"
            | "set_layer_parent"
            | "unparent_selected_layer"
            | "layer_changed"
            | "move_layer_up"
            | "move_layer_down"
            | "duplicate_selected_layer"
            | "toggle_effect"
            | "fit_selected_image_to_composition"
            | "fill_selected_image_to_composition"
            | "cycle_layer_proxy_resolution"
            | "toggle_layer_proxy"
            | "clear_layer_proxy"
            | "generate_layer_proxy"
            | "remove_effect"
            | "move_effect_up"
            | "move_effect_down"
            | "add_gaussian_blur"
            | "add_effect_catalog"
            | "add_color_tint"
            | "add_drop_shadow"
            | "add_glow"
            | "add_vignette"
            | "add_hue_saturation"
            | "add_levels"
            | "add_motion_blur"
            | "add_film_grain"
            | "add_posterize"
            | "add_directional_blur"
            | "add_radial_blur"
            | "add_sharpen"
            | "add_chroma_key"
            | "add_curves"
            | "add_chromatic_aberration"
            | "add_lens_correction"
            | "add_fractal_noise"
            | "add_glitch_displacement"
            | "add_glow_pro"
            | "add_white_balance"
            | "add_turbulent_displace"
            | "add_cc_lens"
            | "add_vortex"
            | "add_light_sweep"
            | "add_luma_key_range"
            | "add_tritone"
            | "add_displacement_map"
            | "add_compound_blur"
            | "add_crt_scanlines"
            | "add_polar_coordinates"
            | "add_wave_warp"
            | "add_mosaic"
            | "add_emboss"
            | "add_bevel_alpha"
            | "add_median_filter"
            | "add_sobel_edges"
            | "add_corner_pin"
            | "add_colorama"
            | "add_rect_mask"
            | "add_ellipse_mask"
            | "remove_mask"
    )
}

impl GraphChannel {
    pub(super) const ALL: [Self; 41] = [
        Self::PositionX,
        Self::PositionY,
        Self::AnchorX,
        Self::AnchorY,
        Self::ScaleX,
        Self::ScaleY,
        Self::Rotation,
        Self::Opacity,
        Self::Position3DX,
        Self::Position3DY,
        Self::Position3DZ,
        Self::Rotation3DX,
        Self::Rotation3DY,
        Self::Rotation3DZ,
        Self::Scale3DX,
        Self::Scale3DY,
        Self::Scale3DZ,
        Self::AudioVolume,
        Self::TimeRemap,
        Self::CameraFov,
        Self::CameraFocus,
        Self::CameraAperture,
        Self::CameraDofBlur,
        Self::CameraDofEnabled,
        Self::LightIntensity,
        Self::MaskFeather,
        Self::MaskOpacity,
        Self::MaskExpansion,
        Self::ShapeWidth,
        Self::ShapeHeight,
        Self::ShapeCornerRadius,
        Self::ShapePoints,
        Self::ShapeInnerRadius,
        Self::ShapeOuterRadius,
        Self::ShapeSides,
        Self::ShapeRadius,
        Self::EffectParameter,
        Self::EffectColorRed,
        Self::EffectColorGreen,
        Self::EffectColorBlue,
        Self::EffectColorAlpha,
    ];

    pub(super) fn is_3d(self) -> bool {
        matches!(
            self,
            Self::Position3DX
                | Self::Position3DY
                | Self::Position3DZ
                | Self::Rotation3DX
                | Self::Rotation3DY
                | Self::Rotation3DZ
                | Self::Scale3DX
                | Self::Scale3DY
                | Self::Scale3DZ
        )
    }

    pub(super) fn supports(self, layer: &kagari_vfx::core::timeline::Layer) -> bool {
        use kagari_vfx::core::timeline::LayerType;
        if self.is_3d() {
            return layer.is_3d;
        }
        match self {
            Self::AudioVolume => matches!(
                &layer.layer_type,
                LayerType::Audio { .. }
                    | LayerType::Video {
                        audio_wav: Some(_),
                        ..
                    }
            ),
            Self::TimeRemap => layer.time_remap.is_some(),
            Self::CameraFov
            | Self::CameraFocus
            | Self::CameraAperture
            | Self::CameraDofBlur
            | Self::CameraDofEnabled => {
                matches!(
                    layer.scene_object.as_ref(),
                    Some(kagari_vfx::core::timeline::SceneObjectRef::Camera { .. })
                )
            }
            Self::LightIntensity => matches!(
                layer.scene_object.as_ref(),
                Some(kagari_vfx::core::timeline::SceneObjectRef::Light { .. })
            ),
            Self::MaskFeather | Self::MaskOpacity | Self::MaskExpansion => !layer.masks.is_empty(),
            Self::ShapeWidth
            | Self::ShapeHeight
            | Self::ShapeCornerRadius
            | Self::ShapePoints
            | Self::ShapeInnerRadius
            | Self::ShapeOuterRadius
            | Self::ShapeSides
            | Self::ShapeRadius => shape_parameter_track(layer, self).is_some(),
            Self::EffectParameter => !layer.effects.is_empty(),
            Self::EffectColorRed
            | Self::EffectColorGreen
            | Self::EffectColorBlue
            | Self::EffectColorAlpha => layer
                .effects
                .iter()
                .any(|effect| !effect_color_parameters(&effect.effect_type).is_empty()),
            _ => true,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::PositionX => "位置 X",
            Self::PositionY => "位置 Y",
            Self::AnchorX => "基準点 X",
            Self::AnchorY => "基準点 Y",
            Self::ScaleX => "倍率 X",
            Self::ScaleY => "倍率 Y",
            Self::Rotation => "回転",
            Self::Opacity => "不透明度",
            Self::Position3DX => "3D位置 X",
            Self::Position3DY => "3D位置 Y",
            Self::Position3DZ => "3D位置 Z",
            Self::Rotation3DX => "3D回転 X",
            Self::Rotation3DY => "3D回転 Y",
            Self::Rotation3DZ => "3D回転 Z",
            Self::Scale3DX => "3D倍率 X",
            Self::Scale3DY => "3D倍率 Y",
            Self::Scale3DZ => "3D倍率 Z",
            Self::AudioVolume => "音量",
            Self::TimeRemap => "ソースフレーム",
            Self::CameraFov => "画角",
            Self::CameraFocus => "焦点距離",
            Self::CameraAperture => "絞り",
            Self::CameraDofBlur => "最大ぼけ",
            Self::CameraDofEnabled => "被写界深度",
            Self::LightIntensity => "ライト強度",
            Self::MaskFeather => "マスクぼかし",
            Self::MaskOpacity => "マスク不透明度",
            Self::MaskExpansion => "マスク拡張",
            Self::ShapeWidth => "シェイプ幅",
            Self::ShapeHeight => "シェイプ高さ",
            Self::ShapeCornerRadius => "角丸",
            Self::ShapePoints => "星の頂点数",
            Self::ShapeInnerRadius => "星の内半径",
            Self::ShapeOuterRadius => "星の外半径",
            Self::ShapeSides => "多角形の辺数",
            Self::ShapeRadius => "多角形の半径",
            Self::EffectParameter => "エフェクト値",
            Self::EffectColorRed => "色 R",
            Self::EffectColorGreen => "色 G",
            Self::EffectColorBlue => "色 B",
            Self::EffectColorAlpha => "色 A",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum MaskValueChannel {
    Feather,
    Opacity,
    Expansion,
}

#[derive(Clone, Copy)]
pub(super) enum LayerToggle {
    FrameBlending,
    MotionBlur,
    PreserveTransparency,
    Guide,
    Shy,
    Effects,
}

#[derive(Clone)]
pub(super) struct GraphKeyDrag {
    pub(super) layer_id: String,
    pub(super) channel: GraphChannel,
    pub(super) current_frame: u32,
    pub(super) min_value: f32,
    pub(super) max_value: f32,
    pub(super) mask_id: Option<String>,
    pub(super) effect_id: Option<String>,
    pub(super) effect_parameter_id: Option<String>,
    pub(super) snap_points: Vec<u32>,
}

#[derive(Clone)]
pub(super) struct MaskVertexDrag {
    pub(super) layer_id: String,
    pub(super) mask_id: String,
    pub(super) vertex_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CameraNavMode {
    Orbit,
    Pan,
    Dolly,
}

#[derive(Clone, Debug)]
pub(super) struct CameraNavDrag {
    pub(super) mode: CameraNavMode,
    pub(super) start_pos: [f32; 3],
    pub(super) start_rot: [f32; 3],
    pub(super) start_screen_point: [f64; 2],
}

#[derive(Clone, Debug)]
pub(super) struct LightGizmoDrag {
    pub(super) light_id: String,
    pub(super) start_pos_3d: [f32; 3],
    pub(super) start_point: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FlowchartWireKind {
    Parent,
    Matte,
}

#[derive(Clone, Debug)]
pub(super) struct FlowchartWireDrag {
    pub(super) source_layer_id: String,
    pub(super) kind: FlowchartWireKind,
    pub(super) start_pos: (f64, f64),
    pub(super) current_pos: (f64, f64),
}

#[derive(Clone)]
pub(super) struct TimelineClipDrag {
    pub(super) layer_id: String,
    pub(super) start_x: f64,
    pub(super) initial_in: u32,
    pub(super) initial_out: u32,
    pub(super) keyframe_source: Option<u32>,
    pub(super) keyframe_current: Option<u32>,
    pub(super) mode: TimelineClipDragMode,
}

pub(super) struct VideoImportJob {
    pub(super) worker: Option<std::thread::JoinHandle<()>>,
    pub(super) receiver:
        std::sync::mpsc::Receiver<Result<kagari_vfx::core::video_import::VideoAsset, String>>,
    pub(super) cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(super) cancel_supported: bool,
    pub(super) is_image_sequence: bool,
    pub(super) source_name: String,
    pub(super) layer_id: String,
    pub(super) replace_layer_id: Option<String>,
    pub(super) replace_project_asset_id: Option<String>,
    pub(super) register_project_asset: bool,
    pub(super) composition_id: String,
    pub(super) project_file_path: Option<std::path::PathBuf>,
    pub(super) project_name: String,
    pub(super) fps: u32,
    pub(super) project_generation: u64,
    pub(super) asset_parent_folder: Option<String>,
}

pub(super) struct LayerProxyGenerationJob {
    pub(super) worker: Option<std::thread::JoinHandle<()>>,
    pub(super) receiver: std::sync::mpsc::Receiver<LayerProxyGenerationMessage>,
    pub(super) cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(super) resolution: kagari_vfx::core::proxy::ProxyResolution,
    pub(super) layer_id: String,
    pub(super) composition_id: String,
    pub(super) project_file_path: Option<std::path::PathBuf>,
    pub(super) project_name: String,
    pub(super) project_generation: u64,
}

pub(super) struct AutosaveJob {
    pub(super) preview_revision: u64,
    pub(super) project_generation: u64,
    pub(super) project_path: std::path::PathBuf,
    pub(super) target_path: std::path::PathBuf,
    pub(super) receiver: std::sync::mpsc::Receiver<(Result<(), String>, Option<String>)>,
    pub(super) worker: Option<std::thread::JoinHandle<()>>,
}

pub(super) enum LayerProxyGenerationMessage {
    Progress { done: u32, total: u32 },
    Finished(Result<String, String>),
}

#[derive(Clone)]
pub(super) struct DetectedOfxPlugin {
    pub(super) name: String,
    pub(super) path: String,
}

#[derive(Clone, Debug)]
pub struct RecentProjectItem {
    pub path: std::path::PathBuf,
    pub name: String,
    pub path_label: String,
    pub date: String,
    pub duration: String,
    pub fps: String,
    pub resolution: String,
    pub layer_count: usize,
    pub comps_info: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Screen {
    Home,
    Projects,
    Activity,
    Documents,
    ProjectSetup,
    Editor,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Workspace {
    Default,
    Composition,
    Scene,
    Timeline,
    Animation,
    Graph,
    Effects,
    Audio,
    TimeRemap,
    Mask,
    Assets,
    Text,
    LayerStyle,
    Color,
    CameraLight,
}

pub(super) const WORKSPACE_LEFT_DEFAULTS: [f64; 15] = [
    0.22, 0.22, 0.30, 0.20, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.34, 0.22, 0.22, 0.22, 0.22,
];
pub(super) const WORKSPACE_CENTER_DEFAULTS: [f64; 15] = [
    0.70, 0.62, 0.72, 0.72, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70,
];
pub(super) const WORKSPACE_VERTICAL_DEFAULT: f64 = 0.54;

impl Workspace {
    pub(super) fn index(self) -> usize {
        match self {
            Self::Default => 0,
            Self::Composition => 1,
            Self::Scene => 2,
            Self::Timeline => 3,
            Self::Animation => 4,
            Self::Graph => 5,
            Self::Effects => 6,
            Self::Audio => 7,
            Self::TimeRemap => 8,
            Self::Mask => 9,
            Self::Assets => 10,
            Self::Text => 11,
            Self::LayerStyle => 12,
            Self::Color => 13,
            Self::CameraLight => 14,
        }
    }
}

pub(super) struct CachedAudioWaveform {
    pub(super) peaks: Vec<f32>,
    pub(super) duration_seconds: f32,
}

pub(super) struct CachedAssetPreview {
    pub(super) pixels: std::sync::Arc<Vec<u8>>,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ViewportPreviewKey {
    pub(super) revision: u64,
    pub(super) media_revision: u64,
    pub(super) frame: u32,
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) struct ViewportPreviewJob {
    pub(super) key: ViewportPreviewKey,
    pub(super) composition: Box<kagari_vfx::core::timeline::Composition>,
    pub(super) cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub(super) struct ViewportPreviewResult {
    pub(super) key: ViewportPreviewKey,
    pub(super) pixels: Option<std::sync::Arc<Vec<u8>>>,
}

#[derive(Clone)]
pub(super) enum AssetPreviewJob {
    Image(String),
    Model3D(String),
    Composition(Box<kagari_vfx::core::timeline::Composition>),
}

#[derive(Clone)]
pub(super) struct SettingsDraft {
    pub(super) restore_previous_state: bool,
    pub(super) confirm_close_project: bool,
    pub(super) confirm_exit: bool,
    pub(super) adaptive_preview: bool,
    pub(super) audio_preview: bool,
    pub(super) show_home_on_startup: bool,
    pub(super) autosave_secs: u64,
    pub(super) gpu_enabled: bool,
    pub(super) memory_limit_gb: f64,
    pub(super) undo_steps: usize,
    pub(super) recovery_slots: usize,
    pub(super) cache_path: String,
    pub(super) cache_limit: String,
    pub(super) accent_color: String,
    pub(super) ui_locale: String,
    pub(super) auto_project_names: bool,
    pub(super) renderer: String,
    pub(super) background_render: bool,
    pub(super) preview_quality: String,
    pub(super) high_quality_preview: bool,
    pub(super) preview_resolution: String,
    pub(super) still_duration_secs: f64,
    pub(super) image_sequence_detect: bool,
    pub(super) proxy_auto_generate: bool,
    pub(super) hardware_decode: bool,
    pub(super) preferred_decoder: String,
    pub(super) media_color_space: String,
    pub(super) grid_visible: bool,
    pub(super) grid_spacing: f64,
    pub(super) safe_area_visible: bool,
    pub(super) color_management: String,
    pub(super) gamma: String,
    pub(super) hdr_enabled: bool,
    pub(super) playback_quality: String,
    pub(super) playback_start_mode: String,
    pub(super) pre_roll_secs: f64,
    pub(super) post_roll_secs: f64,
    pub(super) playback_volume: f64,
    pub(super) playback_cache: bool,
    pub(super) scrub_audio: bool,
    pub(super) show_audio_peaks: bool,
    pub(super) project_dir: String,
    pub(super) preview_cache_dir: String,
    pub(super) temp_dir: String,
    pub(super) autosave_dir: String,
    pub(super) backup_dir: String,
    pub(super) backup_generations: u64,
    pub(super) asset_library_dir: String,
    pub(super) shortcut_filter: String,
}
