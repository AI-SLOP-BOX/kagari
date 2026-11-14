use super::*;

#[test]
fn timecode_respects_project_fps() {
    let mut app = AfterEffectsStudio::new().unwrap();
    assert_eq!(app.project_fps, 30);

    // 00:00:02:12 at 30 fps is 2 * 30 + 12 = 72 frames
    invoke(
        &mut app,
        "time_changed",
        Some(ActionValue::Text("00:00:02:12".to_owned())),
    );
    assert_eq!(app.current_frame, 72);
    assert_eq!(app.current_time, "00:00:02:12");

    // Now set project_fps to 60 and test time_changed
    app.project_fps = 60;
    invoke(
        &mut app,
        "time_changed",
        Some(ActionValue::Text("00:00:02:12".to_owned())),
    );
    // at 60 fps, 2 * 60 + 12 = 132 frames
    assert_eq!(app.current_frame, 132);
    assert_eq!(app.current_time, "00:00:02:12");
}

#[test]
fn duration_format_rolls_over_minutes_and_hours() {
    // Test durations > 60 seconds
    let dur_75_secs = 75u64;
    let formatted = format!(
        "{:02}:{:02}:{:02}",
        dur_75_secs / 3600,
        (dur_75_secs / 60) % 60,
        dur_75_secs % 60
    );
    assert_eq!(formatted, "00:01:15");

    let dur_3665_secs = 3665u64;
    let formatted_h = format!(
        "{:02}:{:02}:{:02}",
        dur_3665_secs / 3600,
        (dur_3665_secs / 60) % 60,
        dur_3665_secs % 60
    );
    assert_eq!(formatted_h, "01:01:05");
}

#[test]
fn playback_actions_work() {
    let mut app = AfterEffectsStudio::new().unwrap();
    assert!(!app.is_playing);
    invoke(&mut app, "play_start", None);
    assert!(app.is_playing);

    invoke(&mut app, "play_stop", None);
    assert!(!app.is_playing);

    invoke(&mut app, "play_rewind", None);
    assert_eq!(app.current_frame, 0);
    assert_eq!(app.current_time, "00:00:00:00");

    invoke(&mut app, "play_forward", None);
    assert_eq!(app.current_frame, 299);
    assert_eq!(app.current_time, format_timecode(299, app.project_fps));
}

#[test]
fn core_ame_studio_actions_coverage() {
    // 1. Scan src/ui/ame for all actions declared in .ame files
    let ame_dir = std::path::Path::new("src/ui/ame");
    if !ame_dir.exists() {
        return;
    }

    let mut declared_actions = std::collections::BTreeSet::new();

    fn extract_action(line: &str) -> Option<String> {
        let idx = line.find("action:")?;
        let rest = line[idx + 7..].trim_start();
        let end = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        let action = &rest[..end];
        if !action.is_empty() {
            Some(action.to_string())
        } else {
            None
        }
    }

    fn visit_dirs(dir: &std::path::Path, acc: &mut std::collections::BTreeSet<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, acc);
                } else if path.extension().and_then(|s| s.to_str()) == Some("ame") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        for line in content.lines() {
                            let mut curr = line;
                            while let Some(action) = extract_action(curr) {
                                acc.insert(action);
                                if let Some(pos) = curr.find("action:") {
                                    curr = &curr[pos + 7..];
                                    if let Some(next_pos) =
                                        curr.find(|c: char| !c.is_alphanumeric() && c != '_')
                                    {
                                        curr = &curr[next_pos..];
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    visit_dirs(ame_dir, &mut declared_actions);
    assert!(
        !declared_actions.is_empty(),
        "Expected to find actions declared in .ame files"
    );

    // 2. Scan all split event-handler modules. Keeping a short hard-coded list
    // here silently stopped checking handlers when the Studio root was split.
    let mut handled_actions = std::collections::BTreeSet::new();
    fn visit_handler_dirs(dir: &std::path::Path, acc: &mut std::collections::BTreeSet<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit_handler_dirs(&path, acc);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let is_handler = file_name.starts_with("event_")
                || matches!(
                    file_name,
                    "preferences.rs" | "composition_action_ops.rs" | "playback_ops.rs"
                );
            if !is_handler {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(path) else {
                continue;
            };
            for line in content.lines() {
                let mut curr = line;
                while let Some(start) = curr.find("(\"") {
                    let rest = &curr[start + 2..];
                    if let Some(end) = rest.find('"') {
                        let action = &rest[..end];
                        if !action.is_empty() && rest[end + 1..].trim_start().starts_with(',') {
                            acc.insert(action.to_string());
                        }
                        curr = &rest[end + 1..];
                    } else {
                        break;
                    }
                }
            }
        }
    }
    visit_handler_dirs(
        std::path::Path::new("src/bin/ame_studio"),
        &mut handled_actions,
    );

    let mut unhandled = Vec::new();
    for action in &declared_actions {
        if !handled_actions.contains(action) {
            unhandled.push(action.clone());
        }
    }

    assert!(
        unhandled.is_empty(),
        "Found {} unhandled .ame actions in Rust handlers: {:?}",
        unhandled.len(),
        unhandled
    );
}

#[test]
fn particle_emitter_extended_controls_update_state() {
    let mut app = AfterEffectsStudio::new().unwrap();
    app.add_particle_layer();
    let selected_id = app
        .selected_layer_id
        .clone()
        .expect("particle layer should be selected");

    // Test updating extended properties via update_selected_emitter
    app.update_selected_emitter("spread", "120.0");
    app.update_selected_emitter("gravity_y", "980.0");
    app.update_selected_emitter("wind_x", "150.0");
    app.update_selected_emitter("turbulence", "45.0");
    app.update_selected_emitter("drag", "0.25");

    let comp = app.project_model.active_composition();
    let layer = comp
        .layers
        .iter()
        .find(|l| l.id == selected_id)
        .expect("layer exists");
    if let kagari_vfx::core::timeline::LayerType::Particle { emitter } = &layer.layer_type {
        assert_eq!(emitter.spread_degrees, 120.0);
        assert_eq!(emitter.gravity[1], 980.0);
        assert_eq!(emitter.wind[0], 150.0);
        assert_eq!(emitter.turbulence, 45.0);
        assert_eq!(emitter.drag, 0.25);
    } else {
        panic!("Expected particle layer");
    }
}
