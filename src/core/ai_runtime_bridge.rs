//! Planned extension points for future AI inference integrations.

#![allow(dead_code)]

use std::path::PathBuf;

/// AI vision task types supported by the modular slot architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiVisionTask {
    /// Deep learning matting & subject cutout (Roto Brush 3 / SAM / BiRefNet)
    SegmentationMatte,
    /// Optical flow / neural frame interpolation (RIFE / FlowNet)
    FrameInterpolation,
    /// Monocular depth map estimation (Depth Anything / MiDaS)
    DepthEstimation,
    /// Content-aware neural inpainting
    NeuralInpaint,
}

/// Target execution hardware provider for opt-in neural inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiExecutionBackend {
    #[default]
    CpuFallback,
    Cuda,
    TensorRt,
    CoreMl,
    DirectMl,
}

/// Configuration for an opt-in neural model slot.
#[derive(Debug, Clone)]
pub struct AiModelSlot {
    pub task: AiVisionTask,
    pub name: String,
    pub model_path: Option<PathBuf>,
    pub backend: AiExecutionBackend,
    pub is_loaded: bool,
}

impl AiModelSlot {
    pub fn new(task: AiVisionTask, name: &str) -> Self {
        Self {
            task,
            name: name.to_string(),
            model_path: None,
            backend: AiExecutionBackend::CpuFallback,
            is_loaded: false,
        }
    }

    /// Model loading is unavailable until an inference runtime is integrated.
    pub fn attach_model(
        &mut self,
        _path: PathBuf,
        _backend: AiExecutionBackend,
    ) -> Result<(), AiRuntimeError> {
        Err(AiRuntimeError::NotImplemented)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiRuntimeError {
    NotImplemented,
}

impl std::fmt::Display for AiRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented => {
                f.write_str("AI model loading and inference are not implemented")
            }
        }
    }
}

impl std::error::Error for AiRuntimeError {}

/// Modular AI Hub managing neural accelerator extensions.
#[derive(Debug, Default)]
pub struct AiRuntimeHub {
    pub slots: Vec<AiModelSlot>,
}

impl AiRuntimeHub {
    pub fn new() -> Self {
        Self {
            slots: vec![
                AiModelSlot::new(
                    AiVisionTask::SegmentationMatte,
                    "Neural Roto (SAM / BiRefNet)",
                ),
                AiModelSlot::new(
                    AiVisionTask::FrameInterpolation,
                    "AI Frame Interpolation (RIFE)",
                ),
                AiModelSlot::new(
                    AiVisionTask::DepthEstimation,
                    "Monocular Depth (Depth Anything)",
                ),
                AiModelSlot::new(
                    AiVisionTask::NeuralInpaint,
                    "Generative Inpaint (ProPainter)",
                ),
            ],
        }
    }

    /// Reports whether a real model and backend are active. No runtime is integrated yet.
    pub fn is_task_accelerated(&self, _task: AiVisionTask) -> bool {
        false
    }

    /// Runs neural segmentation when an inference runtime is available.
    pub fn run_segmentation_matting(
        &self,
        _src_rgb: &[u8],
        _width: u32,
        _height: u32,
        _prompt_box: Option<[f32; 4]>,
    ) -> Result<Vec<u8>, AiRuntimeError> {
        Err(AiRuntimeError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_runtime_does_not_claim_model_loading_or_inference() {
        let mut hub = AiRuntimeHub::new();
        assert_eq!(hub.slots.len(), 4);
        assert!(!hub.is_task_accelerated(AiVisionTask::SegmentationMatte));

        assert_eq!(
            hub.slots[0].attach_model(
                PathBuf::from("/models/sam_vit_h.onnx"),
                AiExecutionBackend::Cuda,
            ),
            Err(AiRuntimeError::NotImplemented)
        );
        assert!(!hub.is_task_accelerated(AiVisionTask::SegmentationMatte));
        assert_eq!(
            hub.run_segmentation_matting(&[128; 400], 10, 10, Some([2.0, 2.0, 8.0, 8.0])),
            Err(AiRuntimeError::NotImplemented)
        );
    }
}
