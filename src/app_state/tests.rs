use super::*;

fn drive_frames(app: &mut KagariApp, n: usize) {
    drive_frames_at_size(app, n, 1600.0, 900.0);
}

fn drive_frames_at_size(app: &mut KagariApp, n: usize, width: f32, height: f32) {
    let ctx = eframe::egui::Context::default();
    for _ in 0..n {
        let _ = ctx.run(
            eframe::egui::RawInput {
                screen_rect: Some(eframe::egui::Rect::from_min_size(
                    eframe::egui::Pos2::ZERO,
                    eframe::egui::vec2(width, height),
                )),
                ..Default::default()
            },
            |ctx| {
                crate::ui::theme::configure_ae_theme(ctx);
                app.update_panels(ctx);
            },
        );
    }
}

#[test]
fn full_studio_frame_renders_without_panic_or_leak() {
    let mut app = KagariApp {
        show_home: false,
        show_welcome: false,
        ..Default::default()
    };
    drive_frames(&mut app, 2);
    assert!(!app.drag_active(), "no transaction left open");
    assert_eq!(app.playback.current_frame, 0, "playhead must not drift");
}

#[test]
fn blocking_dialog_state_tracks_every_modal_surface() {
    let mut app = KagariApp {
        show_welcome: false,
        ..Default::default()
    };
    assert!(!app.has_blocking_dialog());

    app.show_preferences = true;
    assert!(app.has_blocking_dialog());
    app.show_preferences = false;
    app.show_command_palette = true;
    assert!(app.has_blocking_dialog());
    app.show_command_palette = false;
    app.export.show_export_dialog = true;
    assert!(app.has_blocking_dialog());
    app.export.show_export_dialog = false;

    app.show_recovery_dialog = true;
    assert!(app.has_blocking_dialog());
    app.show_recovery_dialog = false;
    app.show_comp_settings = true;
    assert!(app.has_blocking_dialog());
    app.show_comp_settings = false;
    app.show_shortcuts_dialog = true;
    assert!(app.has_blocking_dialog());
    app.show_shortcuts_dialog = false;
    app.show_precompose_dialog = true;
    assert!(app.has_blocking_dialog());
    app.show_precompose_dialog = false;
    app.show_new_comp_dialog = true;
    assert!(app.has_blocking_dialog());
}

#[test]
fn graph_editor_pointer_drag_moves_keyframe_and_commits_one_undo_step() {
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::property::Animatable;
    use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

    let mut app = KagariApp {
        show_home: false,
        show_welcome: false,
        recovery_checked: true,
        ..Default::default()
    };
    app.startup_animation.progress = 1.0;
    app.startup_animation.target = 1.0;
    app.show_graph_editor = true;
    app.selection.selected_layer_idx = Some(1);
    app.selection.selected_property = Some("Position X".to_string());

    let layer_id = {
        let project = app.history.current_mut();
        let comp = project.active_composition_mut();
        comp.layers.push(crate::core::timeline::Layer::new_null(
            "background".into(),
            "Background".into(),
            comp.duration_frames,
        ));
        comp.layers.push(crate::core::timeline::Layer::new_null(
            "animated".into(),
            "Animated".into(),
            comp.duration_frames,
        ));
        let layer = &mut comp.layers[1];
        layer.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [100.0, 540.0], InterpolationType::Linear),
            Keyframe::new(30, [300.0, 540.0], InterpolationType::Linear),
        ]);
        layer.id.clone()
    };

    let ctx = eframe::egui::Context::default();
    let screen_rect = Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(1600.0, 1200.0));
    let run_frame = |app: &mut KagariApp, events: Vec<Event>| {
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(screen_rect),
                events,
                ..Default::default()
            },
            |ctx| {
                crate::ui::theme::configure_ae_theme(ctx);
                app.update_panels(ctx);
            },
        );
    };

    for _ in 0..12 {
        run_frame(&mut app, Vec::new());
    }
    let anchor_id = egui::Id::new(("ae_graph_anchor_rect", &layer_id, "Position X", 30_u32));
    let anchor_rect: Rect = ctx
        .data(|d| d.get_temp(anchor_id))
        .expect("graph editor must expose the rendered keyframe anchor");
    let start = anchor_rect.center();
    let target = start + eframe::egui::vec2(70.0, -12.0);

    run_frame(&mut app, vec![Event::PointerMoved(start)]);
    run_frame(
        &mut app,
        vec![Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }],
    );
    run_frame(&mut app, vec![Event::PointerMoved(target)]);
    run_frame(
        &mut app,
        vec![
            Event::PointerMoved(target),
            Event::PointerButton {
                pos: target,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ],
    );

    let keyframes = app.history.current().active_composition().layers[1]
        .transform
        .position
        .keyframes()
        .expect("position remains animated");
    assert_eq!(keyframes.len(), 2);
    assert_eq!(keyframes[0].frame, 0);
    assert_eq!(keyframes[0].value, [100.0, 540.0]);
    assert!(
        keyframes[1].frame != 30 || (keyframes[1].value[0] - 300.0).abs() > 0.1,
        "pointer drag must change the selected keyframe's frame or value: {keyframes:?}"
    );
    assert_eq!(
        app.history.len(),
        2,
        "one pointer gesture must create one undo step"
    );
    assert!(
        !app.drag_active(),
        "pointer release must close the graph edit transaction"
    );

    assert!(
        app.history.undo().is_some(),
        "the graph edit must be undoable"
    );
    let restored = app.history.current().active_composition().layers[1]
        .transform
        .position
        .keyframes()
        .expect("position remains animated after undo");
    assert_eq!(restored[0].frame, 0);
    assert_eq!(restored[1].frame, 30);
    assert_eq!(restored[1].value, [300.0, 540.0]);
}

#[test]
fn studio_event_loop_closes_drag_transaction_on_pointer_release() {
    let mut app = KagariApp {
        show_home: false,
        show_welcome: false,
        ..Default::default()
    };
    app.begin_drag("integration test drag");
    assert!(app.drag_active());

    let ctx = eframe::egui::Context::default();
    let _ = ctx.run(
        eframe::egui::RawInput {
            screen_rect: Some(eframe::egui::Rect::from_min_size(
                eframe::egui::Pos2::ZERO,
                eframe::egui::vec2(1600.0, 900.0),
            )),
            events: vec![eframe::egui::Event::PointerButton {
                pos: eframe::egui::pos2(24.0, 24.0),
                button: eframe::egui::PointerButton::Primary,
                pressed: false,
                modifiers: eframe::egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| {
            crate::ui::theme::configure_ae_theme(ctx);
            app.update_panels(ctx);
        },
    );

    assert!(
        !app.drag_active(),
        "release must close the open transaction"
    );
}

#[test]
fn studio_event_loop_toggles_command_palette_with_keyboard_input() {
    let mut app = KagariApp {
        show_home: false,
        show_welcome: false,
        recovery_checked: true,
        ..Default::default()
    };
    let ctx = eframe::egui::Context::default();
    let screen_rect = eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::vec2(1600.0, 900.0),
    );

    let key_event = || eframe::egui::Event::Key {
        key: eframe::egui::Key::K,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: eframe::egui::Modifiers {
            command: true,
            ..Default::default()
        },
    };
    let modifiers = eframe::egui::Modifiers {
        command: true,
        ..Default::default()
    };

    let _ = ctx.run(
        eframe::egui::RawInput {
            screen_rect: Some(screen_rect),
            modifiers,
            events: vec![key_event()],
            ..Default::default()
        },
        |ctx| {
            crate::ui::theme::configure_ae_theme(ctx);
            app.update_panels(ctx);
        },
    );
    assert!(
        app.show_command_palette,
        "Cmd+K must open the command palette"
    );
    assert!(app.command_palette_search.is_empty());

    let _ = ctx.run(
        eframe::egui::RawInput {
            screen_rect: Some(screen_rect),
            modifiers,
            events: vec![key_event()],
            ..Default::default()
        },
        |ctx| {
            crate::ui::theme::configure_ae_theme(ctx);
            app.update_panels(ctx);
        },
    );
    assert!(
        !app.show_command_palette,
        "Cmd+K must close the command palette"
    );
}

#[test]
fn command_palette_search_and_enter_executes_layer_action() {
    let mut app = KagariApp {
        show_home: false,
        show_welcome: false,
        recovery_checked: true,
        ..Default::default()
    };
    let ctx = eframe::egui::Context::default();
    let initial_layer_count = app.history.current().active_composition().layers.len();
    let screen_rect = eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::vec2(1600.0, 900.0),
    );
    let command_modifiers = eframe::egui::Modifiers {
        command: true,
        ..Default::default()
    };

    let _ = ctx.run(
        eframe::egui::RawInput {
            screen_rect: Some(screen_rect),
            modifiers: command_modifiers,
            events: vec![eframe::egui::Event::Key {
                key: eframe::egui::Key::K,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: command_modifiers,
            }],
            ..Default::default()
        },
        |ctx| {
            crate::ui::theme::configure_ae_theme(ctx);
            app.update_panels(ctx);
        },
    );
    assert!(app.show_command_palette);

    let _ = ctx.run(
        eframe::egui::RawInput {
            screen_rect: Some(screen_rect),
            events: vec![
                eframe::egui::Event::Text("Add Layer: New Solid Layer".into()),
                eframe::egui::Event::Key {
                    key: eframe::egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: eframe::egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ctx| {
            crate::ui::theme::configure_ae_theme(ctx);
            app.update_panels(ctx);
        },
    );

    assert!(
        !app.show_command_palette,
        "Enter must close after executing a command"
    );
    assert_eq!(
        app.history.current().active_composition().layers.len(),
        initial_layer_count + 1,
        "command palette action must mutate the active composition"
    );
}

#[test]
fn responsive_studio_frames_render_without_panic() {
    for (width, height) in [(900.0, 700.0), (640.0, 480.0)] {
        let mut app = KagariApp {
            show_home: false,
            show_welcome: false,
            ..Default::default()
        };
        drive_frames_at_size(&mut app, 2, width, height);
        assert_eq!(
            app.playback.current_frame, 0,
            "playhead must not drift at {width}px"
        );
    }
}

#[test]
fn full_home_frame_renders_without_panic() {
    let mut app = KagariApp {
        show_home: true,
        home_dir: std::env::temp_dir(),
        ..Default::default()
    };
    drive_frames(&mut app, 2);
    assert!(app.show_home, "home must stay up without input");
}

#[test]
fn startup_animation_survives_extreme_sizes_and_irregular_frame_times() {
    let mut app = KagariApp {
        show_home: true,
        home_dir: std::env::temp_dir(),
        ..Default::default()
    };
    let ctx = eframe::egui::Context::default();
    let sizes = [
        (1536.0, 1024.0),
        (900.0, 700.0),
        (640.0, 480.0),
        (320.0, 240.0),
        (1.0, 1.0),
        (2400.0, 1400.0),
    ];

    for frame in 0..40 {
        let (width, height) = sizes[frame % sizes.len()];
        let dt = match frame {
            0 => 0.0,
            1 => 0.25,
            2 => 0.001,
            3 => 0.033,
            _ if frame % 7 == 0 => 0.05,
            _ => 1.0 / 60.0,
        };
        let _ = ctx.run(
            eframe::egui::RawInput {
                screen_rect: Some(eframe::egui::Rect::from_min_size(
                    eframe::egui::Pos2::ZERO,
                    eframe::egui::vec2(width, height),
                )),
                predicted_dt: dt,
                ..Default::default()
            },
            |ctx| {
                crate::ui::theme::configure_ae_theme(ctx);
                app.update_panels(ctx);
            },
        );
        assert!(app.startup_animation.progress.is_finite());
        assert!((0.0..=1.0).contains(&app.startup_animation.progress));
    }

    assert_eq!(app.startup_animation.progress, 1.0);
    assert!(!app.startup_animation.is_animating());
}
