pub(super) fn parse_binding_value(value: &str) -> Result<(String, f64), String> {
    let (source, raw) = value
        .split_once('=')
        .ok_or("binding value must use source=value")?;
    if source.trim().is_empty() || source != source.trim() {
        return Err("binding source must be non-empty and trimmed".into());
    }
    let number = raw
        .parse::<f64>()
        .map_err(|_| "binding value is not a number")?;
    if !number.is_finite() {
        return Err("binding value must be finite".into());
    }
    Ok((source.to_string(), number))
}

pub(super) fn parse_string_override(value: &str) -> Result<(String, String), String> {
    let (target, raw) = value
        .split_once('=')
        .ok_or("override must use target=value (e.g. Title=Hello or Logo=img.png)")?;
    let target = target.trim();
    if target.is_empty() {
        return Err("override target layer/property name cannot be empty".into());
    }
    Ok((target.to_string(), raw.to_string()))
}

pub(super) fn parse_sample_point(value: &str) -> Result<(i64, f64), String> {
    let (sample, raw) = value
        .split_once('=')
        .ok_or("sample point must use sample=value")?;
    let sample = sample
        .parse::<i64>()
        .map_err(|_| "sample position is not an integer")?;
    let value = raw
        .parse::<f64>()
        .map_err(|_| "sample point value is not a number")?;
    if !value.is_finite() {
        return Err("sample point value must be finite".into());
    }
    Ok((sample, value))
}

pub(super) fn cmd_add_sample_binding(
    project_path: &str,
    source: &str,
    target: &str,
    points: Vec<(i64, f64)>,
    input_range: (f64, f64),
    output_range: (f64, f64),
) -> Result<(), Box<dyn std::error::Error>> {
    let json = kagari_vfx::core::project_migration::read_project_json_file(std::path::Path::new(
        project_path,
    ))
    .map_err(std::io::Error::other)?;
    let mut document = kagari_vfx::core::production_document::ProductionDocument::from_json(&json)
        .map_err(|error| format!("Not a valid production document: {error}"))?;
    document
        .add_sample_automation_binding(source, target, points, input_range, output_range)
        .map_err(std::io::Error::other)?;
    document
        .save_atomic(project_path)
        .map_err(std::io::Error::other)?;
    println!("sample automation binding added: {source} -> {target}");
    Ok(())
}

pub(super) fn cmd_bindings(
    project_path: &str,
    values: &[(String, f64)],
    json_output: bool,
    apply_frame: Option<u32>,
    frame: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = kagari_vfx::core::project_migration::read_project_json_file(std::path::Path::new(
        project_path,
    ))
    .map_err(std::io::Error::other)?;
    let mut document = kagari_vfx::core::production_document::ProductionDocument::from_json(&json)
        .map_err(|error| format!("Not a valid production document: {error}"))?;
    let sources = values
        .iter()
        .cloned()
        .collect::<std::collections::HashMap<_, _>>();
    let mut targets = document.evaluate_bindings(&sources);
    if let Some(frame) = frame {
        let composition = if let Some(comp_ref) = comp_ref {
            if let Ok(index) = comp_ref.parse::<usize>() {
                document.project().compositions.get(index)
            } else {
                document
                    .project()
                    .compositions
                    .iter()
                    .find(|composition| composition.name == comp_ref)
            }
        } else {
            document.project().compositions.first()
        }
        .ok_or("production document has no matching composition")?;
        let rate = kagari_vfx::core::unified_time::FrameRate::new(composition.fps.max(1), 1)
            .ok_or("composition has an invalid frame rate")?;
        let time = kagari_vfx::core::unified_time::Time::from_frame(frame as i64, rate);
        targets.extend(document.evaluate_bindings_at_time(time));
    }
    if let Some(frame) = apply_frame {
        let applied = document.apply_binding_targets_at_frame(&targets, frame);
        document
            .save_atomic(project_path)
            .map_err(std::io::Error::other)?;
        eprintln!("applied {applied} binding target(s) at frame {frame}");
    }
    if json_output {
        println!("{}", serde_json::to_string_pretty(&targets)?);
    } else {
        for (target, value) in targets {
            println!("{target}={value}");
        }
    }
    Ok(())
}
