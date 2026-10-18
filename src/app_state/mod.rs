use std::collections::HashSet;
use std::sync::mpsc::Receiver;

use crate::core::timeline::Project;
use crate::{ExportEvent, TrackerEvent, ViewportMode};

pub use crate::core::audio_types::MixerChannel;

mod domain;
pub use domain::*;
pub mod command;
pub use command::*;
#[cfg(feature = "gui")]
mod initialization;
mod panels;
mod project_state;
#[cfg(feature = "gui")]
mod runner;
mod workspace_draw;

pub enum ScriptConsoleJobOutput {
    Expression(String),
    Automation { project: Project, logs: Vec<String> },
}

pub struct ScriptConsoleCompletion {
    pub command: String,
    pub project_session_id: u64,
    pub history_generation: u64,
    pub elapsed: std::time::Duration,
    pub result: Result<ScriptConsoleJobOutput, String>,
}

pub struct MotionSketchBakeResult {
    pub project_session_id: u64,
    pub history_generation: u64,
    pub composition_id: String,
    pub layer_id: String,
    pub raw_sample_count: usize,
    pub keyframes: Vec<crate::core::keyframe::Keyframe<[f32; 2]>>,
}

static PROJECT_SESSION_SEQUENCE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

fn next_project_session_id() -> u64 {
    PROJECT_SESSION_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

fn dirty_exit_marker_path(process_id: u32) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("kagari_dirty_exit_{process_id}"))
}

fn process_is_running(process_id: u32) -> bool {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(process_id as libc::pid_t, 0) };
        result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(windows)]
    {
        let Ok(output) = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {process_id}"), "/FO", "CSV", "/NH"])
            .output()
        else {
            return true;
        };
        let text = String::from_utf8_lossy(&output.stdout);
        return text.lines().any(|line| {
            line.split(',')
                .nth(1)
                .is_some_and(|field| field.trim().trim_matches('"') == process_id.to_string())
        });
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = process_id;
        true
    }
}

fn find_crashed_session_marker() -> bool {
    let current_process = std::process::id();
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return false;
    };
    let mut found_crash = false;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(process_id) = name
            .strip_prefix("kagari_dirty_exit_")
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        if process_id == current_process || process_is_running(process_id) {
            continue;
        }
        found_crash = true;
        let _ = std::fs::remove_file(path);
    }
    found_crash
}

#[cfg(feature = "gui")]
pub struct KagariApp {
    pub history: crate::core::history::ProjectHistory,
    /// Crash-recovery autosave manager
    pub autosave: crate::core::autosave::AutosaveManager,
    /// History generation already observed by the autosave bridge.
    ///
    /// Some editing surfaces commit through `EditorSession` or directly through
    /// `ProjectHistory`, so the app-level bridge keeps autosave coverage
    /// independent of the individual panel implementation.
    pub autosave_history_generation: u64,
    /// Show the crash-recovery restore prompt at startup
    pub show_recovery_dialog: bool,
    /// Ensures the startup recovery check runs only once
    pub recovery_checked: bool,
    /// Human-readable timestamp of the recovery snapshot (for the dialog)
    pub recovery_snapshot_time: Option<String>,
    /// Version-history list for the recovery dialog, fetched once per dialog
    /// open instead of re-reading every snapshot on every frame.
    pub recovery_generations: Option<Vec<crate::core::autosave::RecoveryGeneration>>,
    /// In-flight background snapshot load for the recovery dialog.
    pub recovery_load_rx: Option<std::sync::mpsc::Receiver<RecoveryLoadResult>>,
    /// Background fetch of the version-history list for the recovery dialog,
    /// so the UI thread does not JSON-parse snapshots while painting.
    pub recovery_generations_rx:
        Option<std::sync::mpsc::Receiver<Vec<crate::core::autosave::RecoveryGeneration>>>,
    /// Which control started the in-flight recovery load (see [`RecoveryLoadKind`]).
    pub recovery_load_kind: RecoveryLoadKind,
    /// Join handles for background threads spawned during the session.
    ///
    /// Kept (rather than `.spawn().ok()` discarding the handle) so `on_exit()`
    /// can join them and avoid detached threads outliving the process.
    /// Wrapped in a `Mutex` so spawn sites that only hold `&KagariApp`
    /// (e.g. Content-Aware Fill / Roto Brush job starters) can still register
    /// their handles via [`KagariApp::track_background_thread`].
    pub background_threads: std::sync::Mutex<Vec<std::thread::JoinHandle<()>>>,
    /// Cooperative-cancel flags for background jobs, registered at spawn time.
    ///
    /// `on_exit()` sets every registered flag **before** joining
    /// [`KagariApp::background_threads`] so long-running workers (export,
    /// proxy bake, CAF, roto, …) abort promptly instead of blocking shutdown
    /// until they finish naturally.
    pub exit_cancel_flags:
        std::sync::Mutex<Vec<std::sync::Arc<std::sync::atomic::AtomicBool>>>,
    /// Playback domain state (transport, frame, work area, volume)
    pub playback: PlaybackDomainState,
    /// Selection domain state (selected layers, expanded, property)
    pub selection: SelectionDomainState,
    /// UI tabs domain state (tab indices, search queries, shy)
    pub ui_tabs: UiTabsDomainState,
    /// Export domain state (dialog, progress, format)
    pub export: ExportDomainState,
    /// Loop playback at the work area / comp end (Preview panel toggle)
    pub loop_playback: bool,
    /// Adaptive preview quality: multiplier (0.125..=1.0) applied to the viewport
    /// render width while playing. When frames take longer than the playback
    /// budget, the factor drops so playback stays smooth; it recovers when fast.
    /// This mirrors AE's automatic resolution reduction during RAM preview.
    pub playback_speed: i32,
    pub u_key_last_press: Option<f64>,
    /// Selected keyframes: (layer_idx, property key, frame).
    pub selected_keyframes: std::collections::HashSet<(usize, String, u32)>,
    pub drag_tx: Option<DragTransaction>,
    /// Effect being dragged from the Effects & Presets library.
    /// Stores (effect_name, create_fn_index) while dragging, None otherwise.
    pub dragging_effect: Option<(String, usize)>,
    #[cfg(feature = "wgpu")]
    pub renderer: Option<crate::core::renderer::WgpuRenderer>,
    #[cfg(feature = "wgpu")]
    pub wgpu_state: Option<eframe::egui_wgpu::RenderState>,
    pub viewport_texture_id: Option<eframe::egui::TextureId>,
    /// RAM preview: (frame -> egui texture id) entries for pre-rendered frames.
    pub ram_texture_ids: Vec<(u32, eframe::egui::TextureId)>,
    /// True if the previous frame had playback active — used to detect playback
    /// start and kick off the RAM preview pre-pass.
    pub was_playing_last_frame: bool,
    /// Internal clipboard for copied keyframes: (prop_key, offset-from-anchor frame, value+interpolation JSON).
    /// Values are stored as serde_json::Value to stay type-erased across tracks.
    pub kf_clipboard: Vec<(String, i32, serde_json::Value)>,
    /// Frame the clipboard anchor was at when copied (paste preserves relative spacing).
    pub kf_clipboard_anchor: u32,
    /// Incremental RAM preview pre-pass state: next frame to pre-render.
    pub ram_prepass_cursor: u32,
    /// Last frame (inclusive) of the current pre-pass.
    pub ram_prepass_end: u32,
    pub viewport_snapshot_texture_id: Option<eframe::egui::TextureId>,
    pub rx_frame: Option<crate::core::integration::LatestFrameReceiver>,
    pub rx_connection: Option<std::sync::mpsc::Receiver<Option<String>>>,
    pub connected_app: Option<String>,
    pub project_path: String,
    pub project_session_id: u64,
    pub pending_project_load: Option<crate::ui::project_io::PendingProjectLoad>,
    pub pending_color_lut_load: Option<crate::ui::project_io::PendingColorLutLoad>,
    pub repaint_context: Option<eframe::egui::Context>,
    /// Single lazily captured pre-edit snapshot for panels that mutate Project in-place.
    pub panel_edit_snapshot: Option<(String, u64, u64, Project)>,
    /// Preserves unified audio/timeline metadata when the GUI opens and saves a project.
    pub production_document: Option<crate::core::production_document::ProductionDocument>,
    /// Last history generation synced to production_document. Avoids cloning the
    /// entire Project every frame when nothing has changed.
    pub doc_sync_generation: u64,
    pub automation_undo: Vec<Vec<crate::core::automation_binding::AutomationBinding>>,
    pub automation_redo: Vec<Vec<crate::core::automation_binding::AutomationBinding>>,
    pub otio_path: String,
    pub expanded_waveform_layers: std::collections::HashSet<usize>,
    /// Breadcrumb trail of composition indices visited via PreComp
    /// double-click navigation (most recent = last). Back = pop.
    pub comp_nav_stack: Vec<usize>,
    pub timeline_fit_to_selection: bool,
    pub timeline_fit_all: bool,
    /// Index of the layer currently being drag-reordered in the timeline.
    pub dragging_layer: Option<usize>,
    pub show_grid: bool,
    pub show_guides: bool,
    pub viewport_show_stats: bool,
    pub viewer_maximized: bool,
    pub show_handles: bool,
    pub show_comp_settings: bool,
    /// Working copy for the composition settings dialog. Keeping this outside
    /// the immediate-mode draw call lets text and numeric edits survive frames
    /// until the user presses OK or Cancel.
    pub comp_settings_draft: Option<Project>,
    pub show_shortcuts_dialog: bool,
    pub show_precompose_dialog: bool,
    pub show_sequence_layers: bool,
    pub show_the_smoother: bool,
    pub show_the_wiggler: bool,
    pub show_motion_sketch: bool,
    pub show_physics: bool,
    pub show_inline_numeric_editor: bool,
    pub precompose_name: String,
    pub precompose_move_attributes: bool,
    pub show_command_palette: bool,
    /// 📊 Vectorscope / RGB Parade floating scope window (Shift+F4)
    pub show_vectorscope: bool,
    pub show_history_panel: bool,
    pub show_quality_check_panel: bool,
    pub show_automation_panel: bool,
    pub selected_automation_binding: Option<usize>,
    pub quality_check_result: Option<crate::ui::quality_check_panel::QualityCheckResult>,
    pub show_welcome: bool,
    /// Skill-level UI mode: Beginner hides advanced panels/menus.
    pub ui_mode: crate::ui::mode::UiMode,
    /// Interactive tutorial state (None = closed).
    pub tutorial: Option<crate::ui::tutorial::TutorialState>,
    pub show_new_comp_dialog: bool,
    pub show_preferences: bool,
    /// Audio preview during playback (Preferences).
    pub audio_preview_enabled: bool,
    /// Handle to the running egui Context (set each frame; used by prefs).
    pub ui_ctx: Option<eframe::egui::Context>,
    pub command_palette_search: String,
    pub command_palette_selected_idx: usize,
    pub snap_to_keyframes: bool,
    pub show_graph_editor: bool,
    pub linked_tangent: bool,
    pub timeline_zoom: f32,
    /// Left edge (frame) of the visible timeline window. Stays fixed while
    /// scrubbing so the ruler does not slide under the cursor; only re-centers
    /// when the playhead leaves the visible range or via navigation keys.
    pub timeline_view_start: u32,
    pub timeline_scroll: f32,
    pub viewport_mode: ViewportMode,
    pub camera_orbit: (f32, f32, f32),
    pub orbit_drag_start: Option<eframe::egui::Pos2>,
    pub active_tool: crate::ui::toolbar::ActiveTool,
    pub viewport_drag_state: Option<(usize, [f32; 2], eframe::egui::Pos2)>,
    /// One-shot request: frame this comp-space bbox (min,max) in the viewport.
    pub viewport_focus_bbox: Option<([f32; 2], [f32; 2])>,
    pub viewport_mask_drag_state: Option<(usize, usize, usize, [f32; 2], eframe::egui::Pos2)>,
    pub viewport_mask_drag_vertices: Vec<(usize, [f32; 2])>,
    /// Pen tool Bezier drag: (tangent_out_screen, tangent_in_screen) while click-dragging
    pub pen_bezier_drag: Option<(eframe::egui::Pos2, eframe::egui::Pos2)>,
    /// Mask multi-vertex selection: (layer_idx, mask_idx, set of vertex indices)
    pub mask_selected_vertices: Option<(usize, usize, std::collections::HashSet<usize>)>,
    /// Mask tangent handle drag: (layer_idx, mask_idx, vertex_idx, is_outgoing, start_handle_pos, start_pointer)
    pub mask_tangent_drag_state: Option<(usize, usize, usize, bool, [f32; 2], eframe::egui::Pos2)>,
    /// Spatial drag of a position keyframe dot on the motion path:
    /// (layer_idx, keyframe_frame, start_value, start_pointer)
    pub viewport_pos_kf_drag_state: Option<(usize, u32, [f32; 2], eframe::egui::Pos2)>,
    /// (layer_idx, kf_frame, handle_type: 0=out/1=in, start_bezier_points, start_pointer)
    pub viewport_tangent_drag_state: Option<(usize, u32, u8, [f32; 4], eframe::egui::Pos2)>,
    pub viewport_linked_tangent: bool,
    /// Pick whip mode: when true, clicking a layer sets it as parent
    pub pick_whip_mode: bool,
    /// Layer index being picked as parent
    pub pick_whip_target: Option<usize>,
    /// Corner-handle scale drag: (layer_idx, start_scale, start_pointer_distance)
    pub viewport_scale_drag: Option<(usize, [f32; 2], f32)>,
    /// Multi-layer group drag: (per-layer start positions, start pointer)
    #[allow(clippy::type_complexity)]
    pub viewport_multi_drag: Option<(Vec<(usize, [f32; 2])>, eframe::egui::Pos2)>,
    /// Rectangle tool rubber-band: (start pointer position in screen space)
    pub rect_drag_start: Option<eframe::egui::Pos2>,
    /// Layer index whose text is being edited inline from the viewport (double-click)
    pub inline_text_edit_layer: Option<usize>,
    /// Pen tool: in-progress mask vertices in composition coordinates
    pub pen_points: Vec<[f32; 2]>,
    /// Pen tool: Bezier tangent handles per point. (tangent_out, tangent_in) relative to vertex.
    /// Empty or shorter than pen_points means zero tangents (corner point).
    pub pen_tangents: Vec<([f32; 2], [f32; 2])>,
    /// Motion Sketch: records position keyframes while playing + dragging.
    pub motion_sketch_active: bool,
    /// Motion Sketch recording buffer: (frame, [x, y]) pairs captured during drag.
    pub motion_sketch_recording: Vec<(u32, [f32; 2])>,
    pub motion_sketch_bake_rx: Option<Receiver<MotionSketchBakeResult>>,
    pub export_format_preset: usize,
    /// Video codec selection shared by Export dialog + Render presets (0=H264,1=ProRes422,2=ProRes4444)
    pub export_codec_idx: usize,
    pub export_resolution_scale: usize,
    pub import_rx: Vec<crate::ui::drop_import::ImportJob>,
    pub collect_files_rx: Option<Receiver<Result<(std::path::PathBuf, usize), String>>>,
    pub viewer_draw_rect: Option<eframe::egui::Rect>,
    pub camera_solve_rx: Option<
        std::sync::mpsc::Receiver<Result<crate::core::camera_track::CameraSolveResult, String>>,
    >,
    pub camera_solve_target: Option<(String, u32, u32, f32, usize)>,
    pub quad_track_rx: Option<std::sync::mpsc::Receiver<QuadTrackJobResult>>,
    pub quad_track_cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Background proxy generation: worker renders low-res media off the UI thread.
    pub proxy_job_id: u64,
    pub proxy_job_rx: Option<std::sync::mpsc::Receiver<ProxyJobMessage>>,
    pub proxy_job_cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// In-flight proxy job identity (layer id + resolution label) for progress display.
    pub proxy_job_label: Option<String>,
    /// Latest `(done, total)` frame progress reported by the proxy worker.
    pub proxy_job_progress: Option<(u32, u32)>,
    /// Path of the most recent export (for the "Reveal in Finder" button).
    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub last_export_path: Option<String>,
    pub export_cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub tracker_cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub master_eq_highpass: f32,
    pub master_eq_lowpass: f32,
    pub master_eq_mid_gain: f32,
    pub master_eq_mid_freq: f32,
    pub master_comp_threshold: f32,
    pub master_comp_ratio: f32,
    pub master_comp_attack: f32,
    pub master_comp_release: f32,
    pub master_comp_makeup: f32,
    pub master_auto_gain: bool,
    pub master_target_level_db: f32,
    pub master_limiter_enabled: bool,
    pub master_limiter_ceiling_db: f32,
    pub master_wet_dry: f32,
    pub master_dsp_enabled: bool,
    /// Temporary A/B bypass state; this is not part of saved correction settings.
    pub master_dsp_bypass: bool,
    pub viewport_cam_view: usize,
    pub viewport_render_resolution: PreviewResolution,
    pub viewport_color_channel: usize,
    pub viewport_fast_preview: usize,
    pub project_search_query: String,
    pub cc_libraries_search: String,
    pub audio_mixer_channels: Vec<MixerChannel>,
    /// Synced audio playback for AV preview (gui builds with an audio device).
    #[cfg(feature = "gui")]
    pub audio_playback: Option<crate::core::audio_playback::AudioPlayback>,
    /// Live audio meter (linear 0..1) from the mix, updated each UI frame.
    pub audio_meter: (f32, f32),
    /// Live 32-band spectrum values derived from the same mix used by the VU meter.
    pub audio_spectrum_bands: Vec<f32>,
    /// Per-layer post-fader peaks for the Audio Mixer strips.
    pub audio_track_meters: Vec<(f32, f32)>,
    pub audio_error_notices: HashSet<String>,
    /// Whether the last viewport render used GPU (updated each frame by viewport)
    pub gpu_rendered: bool,
    /// Layer index being renamed (inline edit), None = not renaming
    pub renaming_layer: Option<usize>,
    /// Tracker panel: target layer index for Apply Motion.
    pub tracker_apply_target: Option<usize>,
    /// Home screen (landing): browser directory, selection, search.
    pub show_home: bool,
    /// One-shot launch transition for the first rendered frame.
    pub startup_animation: crate::ui::panel_animation::PanelAnimation,
    /// Asynchronous/transitional project loading animation state: (project_name, progress 0.0..1.0, target_path)
    pub project_loading: Option<(String, f32, std::path::PathBuf)>,
    pub home_dir: std::path::PathBuf,
    pub home_selected: Option<std::path::PathBuf>,
    pub home_search: String,
    /// Cached starred project keys; refreshed only at startup and after a toggle.
    pub home_starred_projects: Vec<String>,
    /// Cached home-screen textures: banner art, file thumbnails, preview.
    pub home_banner: Option<eframe::egui::TextureHandle>,
    pub home_thumbs: std::collections::HashMap<std::path::PathBuf, eframe::egui::TextureHandle>,
    pub home_preview: Option<(std::path::PathBuf, eframe::egui::TextureHandle)>,
    /// Stable composition IDs awaiting export.
    pub render_queue_items: Vec<String>,
    /// Per-item queue outcome, keyed by composition ID.
    pub render_item_status: std::collections::HashMap<String, QueueItemStatus>,
    /// Sequential batch export: remaining comps to render after the current one.
    pub batch_queue: Vec<String>,
    /// Index of the comp currently rendering within the original queue snapshot.
    pub batch_idx: usize,
    pub camera_view_layout: usize,
    pub camera_view_angle: usize,
    pub font_family_idx: usize,
    pub faux_font_switches: (bool, bool, bool, bool),
    /// Essential Graphics properties currently exposed for the MOGRT editor.
    pub mogrt_properties: Vec<crate::core::mogrt_engine::EssentialProperty>,
    pub toasts: crate::ui::notification::ToastManager,
    pub frame_cache: crate::core::frame_cache::FrameCache,
    pub lazy_evaluator: crate::core::render_pipeline::LazyFrameEvaluator,
    pub async_render_pipeline: Option<crate::core::render_pipeline::RenderPipeline>,
    /// Panel animation states for smooth open/close transitions
    pub inspector_animation: crate::ui::panel_animation::PanelAnimation,
    pub effects_animation: crate::ui::panel_animation::PanelAnimation,
    /// Scripting console state
    pub script_console_output: Option<Vec<String>>,
    pub script_console_history: Option<Vec<String>>,
    pub script_console_command: String,
    pub script_console_rx: Option<Receiver<ScriptConsoleCompletion>>,
    pub script_console_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Persisted LUT selection for the active production document.
    pub color_lut_path: Option<String>,
    pub color_lut_mode: u32,
    /// Custom saved workspaces
    pub custom_workspaces: Vec<crate::ui::workspace_manager::SavedWorkspace>,
    /// Selected expression property index
    pub selected_expression_prop_idx: usize,
    /// User interface complexity level (Beginner vs Pro Studio)
    pub skill_level: SkillLevel,
    /// Guided interactive tutorial modal
    pub show_guided_tutorial: bool,
    pub tutorial_step: usize,
}

#[cfg(feature = "gui")]
impl KagariApp {
    /// Register a background thread handle so `on_exit()` can join it
    /// instead of letting it run detached past process shutdown.
    pub fn track_background_thread(&self, handle: std::thread::JoinHandle<()>) {
        self.background_threads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(handle);
    }

    /// Register a cooperative-cancel flag that `on_exit()` flips before
    /// joining background threads, so long jobs abort instead of hanging
    /// shutdown.
    pub fn register_exit_cancel_flag(
        &self,
        flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        self.exit_cancel_flags
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(flag);
    }
}

#[cfg(feature = "gui")]
#[cfg(test)]
mod tests;

#[cfg(test)]
mod right_panel_tests {
    use super::RightPanel;

    #[test]
    fn right_panel_indices_round_trip_for_legacy_workspace_state() {
        for index in 0..=30 {
            assert_eq!(RightPanel::from_index(index).index(), index);
        }
        assert_eq!(RightPanel::from_index(73), RightPanel::Other(73));
        assert_eq!(RightPanel::Other(73).index(), 73);
    }
}
