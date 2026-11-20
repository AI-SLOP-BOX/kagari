//! A particle-focused VFX editing workspace composed entirely with `.ame`.
//! The example focuses on a scene tree, 3D viewport, asset browser and timeline.

use ame_ui::{
    app::{
        self, App, CanvasBounds, CanvasContext, CanvasPointerEvent, CanvasPointerPhase,
        CanvasScrollEvent, Viewport, WindowConfig,
    },
    asset::{AssetData, AssetKey, AssetMime, AssetStore},
    event::{ActionValue, AmeEvent},
    lang::{compile_with_sources, AmeRuntime, HostAction, Value},
    theme::Theme,
    view::{NodePathSegment, StableKey, ViewTree},
};
use vello::{
    kurbo::{Affine, BezPath, Circle, Line, Point, Rect, RoundedRect, Stroke},
    peniko::{Color, Fill},
    Scene,
};

#[path = "../ui/ame_sources.rs"]
mod ame_sources;
#[path = "ame_studio/asset_placement.rs"]
mod ame_studio_asset_placement;
#[path = "ame_studio/effect_edit_ops.rs"]
mod ame_studio_effect_edit_ops;
#[path = "ame_studio/effect_ops.rs"]
mod ame_studio_effect_ops;
#[path = "ame_studio/engine_hub_handlers/mod.rs"]
mod ame_studio_engine_hub_handlers;
#[path = "ame_studio/event_editor_workspace_handlers.rs"]
mod ame_studio_event_editor_workspace_handlers;
#[path = "ame_studio/event_effect_handlers.rs"]
mod ame_studio_event_effect_handlers;
#[path = "ame_studio/event_graph_handlers.rs"]
mod ame_studio_event_graph_handlers;
#[path = "ame_studio/event_handlers/mod.rs"]
mod ame_studio_event_handlers;
#[path = "ame_studio/event_layer_keyframe_handlers.rs"]
mod ame_studio_event_layer_keyframe_handlers;
#[path = "ame_studio/event_mask_handlers.rs"]
mod ame_studio_event_mask_handlers;
#[path = "ame_studio/event_navigation_setup_handlers.rs"]
mod ame_studio_event_navigation_setup_handlers;
#[path = "ame_studio/event_project_asset_handlers.rs"]
mod ame_studio_event_project_asset_handlers;
#[path = "ame_studio/event_project_export_actions.rs"]
mod ame_studio_event_project_export_actions;
#[path = "ame_studio/event_scene_layer_handlers.rs"]
mod ame_studio_event_scene_layer_handlers;
#[path = "ame_studio/event_text_shape_handlers.rs"]
mod ame_studio_event_text_shape_handlers;
#[path = "ame_studio/event_time_transform_actions.rs"]
mod ame_studio_event_time_transform_actions;
#[path = "ame_studio/event_timeline_markers.rs"]
mod ame_studio_event_timeline_markers;
#[path = "ame_studio/export.rs"]
mod ame_studio_export;
#[path = "ame_studio/export_stills.rs"]
mod ame_studio_export_stills;
#[path = "ame_studio/export_video.rs"]
mod ame_studio_export_video;
#[allow(dead_code)]
#[path = "ame_studio/icons.rs"]
mod ame_studio_icons;
#[path = "ame_studio/layer_controls.rs"]
mod ame_studio_layer_controls;
#[path = "ame_studio/media_import/mod.rs"]
mod ame_studio_media_import;
#[path = "ame_studio/media_proxy.rs"]
mod ame_studio_media_proxy;
#[path = "ame_studio/preset_ops.rs"]
mod ame_studio_preset_ops;
#[path = "ame_studio/project_io.rs"]
mod ame_studio_project_io;
#[path = "ame_studio/roto_brush.rs"]
mod ame_studio_roto_brush;
#[path = "ame_studio/roto_brush_messages.rs"]
mod ame_studio_roto_brush_messages;
#[path = "ame_studio/sync_assets.rs"]
mod ame_studio_sync_assets;
#[path = "ame_studio/sync_bindings.rs"]
mod ame_studio_sync_bindings;
#[path = "ame_studio/sync_effects.rs"]
mod ame_studio_sync_effects;
#[path = "ame_studio/sync_environment.rs"]
mod ame_studio_sync_environment;
#[path = "ame_studio/sync_graph_layer_bindings.rs"]
mod ame_studio_sync_graph_layer_bindings;
#[path = "ame_studio/sync_layer_styles.rs"]
mod ame_studio_sync_layer_styles;
#[path = "ame_studio/sync_media_layer_bindings.rs"]
mod ame_studio_sync_media_layer_bindings;
#[path = "ame_studio/sync_particle_bindings.rs"]
mod ame_studio_sync_particle_bindings;
#[path = "ame_studio/sync_project_timeline.rs"]
mod ame_studio_sync_project_timeline;
#[path = "ame_studio/sync_recents.rs"]
mod ame_studio_sync_recents;
#[path = "ame_studio/sync_scene_assets.rs"]
mod ame_studio_sync_scene_assets;
#[path = "ame_studio/sync_shapes.rs"]
mod ame_studio_sync_shapes;
#[path = "ame_studio/sync_text_layer_bindings.rs"]
mod ame_studio_sync_text_layer_bindings;
#[path = "ame_studio/viewport.rs"]
mod ame_studio_viewport;
#[path = "ame_studio/viewport_overlays.rs"]
mod ame_studio_viewport_overlays;

pub struct AfterEffectsStudio {
    ui: AmeRuntime,
    app_event_sender: Option<ame_ui::app::AppEventSender>,
    assets: AssetStore,
    asset_preview_image: Option<(String, Option<vello::peniko::ImageData>)>,
    asset_preview_failed_asset_id: Option<String>,
    asset_render_preview_cache: std::collections::HashMap<String, Option<CachedAssetPreview>>,
    asset_render_preview_requested: std::collections::HashSet<String>,
    asset_render_preview_pending: std::collections::HashMap<String, AssetPreviewJob>,
    asset_render_preview_request_tx: std::sync::mpsc::SyncSender<(String, AssetPreviewJob)>,
    asset_render_preview_result_rx: std::sync::mpsc::Receiver<(String, Option<CachedAssetPreview>)>,
    asset_preview_worker_failed: bool,
    text_font_families: Vec<String>,
    screen: Screen,
    docs_detail_page: bool,
    docs_title: String,
    docs_body: String,
    engine_hub_status: String,
    compositor_graph: kagari_vfx::core::compositor_node_graph::CompositorNodeGraph,
    node_graph_selected_id: Option<u64>,
    node_graph_next_id: u64,
    node_graph_status: String,
    node_graph_bounds: Option<(f64, f64, f64, f64)>,
    node_graph_positions: std::collections::HashMap<u64, (f64, f64)>,
    node_graph_drag_node: Option<(u64, f64, f64)>,
    node_graph_drag_wire: Option<(u64, (f64, f64), (f64, f64))>,
    node_graph_live_preview: bool,
    engine_hub_expanded: bool,
    corner_pin_drag: Option<usize>,
    tutorial_step: usize,
    effect_category: &'static str,
    clear_cache_confirm: bool,
    settings_category: usize,
    openfx_plugins: Vec<DetectedOfxPlugin>,
    openfx_search_paths: String,
    settings_open: bool,
    command_palette_open: bool,
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
    settings_defaults_confirm_open: bool,
    settings_search: String,
    cache_limit: String,
    cache_usage_bytes: u64,
    accent_color: String,
    auto_project_names: bool,
    renderer: String,
    background_render: bool,
    preview_quality: String,
    high_quality_preview: bool,
    preview_resolution: String,
    still_duration_secs: f64,
    image_sequence_detect: bool,
    proxy_auto_generate: bool,
    hardware_decode: bool,
    preferred_decoder: String,
    media_color_space: String,
    grid_visible: bool,
    grid_spacing: f64,
    safe_area_visible: bool,
    color_management: String,
    gamma: String,
    hdr_enabled: bool,
    playback_quality: String,
    playback_start_mode: String,
    pre_roll_secs: f64,
    post_roll_secs: f64,
    playback_volume: f64,
    playback_cache: bool,
    scrub_audio: bool,
    show_audio_peaks: bool,
    project_dir: String,
    preview_cache_dir: String,
    temp_dir: String,
    autosave_dir: String,
    backup_dir: String,
    backup_generations: u64,
    asset_library_dir: String,
    shortcut_filter: String,
    compact: bool,
    narrow: bool,
    wide: bool,
    render_queue: bool,
    workspace: Workspace,
    selected_tool: &'static str,
    roto_foreground: bool,
    roto_brush_radius: f64,
    roto_feather_radius: f64,
    roto_status: String,
    layer_name: String,
    search: String,
    project_search: String,
    project_sort: String,
    project_sort_desc: bool,
    project_view_grid: bool,
    selected_project_paths: std::collections::BTreeSet<String>,
    active_project_path: Option<String>,
    current_time: String,
    zoom_manually_adjusted: bool,
    current_frame: u32,
    timeline_width: f64,
    timeline_canvas_height: f64,
    graph_canvas_width: f64,
    graph_canvas_height: f64,
    timeline_zoom: f64,
    timeline_start_frame: f64,
    timeline_vertical_scroll: f64,
    virtual_list_ranges: std::collections::HashMap<String, std::ops::Range<usize>>,
    workspace_left_ratios: [f64; 15],
    workspace_center_ratios: [f64; 15],
    workspace_vertical_ratios: [f64; 15],
    viewport_channel_mode: String,
    viewport_exposure_ev: f64,
    viewport_wipe_enabled: bool,
    viewport_wipe_split: f64,
    viewport_wipe_dragging: bool,
    viewport_scale_drag: Option<(String, [f32; 2], [f32; 2], [f64; 2])>,
    preview_revision: u64,
    preview_cache: std::collections::HashMap<(u64, u32, u32, u32, u64), vello::peniko::ImageData>,
    viewport_preview_request_tx: std::sync::mpsc::SyncSender<ViewportPreviewJob>,
    viewport_preview_result_rx: std::sync::mpsc::Receiver<ViewportPreviewResult>,
    viewport_preview_inflight: bool,
    viewport_preview_active_key: Option<ViewportPreviewKey>,
    viewport_preview_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    viewport_preview_worker_failed: bool,
    viewport_preview_failed: Option<ViewportPreviewKey>,
    viewport_mapping: Option<(f64, f64, f64, f64)>,
    viewport_drag: Option<(String, [f32; 2], [f64; 2])>,
    mask_vertex_drag: Option<MaskVertexDrag>,
    viewport_pan: [f64; 2],
    viewport_pan_drag: Option<([f64; 2], [f64; 2])>,
    viewport_pen_start: Option<[f64; 2]>,
    viewport_pen_current: Option<[f64; 2]>,
    roto_live_stroke: Option<RotoLiveStroke>,
    roto_analysis_job: Option<RotoAnalysisJob>,
    roto_propagation_job: Option<RotoPropagationJob>,
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
    color_scope_mode: ColorScopeMode,
    color_wheel_drag: Option<ColorWheelDrag>,
    color_wheels_bounds: Option<(f64, f64, f64, f64)>,
    hide_shy_layers: bool,
    waveform_cache: std::collections::HashMap<String, std::sync::Arc<CachedAudioWaveform>>,
    waveform_cache_revisions: std::collections::HashMap<String, u64>,
    waveform_cache_last_used: std::collections::HashMap<String, u64>,
    waveform_cache_use_clock: u64,
    waveform_requested: std::collections::HashMap<String, u64>,
    waveform_failed: std::collections::HashMap<String, u64>,
    waveform_worker_failed: bool,
    waveform_request_tx: std::sync::mpsc::SyncSender<(String, u64)>,
    waveform_result_rx: std::sync::mpsc::Receiver<(String, u64, Option<CachedAudioWaveform>)>,
    playback_anchor: Option<(std::time::Instant, u32)>,
    last_autosave_at: std::time::Instant,
    autosave_job: Option<AutosaveJob>,
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
    animation_preset_search: String,
    animation_preset_status: String,
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
    effect_clipboard: Option<kagari_vfx::core::timeline::Effect>,
    graph_effect_parameter_id: Option<String>,
    selected_marker_frame: Option<u32>,
    selected_layer_marker_frame: Option<u32>,
    output_path: String,
    still_overwrite_pending: bool,
    still_overwrite_identity_pending: Option<Option<kagari_vfx::core::ffmpeg_export::FileIdentity>>,
    still_status: String,
    still_export_events: Option<std::sync::mpsc::Receiver<Result<std::path::PathBuf, String>>>,
    video_output_path: String,
    video_export_codec_idx: usize,
    video_export_quality: u8,
    video_export_audio_enabled: bool,
    video_overwrite_pending: bool,
    video_overwrite_identity_pending: Option<Option<kagari_vfx::core::ffmpeg_export::FileIdentity>>,
    project_name: String,
    project_file_path: Option<std::path::PathBuf>,
    save_status: String,
    project_is_dirty: bool,
    setup_name: String,
    setup_location: String,
    setup_comp_name: String,
    setup_create_initial_comp: bool,
    setup_advanced_open: bool,
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
    render_terminal_event: Option<
        std::sync::Arc<std::sync::Mutex<Option<kagari_vfx::core::ffmpeg_export::ExportEvent>>>,
    >,
    render_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    render_worker: Option<std::thread::JoinHandle<()>>,
    render_refresh_ticks: u8,
    still_export_worker: Option<std::thread::JoinHandle<()>>,
    still_export_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    video_import: Option<VideoImportJob>,
    proxy_generation: Option<LayerProxyGenerationJob>,
    proxy_status: String,
    proxy_progress: i64,
    recent_projects: Vec<RecentProjectItem>,
    exit_requested: bool,
}

#[path = "ame_studio/types.rs"]
mod ame_studio_types;
pub(crate) use ame_studio_types::*;

#[path = "ame_studio/preferences_storage.rs"]
mod ame_studio_preferences_storage;
pub(crate) use ame_studio_preferences_storage::*;
#[path = "ame_studio/preferences.rs"]
mod ame_studio_preferences;

#[path = "ame_studio/app_init.rs"]
mod ame_studio_app_init;

#[path = "ame_studio/methods.rs"]
mod ame_studio_methods;

#[path = "ame_studio/composition_action_ops.rs"]
mod ame_studio_composition_action_ops;
#[path = "ame_studio/composition_ops.rs"]
mod ame_studio_composition_ops;

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

#[path = "ame_studio/shape_creation.rs"]
mod ame_studio_shape_creation;
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
        if let Some(cancel) = &self.render_cancel {
            cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Some(cancel) = &self.still_export_cancel {
            cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Some(job) = &self.video_import {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(job) = &self.proxy_generation {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(job) = &self.roto_analysis_job {
            job.cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(job) = &self.roto_propagation_job {
            job.cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
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
        if let Some(job) = self.video_import.take() {
            if let Some(worker) = job.worker {
                let _ = worker.join();
            }
        }
        if let Some(job) = self.proxy_generation.take() {
            if let Some(worker) = job.worker {
                let _ = worker.join();
            }
        }
        if let Some(job) = self.autosave_job.take() {
            if let Some(worker) = job.worker {
                let _ = worker.join();
            }
        }
        if let Some(worker) = self.render_worker.take() {
            let _ = worker.join();
        }
        if let Some(worker) = self.still_export_worker.take() {
            let _ = worker.join();
        }
    }
}

#[path = "ame_studio/helpers_effects.rs"]
mod ame_studio_helpers_effects;
pub(crate) use ame_studio_helpers_effects::*;
#[path = "ame_studio/helpers_keyframes.rs"]
mod ame_studio_helpers_keyframes;
pub(crate) use ame_studio_helpers_keyframes::*;
#[path = "ame_studio/helpers_scene.rs"]
mod ame_studio_helpers_scene;
pub(crate) use ame_studio_helpers_scene::*;
#[path = "ame_studio/app.rs"]
mod ame_studio_app;
#[path = "ame_studio/app_canvas_draw.rs"]
mod ame_studio_app_canvas_draw;
#[path = "ame_studio/app_canvas_pointer_graph_viewport.rs"]
mod ame_studio_app_canvas_pointer_graph_viewport;
#[path = "ame_studio/app_canvas_pointer_timeline_flowchart.rs"]
mod ame_studio_app_canvas_pointer_timeline_flowchart;
pub(crate) use ame_studio_roto_brush::*;

#[path = "ame_studio/timeline_asset_preview.rs"]
mod ame_studio_timeline_asset_preview;
pub(crate) use ame_studio_timeline_asset_preview::*;
#[path = "ame_studio/timeline_drawing.rs"]
mod ame_studio_timeline_drawing;
pub(crate) use ame_studio_timeline_drawing::*;
#[path = "ame_studio/timeline_flowchart.rs"]
mod ame_studio_timeline_flowchart;
pub(crate) use ame_studio_timeline_flowchart::*;
#[path = "ame_studio/timeline_graph_channels.rs"]
mod ame_studio_timeline_graph_channels;
pub(crate) use ame_studio_timeline_graph_channels::*;
#[path = "ame_studio/timeline_graph_interpolation.rs"]
mod ame_studio_timeline_graph_interpolation;
pub(crate) use ame_studio_timeline_graph_interpolation::*;
#[path = "ame_studio/timeline_graph_keyframes.rs"]
mod ame_studio_timeline_graph_keyframes;
pub(crate) use ame_studio_timeline_graph_keyframes::*;
#[path = "ame_studio/timeline_graph_render.rs"]
mod ame_studio_timeline_graph_render;
pub(crate) use ame_studio_timeline_graph_render::*;
#[path = "ame_studio/timeline_waveform.rs"]
mod ame_studio_timeline_waveform;
pub(crate) use ame_studio_timeline_waveform::*;
#[path = "ame_studio/color_grading_ui.rs"]
mod ame_studio_color_grading_ui;
pub(crate) use ame_studio_color_grading_ui::*;

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
