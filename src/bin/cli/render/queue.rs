use super::*;

fn safe_queue_component_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "composition".to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub fn cmd_queue(args: QueueArgs) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(&args.project_path)?;
    let log_json = args.log_format.eq_ignore_ascii_case("json");

    if args.threads > 0 {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global();
    }

    if !log_json {
        eprintln!(
            "Starting Batch Render Queue for project: {} ({} compositions)",
            args.project_path,
            project.compositions.len()
        );
    } else {
        println!(
            "{}",
            serde_json::json!({"event":"queue_start", "project":args.project_path, "total_compositions":project.compositions.len()})
        );
    }

    for (idx, comp) in project.compositions.iter().enumerate() {
        let output_stem = format!("{:04}_{}", idx + 1, safe_queue_component_name(&comp.name));
        let comp_out = match args.format {
            OutputFormat::Mp4 => format!("{}/{output_stem}.mp4", args.output),
            OutputFormat::Gif => format!("{}/{output_stem}.gif", args.output),
            OutputFormat::Png | OutputFormat::Exr => format!("{}/{output_stem}", args.output),
        };

        if !log_json {
            eprintln!(
                "\n[{}/{}] Rendering: {}",
                idx + 1,
                project.compositions.len(),
                comp.name
            );
        } else {
            println!(
                "{}",
                serde_json::json!({"event":"queue_comp_start", "index":idx + 1, "name":comp.name, "output":comp_out})
            );
        }

        cmd_render(RenderArgs {
            project_path: args.project_path.clone(),
            comp_ref: Some(idx.to_string()),
            output: comp_out,
            format: args.format,
            frames: None,
            from: 0,
            to: None,
            width: None,
            height: None,
            exposure: 0.0,
            lut: 0,
            lut_file: None,
            color_space: None,
            step: 1,
            threads: args.threads,
            skip_existing: false,
            mfr: true,
            log_format: args.log_format.clone(),
            bindings: Vec::new(),
            overrides: Vec::new(),
            expressions: Vec::new(),
        })?;
    }

    if !log_json {
        eprintln!("\nAll queue compositions completed successfully!");
    } else {
        println!("{{\"event\":\"queue_completed\"}}");
    }

    Ok(())
}
