use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::physics::{
    bake_physics_simulation_to_keyframes, calc_spring_overshoot, PhysicsWorld, RigidBody,
    RigidBodyType,
};
use crate::core::property::Animatable;
use crate::ui::kui::{self, ButtonKind};
use crate::KagariApp;
use eframe::egui;

/// 2D Rigid Body Physics & Spring Dynamics Panel for AE Motion Graphics.
pub fn draw_physics_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    kui::section(ui, "⚛ Physics & Dynamics");
    kui::secondary(ui, "2D Rigid Body Collision Simulation & Keyframe Baker");
    kui::separator(ui);

    let gravity_id = egui::Id::new("ae_physics_gravity_y");
    let mut gravity_y: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(gravity_id, || 980.0));

    let bounciness_id = egui::Id::new("ae_physics_bounciness");
    let mut bounciness: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(bounciness_id, || 0.6));

    let friction_id = egui::Id::new("ae_physics_friction");
    let mut friction: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(friction_id, || 0.3));

    let floor_id = egui::Id::new("ae_physics_floor_enabled");
    let mut floor_enabled: bool = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(floor_id, || true));

    let collider_shape_id = egui::Id::new("ae_physics_collider_shape");
    let mut collider_shape_idx: usize = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(collider_shape_id, || 0));

    let mut bounciness_percent = bounciness * 100.0;
    let mut friction_percent = friction * 100.0;
    let grid = kui::PropertyGrid::new(
        ui,
        &["Gravity Y", "Bounciness", "Friction", "Collider Shape"],
    );
    if grid
        .row(ui, "Gravity Y", |ui| {
            kui::slider(ui, &mut gravity_y, -2000.0..=3000.0, " px/s²")
        })
        .changed()
    {
        ui.ctx().data_mut(|d| d.insert_temp(gravity_id, gravity_y));
    }
    if grid
        .row(ui, "Bounciness", |ui| {
            kui::slider(ui, &mut bounciness_percent, 0.0..=100.0, "%")
        })
        .changed()
    {
        bounciness = bounciness_percent / 100.0;
        ui.ctx()
            .data_mut(|d| d.insert_temp(bounciness_id, bounciness));
    }
    if grid
        .row(ui, "Friction", |ui| {
            kui::slider(ui, &mut friction_percent, 0.0..=100.0, "%")
        })
        .changed()
    {
        friction = friction_percent / 100.0;
        ui.ctx().data_mut(|d| d.insert_temp(friction_id, friction));
    }
    if grid.row(ui, "Collider Shape", |ui| {
        kui::combo(
            ui,
            collider_shape_id,
            &mut collider_shape_idx,
            &[(0, "Box"), (1, "⚪ Circle")],
        )
    }) {
        ui.ctx()
            .data_mut(|d| d.insert_temp(collider_shape_id, collider_shape_idx));
    }
    if kui::toggle(ui, &mut floor_enabled, "Floor Boundary at Bottom of Comp").changed() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(floor_id, floor_enabled));
    }

    kui::separator(ui);

    let comp = app.history.current().active_composition();
    let comp_w = comp.width as f32;
    let comp_h = comp.height as f32;
    let dur = comp.duration_frames;
    let fps = comp.fps as f32;
    let selected_layer_indices: Vec<usize> =
        app.selection.selected_layers.iter().copied().collect();

    if selected_layer_indices.is_empty() {
        kui::empty_state(
            ui,
            "Select one or more layers in the Timeline to simulate.",
            64.0,
        );
    } else {
        kui::secondary(
            ui,
            crate::ui::localization::tr_format(
                "{} layer(s) selected for physics simulation",
                &[&selected_layer_indices.len().to_string()],
            ),
        );

        ui.add_space(kui::space::SM);
        if kui::button(ui, "Simulate & Bake to Keyframes", ButtonKind::Accent).clicked() {
            let mut temp_proj = app.history.current().clone();
            let active_comp = temp_proj.active_composition_mut();

            let mut world = PhysicsWorld::new();
            world.gravity = [0.0, gravity_y];

            // 1. Add static floor if enabled
            if floor_enabled {
                let floor_body = RigidBody::new_box(
                    None,
                    [comp_w * 0.5, comp_h + 20.0],
                    comp_w * 2.0,
                    40.0,
                    0.0,
                    RigidBodyType::Static,
                );
                world.add_body(floor_body);
            }

            // 2. Add selected layers as rigid bodies
            for &idx in &selected_layer_indices {
                if let Some(layer) = active_comp.layers.get(idx) {
                    let pos = layer.transform.position.evaluate(0);
                    let body = if collider_shape_idx == 1 {
                        let mut b = RigidBody::new_circle(
                            Some(idx),
                            pos,
                            30.0,
                            1.0,
                            RigidBodyType::Dynamic,
                        );
                        b.restitution = bounciness;
                        b.friction = friction;
                        b
                    } else {
                        let mut b = RigidBody::new_box(
                            Some(idx),
                            pos,
                            60.0,
                            60.0,
                            1.0,
                            RigidBodyType::Dynamic,
                        );
                        b.restitution = bounciness;
                        b.friction = friction;
                        b
                    };
                    world.add_body(body);
                }
            }

            // 3. Bake simulation
            let baked =
                bake_physics_simulation_to_keyframes(&mut world, 0, dur.saturating_sub(1), fps);

            // 4. Apply baked keyframes to composition layers
            for (layer_idx, (pos_kfs, rot_kfs)) in baked {
                if let Some(layer) = active_comp.layers.get_mut(layer_idx) {
                    layer.transform.position = Animatable::Animated(pos_kfs);
                    layer.transform.rotation = Animatable::Animated(rot_kfs);
                }
            }

            app.commit_project(temp_proj);
            app.toasts.info(crate::ui::localization::tr(
                "Physics simulation baked to keyframes!",
            ));
        }
    }

    kui::separator(ui);
    kui::section_title(ui, "Inertia / Bounce Dynamic Keyframes");
    if kui::button_enabled(
        ui,
        "Bake Inertial Bounce on Scale",
        ButtonKind::Default,
        !selected_layer_indices.is_empty(),
    )
    .clicked()
    {
        if selected_layer_indices.is_empty() {
            app.toasts
                .error(crate::ui::localization::tr("Select a layer first"));
            return;
        }

        let mut temp_proj = app.history.current().clone();
        let active_comp = temp_proj.active_composition_mut();
        for &idx in &selected_layer_indices {
            if let Some(layer) = active_comp.layers.get_mut(idx) {
                let start_f = layer.in_frame;
                let end_f = (start_f + 45).min(layer.out_frame);
                let base_scale = layer.transform.scale.evaluate(start_f);

                let mut kfs = Vec::new();
                for f in start_f..=end_f {
                    let t = (f - start_f) as f32 / fps.max(1.0);
                    let bounce = calc_spring_overshoot(t, 3.5, 6.0, 25.0);
                    let scale_val = [base_scale[0] + bounce, base_scale[1] + bounce];
                    kfs.push(Keyframe::new(f, scale_val, InterpolationType::Linear));
                }
                layer.transform.scale = Animatable::Animated(kfs);
            }
        }

        app.commit_project(temp_proj);
        app.toasts.info(crate::ui::localization::tr(
            "Baked inertial bounce keyframes to Scale!",
        ));
    }
}
