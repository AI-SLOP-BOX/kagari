use super::*;

/// User interface complexity mode (Beginner vs Pro/Studio).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SkillLevel {
    /// Simplified streamlined UI focused on essential timeline and inspector controls.
    Beginner,
    /// Full professional layout with graph editor, scopes, motion tracking, and 3D tooling.
    #[default]
    Pro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewResolution {
    #[default]
    Full,
    Half,
    Third,
    Quarter,
}

/// Playback domain state: transport, current frame, work area bounds, volume.
#[derive(Debug, Clone)]
pub struct PlaybackDomainState {
    pub is_playing: bool,
    /// Adaptive preview quality: multiplier (0.125..=1.0) applied to the viewport
    /// render width while playing.
    pub adaptive_preview_factor: f32,
    pub viewport_pan: eframe::egui::Vec2,
    pub preview_render_ema_ms: f32,
    pub current_frame: u32,
    pub master_volume: f32,
    pub work_area_in: Option<u32>,
    pub work_area_out: Option<u32>,
    /// Wall-clock instant when playback last started (for AV-sync).
    pub play_start_time: Option<std::time::Instant>,
    /// Frame at which playback last started.
    pub play_start_frame: u32,
    /// Cached mixed audio WAV path to avoid re-mixing every UI update.
    pub cached_audio_path: Option<std::path::PathBuf>,
    /// Hash of (comp_id, work_area, mixer_channels, DSP params) to detect when re-mix is needed.
    pub cached_audio_key: u64,
}

impl Default for PlaybackDomainState {
    fn default() -> Self {
        Self {
            is_playing: false,
            adaptive_preview_factor: 1.0,
            viewport_pan: eframe::egui::Vec2::ZERO,
            preview_render_ema_ms: 0.0,
            current_frame: 0,
            master_volume: 1.0,
            work_area_in: None,
            work_area_out: None,
            play_start_time: None,
            play_start_frame: 0,
            cached_audio_path: None,
            cached_audio_key: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelectionDomainState {
    pub selected_layer_idx: Option<usize>,
    pub selected_layers: HashSet<usize>,
    pub selected_property: Option<String>,
    pub expanded_layers: HashSet<usize>,
}

impl SelectionDomainState {
    pub fn select_single(&mut self, idx: usize) {
        self.selected_layers.clear();
        self.selected_layers.insert(idx);
        self.selected_layer_idx = Some(idx);
    }

    pub fn toggle_select(&mut self, idx: usize) {
        if self.selected_layers.contains(&idx) {
            self.selected_layers.remove(&idx);
            if self.selected_layer_idx == Some(idx) {
                self.selected_layer_idx = self.selected_layers.iter().next().copied();
            }
        } else {
            self.selected_layers.insert(idx);
            self.selected_layer_idx = Some(idx);
        }
    }

    pub fn clear(&mut self) {
        self.selected_layers.clear();
        self.selected_layer_idx = None;
    }
}

impl PlaybackDomainState {
    /// Start playback from the current frame, recording wall-clock time for AV sync.
    pub fn start_playing(&mut self) {
        if !self.is_playing {
            self.play_start_time = Some(std::time::Instant::now());
            self.play_start_frame = self.current_frame;
        }
        self.is_playing = true;
    }

    /// Stop playback.
    pub fn stop_playing(&mut self) {
        self.is_playing = false;
        self.play_start_time = None;
    }

    /// Detect and record wall-clock start if playback just transitioned to playing.
    /// Call this at the top of every update() to cover all toggle paths.
    pub fn sync_play_start(&mut self) {
        if self.is_playing && self.play_start_time.is_none() {
            self.play_start_time = Some(std::time::Instant::now());
            self.play_start_frame = self.current_frame;
        }
        if !self.is_playing {
            self.play_start_time = None;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightPanel {
    #[default]
    Effects,
    Align,
    Info,
    Tracker,
    Transport,
    Paint,
    Markers,
    Audio,
    TimeRemap,
    Masks,
    Expressions,
    EssentialGraphics,
    ContentAwareFill,
    Metadata,
    Scripting,
    Workspaces,
    ColorManagement,
    FlowchartInspector,
    CameraViews,
    Color,
    Libraries,
    Fonts,
    RenderPresets,
    AudioMixer,
    SpeedGraph,
    CameraLight,
    LayerStyles,
    Character,
    Paragraph,
    Reserved,
    Properties,
    Other(usize),
}

impl RightPanel {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Effects,
            1 => Self::Align,
            2 => Self::Info,
            3 => Self::Tracker,
            4 => Self::Transport,
            5 => Self::Paint,
            6 => Self::Markers,
            7 => Self::Audio,
            8 => Self::TimeRemap,
            9 => Self::Masks,
            10 => Self::Expressions,
            11 => Self::EssentialGraphics,
            12 => Self::ContentAwareFill,
            13 => Self::Metadata,
            14 => Self::Scripting,
            15 => Self::Workspaces,
            16 => Self::ColorManagement,
            17 => Self::FlowchartInspector,
            18 => Self::CameraViews,
            19 => Self::Color,
            20 => Self::Libraries,
            21 => Self::Fonts,
            22 => Self::RenderPresets,
            23 => Self::AudioMixer,
            24 => Self::SpeedGraph,
            25 => Self::CameraLight,
            26 => Self::LayerStyles,
            27 => Self::Character,
            28 => Self::Paragraph,
            29 => Self::Reserved,
            30 => Self::Properties,
            other => Self::Other(other),
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Effects => 0,
            Self::Align => 1,
            Self::Info => 2,
            Self::Tracker => 3,
            Self::Transport => 4,
            Self::Paint => 5,
            Self::Markers => 6,
            Self::Audio => 7,
            Self::TimeRemap => 8,
            Self::Masks => 9,
            Self::Expressions => 10,
            Self::EssentialGraphics => 11,
            Self::ContentAwareFill => 12,
            Self::Metadata => 13,
            Self::Scripting => 14,
            Self::Workspaces => 15,
            Self::ColorManagement => 16,
            Self::FlowchartInspector => 17,
            Self::CameraViews => 18,
            Self::Color => 19,
            Self::Libraries => 20,
            Self::Fonts => 21,
            Self::RenderPresets => 22,
            Self::AudioMixer => 23,
            Self::SpeedGraph => 24,
            Self::CameraLight => 25,
            Self::LayerStyles => 26,
            Self::Character => 27,
            Self::Paragraph => 28,
            Self::Reserved => 29,
            Self::Properties => 30,
            Self::Other(index) => index,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiTabsDomainState {
    pub left_tab_idx: usize,
    pub right_panel: RightPanel,
    pub bottom_dock_tab: usize,
    pub viewport_mag_ratio: f32,
    pub show_switches_pane: bool,
    pub global_shy_active: bool,
    pub layer_filter_text: String,
    pub effects_search_query: String,
}

impl Default for UiTabsDomainState {
    fn default() -> Self {
        Self {
            left_tab_idx: 0,
            right_panel: RightPanel::default(),
            bottom_dock_tab: 0,
            // Fit keeps the composition legible on first open and lets the
            // Viewer remain the visual center at every window size.
            viewport_mag_ratio: 0.0,
            show_switches_pane: true,
            global_shy_active: false,
            layer_filter_text: String::new(),
            effects_search_query: String::new(),
        }
    }
}

/// Outcome of a single render queue item, keyed by composition ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueItemStatus {
    Queued,
    Rendering,
    Done,
    Failed,
}
pub struct ExportDomainState {
    pub show_export_dialog: bool,
    pub export_status: Option<String>,
    pub export_progress: f32,
    pub export_fps: u32,
    pub export_output_path: String,
    /// Stable composition identity currently rendering (for per-item queue status).
    pub export_comp_id: Option<String>,
    /// Composition display name currently rendering.
    pub export_comp_name: Option<String>,
    pub is_exporting: bool,
    pub export_rx: Option<Receiver<ExportEvent>>,
    pub tracker_rx: Option<Receiver<TrackerEvent>>,
}

#[derive(Clone, Copy)]
pub enum QuadTrackPurpose {
    Perspective,
    PlanarSurface,
}

/// Result of a background proxy-generation job: the worker renders the
/// low-resolution media off the UI thread and reports the proxy path back.
pub struct ProxyJobResult {
    pub composition_name: String,
    pub layer_id: String,
    pub layer_idx: usize,
    pub resolution: crate::core::proxy::ProxyResolution,
    pub proxy_path: Result<String, String>,
}

/// Messages streamed from the proxy worker thread: periodic `(done, total)`
/// progress updates and exactly one [`ProxyJobMessage::Finished`].
pub enum ProxyJobMessage {
    Progress { done: u32, total: u32 },
    Finished(ProxyJobResult),
}

/// Which recovery-dialog control started the in-flight snapshot load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecoveryLoadKind {
    /// "復元する" — restore the newest generation, then clear recovery.
    #[default]
    Latest,
    /// A version-history entry — restore that generation but keep the history.
    Generation,
}

/// Outcome of a background recovery snapshot load (parsed off the UI thread).
pub struct RecoveryLoadResult {
    /// The snapshot file that was read; recorded for `clear_recovery` bookkeeping.
    pub path: std::path::PathBuf,
    /// `None` when the file could not be read or failed validation.
    pub loaded: Option<(
        crate::core::timeline::Project,
        Option<crate::core::production_document::ProductionDocument>,
    )>,
}

pub struct QuadTrackJobResult {
    pub purpose: QuadTrackPurpose,
    pub composition_name: String,
    pub composition_size: (u32, u32),
    pub layer_id: String,
    pub layer_idx: usize,
    pub track: crate::core::tracker_engine::QuadTrackData,
    pub average_confidence: f32,
}

impl Default for ExportDomainState {
    fn default() -> Self {
        Self {
            show_export_dialog: false,
            export_status: None,
            export_progress: 0.0,
            export_fps: 30,
            export_output_path: String::new(),
            export_comp_id: None,
            export_comp_name: None,
            is_exporting: false,
            export_rx: None,
            tracker_rx: None,
        }
    }
}

#[derive(Clone)]
pub struct DragTransaction {
    pub snapshot: Project,
    pub label: &'static str,
}

impl DragTransaction {
    pub fn new(snapshot: Project, label: &'static str) -> Self {
        Self { snapshot, label }
    }
}
