use super::*;
use kagari_vfx::core::roto_brush_engine::{
    MAX_ROTO_FRAME_PIXELS, RotoBrushSettings, RotoStroke, RotoStrokeType,
};

#[path = "roto_brush/analysis_worker.rs"]
mod analysis_worker;

pub(crate) const ROTO_ANALYSIS_READY_MESSAGE: &str = "kagari_roto_analysis_ready";
pub(crate) const ROTO_PROPAGATION_READY_MESSAGE: &str = "kagari_roto_propagation_ready";
const MAX_ROTO_STROKE_POINTS: usize = 100_000;
pub(crate) struct RotoLiveStroke {
    pub(crate) composition_id: String,
    pub(crate) layer_id: String,
    pub(crate) project_generation: u64,
    pub(crate) composition_width: u32,
    pub(crate) composition_height: u32,
    pub(crate) frame: u32,
    pub(crate) stroke: RotoStroke,
}

pub(crate) struct RotoAnalysisJob {
    receiver: std::sync::mpsc::Receiver<RotoAnalysisCompletion>,
    pub(crate) cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub(crate) struct RotoPropagationJob {
    receiver: std::sync::mpsc::Receiver<RotoPropagationCompletion>,
    pub(crate) cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

struct RotoPropagationCompletion {
    composition_id: String,
    composition_width: u32,
    composition_height: u32,
    composition_fps: u32,
    composition_duration_frames: u32,
    project_generation: u64,
    layer_id: String,
    mask_id: String,
    source_layer_snapshot: Vec<u8>,
    result: Result<kagari_vfx::core::property::Animatable<Vec<[f32; 2]>>, String>,
}

struct RotoAnalysisCompletion {
    composition_id: String,
    composition_width: u32,
    composition_height: u32,
    composition_fps: u32,
    composition_duration_frames: u32,
    project_generation: u64,
    layer_id: String,
    frame: u32,
    feather_radius: f32,
    source_layer_snapshot: Vec<u8>,
    result: Result<Option<Vec<[f32; 2]>>, String>,
}

impl RotoLiveStroke {
    fn add_point(
        &mut self,
        point: [f64; 2],
        pressure: Option<f32>,
        width: u32,
        height: u32,
    ) -> bool {
        if !point.iter().all(|value| value.is_finite())
            || point[0] < 0.0
            || point[1] < 0.0
            || point[0] >= f64::from(width)
            || point[1] >= f64::from(height)
        {
            return false;
        }
        let next = [point[0] as f32, point[1] as f32];
        let pressure = pressure
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(0.0, 1.0))
            .unwrap_or(1.0);
        let Some(previous) = self.stroke.points.last().copied() else {
            self.stroke.points.push(next);
            self.stroke.pressures.push(pressure);
            return true;
        };
        let distance = f64::from((next[0] - previous[0]).hypot(next[1] - previous[1]));
        let spacing = (f64::from(self.stroke.radius) * 0.3).clamp(0.5, 6.0);
        let steps = (distance / spacing).ceil().max(1.0) as usize;
        let remaining = MAX_ROTO_STROKE_POINTS.saturating_sub(self.stroke.points.len());
        if remaining == 0 {
            return false;
        }
        let steps = steps.min(remaining);
        let previous_pressure = self.stroke.pressures.last().copied().unwrap_or(1.0);
        for step in 1..=steps {
            let amount = step as f32 / steps as f32;
            self.stroke.points.push([
                previous[0] + (next[0] - previous[0]) * amount,
                previous[1] + (next[1] - previous[1]) * amount,
            ]);
            self.stroke
                .pressures
                .push(previous_pressure + (pressure - previous_pressure) * amount);
        }
        true
    }
}

impl AfterEffectsStudio {
    pub(crate) fn begin_roto_stroke_from_event(
        &mut self,
        point: [f64; 2],
        pressure: Option<f32>,
        samples: &[([f64; 2], Option<f32>)],
        background: bool,
    ) {
        let first = samples.first();
        self.begin_roto_stroke(
            first.map_or(point, |sample| sample.0),
            first.and_then(|sample| sample.1).or(pressure),
            background,
        );
        for (point, pressure) in samples.iter().skip(1) {
            self.extend_roto_stroke(*point, *pressure);
        }
        if first.is_some() {
            self.extend_roto_stroke(point, pressure);
        }
    }

    pub(crate) fn extend_roto_stroke_from_event(
        &mut self,
        point: [f64; 2],
        pressure: Option<f32>,
        samples: &[([f64; 2], Option<f32>)],
    ) {
        for (sample_point, sample_pressure) in samples {
            self.extend_roto_stroke(*sample_point, *sample_pressure);
        }
        self.extend_roto_stroke(point, pressure);
    }

    fn roto_job_busy(&self) -> bool {
        self.roto_analysis_job.is_some() || self.roto_propagation_job.is_some()
    }

    pub(crate) fn invalidate_roto_operations(&mut self) {
        let had_roto_operation = self.roto_analysis_job.is_some()
            || self.roto_propagation_job.is_some()
            || self.roto_live_stroke.is_some();
        if let Some(job) = self.roto_analysis_job.take() {
            job.cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(job) = self.roto_propagation_job.take() {
            job.cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.roto_live_stroke = None;
        if had_roto_operation {
            self.roto_status = "プロジェクト変更に伴いロト処理を破棄しました".to_owned();
        }
    }

    pub(crate) fn cancel_roto_jobs(&mut self) {
        let mut cancelled = false;
        if let Some(job) = &self.roto_analysis_job {
            job.cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
            cancelled = true;
        }
        if let Some(job) = &self.roto_propagation_job {
            job.cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
            cancelled = true;
        }
        self.roto_status = if cancelled {
            "ロト処理を中止しています…".to_owned()
        } else {
            "実行中のロト処理はありません".to_owned()
        };
        self.sync_bindings_logged();
    }

    pub(crate) fn start_roto_propagation(&mut self) -> Result<(), String> {
        if self.roto_job_busy() {
            return Err("ロト処理が実行中です".to_owned());
        }
        let event_sender = self
            .app_event_sender
            .clone()
            .ok_or_else(|| "UIイベントループが初期化されていません".to_owned())?;
        let composition = self.project_model.active_composition().clone();
        let layer_index = composition
            .layers
            .iter()
            .position(|layer| Some(layer.id.as_str()) == self.selected_layer_id.as_deref())
            .ok_or_else(|| "伝播する素材レイヤーを選択してください".to_owned())?;
        let layer = &composition.layers[layer_index];
        if layer.locked {
            return Err("レイヤーのロックを解除してください".to_owned());
        }
        let mask = layer
            .masks
            .iter()
            .find(|mask| mask.id == kagari_vfx::core::roto_assist::ROTO_BRUSH_MATTE_ID)
            .ok_or_else(|| "先にロトストロークからマットを作成してください".to_owned())?
            .clone();
        let start_frame = layer.in_frame;
        let end_frame = layer.out_frame.saturating_sub(1).max(start_frame);
        let anchor_frame = self.current_frame;
        if anchor_frame < start_frame || anchor_frame > end_frame {
            return Err("再生ヘッドをレイヤーの表示範囲内へ移動してください".to_owned());
        }
        let source_layer_snapshot = serde_json::to_vec(layer)
            .map_err(|error| format!("素材レイヤーを記録できません: {error}"))?;
        let source_strokes = layer.roto_brush_strokes.clone();
        let layer_id = layer.id.clone();
        let mask_id = mask.id.clone();
        let composition_id = composition.id.clone();
        let composition_width = composition.width;
        let composition_height = composition.height;
        let composition_fps = composition.fps;
        let composition_duration_frames = composition.duration_frames;
        let project_generation = self.project_generation;
        let source_comp =
            kagari_vfx::core::roto_assist::prepare_roto_brush_source(&composition, layer_index)
                .ok_or_else(|| "選択した素材レイヤーを解析できません".to_owned())?;
        let (width, height) = (source_comp.width, source_comp.height);
        let expected_pixels = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "コンポジションサイズが大きすぎます".to_owned())?;
        if expected_pixels == 0 || expected_pixels > MAX_ROTO_FRAME_PIXELS.saturating_mul(4) {
            return Err("ロト伝播のコンポジションサイズが上限を超えています".to_owned());
        }
        let settings = RotoBrushSettings {
            feather_radius: self.roto_feather_radius.clamp(0.0, 32.0) as f32,
            ..Default::default()
        };
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("kagari-ame-roto-propagation".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    kagari_vfx::core::roto_assist::bake_optical_flow_roto_matte_cancellable(
                        &mask,
                        &source_strokes,
                        &settings,
                        anchor_frame,
                        start_frame,
                        end_frame,
                        width,
                        height,
                        |frame| {
                            if worker_cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                                return None;
                            }
                            let pixels = kagari_vfx::core::software_renderer::with_render_cancel_flag(
                                worker_cancelled.clone(),
                                || {
                                    kagari_vfx::core::software_renderer::render_frame_to_pixels_filtered(
                                    &source_comp,
                                    frame,
                                    width,
                                    height,
                                    0.0,
                                    0,
                                    Some(layer_index),
                                    )
                                },
                            );
                            (pixels.len() == expected_pixels).then_some(pixels)
                        },
                        || worker_cancelled.load(std::sync::atomic::Ordering::Relaxed),
                    )
                }))
                .unwrap_or_else(|_| Err("ロト伝播中に内部エラーが発生しました".to_owned()));
                let result = if worker_cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                    Err("ロト伝播を中止しました".to_owned())
                } else {
                    result
                };
                let _ = sender.send(RotoPropagationCompletion {
                    composition_id,
                    composition_width,
                    composition_height,
                    composition_fps,
                    composition_duration_frames,
                    project_generation,
                    layer_id,
                    mask_id,
                    source_layer_snapshot,
                    result,
                });
                let _ = event_sender.send(ROTO_PROPAGATION_READY_MESSAGE);
            })
            .map_err(|error| format!("伝播ワーカーを開始できません: {error}"))?;
        self.roto_propagation_job = Some(RotoPropagationJob {
            receiver,
            cancelled,
        });
        self.roto_status = format!(
            "{}〜{}フレームを光学フローで伝播中…",
            start_frame, end_frame
        );
        self.sync_bindings_logged();
        Ok(())
    }

    pub(crate) fn complete_roto_propagation(&mut self) {
        let Some(job) = self.roto_propagation_job.take() else {
            return;
        };
        let completion = match job.receiver.try_recv() {
            Ok(completion) => completion,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                self.roto_propagation_job = Some(job);
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.roto_status = "ロト伝播ワーカーが停止しました".to_owned();
                self.sync_bindings_logged();
                return;
            }
        };
        if job.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            self.roto_status = "ロト伝播を中止しました".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let active_composition = self.project_model.active_composition();
        if completion.project_generation != self.project_generation
            || completion.composition_id != active_composition.id
            || completion.composition_width != active_composition.width
            || completion.composition_height != active_composition.height
            || completion.composition_fps != active_composition.fps
            || completion.composition_duration_frames != active_composition.duration_frames
        {
            self.roto_status = "プロジェクトが変わったため伝播結果を破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let Some(layer_index) = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .position(|layer| layer.id == completion.layer_id)
        else {
            self.roto_status = "素材レイヤーが削除されたため伝播結果を破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        };
        let composition = self.project_model.active_composition_mut();
        let layer = &mut composition.layers[layer_index];
        if serde_json::to_vec(layer).ok().as_deref()
            != Some(completion.source_layer_snapshot.as_slice())
        {
            self.roto_status = "素材が変更されたため伝播結果を破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let Some(mask) = layer
            .masks
            .iter_mut()
            .find(|mask| mask.id == completion.mask_id)
        else {
            self.roto_status = "ロトマットが削除されたため伝播結果を破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        };
        let baked = match completion.result {
            Ok(baked) => baked,
            Err(error) => {
                self.roto_status =
                    super::ame_studio_roto_brush_messages::localize_propagation_error(&error);
                self.sync_bindings_logged();
                return;
            }
        };
        mask.path.vertices = baked;
        self.mark_project_changed();
        self.roto_status = "ロトマットをレイヤー範囲へ伝播しました".to_owned();
        self.sync_selected_layer_controls();
        self.sync_bindings_logged();
    }

    pub(crate) fn clear_roto_brush(&mut self) {
        if self.roto_job_busy() || self.roto_live_stroke.is_some() {
            self.roto_status = "ロト操作が完了してから消去してください".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            self.roto_status = "消去するレイヤーを選択してください".to_owned();
            self.sync_bindings_logged();
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            self.roto_status = "選択レイヤーが見つからないか、ロックされています".to_owned();
            self.sync_bindings_logged();
            return;
        };
        let had_roto = !layer.roto_brush_strokes.is_empty()
            || layer
                .masks
                .iter()
                .any(|mask| mask.id == kagari_vfx::core::roto_assist::ROTO_BRUSH_MATTE_ID);
        layer.roto_brush_strokes.clear();
        layer
            .masks
            .retain(|mask| mask.id != kagari_vfx::core::roto_assist::ROTO_BRUSH_MATTE_ID);
        if had_roto {
            self.mark_project_changed();
        }
        self.roto_status = "ロトストロークとマットを消去しました".to_owned();
        self.sync_selected_layer_controls();
        self.sync_bindings_logged();
    }

    pub(crate) fn begin_roto_stroke(
        &mut self,
        point: [f64; 2],
        pressure: Option<f32>,
        background: bool,
    ) {
        if self.roto_job_busy() {
            self.roto_status = "マット解析中です。完了後に続けてください".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let Some(layer_id) = self.selected_layer_id.clone() else {
            self.roto_status = "先に画像・動画レイヤーを選択してください".to_owned();
            self.sync_bindings_logged();
            return;
        };
        let composition = self.project_model.active_composition();
        let Some(layer) = composition.layers.iter().find(|layer| layer.id == layer_id) else {
            self.roto_status = "選択レイヤーが見つかりません".to_owned();
            self.sync_bindings_logged();
            return;
        };
        if layer.locked {
            self.roto_status = "レイヤーのロックを解除してください".to_owned();
            self.sync_bindings_logged();
            return;
        }
        if !matches!(
            layer.layer_type,
            kagari_vfx::core::timeline::LayerType::Image { .. }
                | kagari_vfx::core::timeline::LayerType::Video { .. }
                | kagari_vfx::core::timeline::LayerType::PreComp { .. }
        ) {
            self.roto_status = "ロトブラシは画像・動画・プリコンポジションに使えます".to_owned();
            self.sync_bindings_logged();
            return;
        }
        if !layer.is_active(self.current_frame) {
            self.roto_status = "選択レイヤーの表示範囲内へ再生ヘッドを移動してください".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let (composition_id, width, height) = (
            composition.id.clone(),
            composition.width,
            composition.height,
        );
        let mut live = RotoLiveStroke {
            composition_id,
            layer_id,
            project_generation: self.project_generation,
            composition_width: width,
            composition_height: height,
            frame: self.current_frame,
            stroke: RotoStroke {
                stroke_type: if background || !self.roto_foreground {
                    RotoStrokeType::Background
                } else {
                    RotoStrokeType::Foreground
                },
                points: Vec::new(),
                pressures: Vec::new(),
                radius: self.roto_brush_radius.clamp(1.0, 128.0) as f32,
                frame: Some(self.current_frame),
            },
        };
        if !live.add_point(point, pressure, width, height) {
            self.roto_status = "コンポジション内からストロークを開始してください".to_owned();
            self.sync_bindings_logged();
            return;
        }
        self.roto_live_stroke = Some(live);
        self.roto_foreground = self
            .roto_live_stroke
            .as_ref()
            .is_some_and(|live| live.stroke.stroke_type == RotoStrokeType::Foreground);
        self.roto_status = if self.roto_foreground {
            "前景ストロークを入力中".to_owned()
        } else {
            "背景ストロークを入力中".to_owned()
        };
        self.sync_bindings_logged();
    }

    pub(crate) fn extend_roto_stroke(&mut self, point: [f64; 2], pressure: Option<f32>) {
        let composition = self.project_model.active_composition();
        let (width, height) = (composition.width, composition.height);
        let composition_id = composition.id.as_str();
        let project_generation = self.project_generation;
        if self.roto_live_stroke.as_ref().is_some_and(|live| {
            live.project_generation != project_generation
                || live.composition_id != composition_id
                || live.composition_width != width
                || live.composition_height != height
        }) {
            self.roto_live_stroke = None;
            self.roto_status =
                "コンポジション設定が変わったためストロークを破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let Some(live) = self.roto_live_stroke.as_mut() else {
            return;
        };
        if !live.add_point(point, pressure, width, height) {
            self.roto_status = "ストローク上限に達しました。離して解析してください".to_owned();
        }
    }

    pub(crate) fn finish_roto_stroke(&mut self, point: Option<[f64; 2]>, pressure: Option<f32>) {
        let Some(mut live) = self.roto_live_stroke.take() else {
            return;
        };
        let composition = self.project_model.active_composition();
        if live.project_generation != self.project_generation
            || composition.id != live.composition_id
            || composition.width != live.composition_width
            || composition.height != live.composition_height
        {
            self.roto_status =
                "コンポジション設定が変わったためストロークを破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        }
        if let Some(point) = point {
            live.add_point(
                point,
                pressure,
                live.composition_width,
                live.composition_height,
            );
        }
        let frame = live.frame;
        let Some(layer_index) = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .position(|layer| layer.id == live.layer_id && !layer.locked)
        else {
            self.roto_status = "選択レイヤーが変わったためストロークを破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        };
        let frame_strokes = {
            let composition = self.project_model.active_composition_mut();
            let Some(layer) = composition.layers.get_mut(layer_index) else {
                return;
            };
            layer.roto_brush_strokes.push(live.stroke);
            kagari_vfx::core::roto_brush_engine::strokes_for_frame(&layer.roto_brush_strokes, frame)
        };
        self.mark_project_changed();
        self.roto_status = if frame_strokes
            .iter()
            .any(|stroke| stroke.stroke_type == RotoStrokeType::Foreground)
        {
            "マットを解析中…".to_owned()
        } else {
            "前景をなぞるとマットが生成されます".to_owned()
        };
        if frame_strokes
            .iter()
            .any(|stroke| stroke.stroke_type == RotoStrokeType::Foreground)
        {
            if let Err(error) = self.start_roto_analysis(layer_index, frame, frame_strokes) {
                self.roto_status = error;
            }
        }
        self.sync_bindings_logged();
    }

    pub(crate) fn complete_roto_analysis(&mut self) {
        let Some(job) = self.roto_analysis_job.take() else {
            return;
        };
        let completion = match job.receiver.try_recv() {
            Ok(completion) => completion,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                self.roto_analysis_job = Some(job);
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.roto_status = "ロト解析ワーカーが停止しました".to_owned();
                self.sync_bindings_logged();
                return;
            }
        };
        if job.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            self.roto_status = "ロト解析を中止しました".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let active_composition = self.project_model.active_composition();
        if completion.project_generation != self.project_generation
            || completion.composition_id != active_composition.id
            || completion.composition_width != active_composition.width
            || completion.composition_height != active_composition.height
            || completion.composition_fps != active_composition.fps
            || completion.composition_duration_frames != active_composition.duration_frames
        {
            self.roto_status = "プロジェクトが変わったため解析結果を破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let Some(layer_index) = self
            .project_model
            .active_composition()
            .layers
            .iter()
            .position(|layer| layer.id == completion.layer_id)
        else {
            self.roto_status = "素材レイヤーが削除されたため解析結果を破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        };
        let layer_matches =
            serde_json::to_vec(&self.project_model.active_composition().layers[layer_index])
                .ok()
                .as_deref()
                == Some(completion.source_layer_snapshot.as_slice());
        if !layer_matches {
            self.roto_status = "素材が変更されたため解析結果を破棄しました".to_owned();
            self.sync_bindings_logged();
            return;
        }
        let polygon = match completion.result {
            Ok(Some(polygon)) => polygon,
            Ok(None) => {
                self.roto_status =
                    "輪郭を作れませんでした。前景／背景ストロークを追加してください".to_owned();
                self.sync_bindings_logged();
                return;
            }
            Err(error) => {
                self.roto_status = error;
                self.sync_bindings_logged();
                return;
            }
        };
        let composition = self.project_model.active_composition_mut();
        let layer = &mut composition.layers[layer_index];
        if let Some(mask) = layer
            .masks
            .iter_mut()
            .find(|mask| mask.id == kagari_vfx::core::roto_assist::ROTO_BRUSH_MATTE_ID)
        {
            kagari_vfx::core::roto_assist::set_roto_matte_keyframe(mask, completion.frame, polygon);
            mask.feather =
                kagari_vfx::core::property::Animatable::new_constant(completion.feather_radius);
        } else {
            let mut mask = kagari_vfx::core::mask::Mask::new_closed(
                kagari_vfx::core::roto_assist::ROTO_BRUSH_MATTE_ID.to_owned(),
                "Roto Brush Matte".to_owned(),
                polygon.clone(),
            );
            kagari_vfx::core::roto_assist::set_roto_matte_keyframe(
                &mut mask,
                completion.frame,
                polygon,
            );
            mask.feather =
                kagari_vfx::core::property::Animatable::new_constant(completion.feather_radius);
            layer.masks.push(mask);
        }
        self.mark_project_changed();
        self.roto_status = format!("{}フレームのマットを更新しました", completion.frame);
        self.sync_selected_layer_controls();
        self.sync_bindings_logged();
    }
}
