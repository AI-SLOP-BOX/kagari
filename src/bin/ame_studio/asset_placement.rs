use super::*;

impl AfterEffectsStudio {
    pub(crate) fn place_project_asset(&mut self, item_id: &str, fill_image: bool) {
        use kagari_vfx::core::timeline::{Layer, LayerType, ProjectItemType};
        let Some(item) = self
            .project_model
            .assets
            .iter()
            .find(|item| item.id == item_id)
            .cloned()
        else {
            return;
        };
        if let ProjectItemType::Video { path, .. } = &item.item_type {
            self.start_video_import(std::path::PathBuf::from(path), None, None, false);
            return;
        }
        if let ProjectItemType::Composition { comp_idx } = &item.item_type {
            self.place_project_composition_asset(*comp_idx, item.name);
            return;
        }
        let composition = self.project_model.active_composition();
        let (width, height, duration) = (
            composition.width as f32,
            composition.height as f32,
            composition.duration_frames,
        );
        let id = format!(
            "ameui_asset_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default(),
        );
        let item_name = item.name.clone();
        let stored_image_dimensions = match &item.item_type {
            ProjectItemType::Image { width, height, .. } => (*width, *height),
            _ => (0, 0),
        };
        let layer_type = match item.item_type {
            ProjectItemType::Image { path, .. } => LayerType::Image { path },
            ProjectItemType::Model3D { path } => LayerType::Model3D { path },
            ProjectItemType::Audio { path, .. } => LayerType::Audio {
                path,
                volume: kagari_vfx::core::timeline::Animatable::new_constant(0.0),
            },
            ProjectItemType::Solid { color } => LayerType::Solid { color },
            ProjectItemType::Video { .. }
            | ProjectItemType::Composition { .. }
            | ProjectItemType::Folder { .. } => return,
        };
        let is_model = matches!(&layer_type, LayerType::Model3D { .. });
        let is_image = matches!(&layer_type, LayerType::Image { .. });
        let mut layer = Layer::new(id.clone(), item_name.clone(), layer_type, duration);
        layer.is_3d = is_model;
        layer.transform.position =
            kagari_vfx::core::timeline::Animatable::new_constant([width * 0.5, height * 0.5]);
        if is_image {
            let image_dimensions = if stored_image_dimensions.0 > 0 && stored_image_dimensions.1 > 0
            {
                stored_image_dimensions
            } else if let LayerType::Image { path } = &layer.layer_type {
                kagari_vfx::core::image_cache::image_dimensions(path).unwrap_or((0, 0))
            } else {
                (0, 0)
            };
            if image_dimensions.0 > 0 && image_dimensions.1 > 0 {
                let scale = if fill_image {
                    (width / image_dimensions.0 as f32).max(height / image_dimensions.1 as f32)
                } else {
                    (width / image_dimensions.0 as f32)
                        .min(height / image_dimensions.1 as f32)
                        .min(1.0)
                } * 100.0;
                layer.transform.scale =
                    kagari_vfx::core::timeline::Animatable::new_constant([scale, scale]);
            }
        }
        self.project_model.active_composition_mut().add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.asset_status = format!("素材をシーンに配置しました: {}", item_name);
        self.mark_project_changed();
    }
}
