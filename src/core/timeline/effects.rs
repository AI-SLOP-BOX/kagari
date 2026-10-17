use super::*;

// ─── Color Space Convert Modes ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorConversionMode {
    #[default]
    LogCToLinear,
    LinearToLogC,
    SLog3ToLinear,
    LinearToSLog3,
}

// ─── Effects ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectType {
    GaussianBlur {
        blur_radius: Animatable<f32>,
    },
    ColorTint {
        color: Animatable<[f32; 4]>,
        intensity: Animatable<f32>,
    },
    DropShadow {
        color: Animatable<[f32; 4]>,
        opacity: Animatable<f32>,
        direction: Animatable<f32>,
        distance: Animatable<f32>,
        softness: Animatable<f32>,
    },
    ChromaticAberration {
        shift_r: Animatable<f32>,
        shift_b: Animatable<f32>,
        edge_falloff: Animatable<f32>,
        /// When true, the aberration shift scales with the layer's DOF
        /// CoC radius and the number of iris blades for physically-plausible
        /// camera-lens color fringing.
        #[serde(default = "default_true")]
        iris_linked: bool,
    },
    Vignette {
        /// Darkening strength, slider units 0..100 (normalized at dispatch).
        intensity: Animatable<f32>,
        roundness: Animatable<f32>,
        /// Edge softness, slider units 0..100 (normalized at dispatch).
        feather: Animatable<f32>,
        color: Animatable<[f32; 4]>,
    },
    Levels {
        input_black: Animatable<f32>,
        input_white: Animatable<f32>,
        gamma: Animatable<f32>,
        output_black: Animatable<f32>,
        output_white: Animatable<f32>,
    },
    HueSaturation {
        hue_shift: Animatable<f32>,
        saturation: Animatable<f32>,
        lightness: Animatable<f32>,
    },
    Glow {
        threshold: Animatable<f32>,
        radius: Animatable<f32>,
        intensity: Animatable<f32>,
        color: Animatable<[f32; 4]>,
    },
    /// GPU-first screen-space optical flare (core + rings + star streaks).
    /// When `link_to_light` names a comp light, the flare source tracks that
    /// light's projected position each frame (overrides position_x/y).
    LensFlare {
        enabled: Animatable<f32>,
        position_x: Animatable<f32>,
        position_y: Animatable<f32>,
        intensity: Animatable<f32>,
        threshold: Animatable<f32>,
        color: Animatable<[f32; 4]>,
        #[serde(default)]
        link_to_light: Option<String>,
    },
    MotionBlur {
        shutter_angle: Animatable<f32>,
        samples: u32,
    },
    MeshWarp {
        top_left: Animatable<[f32; 2]>,
        top_right: Animatable<[f32; 2]>,
        bottom_left: Animatable<[f32; 2]>,
        bottom_right: Animatable<[f32; 2]>,
    },
    /// AE Corner Pin: 8-DOF homography mapping the layer rectangle onto an
    /// arbitrary target quadrilateral (screen inserts, perspective billboards).
    CornerPin {
        top_left: Animatable<[f32; 2]>,
        top_right: Animatable<[f32; 2]>,
        bottom_right: Animatable<[f32; 2]>,
        bottom_left: Animatable<[f32; 2]>,
    },
    ColorGradeLUT {
        lut_path: String,
        intensity: Animatable<f32>,
    },
    ColorSpaceConvert {
        mode: ColorConversionMode,
    },
    FilmGrain {
        intensity: Animatable<f32>,
        grain_size: f32,
        color_film: bool,
    },

    // ── CPU pixel-effect kernels (wired to core::cpu_effects → ae_effects_pack) ──
    // These run on the software/CPU render path so effects are visible even
    // without the GPU pipeline, and reuse the orphaned ae_effects_pack kernels.
    Twirl {
        angle: Animatable<f32>,
        radius: Animatable<f32>,
    },
    Bulge {
        amount: Animatable<f32>,
        radius: Animatable<f32>,
    },
    Posterize {
        levels: Animatable<f32>,
    },
    Invert {
        invert_alpha: bool,
    },
    Offset {
        shift_x: Animatable<f32>,
        shift_y: Animatable<f32>,
    },
    DirectionalBlur {
        angle: Animatable<f32>,
        length: Animatable<f32>,
    },
    RadialBlur {
        amount: Animatable<f32>,
    },
    Sharpen {
        amount: Animatable<f32>,
    },
    Threshold {
        threshold: Animatable<f32>,
    },
    LinearWipe {
        completion: Animatable<f32>,
        angle: Animatable<f32>,
    },
    SimpleChoker {
        choke_amount: Animatable<f32>,
    },
    ChromaKey {
        screen_color: Animatable<[f32; 3]>,
        screen_gain: Animatable<f32>,
        clip_black: Animatable<f32>,
        clip_white: Animatable<f32>,
    },
    Spherize {
        radius: Animatable<f32>,
        refractive_index: Animatable<f32>,
    },
    TurbulentDisplace {
        amount: Animatable<f32>,
        size: Animatable<f32>,
        evolution: Animatable<f32>,
        complexity: Animatable<f32>,
    },
    Colorama {
        preset_index: Animatable<f32>,
        cycle_phase: Animatable<f32>,
    },
    // ── New AE-standard effects ──
    FractalNoise {
        fractal_type: Animatable<f32>,
        contrast: Animatable<f32>,
        brightness: Animatable<f32>,
        complexity: Animatable<f32>,
        evolution: Animatable<f32>,
    },
    Curves {
        channel: Animatable<f32>,
    },
    DisplacementMap {
        source_layer: Animatable<f32>,
        max_horizontal: Animatable<f32>,
        max_vertical: Animatable<f32>,
    },
    CompoundBlur {
        source_layer: Animatable<f32>,
        max_blur: Animatable<f32>,
    },
    Minimax {
        operation: Animatable<f32>,
        radius: Animatable<f32>,
    },
    ShiftChannels {
        take_red: Animatable<f32>,
        take_green: Animatable<f32>,
        take_blue: Animatable<f32>,
        take_alpha: Animatable<f32>,
    },
    // ── Effects migrated from ExtEffect (effect_registry_ext) ──
    WaveWarp {
        wave_height: Animatable<f32>,
        wave_width: Animatable<f32>,
        speed: Animatable<f32>,
        direction_deg: Animatable<f32>,
        wave_type: u8,
        pinning: u8,
    },
    CcLens {
        convergence: Animatable<f32>,
        zoom: Animatable<f32>,
    },
    PolarCoordinates {
        to_polar: bool,
        interpolation: Animatable<f32>,
    },
    OpticsCompensation {
        field_of_view_deg: Animatable<f32>,
        reverse: bool,
        zoom: Animatable<f32>,
    },
    ColorBalance {
        shadows: [f32; 3],
        midtones: [f32; 3],
        highlights: [f32; 3],
        preserve_luminosity: bool,
    },
    ChannelMixer {
        matrix: [[f32; 3]; 3],
        monochrome: bool,
    },
    LightSweep {
        direction_deg: Animatable<f32>,
        center: Animatable<f32>,
        width: Animatable<f32>,
        sweep_intensity: Animatable<f32>,
        edge_intensity: Animatable<f32>,
    },
    RadialFastBlur {
        amount: Animatable<f32>,
        samples: u32,
    },
    BendIt {
        top_offset: Animatable<f32>,
        bottom_offset: Animatable<f32>,
    },
    Tiler {
        scale_percent: Animatable<f32>,
        mirror: bool,
    },
    // ── Effects from ae_effects_pack not yet wired ──
    Tritone {
        shadow_color: Animatable<[f32; 3]>,
        mid_color: Animatable<[f32; 3]>,
        highlight_color: Animatable<[f32; 3]>,
    },
    MatteChoker {
        choke_amount: Animatable<f32>,
        gray_level: Animatable<f32>,
    },
    VenetianBlinds {
        completion: Animatable<f32>,
        width: Animatable<f32>,
    },
    // ── Lumetri Basic Correction (Vibrance / WB / HSL Secondary) ──
    Vibrance {
        amount: Animatable<f32>,
    },
    WhiteBalance {
        temperature: Animatable<f32>,
        tint: Animatable<f32>,
    },
    HslAdjust {
        hue_deg: Animatable<f32>,
        saturation: Animatable<f32>,
        lightness: Animatable<f32>,
    },
    /// Threshold bloom glow (AE Glow): bright pixels bleed outwards.
    GlowPro {
        threshold: Animatable<f32>,
        radius: Animatable<f32>,
        intensity: Animatable<f32>,
    },
    /// CRT scanline TV distortion.
    CrtScanlines {
        line_spacing: Animatable<f32>,
        intensity: Animatable<f32>,
    },
    /// Attenuated spiral vortex around the layer centre.
    Vortex {
        radius: Animatable<f32>,
        angle_deg: Animatable<f32>,
    },
    /// Rising thermal turbulence; time advances with `speed` per second.
    HeatDistortion {
        strength: Animatable<f32>,
        speed: Animatable<f32>,
    },
    /// Water drop / rain ripple ring displacement.
    RainRipples {
        drop_count: Animatable<f32>,
        wave_strength: Animatable<f32>,
    },
    /// Fisheye lens bulge distortion (−strength = pincushion).
    Fisheye {
        strength: Animatable<f32>,
    },
    /// Barrel (+k1) / pincushion (−k1) lens correction with k2 term.
    LensCorrection {
        k1: Animatable<f32>,
        k2: Animatable<f32>,
    },
    /// Deterministic digital block glitch displacement.
    GlitchDisplacement {
        seed: Animatable<f32>,
        amount: Animatable<f32>,
    },
    /// Morphological matte choke (erode) or spread (dilate).
    MatteChokeSpread {
        radius: Animatable<f32>,
        expand: bool,
    },
    /// Soft blur applied to the alpha edge only.
    AlphaFeather {
        radius: Animatable<f32>,
    },
    /// Replace alpha with luminance (optionally inverted).
    AlphaFromLuminance {
        invert: bool,
    },
    /// Phosphor-green night vision look with per-frame film noise.
    NightVision {
        amplification: Animatable<f32>,
    },
    /// Circular iris wipe transition (0 = fully covered).
    IrisWipe {
        completion: Animatable<f32>,
    },
    /// Sweeping radial wipe transition (0 = fully covered).
    RadialWipe {
        completion: Animatable<f32>,
    },
    /// ASC CDL film emulation: lift/gamma/gain grade + YCbCr hue rotation.
    FilmEmulation {
        lift: Animatable<f32>,
        gamma: Animatable<f32>,
        gain: Animatable<f32>,
        hue_shift_deg: Animatable<f32>,
    },
    /// Volumetric light scattering from a sun position (normalized 0..1).
    GodRays {
        sun_x: Animatable<f32>,
        sun_y: Animatable<f32>,
        samples: Animatable<f32>,
        decay: Animatable<f32>,
        weight: Animatable<f32>,
    },
    /// Audio Spectrum: paint a real-time frequency-bar overlay sourced from
    /// the current frame's analyzed audio bands (see
    /// expression_engine::set_audio_data).
    AudioSpectrum {
        enabled: Animatable<f32>,
        bands: Animatable<f32>,
        opacity: Animatable<f32>,
        color_start: [f32; 4],
        color_end: [f32; 4],
        // Normalized position (0..1) of the bar strip's lower-left.
        position_x: Animatable<f32>,
        position_y: Animatable<f32>,
        // Strip width/height as fraction of the layer quad.
        width: Animatable<f32>,
        height: Animatable<f32>,
    },
    /// Centre zoom motion blur.
    RadialBlurZoom {
        amount: Animatable<f32>,
    },
    /// Median filter — salt-and-pepper noise removal.
    MedianFilter {
        radius: Animatable<f32>,
    },
    /// Sobel edge outline.
    SobelEdges {
        invert: bool,
    },
    /// Block pixelation.
    Mosaic {
        block_w: Animatable<f32>,
        block_h: Animatable<f32>,
    },
    /// Physical Optical Lens Flare generator (AE Parity).
    OpticalFlares {
        position: Animatable<[f32; 2]>,
        brightness: Animatable<f32>,
        scale: Animatable<f32>,
    },
    /// Motion Tile & CC RepeTile (AE Parity).
    MotionTile {
        tile_center: Animatable<[f32; 2]>,
        tile_width: Animatable<f32>,
        tile_height: Animatable<f32>,
        output_width: Animatable<f32>,
        output_height: Animatable<f32>,
        mirror_edges: bool,
        phase: Animatable<f32>,
    },
    /// CC Page Turn 3D cylindrical paper curl & peel (AE Parity).
    PageTurn {
        fold_position: Animatable<[f32; 2]>,
        fold_radius: Animatable<f32>,
        fold_direction_deg: Animatable<f32>,
        light_direction_deg: Animatable<f32>,
        back_opacity: Animatable<f32>,
        back_color: Animatable<[f32; 4]>,
    },
    /// Set Matte channel composite (AE Parity).
    SetMatte {
        source_layer_idx: usize,
        source_channel: crate::core::set_matte::MatteSourceChannel,
        invert_matte: bool,
        composite_mode: crate::core::set_matte::MatteCompositeMode,
    },
    /// Echo Time Effect (AE Parity).
    Echo {
        echo_time_seconds: Animatable<f32>,
        num_echoes: u32,
        starting_intensity: Animatable<f32>,
        decay: Animatable<f32>,
        operator: crate::core::echo_effect::EchoOperator,
    },
    /// Find Edges (AE Parity).
    FindEdges {
        invert: bool,
    },
    /// Transform Effect (AE Parity - Distort > Transform).
    Transform {
        anchor_point: Animatable<[f32; 2]>,
        position: Animatable<[f32; 2]>,
        scale_width: Animatable<f32>,
        scale_height: Animatable<f32>,
        uniform_scale: bool,
        skew_deg: Animatable<f32>,
        skew_axis_deg: Animatable<f32>,
        rotation_deg: Animatable<f32>,
        opacity: Animatable<f32>,
    },
    /// Camera Lens Blur (AE Parity - Blur & Sharpen > Camera Lens Blur).
    CameraLensBlur {
        blur_radius: Animatable<f32>,
        iris_blades: u32,
        iris_rotation_deg: Animatable<f32>,
        iris_roundness: Animatable<f32>,
        highlight_gain: Animatable<f32>,
        highlight_threshold: Animatable<f32>,
    },
    /// Linear Color Key (AE Parity - Keying > Linear Color Key).
    LinearColorKey {
        key_color: Animatable<[f32; 3]>,
        match_mode: crate::core::linear_color_key::ColorMatchMode,
        tolerance: Animatable<f32>,
        softness: Animatable<f32>,
    },
    /// Channel Combiner (AE Parity - Channel > Channel Combiner).
    ChannelCombiner {
        from_channel: crate::core::channel_combiner::ChannelCombinerFrom,
        to_target: crate::core::channel_combiner::ChannelCombinerTo,
        invert: bool,
    },
    /// Tilt-shift miniature focus band.
    TiltShift {
        focus_y: Animatable<f32>,
        focus_height: Animatable<f32>,
        max_blur: Animatable<f32>,
    },
    /// Surface relief emboss.
    Emboss {
        angle_deg: Animatable<f32>,
        depth: Animatable<f32>,
    },
    /// Parallax star field generator.
    StarField {
        num_stars: Animatable<f32>,
        depth_speed: Animatable<f32>,
    },
    /// Procedural lightning bolt between two normalized points.
    LightningArc {
        start_x: Animatable<f32>,
        start_y: Animatable<f32>,
        end_x: Animatable<f32>,
        end_y: Animatable<f32>,
        seed: Animatable<f32>,
        glow: Animatable<f32>,
    },
    /// Laser beam effect with customizable core, glow, thickness, and progress.
    LaserBeam {
        start_x: Animatable<f32>,
        start_y: Animatable<f32>,
        end_x: Animatable<f32>,
        end_y: Animatable<f32>,
        progress: Animatable<f32>,
        length: Animatable<f32>,
        starting_thickness: Animatable<f32>,
        ending_thickness: Animatable<f32>,
        core_color: Animatable<[f32; 4]>,
        glow_color: Animatable<[f32; 4]>,
    },
    /// Cellular-automaton fire rising from the bottom edge.
    FireAutomaton {
        intensity: Animatable<f32>,
    },
    /// Luminance-range key — pixels outside the luma band become transparent.
    LumaKeyRange {
        low_threshold: Animatable<f32>,
        high_threshold: Animatable<f32>,
        invert: bool,
    },
    /// Halftone dot screen rasterization.
    Halftone {
        cell_size: Animatable<f32>,
    },
    /// Solar inversion of bright values above a threshold.
    Solarize {
        threshold: Animatable<f32>,
    },
    /// Column-wise pixel sorting glitch.
    PixelSort {
        threshold: Animatable<f32>,
    },
    /// Pinch (amount > 0) / punch (amount < 0) polar distortion around centre.
    PinchPunch {
        radius: Animatable<f32>,
        amount: Animatable<f32>,
    },
    /// Horizontal scanline jitter glitch.
    ScanlineGlitch {
        jitter_amount: Animatable<f32>,
        seed: Animatable<f32>,
    },
    RGBSplit {
        red_offset: Animatable<[f32; 2]>,
        green_offset: Animatable<[f32; 2]>,
        blue_offset: Animatable<[f32; 2]>,
    },
    Flicker {
        amount: Animatable<f32>,
        speed: Animatable<f32>,
        seed: Animatable<f32>,
    },
    BlockGlitch {
        block_size: Animatable<f32>,
        amount: Animatable<f32>,
        seed: Animatable<f32>,
        corruption: Animatable<f32>,
    },
    SliceTear {
        slices: Animatable<f32>,
        max_offset: Animatable<f32>,
        seed: Animatable<f32>,
        vertical: bool,
    },
    /// Glass edge bevel with specular refraction on layer borders.
    GlassEdgeBevel {
        bevel_size: Animatable<f32>,
        refraction: Animatable<f32>,
    },
    /// Sharpen along an arbitrary direction.
    DirectionalSharpen {
        angle_deg: Animatable<f32>,
        strength: Animatable<f32>,
    },
    /// Spherical glass-ball refraction lens.
    RefractionLens {
        radius: Animatable<f32>,
        ior: Animatable<f32>,
    },
    /// 3-colour gradient map (shadow / mid / high ramp).
    GradientMap {
        low_color: Animatable<[f32; 3]>,
        mid_color: Animatable<[f32; 3]>,
        high_color: Animatable<[f32; 3]>,
    },
    /// Cinematic warm light leak from a normalized position.
    LightLeak {
        pos_x: Animatable<f32>,
        pos_y: Animatable<f32>,
        intensity: Animatable<f32>,
    },
    /// Alpha bevel with directional lighting.
    BevelAlpha {
        depth: Animatable<f32>,
        light_angle_deg: Animatable<f32>,
    },
    /// Ink cross-hatch sketch stylize.
    CrossHatch {
        line_gap: Animatable<f32>,
        threshold: Animatable<f32>,
    },
    /// CMYK four-plate halftone rasterization.
    CmykHalftone {
        dot_size: Animatable<f32>,
    },
    /// Mirror reflection below a horizon line with distance fade.
    ReflectionMap {
        reflect_y: Animatable<f32>,
        fade_dist: Animatable<f32>,
        opacity: Animatable<f32>,
    },
    /// Perlin flow vector noise field (animated).
    PerlinFlow {
        scale: Animatable<f32>,
    },
    /// Fractal Brownian motion turbulence.
    FbmTurbulence {
        octaves: Animatable<f32>,
        amplitude: Animatable<f32>,
    },
    // ── Expression Controls (non-rendering utility effects, AE parity) ──
    SliderControl {
        value: Animatable<f32>,
    },
    AngleControl {
        angle_degrees: Animatable<f32>,
    },
    PointControl {
        point: Animatable<[f32; 2]>,
    },
    ColorControl {
        color: Animatable<[f32; 4]>,
    },
    /// AE Checkbox Control: boolean toggle for expressions
    CheckboxControl {
        checked: bool,
    },
    /// AE Dropdown/Menu Control: integer selector for expressions
    DropdownControl {
        value: i32,
        options: Vec<String>,
    },
    /// AE 3D Point Control: XYZ position for expressions
    Point3DControl {
        point: Animatable<[f32; 3]>,
    },
    /// Cinematic letterbox bars (fraction of frame height, total both sides).
    Letterbox {
        frac: Animatable<f32>,
    },
    /// Custom WGSL shader effect (runtime compiled).
    CustomShader {
        /// WGSL source code for the fragment shader.
        wgsl_source: String,
        /// Uniform float values passed to the shader (up to 16).
        uniform_values: Vec<f32>,
    },
    /// Merge Paths boolean operation on shape sub-paths.
    MergePaths {
        /// Operation: 0=Add, 1=Subtract, 2=Intersect, 3=Exclude.
        operation: Animatable<f32>,
    },
    /// Offset Path: expand or contract a shape path along its normals.
    OffsetPath {
        /// Offset amount in pixels (positive = expand, negative = contract).
        amount: Animatable<f32>,
        /// Line join: 0=Miter, 1=Round, 2=Bevel.
        line_join: Animatable<f32>,
        /// Miter limit for miter joins.
        miter_limit: Animatable<f32>,
    },
    /// Two-band bass/treble equalizer.
    BassTreble {
        /// Bass gain in dB (-24..+24).
        bass_gain: Animatable<f32>,
        /// Treble gain in dB (-24..+24).
        treble_gain: Animatable<f32>,
        /// Crossover frequency in Hz (80..1200).
        crossover_freq: Animatable<f32>,
    },
    /// Flanger audio effect (modulated delay).
    Flanger {
        /// Maximum delay in ms (1..10).
        max_delay_ms: Animatable<f32>,
        /// LFO rate in Hz (0.1..10).
        lfo_rate: Animatable<f32>,
        /// Feedback amount (0.0..0.95).
        feedback: Animatable<f32>,
        /// Wet/dry mix (0..1).
        wet_dry: Animatable<f32>,
    },
    /// Chorus audio effect (multiple detuned delays).
    Chorus {
        /// Delay time in ms (1..30).
        delay_ms: Animatable<f32>,
        /// Modulation depth in ms (0.5..10).
        depth_ms: Animatable<f32>,
        /// LFO rate in Hz (0.1..6).
        rate_hz: Animatable<f32>,
        /// Number of voices (2..8).
        voices: Animatable<f32>,
        /// Feedback (0.0..0.9).
        feedback: Animatable<f32>,
    },
    /// Parametric equalizer (2-band bell filter).
    ParametricEQ {
        /// Center frequency in Hz (60..18000).
        freq_hz: Animatable<f32>,
        /// Gain in dB (-24..+24).
        gain_db: Animatable<f32>,
        /// Q factor (0.5..12).
        q_factor: Animatable<f32>,
    },
    /// VR Horizon (3-axis spherical orientation realignment: Pitch, Yaw, Roll).
    VRHorizon {
        pitch_deg: Animatable<f32>,
        yaw_deg: Animatable<f32>,
        roll_deg: Animatable<f32>,
    },
    /// VR Spherical Blur (Geodesic latitude-compensated Gaussian blur for 360 equirectangular footage).
    VRSphericalBlur {
        radius: Animatable<f32>,
    },
    /// Spatio-temporal noise reduction across consecutive frame buffers.
    TemporalDenoise {
        strength: Animatable<f32>,
        temporal_mix: Animatable<f32>,
        feature_threshold: Animatable<f32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub id: String,
    pub name: String,
    pub effect_type: EffectType,
    pub enabled: bool,
}
