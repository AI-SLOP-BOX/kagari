use super::{mocap::parse_point, parse_binding_value, parse_sample_point, parse_string_override};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "kagari")]
#[command(about = "Kagari VFX — Headless compositing & motion-graphics engine", long_about = None)]
#[command(version)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Commands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum OutputFormat {
    Png,
    Exr,
    Mp4,
    Gif,
}

#[derive(Subcommand)]
pub(super) enum Commands {
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

        /// Frame range or list (e.g. "1-10,15,20-50" or "0-100"). Overrides from/to if set.
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

        /// LUT mode: 0=none, 1=linear, 2=ACES
        #[arg(long, default_value = "0")]
        lut: u32,

        /// Override working color space: rec709, display_p3, rec2020, linear, acescg
        #[arg(long)]
        color_space: Option<String>,

        /// Frame step / cadence (e.g. 2 renders every second frame like AE)
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

    /// List available effects and their parameters
    Effects {
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Show project composition info
    Info {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
    },

    /// Show unified audio, tempo, and cross-domain binding information
    ProductionInfo {
        /// Path to a unified production JSON file
        #[arg(short, long)]
        project: String,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Evaluate cross-domain automation bindings from source=value pairs
    Bindings {
        /// Path to a unified production JSON file
        #[arg(short, long)]
        project: String,
        /// Source values such as audio.bass=0.75; may be repeated
        #[arg(long = "value", value_parser = parse_binding_value)]
        values: Vec<(String, f64)>,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Write resolved targets as opacity keyframes at this frame
        #[arg(long)]
        apply_frame: Option<u32>,
        /// Evaluate saved automation curves at this video frame
        #[arg(long)]
        frame: Option<u32>,
        /// Composition name or zero-based index used with --frame
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Add Logic Pro-style sample-position automation to a production file
    AddSampleBinding {
        /// Path to a unified production JSON file
        #[arg(short, long)]
        project: String,
        /// Audio/DAW source endpoint
        #[arg(long)]
        source: String,
        /// VFX target endpoint
        #[arg(long)]
        target: String,
        /// Repeated absolute sample points in `sample=value` form
        #[arg(long = "sample-point", value_parser = parse_sample_point)]
        points: Vec<(i64, f64)>,
        /// Source automation minimum
        #[arg(long)]
        input_min: f64,
        /// Source automation maximum
        #[arg(long)]
        input_max: f64,
        /// VFX target minimum
        #[arg(long)]
        output_min: f64,
        /// VFX target maximum
        #[arg(long)]
        output_max: f64,
    },

    /// Validate a project file
    Validate {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
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

    /// Render a single frame to PNG (for testing)
    /// Run an automation script (Rhai) against a project
    Script {
        /// Path to project JSON file (created/overwritten by save_project)
        #[arg(short, long)]
        project: String,

        /// Path to .rhai script file
        #[arg(short, long)]
        file: String,
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

    /// Track points through a PNG frame sequence using markerless optical flow
    Mocap {
        /// Directory containing frame_*.png files
        #[arg(long)]
        frames: String,
        /// Initial point as x,y; may be repeated
        #[arg(long = "point", value_parser = parse_point)]
        points: Vec<[f32; 2]>,
        /// Output JSON path
        #[arg(short, long, default_value = "./mocap.json")]
        output: String,
        /// Block matching radius
        #[arg(long, default_value = "2")]
        block_radius: i32,
        /// Search radius in pixels
        #[arg(long, default_value = "16")]
        search_radius: i32,
        /// Discard samples below this confidence
        #[arg(long, default_value = "0.0")]
        min_confidence: f32,
        /// Estimate a 17-joint humanoid pose instead of tracking explicit points
        #[arg(long)]
        pose: bool,
        /// Output frame rate used for BVH frame time
        #[arg(long, default_value = "30.0")]
        fps: f32,
        /// Maximum automatically detected features used for pose estimation
        #[arg(long, default_value = "64")]
        max_features: usize,
        /// Minimum spacing between automatically detected features
        #[arg(long, default_value = "12")]
        feature_spacing: u32,
    },

    /// Initialize a new project JSON file
    Init {
        /// Output project JSON path
        #[arg(short, long, default_value = "./project.json")]
        project: String,
        /// Composition name
        #[arg(short, long, default_value = "Main Comp")]
        name: String,
        /// Width in pixels
        #[arg(long, default_value = "1920")]
        width: u32,
        /// Height in pixels
        #[arg(long, default_value = "1080")]
        height: u32,
        /// Frame rate (fps)
        #[arg(long, default_value = "30")]
        fps: u32,
        /// Total duration in frames
        #[arg(long, default_value = "300")]
        duration: u32,
    },

    /// Add a layer to an existing project file
    AddLayer {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Layer name
        #[arg(short, long)]
        name: String,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
        /// Layer type: text, image, solid, video
        #[arg(long, default_value = "text")]
        kind: String,
        /// Content value (text string, file path, or RGBA color)
        #[arg(long)]
        content: Option<String>,
        /// In-point frame
        #[arg(long, default_value = "0")]
        in_frame: u32,
        /// Out-point frame (default: composition duration)
        #[arg(long)]
        out_frame: Option<u32>,
    },

    /// Animate a property of a layer with keyframes
    Animate {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Property to animate: position, scale, opacity, rotation
        #[arg(long)]
        property: String,
        /// Keyframe specification in frame=value form (e.g. 0=0 or 30=960,540); may be repeated
        #[arg(long = "keyframe")]
        keyframes: Vec<String>,
        /// Easing preset: linear, ease-in, ease-out, ease-in-out (default: ease-in-out)
        #[arg(long, default_value = "ease-in-out")]
        ease: String,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Add an effect to a layer (e.g. GaussianBlur, Glow, DropShadow, Levels, Vignette)
    AddEffect {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Effect type: blur, glow, shadow, levels, vignette, tint, chromatic_aberration
        #[arg(long)]
        kind: String,
        /// Initial parameter value (e.g. radius for blur, intensity for glow)
        #[arg(long)]
        param: Option<f32>,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Trim in-point and out-point of a layer (like [ and ] in AE)
    Trim {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target layer name or ID
        #[arg(short, long)]
        layer: String,
        /// In-point frame
        #[arg(long)]
        in_frame: Option<u32>,
        /// Out-point frame
        #[arg(long)]
        out_frame: Option<u32>,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },
}
