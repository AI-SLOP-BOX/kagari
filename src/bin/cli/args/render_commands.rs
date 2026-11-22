use super::*;

#[derive(Subcommand)]
pub(crate) enum RenderCommands {
    /// Render a composition to PNG sequence or MP4
    Render {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,

        /// Composition name or index (default: first)
        #[arg(short, long)]
        comp: Option<String>,

        /// Output directory (for PNG sequence) or file path (for MP4)
        #[arg(short, long, default_value = "./output")]
        output: String,

        /// Output format: png, mp4, gif
        #[arg(short, long, default_value = "png", value_enum)]
        format: OutputFormat,

        /// Frame range or list (e.g. "1-10,15,20-50" or "0-100"). Sparse selections are supported for PNG/EXR; MP4/GIF require contiguous frames. Overrides from/to if set.
        #[arg(long)]
        frames: Option<String>,

        /// Start frame (inclusive, AE alias: -s)
        #[arg(short = 's', long, default_value = "0")]
        from: u32,

        /// End frame (inclusive, AE alias: -e, default: last frame)
        #[arg(short = 'e', long)]
        to: Option<u32>,

        /// Output width (overrides composition size)
        #[arg(long)]
        width: Option<u32>,

        /// Output height (overrides composition size)
        #[arg(long)]
        height: Option<u32>,

        /// Exposure value for color grading (-5.0 to 5.0)
        #[arg(long, default_value = "0.0")]
        exposure: f32,

        /// LUT mode: 0=none, 1=linear, 2=ACES, 3=active .cube 3D LUT
        #[arg(long, default_value = "0")]
        lut: u32,

        /// Path to custom .cube 3D LUT file for cinematic color grading (sets lut mode to 3)
        #[arg(long)]
        lut_file: Option<String>,

        /// Override working color space: rec709, display_p3, rec2020, linear, acescg
        #[arg(long)]
        color_space: Option<String>,

        /// Frame step / cadence for PNG/EXR output (e.g. 2 renders every second frame like AE); MP4/GIF require step 1
        #[arg(long, default_value = "1")]
        step: u32,

        /// Number of worker threads for parallel frame rendering (0 = auto CPU cores)
        #[arg(short = 'j', long, default_value = "0")]
        threads: usize,

        /// Skip already rendered frames on disk (for distributed rendering / render farms)
        #[arg(long)]
        skip_existing: bool,

        /// Enable Multi-Frame Rendering (MFR) across CPU cores
        #[arg(long, default_value = "true")]
        mfr: bool,

        /// Progress log format: standard (interactive terminal) or json (for render farm controllers/Deadline)
        #[arg(long, default_value = "standard")]
        log_format: String,

        /// Live binding values such as audio.bass=0.75; may be repeated
        #[arg(long = "binding", value_parser = parse_binding_value)]
        bindings: Vec<(String, f64)>,

        /// Dynamic template overrides in LayerName=Value or LayerName.text=Value form; may be repeated
        #[arg(long = "override", value_parser = parse_string_override)]
        overrides: Vec<(String, String)>,

        /// Dynamic expression injection in LayerName.property=expression form (e.g. "Logo.rotation=wiggle(3.0, 25.0)" or "Text.opacity=time*50"); may be repeated
        #[arg(long = "expr", alias = "expression", value_parser = parse_string_override)]
        expressions: Vec<(String, String)>,
    },

    /// Batch render all compositions in the project (AE Render Queue equivalent)
    Queue {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,

        /// Output root directory where subfolders or files for each comp are written
        #[arg(short, long, default_value = "./output")]
        output: String,

        /// Output format: png, exr, mp4, gif
        #[arg(short, long, default_value = "png", value_enum)]
        format: OutputFormat,

        /// Number of worker threads for parallel frame rendering (0 = auto CPU cores)
        #[arg(short = 'j', long, default_value = "0")]
        threads: usize,

        /// Progress log format: standard or json
        #[arg(long, default_value = "standard")]
        log_format: String,
    },

    /// Render and analyze a frame for exposure clipping and focus quality
    Qc {
        #[arg(short, long)]
        project: String,
        /// Composition name or index (default: first)
        #[arg(short, long)]
        comp: Option<String>,
        /// Analyze one frame; overrides --from and --to
        #[arg(short, long)]
        frame: Option<u32>,
        /// First frame for batch analysis
        #[arg(long, default_value = "0")]
        from: u32,
        /// Last frame for batch analysis (default: composition end)
        #[arg(long)]
        to: Option<u32>,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Optional report output path; stdout is used when omitted
        #[arg(short, long)]
        output: Option<String>,
        /// Return a failing exit status when any QC warning is found
        #[arg(long)]
        fail_on_warnings: bool,
    },

    /// Render a single frame to PNG (convenience shorthand)
    Frame {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,

        /// Frame number to render
        #[arg(short, long, default_value = "0")]
        frame: u32,

        /// Output PNG path
        #[arg(short, long, default_value = "./frame.png")]
        output: String,

        /// Output width
        #[arg(long)]
        width: Option<u32>,

        /// Output height
        #[arg(long)]
        height: Option<u32>,

        /// Render with the experimental WGPU preview renderer
        #[arg(long)]
        gpu: bool,
    },

    /// Export the project to Lottie / Bodymovin JSON (transforms + keyframes; effects not included)
    Lottie {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,

        /// Output JSON path
        #[arg(short, long, default_value = "./lottie_export.json")]
        output: String,
    },

    /// Export or Import composition timeline to/from OpenTimelineIO (.otio.json) for Kdenlive, DaVinci, Premiere
    Otio {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,

        /// Composition name or zero-based index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,

        /// OpenTimelineIO JSON path (exported to or imported from)
        #[arg(short, long, default_value = "./timeline.otio.json")]
        output: String,

        /// Import the OTIO file into the project instead of exporting
        #[arg(long)]
        import: bool,
    },

    /// Autonomous Visual & Rhythm Critique: inspect frames for contrast, clipping, sharpness, audio sync, and emit actionable tuning suggestions
    Critique {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
        /// Sample frame interval for critique inspection (default: 15)
        #[arg(long, default_value = "15")]
        sample_step: u32,
        /// Emit machine-readable JSON critique report with specific tuning scores
        #[arg(long)]
        json: bool,
    },

    /// Cinematic Composite Finishing: add 35mm film grain, anamorphic letterbox, lens chromatic aberration, and vignette
    Cinematic {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Preset style: 35mm, vintage, modern-action, anamorphic, noir
        #[arg(long, default_value = "35mm")]
        preset: String,
        /// Physical 35mm film grain intensity (0.0..1.0, default: 0.15)
        #[arg(long, default_value = "0.15")]
        grain: f32,
        /// Anamorphic 2.39:1 letterbox aspect ratio
        #[arg(long, default_value = "true")]
        letterbox: bool,
        /// Lens chromatic aberration fringe offset in pixels (default: 2.5)
        #[arg(long, default_value = "2.5")]
        aberration: f32,
        /// Lens edge vignette darkening intensity (0.0..1.0, default: 0.35)
        #[arg(long, default_value = "0.35")]
        vignette: f32,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Autonomous Music Video Production: generate viral kinetic 3D typography, audio-reactive VFX, beat-synced camera work, and final MP4 export
    Mv {
        /// Background song / audio file (MP3, WAV, FLAC, AAC)
        #[arg(short = 'a', long)]
        audio: String,
        /// Lyrics, titles, or prompt keywords (e.g. "YOU ARE ABSOLUTELY RIGHT / LGTM / HUMAN IN THE LOOP")
        #[arg(short = 'l', long)]
        lyrics: Option<String>,
        /// Path to lyrics text file (.txt) with timestamps or lines
        #[arg(long)]
        lyrics_file: Option<String>,
        /// Output project JSON or direct video path (ends in .json or .mp4)
        #[arg(short, long, default_value = "./music_video.mp4")]
        output: String,
        /// Visual aesthetic style: cyberpunk-neon, analog-grunge, kinetic-swiss, deep-space, hyper-pop
        #[arg(long, default_value = "cyberpunk-neon")]
        style: String,
        /// Video resolution width (default: 1920, supports 4K 3840)
        #[arg(long, default_value = "1920")]
        width: u32,
        /// Video resolution height (default: 1080, supports 4K 2160 or 9:16 vertical 1080x1920 for Shorts/TikTok)
        #[arg(long, default_value = "1080")]
        height: u32,
        /// Frame rate (default: 60 for ultra-smooth kinetic motion)
        #[arg(long, default_value = "60")]
        fps: u32,
        /// Maximum duration in seconds to generate (default: auto from audio length or 30s)
        #[arg(short = 'd', long)]
        duration_sec: Option<u32>,
        /// Fast render mode (skip final MP4 render and write project.json only)
        #[arg(long)]
        project_only: bool,
    },
}
