//! A particle-focused VFX editing workspace composed entirely with `.ame`.
//! The example focuses on a scene tree, 3D viewport, asset browser and timeline.

use ame_ui::{
    app::{
        self, App, CanvasBounds, CanvasContext, CanvasPointerEvent, CanvasPointerPhase, Viewport,
        WindowConfig,
    },
    asset::{AssetData, AssetKey, AssetMime, AssetStore},
    event::{ActionValue, AmeEvent},
    lang::{compile_with_sources, AmeRuntime, HostAction, Value},
    theme::Theme,
    view::{NodePathSegment, StableKey, ViewTree},
};
use vello::{
    kurbo::{Affine, BezPath, Circle, Line, Rect, RoundedRect, Stroke},
    peniko::{Color, Fill},
    Scene,
};

#[path = "../ui/ame_sources.rs"]
mod ame_sources;
#[allow(dead_code)]
#[path = "ame_studio/icons.rs"]
mod ame_studio_icons;
#[path = "ame_studio/sync_bindings.rs"]
mod ame_studio_sync_bindings;
#[path = "ame_studio/sync_recents.rs"]
mod ame_studio_sync_recents;
#[path = "ame_studio/sync_assets.rs"]
mod ame_studio_sync_assets;
#[path = "ame_studio/sync_layer_styles.rs"]
mod ame_studio_sync_layer_styles;
#[path = "ame_studio/sync_environment.rs"]
mod ame_studio_sync_environment;
#[path = "ame_studio/sync_effects.rs"]
mod ame_studio_sync_effects;
#[path = "ame_studio/sync_shapes.rs"]
mod ame_studio_sync_shapes;
#[path = "ame_studio/event_handlers.rs"]
mod ame_studio_event_handlers;
#[path = "ame_studio/viewport.rs"]
mod ame_studio_viewport;
#[path = "ame_studio/layer_controls.rs"]
mod ame_studio_layer_controls;
#[path = "ame_studio/export.rs"]
mod ame_studio_export;
#[path = "ame_studio/media_import.rs"]
mod ame_studio_media_import;
#[path = "ame_studio/project_io.rs"]
mod ame_studio_project_io;
#[path = "ame_studio/effect_ops.rs"]
mod ame_studio_effect_ops;

pub struct AfterEffectsStudio {
    ui: AmeRuntime,
    assets: AssetStore,
    asset_preview_image: Option<(String, Option<vello::peniko::ImageData>)>,
    asset_preview_failed_asset_id: Option<String>,
    asset_render_preview_cache: std::collections::HashMap<String, Option<CachedAssetPreview>>,
    asset_render_preview_requested: std::collections::HashSet<String>,
    asset_render_preview_pending: std::collections::HashMap<String, AssetPreviewJob>,
    asset_render_preview_request_tx: std::sync::mpsc::SyncSender<(String, AssetPreviewJob)>,
    asset_render_preview_result_rx:
        std::sync::mpsc::Receiver<(String, Option<CachedAssetPreview>)>,
    text_font_families: Vec<String>,
    screen: Screen,
    docs_detail_page: bool,
    docs_title: String,
    docs_body: String,
    clear_cache_confirm: bool,
    settings_category: usize,
    openfx_plugins: Vec<DetectedOfxPlugin>,
    openfx_search_paths: String,
    settings_open: bool,
    exit_confirm_open: bool,
    project_switch_confirm_open: bool,
    composition_remove_confirm_open: bool,
    layer_remove_confirm_open: bool,
    pending_project_path: Option<std::path::PathBuf>,
    pending_new_project: bool,
    composition_settings_status: String,
    settings_snapshot: Option<SettingsDraft>,
    restore_previous_state: bool,
    confirm_close_project: bool,
    confirm_exit: bool,
    adaptive_preview: bool,
    audio_preview: bool,
    show_home_on_startup: bool,
    autosave_secs: u64,
    gpu_enabled: bool,
    memory_limit_gb: f64,
    undo_steps: usize,
    recovery_slots: usize,
    cache_path: String,
    cache_path_status: String,
    settings_feedback: String,
    cache_limit: String,
    cache_usage_bytes: u64,
    compact: bool,
    narrow: bool,
    wide: bool,
    render_queue: bool,
    workspace: Workspace,
    selected_tool: &'static str,
    layer_name: String,
    search: String,
    project_search: String,
    current_time: String,
    zoom_manually_adjusted: bool,
    current_frame: u32,
    timeline_width: f64,
    graph_canvas_height: f64,
    timeline_zoom: f64,
    timeline_start_frame: f64,
    workspace_left_ratios: [f64; 15],
    workspace_center_ratios: [f64; 15],
    workspace_vertical_ratios: [f64; 15],
    preview_revision: u64,
    preview_cache: std::collections::HashMap<(u64, u32, u32, u32), vello::peniko::ImageData>,
    viewport_preview_request_tx: std::sync::mpsc::SyncSender<ViewportPreviewJob>,
    viewport_preview_result_rx: std::sync::mpsc::Receiver<ViewportPreviewResult>,
    viewport_preview_inflight: bool,
    viewport_preview_failed: Option<ViewportPreviewKey>,
    viewport_mapping: Option<(f64, f64, f64, f64)>,
    viewport_drag: Option<(String, [f32; 2], [f64; 2])>,
    mask_vertex_drag: Option<MaskVertexDrag>,
    viewport_pan: [f64; 2],
    viewport_pan_drag: Option<([f64; 2], [f64; 2])>,
    viewport_pen_start: Option<[f64; 2]>,
    viewport_pen_current: Option<[f64; 2]>,
    light_gizmo_drag: Option<LightGizmoDrag>,
    camera_nav_drag: Option<CameraNavDrag>,
    timeline_scrubbing: bool,
    timeline_clip_drag: Option<TimelineClipDrag>,
    timeline_work_area_drag: Option<TimelineWorkAreaDragMode>,
    timeline_snap_frame: Option<u32>,
    timeline_view: TimelineView,
    properties_section: PropertiesSection,
    timeline_snap_enabled: bool,
    graph_channel: GraphChannel,
    graph_key_drag: Option<GraphKeyDrag>,
    flowchart_wire_drag: Option<FlowchartWireDrag>,
    hide_shy_layers: bool,
    waveform_cache: std::collections::HashMap<String, std::sync::Arc<CachedAudioWaveform>>,
    waveform_requested: std::collections::HashSet<String>,
    waveform_failed: std::collections::HashSet<String>,
    waveform_request_tx: std::sync::mpsc::SyncSender<String>,
    waveform_result_rx: std::sync::mpsc::Receiver<(String, Option<CachedAudioWaveform>)>,
    playback_anchor: Option<(std::time::Instant, u32)>,
    last_autosave_at: std::time::Instant,
    zoom: String,
    position_x: f64,
    position_y: f64,
    anchor_x: f64,
    anchor_y: f64,
    rotation_degrees: f64,
    scale_x: f64,
    scale_y: f64,
    opacity: f64,
    effect_search: String,
    text_font_search: String,
    asset_status: String,
    asset_folder_id: Option<String>,
    new_asset_folder_name: String,
    project_asset_search: String,
    project_asset_search_all_folders: bool,
    project_assets_sort_by_type: bool,
    selected_project_asset_id: Option<String>,
    selected_layer: bool,
    selected_layer_id: Option<String>,
    selected_mask_id: Option<String>,
    selected_tracker_idx: usize,
    selected_tracker_target_id: Option<String>,
    tracker_status: String,
    collapsed_scene_layers: std::collections::HashSet<String>,
    align_selected_layer_ids: std::collections::HashSet<String>,
    align_to_composition: bool,
    selected_effect_id: Option<String>,
    graph_effect_parameter_id: Option<String>,
    selected_marker_frame: Option<u32>,
    selected_layer_marker_frame: Option<u32>,
    output_path: String,
    still_overwrite_pending: bool,
    still_status: String,
    video_output_path: String,
    video_export_codec_idx: usize,
    video_overwrite_pending: bool,
    project_name: String,
    project_file_path: Option<std::path::PathBuf>,
    save_status: String,
    setup_name: String,
    setup_width: String,
    setup_height: String,
    setup_fps: String,
    setup_duration_seconds: String,
    setup_error: String,
    setup_return_screen: Screen,
    project_model: kagari_vfx::core::timeline::Project,
    project_generation: u64,
    project_fps: u32,
    history: kagari_vfx::core::history::ProjectHistory,
    history_interaction_snapshot: Option<kagari_vfx::core::timeline::Project>,
    is_playing: bool,
    playback_loop: bool,
    playback_range: Option<(u32, u32)>,
    render_progress: i64,
    render_status: String,
    render_events: Option<std::sync::mpsc::Receiver<kagari_vfx::core::ffmpeg_export::ExportEvent>>,
    render_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    render_refresh_ticks: u8,
    video_import: Option<VideoImportJob>,
    proxy_generation: Option<LayerProxyGenerationJob>,
    proxy_status: String,
    proxy_progress: i64,
    recent_projects: Vec<RecentProjectItem>,
    exit_requested: bool,
}

#[derive(Clone, Copy)]
enum TimelineClipDragMode {
    Move,
    TrimStart,
    TrimEnd,
    Keyframe,
}

#[derive(Clone, Copy)]
enum TimelineWorkAreaDragMode {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimelineView {
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
enum PropertiesSection {
    Transform,
    Content,
    Composite,
    Advanced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GraphChannel {
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

fn action_edits_selected_layer(action: &str) -> bool {
    const PREFIXES: &[&str] = &[
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
            | "cycle_shape_fill_type"
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
            | "layer_style_parameter_changed"
            | "cycle_constraint_horizontal"
            | "cycle_constraint_vertical"
            | "layer_in_frame_changed"
            | "layer_out_frame_changed"
            | "layer_in_to_playhead"
            | "layer_out_to_playhead"
            | "cycle_auto_orient"
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
            | "add_rect_mask"
            | "add_ellipse_mask"
            | "remove_mask"
    )
}

impl GraphChannel {
    const ALL: [Self; 41] = [
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

    fn is_3d(self) -> bool {
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

    fn supports(self, layer: &kagari_vfx::core::timeline::Layer) -> bool {
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

    fn label(self) -> &'static str {
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
enum MaskValueChannel {
    Feather,
    Opacity,
    Expansion,
}

#[derive(Clone, Copy)]
enum LayerToggle {
    FrameBlending,
    MotionBlur,
    PreserveTransparency,
    Guide,
    Shy,
    Effects,
}

#[derive(Clone)]
struct GraphKeyDrag {
    layer_id: String,
    channel: GraphChannel,
    current_frame: u32,
    min_value: f32,
    max_value: f32,
    mask_id: Option<String>,
    effect_id: Option<String>,
    effect_parameter_id: Option<String>,
    snap_points: Vec<u32>,
}

#[derive(Clone)]
struct MaskVertexDrag {
    layer_id: String,
    mask_id: String,
    vertex_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CameraNavMode {
    Orbit,
    Pan,
    Dolly,
}

#[derive(Clone, Debug)]
struct CameraNavDrag {
    mode: CameraNavMode,
    start_pos: [f32; 3],
    start_rot: [f32; 3],
    start_screen_point: [f64; 2],
}

#[derive(Clone, Debug)]
struct LightGizmoDrag {
    light_id: String,
    start_pos_3d: [f32; 3],
    start_point: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlowchartWireKind {
    Parent,
    Matte,
}

#[derive(Clone, Debug)]
struct FlowchartWireDrag {
    source_layer_id: String,
    kind: FlowchartWireKind,
    start_pos: (f64, f64),
    current_pos: (f64, f64),
}

#[derive(Clone)]
struct TimelineClipDrag {
    layer_id: String,
    start_x: f64,
    initial_in: u32,
    initial_out: u32,
    keyframe_source: Option<u32>,
    keyframe_current: Option<u32>,
    mode: TimelineClipDragMode,
}

struct VideoImportJob {
    receiver: std::sync::mpsc::Receiver<Result<kagari_vfx::core::video_import::VideoAsset, String>>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    cancel_supported: bool,
    is_image_sequence: bool,
    source_name: String,
    layer_id: String,
    replace_layer_id: Option<String>,
    replace_project_asset_id: Option<String>,
    register_project_asset: bool,
    composition_id: String,
    project_file_path: Option<std::path::PathBuf>,
    project_name: String,
    fps: u32,
    project_generation: u64,
    asset_parent_folder: Option<String>,
}

struct LayerProxyGenerationJob {
    receiver: std::sync::mpsc::Receiver<LayerProxyGenerationMessage>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    layer_id: String,
    composition_id: String,
    project_file_path: Option<std::path::PathBuf>,
    project_name: String,
    project_generation: u64,
}

enum LayerProxyGenerationMessage {
    Progress { done: u32, total: u32 },
    Finished(Result<String, String>),
}

#[derive(Clone)]
struct DetectedOfxPlugin {
    name: String,
    path: String,
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
enum Screen {
    Home,
    Projects,
    Activity,
    Documents,
    ProjectSetup,
    Editor,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Workspace {
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

const WORKSPACE_LEFT_DEFAULTS: [f64; 15] = [
    0.22, 0.22, 0.30, 0.20, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.34, 0.22, 0.22, 0.22, 0.22,
];
const WORKSPACE_CENTER_DEFAULTS: [f64; 15] = [
    0.70, 0.62, 0.72, 0.72, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70, 0.70,
    0.70,
];
const WORKSPACE_VERTICAL_DEFAULT: f64 = 0.54;

impl Workspace {
    fn index(self) -> usize {
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

struct CachedAudioWaveform {
    peaks: Vec<f32>,
    duration_seconds: f32,
}

struct CachedAssetPreview {
    pixels: std::sync::Arc<Vec<u8>>,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ViewportPreviewKey {
    revision: u64,
    frame: u32,
    width: u32,
    height: u32,
}

struct ViewportPreviewJob {
    key: ViewportPreviewKey,
    composition: Box<kagari_vfx::core::timeline::Composition>,
}

struct ViewportPreviewResult {
    key: ViewportPreviewKey,
    pixels: Option<std::sync::Arc<Vec<u8>>>,
}

#[derive(Clone)]
enum AssetPreviewJob {
    Image(String),
    Model3D(String),
    Composition(Box<kagari_vfx::core::timeline::Composition>),
}

#[derive(Clone)]
struct SettingsDraft {
    restore_previous_state: bool,
    confirm_close_project: bool,
    confirm_exit: bool,
    adaptive_preview: bool,
    audio_preview: bool,
    show_home_on_startup: bool,
    autosave_secs: u64,
    gpu_enabled: bool,
    memory_limit_gb: f64,
    undo_steps: usize,
    recovery_slots: usize,
    cache_path: String,
    cache_limit: String,
}

#[path = "ame_studio/preferences.rs"]
mod ame_studio_preferences;
pub(crate) use ame_studio_preferences::*;

#[path = "ame_studio/app_init.rs"]
mod ame_studio_app_init;


#[path = "ame_studio/methods.rs"]
mod ame_studio_methods;

#[path = "ame_studio/composition_ops.rs"]
mod ame_studio_composition_ops;
#[path = "ame_studio/composition_action_ops.rs"]
mod ame_studio_composition_action_ops;

#[path = "ame_studio/transform_3d_ops.rs"]
mod ame_studio_transform_3d_ops;

#[path = "ame_studio/text_ops.rs"]
mod ame_studio_text_ops;

#[path = "ame_studio/mask_ops.rs"]
mod ame_studio_mask_ops;

#[path = "ame_studio/layer_ops.rs"]
mod ame_studio_layer_ops;

#[path = "ame_studio/timeline_ops.rs"]
mod ame_studio_timeline_ops;

#[path = "ame_studio/preview_workers.rs"]
mod ame_studio_preview_workers;
pub(crate) use ame_studio_preview_workers::*;

#[path = "ame_studio/shape_ops.rs"]
mod ame_studio_shape_ops;

#[path = "ame_studio/camera_light_ops.rs"]
mod ame_studio_camera_light_ops;

#[path = "ame_studio/particle_ops.rs"]
mod ame_studio_particle_ops;

#[path = "ame_studio/audio_video_ops.rs"]
mod ame_studio_audio_video_ops;

#[path = "ame_studio/playback_ops.rs"]
mod ame_studio_playback_ops;

impl Drop for AfterEffectsStudio {
    fn drop(&mut self) {
        let _ = write_preference(
            "workspace_left_ratios",
            serde_json::json!(self.workspace_left_ratios),
        );
        let _ = write_preference(
            "workspace_center_ratios",
            serde_json::json!(self.workspace_center_ratios),
        );
        let _ = write_preference(
            "workspace_vertical_ratios",
            serde_json::json!(self.workspace_vertical_ratios),
        );
        if let Some(cancel) = &self.render_cancel {
            cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Some(job) = &self.video_import {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(job) = &self.proxy_generation {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

#[path = "ame_studio/app.rs"]
mod ame_studio_app;
pub(crate) use ame_studio_app::*;

#[path = "ame_studio/timeline_canvas.rs"]
mod ame_studio_timeline_canvas;
pub(crate) use ame_studio_timeline_canvas::*;

fn parse_timecode(value: &str, fps: u32) -> Option<u32> {
    let [hours, minutes, seconds, frames] = value
        .split(':')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?
        .try_into()
        .ok()?;
    let fps = fps.max(1);
    if minutes >= 60 || seconds >= 60 || frames >= fps {
        return None;
    }
    hours
        .checked_mul(3600)?
        .checked_add(minutes.checked_mul(60)?)?
        .checked_add(seconds)?
        .checked_mul(fps)?
        .checked_add(frames)
}

fn project_asset_folder_path(
    assets: &[kagari_vfx::core::timeline::ProjectItem],
    folder_id: Option<&str>,
) -> String {
    let mut names = Vec::new();
    let mut next = folder_id;
    let mut visited = std::collections::HashSet::new();
    while let Some(id) = next {
        if !visited.insert(id) {
            break;
        }
        let Some(folder) = assets.iter().find(|item| item.id == id) else {
            break;
        };
        names.push(folder.name.as_str());
        next = folder.parent_folder.as_deref();
    }
    names.reverse();
    names.join(" / ")
}

fn project_asset_can_move_to_folder(
    assets: &[kagari_vfx::core::timeline::ProjectItem],
    asset_id: &str,
    destination_folder: Option<&str>,
) -> bool {
    use kagari_vfx::core::timeline::ProjectItemType;

    let Some(asset) = assets.iter().find(|asset| asset.id == asset_id) else {
        return false;
    };
    if asset.parent_folder.as_deref() == destination_folder {
        return false;
    }
    if let Some(destination_id) = destination_folder {
        let Some(destination) = assets.iter().find(|asset| asset.id == destination_id) else {
            return false;
        };
        if !matches!(&destination.item_type, ProjectItemType::Folder { .. }) {
            return false;
        }
    }

    if matches!(&asset.item_type, ProjectItemType::Folder { .. }) {
        let mut ancestor = destination_folder;
        let mut visited = std::collections::HashSet::new();
        while let Some(folder_id) = ancestor {
            if folder_id == asset_id || !visited.insert(folder_id) {
                return false;
            }
            ancestor = assets
                .iter()
                .find(|candidate| candidate.id == folder_id)
                .and_then(|folder| folder.parent_folder.as_deref());
        }
    }
    true
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run(AfterEffectsStudio::new()?)?;
    Ok(())
}

fn remove_generated_proxy(path: &str) {
    let path = std::path::Path::new(path);
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
#[path = "ame_studio/tests.rs"]
mod ame_studio_tests;
