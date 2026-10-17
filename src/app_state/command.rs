//! Command & Dispatch pattern for Kagari.
//!
//! Decouples UI interactions from direct project and state mutations.
//! Every user-initiated operation is expressed as an [`AppCommand`] and processed
//! through [`crate::app_state::KagariApp::dispatch`].
//!
//! This guarantees:
//! - Uniform Undo/Redo recording via [`crate::core::editor::EditorSession`]
//! - Automatic version bumping & frame cache invalidation
//! - Reliable dirty-state tracking for autosave
//! - Testability without headless UI workarounds

use crate::core::property::Animatable;
use crate::core::timeline::{BlendMode, LayerType, TrackMatteMode};

/// Commands that represent user intents and editing actions across the application.
#[derive(Debug, Clone)]
pub enum AppCommand {
    // ------------------------------------------------------------------------
    // Undo / Redo
    // ------------------------------------------------------------------------
    Undo,
    Redo,

    // ------------------------------------------------------------------------
    // Navigation & Selection
    // ------------------------------------------------------------------------
    SelectComposition(usize),
    SelectLayer { layer_idx: usize, multi: bool },
    ClearSelection,
    SetCurrentFrame(u32),

    // ------------------------------------------------------------------------
    // Layer Creation / Deletion / Hierarchy
    // ------------------------------------------------------------------------
    AddLayer {
        name: String,
        layer_type: LayerType,
    },
    DeleteSelectedLayers,
    DuplicateSelectedLayers,
    ReorderLayer { from_idx: usize, to_idx: usize },

    // ------------------------------------------------------------------------
    // Layer Visibility / Solo / Lock
    // ------------------------------------------------------------------------
    ToggleLayerSolo(usize),
    ToggleLayerLock(usize),
    ToggleLayerVisibility(usize),

    // ------------------------------------------------------------------------
    // Layer Transform & Blend Properties
    // ------------------------------------------------------------------------
    SetLayerPosition {
        layer_idx: usize,
        position: [f32; 2],
    },
    SetLayerScale {
        layer_idx: usize,
        scale: [f32; 2],
    },
    SetLayerRotation {
        layer_idx: usize,
        rotation: f32,
    },
    SetLayerOpacity {
        layer_idx: usize,
        opacity: f32,
    },
    SetLayerBlendMode {
        layer_idx: usize,
        mode: BlendMode,
    },
    SetLayerTrackMatte {
        layer_idx: usize,
        matte: TrackMatteMode,
    },

    // ------------------------------------------------------------------------
    // Continuous Interactions (Slider Drag, Scrubbing, Gizmo Move)
    // ------------------------------------------------------------------------
    BeginInteraction(&'static str),
    EndInteraction,
    CancelInteraction,

    // ------------------------------------------------------------------------
    // Timing / In-Out Points
    // ------------------------------------------------------------------------
    SetLayerInFrame {
        layer_idx: usize,
        frame: u32,
    },
    SetLayerOutFrame {
        layer_idx: usize,
        frame: u32,
    },
    SplitLayerAtCurrentFrame(usize),

    // ------------------------------------------------------------------------
    // Composition Settings
    // ------------------------------------------------------------------------
    SetBackgroundColor([f32; 4]),
    SetMotionBlurShutterAngle(f32),

    // ------------------------------------------------------------------------
    // Masks
    // ------------------------------------------------------------------------
    AddMaskToLayer {
        layer_idx: usize,
        mask: crate::core::mask::Mask,
    },
}

#[cfg(feature = "gui")]
impl crate::app_state::KagariApp {
    /// Invalidate rendering caches and mark frame generation updated.
    fn notify_project_mutation(&mut self) {
        crate::core::frame_cache::bump_version();
        self.autosave.mark_dirty();
        if let Some(ctx) = self.repaint_context.as_ref() {
            ctx.request_repaint();
        }
    }

    /// Dispatches an [`AppCommand`], executing the operation through a unified mutation boundary.
    pub fn dispatch(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::Undo => {
                if let Some(restored) = self.history.undo() {
                    let _ = restored;
                    self.notify_project_mutation();
                }
            }
            AppCommand::Redo => {
                if let Some(restored) = self.history.redo() {
                    let _ = restored;
                    self.notify_project_mutation();
                }
            }
            AppCommand::BeginInteraction(label) => {
                self.begin_drag(label);
            }
            AppCommand::EndInteraction => {
                self.commit_drag();
            }
            AppCommand::CancelInteraction => {
                if let Some(tx) = self.drag_tx.take() {
                    *self.history.current_mut() = tx.snapshot;
                    self.notify_project_mutation();
                }
            }
            AppCommand::SelectComposition(idx) => {
                let comp_count = self.history.current().compositions.len();
                if idx < comp_count {
                    let mut session = crate::core::editor::EditorSession::new(
                        &mut self.history,
                        "Select Composition",
                    );
                    session.current_mut().active_composition_idx = idx;
                    session.commit();
                    self.selection.selected_layers.clear();
                    self.notify_project_mutation();
                }
            }
            AppCommand::SelectLayer { layer_idx, multi } => {
                if !multi {
                    self.selection.selected_layers.clear();
                }
                self.selection.selected_layers.insert(layer_idx);
            }
            AppCommand::ClearSelection => {
                self.selection.selected_layers.clear();
                self.selected_keyframes.clear();
            }
            AppCommand::SetCurrentFrame(frame) => {
                let max_frame = self
                    .history
                    .current()
                    .active_composition()
                    .duration_frames
                    .saturating_sub(1);
                self.playback.current_frame = frame.min(max_frame);
            }
            AppCommand::ToggleLayerSolo(layer_idx) => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Toggle Solo");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.solo = !layer.solo;
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::ToggleLayerLock(layer_idx) => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Toggle Lock");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.locked = !layer.locked;
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::ToggleLayerVisibility(layer_idx) => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Toggle Visibility");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.visible = !layer.visible;
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::SetLayerPosition {
                layer_idx,
                position,
            } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Set Position");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.transform.position = Animatable::Constant(position);
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::SetLayerScale { layer_idx, scale } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Set Scale");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.transform.scale = Animatable::Constant(scale);
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::SetLayerRotation {
                layer_idx,
                rotation,
            } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Set Rotation");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.transform.rotation = Animatable::Constant(rotation);
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::SetLayerOpacity { layer_idx, opacity } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Set Opacity");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.transform.opacity = Animatable::Constant(opacity.clamp(0.0, 100.0));
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::SetLayerBlendMode { layer_idx, mode } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Set Blend Mode");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.blend_mode = mode;
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::SetLayerTrackMatte { layer_idx, matte } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Set Track Matte");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.track_matte = matte;
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::SetLayerInFrame { layer_idx, frame } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Set In Frame");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.in_frame = frame;
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::SetLayerOutFrame { layer_idx, frame } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Set Out Frame");
                if let Some(layer) = session
                    .current_mut()
                    .active_composition_mut()
                    .layers
                    .get_mut(layer_idx)
                {
                    layer.out_frame = frame;
                    session.commit();
                    self.notify_project_mutation();
                }
            }
            AppCommand::AddLayer { name, layer_type } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Add Layer");
                let comp = session.current_mut().active_composition_mut();
                let layer_id = format!("layer_{}", comp.layers.len() + 1);
                let new_layer = crate::core::timeline::Layer::new(
                    layer_id,
                    name,
                    layer_type,
                    comp.duration_frames,
                );
                comp.layers.push(new_layer);
                let new_idx = comp.layers.len() - 1;
                session.commit();
                self.selection.selected_layers.clear();
                self.selection.selected_layers.insert(new_idx);
                self.notify_project_mutation();
            }
            AppCommand::DeleteSelectedLayers => {
                if self.selection.selected_layers.is_empty() {
                    return;
                }
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Delete Layers");
                let comp = session.current_mut().active_composition_mut();
                let mut sorted_indices: Vec<usize> =
                    self.selection.selected_layers.iter().copied().collect();
                sorted_indices.sort_unstable_by(|a, b| b.cmp(a)); // remove from back
                for idx in sorted_indices {
                    if idx < comp.layers.len() {
                        comp.layers.remove(idx);
                    }
                }
                session.commit();
                self.selection.selected_layers.clear();
                self.notify_project_mutation();
            }
            AppCommand::DuplicateSelectedLayers => {
                if self.selection.selected_layers.is_empty() {
                    return;
                }
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Duplicate Layers");
                let comp = session.current_mut().active_composition_mut();
                let mut new_selection = std::collections::HashSet::new();
                for &idx in &self.selection.selected_layers {
                    if idx < comp.layers.len() {
                        let mut duplicated = comp.layers[idx].clone();
                        duplicated.id = format!("{}_copy", duplicated.id);
                        duplicated.name = format!("{} Copy", duplicated.name);
                        comp.layers.insert(idx + 1, duplicated);
                        new_selection.insert(idx + 1);
                    }
                }
                session.commit();
                self.selection.selected_layers = new_selection;
                self.notify_project_mutation();
            }
            AppCommand::ReorderLayer { from_idx, to_idx } => {
                let comp_len = self.history.current().active_composition().layers.len();
                if from_idx < comp_len && to_idx < comp_len && from_idx != to_idx {
                    let mut session =
                        crate::core::editor::EditorSession::new(&mut self.history, "Reorder Layer");
                    let comp = session.current_mut().active_composition_mut();
                    let removed = comp.layers.remove(from_idx);
                    comp.layers.insert(to_idx, removed);
                    session.commit();
                    self.selection.selected_layers.clear();
                    self.selection.selected_layers.insert(to_idx);
                    self.notify_project_mutation();
                }
            }
            AppCommand::SplitLayerAtCurrentFrame(layer_idx) => {
                let current_frame = self.playback.current_frame;
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Split Layer");
                let comp = session.current_mut().active_composition_mut();
                if let Some(layer) = comp.layers.get_mut(layer_idx) {
                    if current_frame > layer.in_frame && current_frame < layer.out_frame {
                        let old_out = layer.out_frame;
                        layer.out_frame = current_frame;
                        let mut split_part = layer.clone();
                        split_part.id = format!("{}_split", split_part.id);
                        split_part.in_frame = current_frame;
                        split_part.out_frame = old_out;
                        split_part.name = format!("{} (Split)", split_part.name);
                        comp.layers.insert(layer_idx + 1, split_part);
                        session.commit();
                        self.selection.selected_layers.clear();
                        self.selection.selected_layers.insert(layer_idx + 1);
                        self.notify_project_mutation();
                    }
                }
            }
            AppCommand::SetBackgroundColor(color) => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Change Background Color");
                session.current_mut().active_composition_mut().background_color = color;
                session.commit();
                self.notify_project_mutation();
            }
            AppCommand::SetMotionBlurShutterAngle(angle) => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Change Shutter Angle");
                session.current_mut().active_composition_mut().motion_blur_shutter_angle = angle;
                session.commit();
                self.notify_project_mutation();
            }
            AppCommand::AddMaskToLayer { layer_idx, mask } => {
                let mut session =
                    crate::core::editor::EditorSession::new(&mut self.history, "Add Mask");
                let comp = session.current_mut().active_composition_mut();
                if let Some(layer) = comp.layers.get_mut(layer_idx) {
                    layer.masks.push(mask);
                    session.commit();
                    self.notify_project_mutation();
                }
            }
        }
    }
}
