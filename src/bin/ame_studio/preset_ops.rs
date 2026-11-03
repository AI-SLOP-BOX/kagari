use super::*;

impl AfterEffectsStudio {
    pub(crate) fn apply_animation_preset(&mut self, preset_id: &str) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            self.animation_preset_status = "レイヤーを選択してください。".to_owned();
            return;
        };
        let composition = self.project_model.active_composition_mut();
        let Some(layer) = composition
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            self.animation_preset_status = "レイヤーがロックされています。".to_owned();
            return;
        };
        let applied = kagari_vfx::core::animation_presets::apply_by_name(
            preset_id,
            layer,
            self.current_frame,
        );
        self.animation_preset_status = if applied {
            format!("適用しました: {preset_id}")
        } else {
            "プリセットを適用できませんでした。".to_owned()
        };
        if applied {
            self.mark_project_changed();
            self.sync_selected_layer_controls();
        }
    }

    pub(crate) fn animation_preset_catalog(
    ) -> &'static [(&'static str, &'static str, &'static str, &'static str)] {
        &[
            // Position
            (
                "Slide In from Left",
                "左からスライドイン",
                "位置",
                "slide left",
            ),
            (
                "Slide In from Right",
                "右からスライドイン",
                "位置",
                "slide right",
            ),
            (
                "Slide In from Top",
                "上からスライドイン",
                "位置",
                "slide top",
            ),
            (
                "Slide In from Bottom",
                "下からスライドイン",
                "位置",
                "slide bottom",
            ),
            ("Bounce In", "バウンスイン", "位置", "bounce"),
            ("Elastic Pop", "イラスティックポップ", "位置", "elastic"),
            // Scale
            ("Pop In", "ポップイン", "スケール", "scale pop"),
            ("Scale Down", "スケールダウン", "スケール", "scale down"),
            ("Breathing", "呼吸", "スケール", "breathe"),
            (
                "Squash and Stretch",
                "スカッシュ＆ストレッチ",
                "スケール",
                "squash stretch",
            ),
            // Rotation
            ("Spin Clockwise", "時計回りに回転", "回転", "spin cw"),
            (
                "Spin Counter-Clockwise",
                "反時計回りに回転",
                "回転",
                "spin ccw",
            ),
            ("Wobble", "ぐらつき", "回転", "wobble"),
            // Opacity
            ("Fade In", "フェードイン", "不透明度", "fade in"),
            ("Fade Out", "フェードアウト", "不透明度", "fade out"),
            (
                "Fade In + Hold + Fade Out",
                "フェードイン＆アウト",
                "不透明度",
                "fade hold",
            ),
            ("Flash", "フラッシュ", "不透明度", "flash"),
            // Combo (single-property implementations, filtered to motion-safe ones)
            (
                "Pop and Fade In",
                "ポップ＆フェードイン",
                "コンボ",
                "pop fade",
            ),
            (
                "Shrink and Fade Out",
                "縮小＆フェードアウト",
                "コンボ",
                "shrink fade",
            ),
            (
                "Spin and Scale In",
                "回転＆スケールイン",
                "コンボ",
                "spin scale",
            ),
            ("Drop and Bounce", "落下＆バウンス", "コンボ", "drop bounce"),
            (
                "Slide and Rotate In",
                "スライド＆回転イン",
                "コンボ",
                "slide rotate",
            ),
        ]
    }
}
