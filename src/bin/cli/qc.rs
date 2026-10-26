use super::*;

#[derive(Debug, serde::Serialize)]
pub(super) struct QcReport {
    pub(super) composition: String,
    pub(super) frame: u32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) shadow_clipping_percent: f32,
    pub(super) highlight_clipping_percent: f32,
    pub(super) sharpness_score: f32,
    pub(super) warnings: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub(super) struct QcBatchReport {
    pub(super) schema_version: u32,
    pub(super) composition: String,
    pub(super) from_frame: u32,
    pub(super) to_frame: u32,
    pub(super) analyzed_frames: usize,
    pub(super) warning_frames: usize,
    pub(super) frames: Vec<QcReport>,
}

pub(super) struct QcArgs {
    pub(super) project_path: String,
    pub(super) comp_ref: Option<String>,
    pub(super) frame: Option<u32>,
    pub(super) from: u32,
    pub(super) to: Option<u32>,
    pub(super) json: bool,
    pub(super) output: Option<String>,
    pub(super) fail_on_warnings: bool,
}

pub(super) fn cmd_qc(args: QcArgs) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(&args.project_path)?;
    let comp = find_comp(&project, args.comp_ref.as_deref())?;
    if comp.duration_frames == 0 {
        return Err(format!("Composition '{}' has no frames", comp.name).into());
    }
    let (from, to) = match args.frame {
        Some(frame) => (frame, frame),
        None => (args.from, args.to.unwrap_or(comp.duration_frames - 1)),
    };
    if from > to {
        return Err(format!("Invalid QC range: from {from} is after to {to}").into());
    }
    if to >= comp.duration_frames {
        return Err(format!(
            "Frame {} is outside composition '{}' (0..{})",
            to,
            comp.name,
            comp.duration_frames.saturating_sub(1)
        )
        .into());
    }
    let frame_count = to.saturating_sub(from) as usize + 1;
    if frame_count > 1_000_000 {
        return Err("QC range exceeds the 1,000,000 frame safety limit".into());
    }
    let mut reports = Vec::with_capacity(frame_count);
    for current in from..=to {
        let pixels = render_frame_to_pixels(comp, current, comp.width, comp.height, 0.0, 0);
        reports.push(build_qc_report(comp, current, &pixels)?);
        if frame_count > 1 && (reports.len() % 10 == 0 || current == to) {
            eprint!("\rQC analyzed {}/{} frames", reports.len(), frame_count);
        }
    }
    if frame_count > 1 {
        eprintln!();
    }
    let warning_frames = reports
        .iter()
        .filter(|report| !report.warnings.is_empty())
        .count();
    let batch = QcBatchReport {
        schema_version: 1,
        composition: comp.name.clone(),
        from_frame: from,
        to_frame: to,
        analyzed_frames: reports.len(),
        warning_frames,
        frames: reports,
    };
    let rendered = if args.json {
        serde_json::to_string_pretty(&batch)?
    } else {
        format_qc_batch_report(&batch)
    };
    if let Some(path) = args.output.as_deref() {
        std::fs::write(path, format!("{rendered}\n"))?;
        eprintln!("QC report saved → {path}");
    } else {
        println!("{rendered}");
    }
    if args.fail_on_warnings && warning_frames > 0 {
        return Err(format!("QC found warnings in {warning_frames} frame(s)").into());
    }
    Ok(())
}

pub(super) fn build_qc_report(
    comp: &Composition,
    frame: u32,
    pixels: &[u8],
) -> Result<QcReport, Box<dyn std::error::Error>> {
    use kagari_vfx::core::editor_assist::{analyze_exposure_clipping, sharpness_score};

    let exposure = analyze_exposure_clipping(pixels, comp.width, comp.height, 4, 250)
        .ok_or("Rendered frame has an invalid pixel buffer")?;
    let sharpness = sharpness_score(pixels, comp.width, comp.height).unwrap_or(0.0);
    let shadow_percent = exposure.shadow_clipped_fraction * 100.0;
    let highlight_percent = exposure.highlight_clipped_fraction * 100.0;
    let mut warnings = Vec::new();
    if shadow_percent > 2.0 {
        warnings.push(format!("Shadow clipping is high ({shadow_percent:.2}%)"));
    }
    if highlight_percent > 2.0 {
        warnings.push(format!(
            "Highlight clipping is high ({highlight_percent:.2}%)"
        ));
    }
    if sharpness < 20.0 {
        warnings.push(format!(
            "Possible soft-focus frame (sharpness {sharpness:.1})"
        ));
    }
    Ok(QcReport {
        composition: comp.name.clone(),
        frame,
        width: comp.width,
        height: comp.height,
        shadow_clipping_percent: shadow_percent,
        highlight_clipping_percent: highlight_percent,
        sharpness_score: sharpness,
        warnings,
    })
}

fn format_qc_report(report: &QcReport) -> String {
    let status = if report.warnings.is_empty() {
        "PASS"
    } else {
        "WARN"
    };
    let mut lines = vec![
        format!(
            "QC {status} — {} frame {}",
            report.composition, report.frame
        ),
        format!("Size: {}x{}", report.width, report.height),
        format!("Shadow clipping: {:.2}%", report.shadow_clipping_percent),
        format!(
            "Highlight clipping: {:.2}%",
            report.highlight_clipping_percent
        ),
        format!("Sharpness: {:.1}", report.sharpness_score),
    ];
    lines.extend(report.warnings.iter().map(|warning| format!("⚠ {warning}")));
    lines.join("\n")
}

fn format_qc_batch_report(batch: &QcBatchReport) -> String {
    let status = if batch.warning_frames == 0 {
        "PASS"
    } else {
        "WARN"
    };
    let mut lines = vec![format!(
        "QC {status} — {} frames {}..{} ({} analyzed, {} with warnings)",
        batch.composition,
        batch.from_frame,
        batch.to_frame,
        batch.analyzed_frames,
        batch.warning_frames
    )];
    for report in &batch.frames {
        lines.push(String::new());
        lines.push(format_qc_report(report));
    }
    lines.join("\n")
}
