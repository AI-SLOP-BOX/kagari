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

        /// LUT mode: 0=none, 1=linear, 2=ACES, 3=active .cube 3D LUT
        #[arg(long, default_value = "0")]
        lut: u32,

        /// Path to custom .cube 3D LUT file for cinematic color grading (sets lut mode to 3)
        #[arg(long)]
        lut_file: Option<String>,

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
        /// Easing preset: linear, hold, ease-in, ease-out, ease-in-out (default: ease-in-out)
        #[arg(long, default_value = "ease-in-out")]
        ease: String,
        /// Custom cubic bezier curve in "x1,y1,x2,y2" form (e.g. "0.17,0.67,0.83,0.67" or "0.05,0.9,0.1,1.0")
        #[arg(long)]
        bezier: Option<String>,
        /// AE velocity influence (0.0 to 1.0) applied to incoming/outgoing keyframe handles
        #[arg(long)]
        influence: Option<f32>,
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

    /// Apply a transition preset (fade_in, fade_out, cross_dissolve, zoom_out, slide_out) to a layer
    Transition {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Transition type: fade-in, fade-out, zoom-out, slide-out
        #[arg(long)]
        kind: String,
        /// Transition duration in frames (default: 15)
        #[arg(long, default_value = "15")]
        duration: u32,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Set layer parenting / hierarchy (Parent Pickwhip equivalent in AE)
    Parent {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Child layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Parent layer name or ID (or "none" to unparent)
        #[arg(long)]
        parent: String,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Set track matte mode on a layer (alpha, alpha_inv, luma, luma_inv, none)
    Matte {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Consumer layer name or ID (the layer to be masked)
        #[arg(short, long)]
        layer: String,
        /// Matte mode: alpha, alpha-inv, luma, luma-inv, none
        #[arg(long)]
        mode: String,
        /// Source matte layer name or ID (default: adjacent layer above)
        #[arg(long)]
        source: Option<String>,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Add or modify a bezier mask on a layer (rect, ellipse, or closed polygon vertices)
    Mask {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Mask shape type: rect, ellipse, or polygon
        #[arg(long, default_value = "rect")]
        shape: String,
        /// Comma-separated parameters: for rect: "x,y,w,h"; for ellipse: "cx,cy,rx,ry"; for polygon: "x1,y1,x2,y2,x3,y3..."
        #[arg(long)]
        coords: String,
        /// Mask edge feather radius in pixels
        #[arg(long, default_value = "0.0")]
        feather: f32,
        /// Invert the mask
        #[arg(long)]
        inverted: bool,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Analyze an audio file and output transient beat frames for exact rhythm/cut synchronization
    Beats {
        /// Path to audio or video file
        #[arg(short, long)]
        file: String,
        /// Frame rate (fps) for time-to-frame conversion (default: 30.0)
        #[arg(long, default_value = "30.0")]
        fps: f32,
        /// Maximum duration in frames to analyze (default: 3000)
        #[arg(long, default_value = "3000")]
        duration: u32,
        /// Emit machine-readable JSON array of frame indices
        #[arg(long)]
        json: bool,
    },

    /// Ultra-fast low-resolution draft preview render for rapid rhythm & motion review
    Preview {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target composition name or index
        #[arg(short, long)]
        comp: Option<String>,
        /// Output preview MP4 path
        #[arg(short, long, default_value = "./preview.mp4")]
        output: String,
        /// Preview scale factor: 0.25 (quarter resolution), 0.5 (half resolution)
        #[arg(long, default_value = "0.5")]
        scale: f32,
        /// Frame step (e.g. 2 = every 2nd frame for 2x faster review)
        #[arg(long, default_value = "1")]
        step: u32,
    },

    /// Collect all project assets (footage, audio, images) into a self-contained bundle directory
    Bundle {
        /// Path to source project JSON file
        #[arg(short, long)]
        project: String,
        /// Target destination directory for the bundle
        #[arg(short, long, default_value = "./project_bundle")]
        output: String,
    },

    /// Time Remap & Speed Control: stretch, reverse, freeze, or set playback speed
    Remap {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Speed stretch factor (e.g. 2.0 = half speed / slow-motion, 0.5 = 2x fast-forward)
        #[arg(long)]
        stretch: Option<f32>,
        /// Reverse playback of layer's source media
        #[arg(long)]
        reverse: bool,
        /// Freeze frame at specified source frame index
        #[arg(long)]
        freeze: Option<u32>,
        /// Enable adjacent frame blending for smooth speed ramps
        #[arg(long)]
        frame_blending: bool,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Motion Tracking: track feature points on footage and apply/bake to transform or target layer
    Track {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Source footage layer name or ID to track
        #[arg(short, long)]
        layer: String,
        /// Tracker point index (default: 0)
        #[arg(long, default_value = "0")]
        tracker_idx: usize,
        /// Initial feature center coordinates "x,y" (default: center of comp/layer)
        #[arg(long)]
        pos: Option<String>,
        /// Feature search window size in pixels (default: 24.0)
        #[arg(long, default_value = "24.0")]
        search_size: f32,
        /// Target layer to apply tracked motion data to (e.g. Null or Text layer)
        #[arg(long)]
        apply_to: Option<String>,
        /// Apply to target position
        #[arg(long, default_value = "true")]
        apply_pos: bool,
        /// Apply to target rotation
        #[arg(long, default_value = "false")]
        apply_rot: bool,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Text Animator: configure per-character typewriter, cascading blur, scale, or tracking
    TextAnim {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target text layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Animation preset: typewriter, cascade, blur-in, tracking, wave
        #[arg(long, default_value = "typewriter")]
        preset: String,
        /// Duration of text animation sweep in frames (default: 30)
        #[arg(long, default_value = "30")]
        duration: u32,
        /// Start frame of animation sweep (default: layer in_frame)
        #[arg(long)]
        start_frame: Option<u32>,
        /// Selector shape: square, ramp_up, ramp_down, smooth, triangle, wobble, random
        #[arg(long, default_value = "ramp_up")]
        shape: String,
        /// Per-character blur radius (px)
        #[arg(long, default_value = "0.0")]
        blur: f32,
        /// Per-character tracking adjustment
        #[arg(long, default_value = "0.0")]
        tracking: f32,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// 3D Camera & Depth of Field: configure focal length, aperture, and cinematic bokeh
    Camera {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Camera position "x,y,z" (e.g. "960,540,-1500")
        #[arg(long)]
        pos: Option<String>,
        /// Camera rotation in degrees "rx,ry,rz" (e.g. "10,-15,0")
        #[arg(long)]
        rot: Option<String>,
        /// Field of View (FOV) in degrees (default: 50.0)
        #[arg(long)]
        fov: Option<f32>,
        /// Focus distance in scene units (default: 1000.0)
        #[arg(long)]
        focus_dist: Option<f32>,
        /// Lens aperture f-stop (e.g. 1.4, 2.8, 5.6)
        #[arg(long)]
        aperture: Option<f32>,
        /// Enable cinematic Depth of Field (DOF) bokeh blur
        #[arg(long)]
        dof: bool,
        /// Maximum DOF bokeh blur radius in pixels (default: 24.0)
        #[arg(long)]
        max_blur: Option<f32>,
        /// Iris diaphragm blade count (0=circular, 5=pentagon, 6=hexagon, 8=octagon)
        #[arg(long)]
        iris_sides: Option<u32>,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Shape Animator: configure Trim Paths (stroke trimming) and Repeater (geometric duplication)
    ShapeAnim {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target shape layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Trim Paths animated sweep start percentage (0.0..100.0)
        #[arg(long)]
        trim_start: Option<f32>,
        /// Trim Paths animated sweep end percentage (0.0..100.0)
        #[arg(long)]
        trim_end: Option<f32>,
        /// Trim Paths sweep duration in frames (animates trim_end from 0 to 100)
        #[arg(long)]
        trim_duration: Option<u32>,
        /// Repeater copy count
        #[arg(long)]
        repeater_copies: Option<u32>,
        /// Repeater position offset per copy "x,y" (e.g. "80,0" or "0,60")
        #[arg(long)]
        repeater_offset: Option<String>,
        /// Repeater rotation offset per copy in degrees (e.g. "15" or "30" for radial arrays)
        #[arg(long)]
        repeater_rotation: Option<f32>,
        /// Repeater scale multiplier per copy "sx,sy" (e.g. "0.9,0.9")
        #[arg(long)]
        repeater_scale: Option<String>,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Particle System: emit sparks, dust, smoke, or fire particles
    Particle {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Particle layer name (created if not exists)
        #[arg(short, long, default_value = "Particles")]
        layer: String,
        /// Particle emission preset: sparks, dust, fire, smoke, snow, explosion
        #[arg(long, default_value = "sparks")]
        preset: String,
        /// Emission rate (particles per second)
        #[arg(long)]
        rate: Option<f32>,
        /// Maximum alive particle count
        #[arg(long)]
        max_particles: Option<u32>,
        /// Emitter position "x,y" (default: composition center)
        #[arg(long)]
        pos: Option<String>,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Audio Spectrum Visualizer: generate frequency bars or waveforms from an audio file
    AudioSpectrum {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Audio spectrum visualizer layer name
        #[arg(short, long, default_value = "Audio Spectrum")]
        layer: String,
        /// Source audio file for frequency spectrum analysis
        #[arg(short, long)]
        audio_file: String,
        /// Number of frequency bands / bars (default: 64)
        #[arg(long, default_value = "64")]
        bands: usize,
        /// Frequency range minimum Hz (default: 20.0)
        #[arg(long, default_value = "20.0")]
        min_freq: f32,
        /// Frequency range maximum Hz (default: 16000.0)
        #[arg(long, default_value = "16000.0")]
        max_freq: f32,
        /// Height multiplier scale for spectrum bars
        #[arg(long, default_value = "250.0")]
        max_height: f32,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Roto Brush: semi-automatic subject isolation via foreground & background guide strokes
    Roto {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target footage layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Stroke points "x1,y1,x2,y2..." in layer space (e.g. "960,300,960,700")
        #[arg(long)]
        points: String,
        /// Guide stroke class: fg (foreground subject), bg (background exclusion)
        #[arg(long, default_value = "fg")]
        kind: String,
        /// Brush stroke radius in pixels (default: 20.0)
        #[arg(long, default_value = "20.0")]
        radius: f32,
        /// Keyframe at which stroke is drawn (default: layer in_frame)
        #[arg(long)]
        frame: Option<u32>,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Puppet Pin: place and animate deformation pins for organic character/mesh warping
    Puppet {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Pin name or ID (created if not exists)
        #[arg(short = 'n', long, default_value = "Pin 1")]
        name: String,
        /// Pin rest/source position "x,y" (default: center of comp/layer)
        #[arg(long)]
        pos: Option<String>,
        /// Deformed target position "x,y" at animated frame
        #[arg(long)]
        target: Option<String>,
        /// Frame number for animated target position (default: in_frame)
        #[arg(long)]
        frame: Option<u32>,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Chroma Key: green screen and blue screen removal with spill suppression
    ChromaKey {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target footage layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Screen color to key out: green, blue, or hex "#00FF00"
        #[arg(long, default_value = "green")]
        color: String,
        /// Screen color gain/sensitivity (0.0..2.0, default: 1.0)
        #[arg(long, default_value = "1.0")]
        gain: f32,
        /// Clip black threshold (clean up background noise, default: 0.0)
        #[arg(long, default_value = "0.0")]
        clip_black: f32,
        /// Clip white threshold (restore foreground transparency, default: 1.0)
        #[arg(long, default_value = "1.0")]
        clip_white: f32,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Audio Track: volume automation, fade in/out envelopes, and pan
    AudioTrack {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
        /// Target audio or video layer name or ID
        #[arg(short, long)]
        layer: String,
        /// Master volume multiplier (1.0 = 0dB, 0.5 = -6dB, 0.0 = mute)
        #[arg(long)]
        volume: Option<f32>,
        /// Fade-in envelope duration in frames
        #[arg(long)]
        fade_in: Option<u32>,
        /// Fade-out envelope duration in frames
        #[arg(long)]
        fade_out: Option<u32>,
        /// Target composition name or index (default: active comp)
        #[arg(short, long)]
        comp: Option<String>,
    },
}
