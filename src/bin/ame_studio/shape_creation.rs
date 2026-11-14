use super::*;

impl AfterEffectsStudio {
    pub(crate) fn add_shape_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let position = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        self.add_shape_at(position);
    }

    pub(crate) fn add_ellipse_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let center = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        let layer_type = kagari_vfx::core::timeline::LayerType::Shape {
            shape_type: kagari_vfx::core::timeline::ShapeType::Ellipse {
                width: kagari_vfx::core::timeline::Animatable::new_constant(100.0),
                height: kagari_vfx::core::timeline::Animatable::new_constant(100.0),
            },
            color: [1.0, 0.42, 0.12, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            fill_type: kagari_vfx::core::timeline::ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        };
        self.add_canvas_layer("楕円".to_owned(), layer_type, center);
        let _ = self.sync_bindings();
    }

    pub(crate) fn add_star_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let center = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        let layer_type = kagari_vfx::core::timeline::LayerType::Shape {
            shape_type: kagari_vfx::core::timeline::ShapeType::Star {
                points: kagari_vfx::core::timeline::Animatable::new_constant(5.0),
                inner_radius: kagari_vfx::core::timeline::Animatable::new_constant(32.0),
                outer_radius: kagari_vfx::core::timeline::Animatable::new_constant(80.0),
            },
            color: [1.0, 0.42, 0.12, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            fill_type: kagari_vfx::core::timeline::ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        };
        self.add_canvas_layer("星形".to_owned(), layer_type, center);
    }

    pub(crate) fn add_polygon_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let center = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        let layer_type = kagari_vfx::core::timeline::LayerType::Shape {
            shape_type: kagari_vfx::core::timeline::ShapeType::Polygon {
                sides: kagari_vfx::core::timeline::Animatable::new_constant(6.0),
                radius: kagari_vfx::core::timeline::Animatable::new_constant(80.0),
            },
            color: [1.0, 0.42, 0.12, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            fill_type: kagari_vfx::core::timeline::ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        };
        self.add_canvas_layer("多角形".to_owned(), layer_type, center);
    }

    pub(crate) fn add_solid_layer(&mut self) {
        let composition = self.project_model.active_composition();
        let center = [
            composition.width as f64 * 0.5,
            composition.height as f64 * 0.5,
        ];
        self.add_canvas_layer(
            "平面".to_owned(),
            kagari_vfx::core::timeline::LayerType::Solid {
                color: [0.08, 0.10, 0.12, 1.0],
            },
            center,
        );
    }

    pub(crate) fn add_shape_at(&mut self, position: [f64; 2]) {
        use kagari_vfx::core::timeline::{LayerType, ShapeFillType, ShapeType};
        self.add_canvas_layer(
            "長方形".to_owned(),
            LayerType::Shape {
                shape_type: ShapeType::Rectangle {
                    width: kagari_vfx::core::timeline::Animatable::new_constant(100.0),
                    height: kagari_vfx::core::timeline::Animatable::new_constant(100.0),
                    corner_radius: kagari_vfx::core::timeline::Animatable::new_constant(0.0),
                },
                color: [1.0, 0.42, 0.12, 1.0],
                stroke_color: [1.0, 1.0, 1.0, 1.0],
                stroke_width: 0.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            position,
        );
        let _ = self.sync_bindings();
    }

    pub(crate) fn add_pen_line(&mut self, start: [f64; 2], end: [f64; 2]) {
        use kagari_vfx::core::timeline::{LayerType, ShapeFillType, ShapeType};
        let delta = [end[0] - start[0], end[1] - start[1]];
        let points = vec![
            [0.0, 0.0],
            [(delta[0] * 0.5) as f32, (delta[1] * 0.5) as f32],
            [delta[0] as f32, delta[1] as f32],
        ];
        self.add_canvas_layer(
            "ペンの線".to_owned(),
            LayerType::Shape {
                shape_type: ShapeType::FreeformBezier {
                    points,
                    tangents: vec![([0.0, 0.0], [0.0, 0.0]); 3],
                    closed: false,
                },
                color: [0.0, 0.0, 0.0, 0.0],
                stroke_color: [1.0, 0.42, 0.12, 1.0],
                stroke_width: 6.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            start,
        );
        let _ = self.sync_bindings();
    }
}
