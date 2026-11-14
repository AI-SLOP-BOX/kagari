use super::*;

impl AfterEffectsStudio {
    pub(super) fn start_roto_analysis(
        &mut self,
        layer_index: usize,
        frame: u32,
        strokes: Vec<RotoStroke>,
    ) -> Result<(), String> {
        if self.roto_job_busy() {
            return Err("ロト解析がすでに実行中です".to_owned());
        }
        let Some(event_sender) = self.app_event_sender.clone() else {
            return Err("UIイベントループが初期化されていません".to_owned());
        };
        let composition = self.project_model.active_composition().clone();
        let frame_pixels = (composition.width as usize)
            .checked_mul(composition.height as usize)
            .ok_or_else(|| "コンポジションサイズが大きすぎます".to_owned())?;
        if frame_pixels == 0 || frame_pixels > MAX_ROTO_FRAME_PIXELS {
            return Err("ロト解析は最大約840万画素（4K UHD相当）に対応します".to_owned());
        }
        let source_comp =
            kagari_vfx::core::roto_assist::prepare_roto_brush_source(&composition, layer_index)
                .ok_or_else(|| "選択した素材レイヤーが見つかりません".to_owned())?;
        let layer = composition
            .layers
            .get(layer_index)
            .ok_or_else(|| "選択した素材レイヤーが見つかりません".to_owned())?;
        let source_layer_snapshot = serde_json::to_vec(layer)
            .map_err(|error| format!("素材レイヤーを記録できません: {error}"))?;
        let layer_id = layer.id.clone();
        let composition_id = composition.id.clone();
        let composition_width = composition.width;
        let composition_height = composition.height;
        let composition_fps = composition.fps;
        let composition_duration_frames = composition.duration_frames;
        let project_generation = self.project_generation;
        let settings = RotoBrushSettings {
            feather_radius: self.roto_feather_radius.clamp(0.0, 32.0) as f32,
            ..Default::default()
        };
        let feather_radius = settings.feather_radius;
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("kagari-ame-roto-analysis".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let pixels =
                        kagari_vfx::core::software_renderer::render_frame_to_pixels_filtered(
                            &source_comp,
                            frame,
                            source_comp.width,
                            source_comp.height,
                            0.0,
                            0,
                            Some(layer_index),
                        );
                    let expected = frame_pixels
                        .checked_mul(4)
                        .ok_or_else(|| "解析画像サイズが大きすぎます".to_owned())?;
                    if pixels.len() != expected {
                        return Err("解析用の素材フレームを描画できませんでした".to_owned());
                    }
                    if worker_cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                        return Err("解析をキャンセルしました".to_owned());
                    }
                    let Some(matte) =
                        kagari_vfx::core::roto_brush_engine::generate_rotobrush_matte_cancellable(
                            &pixels,
                            source_comp.width,
                            source_comp.height,
                            &strokes,
                            &settings,
                            || worker_cancelled.load(std::sync::atomic::Ordering::Relaxed),
                        )
                    else {
                        return Err(if worker_cancelled
                            .load(std::sync::atomic::Ordering::Relaxed)
                        {
                            "解析をキャンセルしました".to_owned()
                        } else {
                            "ロト解析のマットを生成できませんでした（メモリ不足または入力データが無効です）".to_owned()
                        });
                    };
                    if matte.len() != frame_pixels {
                        return Err("ロト解析の画像サイズが上限を超えているか無効です".to_owned());
                    }
                    if worker_cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                        return Err("解析をキャンセルしました".to_owned());
                    }
                    let polygon = kagari_vfx::core::roto_assist::trace_roto_matte_contour(
                        &matte,
                        source_comp.width,
                        source_comp.height,
                        &strokes,
                        2.0,
                    );
                    if worker_cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                        return Err("解析をキャンセルしました".to_owned());
                    }
                    Ok((polygon.len() >= 3).then_some(polygon))
                }))
                .unwrap_or_else(|_| Err("ロトブラシ解析で内部エラーが発生しました".to_owned()));
                let _ = sender.send(RotoAnalysisCompletion {
                    composition_id,
                    composition_width,
                    composition_height,
                    composition_fps,
                    composition_duration_frames,
                    project_generation,
                    layer_id,
                    frame,
                    feather_radius,
                    source_layer_snapshot,
                    result,
                });
                let _ = event_sender.send(ROTO_ANALYSIS_READY_MESSAGE);
            })
            .map_err(|error| format!("解析ワーカーを開始できません: {error}"))?;
        self.roto_analysis_job = Some(RotoAnalysisJob {
            receiver,
            cancelled,
        });
        Ok(())
    }
}
