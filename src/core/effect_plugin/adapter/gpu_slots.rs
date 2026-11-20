use crate::core::timeline::EffectType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpuEffectSlot {
    // Reserved until spatial effects pass full-frame GPU/software parity for
    // every animated parameter. They currently use the CPU preview fallback.
    Distortion = 0,
    // 1. Lens flare inside sample_layer_color (line 655)
    LensFlare = 1,
    // 2. ColorTint after sample_layer_color (line 777)
    ColorTint = 2,
    // 3. Levels (line 868)
    Levels = 3,
    // 4. Invert (line 930)
    Invert = 4,
    // 5. Posterize (line 940)
    Posterize = 5,
}

impl GpuEffectSlot {
    /// Returns the execution order in the GPU fragment shader.
    pub fn execution_order(self) -> u32 {
        self as u32
    }
}

/// Returns whether the realtime GPU preview can represent this effect without
/// silently dropping parameters.
pub fn gpu_preview_effect_supported(effect: &EffectType) -> bool {
    gpu_preview_effect_slot(effect).is_some()
}

/// Returns whether the realtime GPU preview has a WGSL shader implementation for
/// this effect (either single-pass or multi-pass scratch).
pub(crate) fn has_wgsl_shader_effect(effect: &EffectType) -> bool {
    matches!(
        effect,
        EffectType::GaussianBlur { .. }
            | EffectType::ColorTint { .. }
            | EffectType::DropShadow { .. }
            | EffectType::ChromaticAberration { .. }
            | EffectType::Vignette { .. }
            | EffectType::ColorSpaceConvert { .. }
            | EffectType::FilmGrain { .. }
            | EffectType::Levels { .. }
            | EffectType::HueSaturation { .. }
            | EffectType::Glow { .. }
            | EffectType::GlowPro { .. }
            | EffectType::MeshWarp { .. }
            | EffectType::MotionBlur { .. }
            | EffectType::LensFlare {
                link_to_light: None,
                ..
            }
            | EffectType::Invert { .. }
            | EffectType::Posterize { .. }
            | EffectType::Threshold { .. }
            | EffectType::Tritone { .. }
            | EffectType::CrtScanlines { .. }
            | EffectType::FractalNoise { .. }
            | EffectType::TurbulentDisplace { .. }
            | EffectType::WaveWarp { .. }
            | EffectType::Twirl { .. }
            | EffectType::Bulge { .. }
            | EffectType::Spherize { .. }
            | EffectType::HeatDistortion { .. }
            | EffectType::RainRipples { .. }
            | EffectType::DirectionalBlur { .. }
            | EffectType::CornerPin { .. }
            | EffectType::Mosaic { .. }
            | EffectType::Sharpen { .. }
            | EffectType::Solarize { .. }
            | EffectType::LinearWipe { .. }
            | EffectType::ChromaKey { .. }
            | EffectType::Vibrance { .. }
            | EffectType::WhiteBalance { .. }
            | EffectType::SobelEdges { .. }
            | EffectType::FindEdges { .. }
            | EffectType::Letterbox { .. }
            | EffectType::Checkerboard { .. }
            | EffectType::GradientRamp { .. }
            | EffectType::RadialBlur { .. }
            | EffectType::RadialBlurZoom { .. }
            | EffectType::VenetianBlinds { .. }
            | EffectType::IrisWipe { .. }
            | EffectType::RadialWipe { .. }
            | EffectType::RGBSplit { .. }
            | EffectType::CCKaleida { .. }
            | EffectType::Halftone { .. }
            | EffectType::Flicker { .. }
            | EffectType::LumaKeyRange { .. }
            | EffectType::AlphaFromLuminance { .. }
            | EffectType::NightVision { .. }
            | EffectType::ColorBalance { .. }
            | EffectType::SimpleChoker { .. }
            | EffectType::Offset { .. }
            | EffectType::Tiler { .. }
            | EffectType::Fisheye { .. }
            | EffectType::CcLens { .. }
            | EffectType::PolarCoordinates { .. }
            | EffectType::Vortex { .. }
            | EffectType::FilmEmulation { .. }
            | EffectType::PinchPunch { .. }
            | EffectType::ScanlineGlitch { .. }
            | EffectType::LightSweep { .. }
            | EffectType::ShiftChannels { .. }
            | EffectType::OpticsCompensation { .. }
            | EffectType::LensCorrection { .. }
            | EffectType::RadialFastBlur { .. }
            | EffectType::TiltShift { .. }
            | EffectType::Emboss { .. }
            | EffectType::LinearColorKey { .. }
            | EffectType::BlockGlitch { .. }
            | EffectType::SliceTear { .. }
            | EffectType::LightLeak { .. }
            | EffectType::CmykHalftone { .. }
            | EffectType::StrobeLight { .. }
            | EffectType::ChannelMixer { .. }
            | EffectType::HslAdjust { .. }
            | EffectType::DirectionalSharpen { .. }
            | EffectType::BevelAlpha { .. }
            | EffectType::GlassEdgeBevel { .. }
            | EffectType::MatteChoker { .. }
            | EffectType::MatteChokeSpread { .. }
            | EffectType::AlphaFeather { .. }
            | EffectType::Minimax { .. }
            | EffectType::MedianFilter { .. }
            | EffectType::BilateralFilter { .. }
            | EffectType::MotionTile { .. }
            | EffectType::Transform { .. }
            | EffectType::PageTurn { .. }
            | EffectType::BendIt { .. }
            | EffectType::GlitchDisplacement { .. }
            | EffectType::PixelSort { .. }
            | EffectType::CrossHatch { .. }
            | EffectType::GodRays { .. }
            | EffectType::OpticalFlares { .. }
            | EffectType::StarField { .. }
            | EffectType::LightningArc { .. }
            | EffectType::LaserBeam { .. }
            | EffectType::PerlinFlow { .. }
            | EffectType::FbmTurbulence { .. }
            | EffectType::VRHorizon { .. }
            | EffectType::CCPixelPolly { .. }
            | EffectType::Colorama { .. }
            | EffectType::GradientMap { .. }
            | EffectType::RefractionLens { .. }
            | EffectType::ChannelCombiner { .. }
            | EffectType::CameraLensBlur { .. }
            | EffectType::VRSphericalBlur { .. }
            | EffectType::AudioSpectrum { .. }
            | EffectType::FireAutomaton { .. }
            | EffectType::Curves { .. }
            | EffectType::ColorGradeLUT { .. }
            | EffectType::ZDepthFog { .. }
            | EffectType::SSAO { .. }
            | EffectType::ReflectionMap { .. }
            | EffectType::CompoundBlur { .. }
            | EffectType::DisplacementMap { .. }
            | EffectType::TemporalDenoise { .. }
            | EffectType::SetMatte { .. }
            | EffectType::Echo { .. }
            | EffectType::CustomShader { .. }
            | EffectType::BassTreble { .. }
            | EffectType::Flanger { .. }
            | EffectType::Chorus { .. }
            | EffectType::ParametricEQ { .. }
            | EffectType::MergePaths { .. }
            | EffectType::OffsetPath { .. }
            | EffectType::SliderControl { .. }
            | EffectType::AngleControl { .. }
            | EffectType::PointControl { .. }
            | EffectType::ColorControl { .. }
            | EffectType::CheckboxControl { .. }
            | EffectType::DropdownControl { .. }
            | EffectType::Point3DControl { .. }
    )
}

/// Returns the specific GPU shader pipeline slot for an effect, or None if unsupported on GPU preview.
pub fn gpu_preview_effect_slot(effect: &EffectType) -> Option<GpuEffectSlot> {
    match effect {
        EffectType::LensFlare {
            link_to_light: None,
            ..
        } => Some(GpuEffectSlot::LensFlare),
        EffectType::ColorTint { .. } => Some(GpuEffectSlot::ColorTint),
        EffectType::Levels { .. } => Some(GpuEffectSlot::Levels),
        EffectType::Invert {
            invert_alpha: false,
        } => Some(GpuEffectSlot::Invert),
        EffectType::Posterize { .. } => Some(GpuEffectSlot::Posterize),
        _ => None,
    }
}
