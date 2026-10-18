use clap::Parser;
use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::{Composition, Project};

#[path = "cli/args.rs"]
mod args;
#[path = "cli/mocap.rs"]
mod mocap;
#[path = "cli/qc.rs"]
mod qc;
use args::{Cli, Commands, OutputFormat};
use qc::{cmd_qc, QcArgs};
#[path = "cli/bindings.rs"]
mod bindings;
#[path = "cli/edit.rs"]
mod edit;
#[path = "cli/frame.rs"]
mod frame;
#[path = "cli/info.rs"]
mod info;
#[path = "cli/render.rs"]
mod render;
#[path = "cli/validate.rs"]
mod validate;
use bindings::{cmd_add_sample_binding, cmd_bindings, parse_binding_value, parse_sample_point, parse_string_override};
use edit::{cmd_add_effect, cmd_add_layer, cmd_animate, cmd_init, cmd_mask, cmd_matte, cmd_parent, cmd_transition, cmd_trim};
use frame::{cmd_frame, cmd_lottie, cmd_otio};
use info::{cmd_effects, cmd_info, cmd_production_info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Render {
            project,
            comp,
            output,
            format,
            frames,
            from,
            to,
            width,
            height,
            exposure,
            lut,
            lut_file,
            color_space,
            step,
            threads,
            skip_existing,
            mfr,
            log_format,
            bindings,
            overrides,
            expressions,
        } => {
            render::cmd_render(render::RenderArgs {
                project_path: project,
                comp_ref: comp,
                output,
                format,
                frames,
                from,
                to,
                width,
                height,
                exposure,
                lut,
                lut_file,
                color_space,
                step,
                threads,
                skip_existing,
                mfr,
                log_format,
                bindings,
                overrides,
                expressions,
            })?;
        }
        Commands::Queue {
            project,
            output,
            format,
            threads,
            log_format,
        } => {
            render::cmd_queue(render::QueueArgs {
                project_path: project,
                output,
                format,
                threads,
                log_format,
            })?;
        }
        Commands::Effects { json } => {
            cmd_effects(json);
        }
        Commands::Info { project } => {
            cmd_info(&project)?;
        }
        Commands::ProductionInfo { project, json } => {
            cmd_production_info(&project, json)?;
        }
        Commands::Bindings {
            project,
            values,
            json,
            apply_frame,
            frame,
            comp,
        } => {
            cmd_bindings(&project, &values, json, apply_frame, frame, comp.as_deref())?;
        }
        Commands::AddSampleBinding {
            project,
            source,
            target,
            points,
            input_min,
            input_max,
            output_min,
            output_max,
        } => {
            cmd_add_sample_binding(
                &project,
                &source,
                &target,
                points,
                (input_min, input_max),
                (output_min, output_max),
            )?;
        }
        Commands::Validate { project } => {
            validate::cmd_validate(&project)?;
        }
        Commands::Qc {
            project,
            comp,
            frame,
            from,
            to,
            json,
            output,
            fail_on_warnings,
        } => {
            cmd_qc(QcArgs {
                project_path: project,
                comp_ref: comp,
                frame,
                from,
                to,
                json,
                output,
                fail_on_warnings,
            })?;
        }
        Commands::Script { project, file } => {
            let project_meta = std::fs::metadata(&project).map_err(|e| {
                std::io::Error::new(e.kind(), format!("cannot read project: {}", e))
            })?;
            if project_meta.len() > 50 * 1024 * 1024 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "project file too large ({} MB, limit 50 MB)",
                        project_meta.len() / (1024 * 1024)
                    ),
                )
                .into());
            }
            let json = kagari_vfx::core::project_migration::read_project_json_file(
                std::path::Path::new(&project),
            )
            .map_err(std::io::Error::other)?;
            let mut production =
                kagari_vfx::core::production_document::ProductionDocument::from_json(&json).ok();
            let mut proj = if let Some(document) = production.as_mut() {
                document.project().clone()
            } else {
                load_project(&project)?
            };
            // Tolerate wrapped ProjectFile format too
            let source = kagari_vfx::core::project_migration::read_bounded_text_file(
                std::path::Path::new(&file),
                1024 * 1024,
            )
            .map_err(std::io::Error::other)?;
            let logs = kagari_vfx::automation::run_script(&mut proj, &source)?;
            for l in logs {
                println!("{l}");
            }
            if let Some(document) = production.as_mut() {
                *document.project_mut() = proj;
                document
                    .save_atomic(&project)
                    .map_err(std::io::Error::other)?;
            } else {
                let out = serde_json::to_string_pretty(&proj)?;
                let parent = std::path::Path::new(&project)
                    .parent()
                    .unwrap_or(std::path::Path::new("."));
                let tmp = parent.join(format!(".kagari_save_{}", std::process::id()));
                std::fs::write(&tmp, &out)?;
                std::fs::rename(&tmp, &project)?;
            }
            println!("project saved → {}", project);
        }
        Commands::Frame {
            project,
            frame,
            output,
            width,
            height,
            gpu,
        } => {
            cmd_frame(&project, frame, &output, width, height, gpu)?;
        }
        Commands::Lottie { project, output } => {
            cmd_lottie(&project, &output)?;
        }
        Commands::Otio {
            project,
            comp,
            output,
            import,
        } => {
            cmd_otio(&project, comp.as_deref(), &output, import)?;
        }
        Commands::Mocap {
            frames,
            points,
            output,
            block_radius,
            search_radius,
            min_confidence,
            pose,
            fps,
            max_features,
            feature_spacing,
        } => {
            mocap::cmd_mocap(
                &frames,
                &points,
                &output,
                block_radius,
                search_radius,
                min_confidence,
                pose,
                fps,
                max_features,
                feature_spacing,
            )?;
        }
        Commands::Init {
            project,
            name,
            width,
            height,
            fps,
            duration,
        } => {
            cmd_init(&project, &name, width, height, fps, duration)?;
        }
        Commands::AddLayer {
            project,
            name,
            comp,
            kind,
            content,
            in_frame,
            out_frame,
        } => {
            cmd_add_layer(
                &project,
                &name,
                comp.as_deref(),
                &kind,
                content.as_deref(),
                in_frame,
                out_frame,
            )?;
        }
        Commands::Animate {
            project,
            layer,
            property,
            keyframes,
            ease,
            bezier,
            influence,
            comp,
        } => {
            cmd_animate(
                &project,
                &layer,
                comp.as_deref(),
                &property,
                &keyframes,
                &ease,
                bezier.as_deref(),
                influence,
            )?;
        }
        Commands::AddEffect {
            project,
            layer,
            kind,
            param,
            comp,
        } => {
            cmd_add_effect(
                &project,
                &layer,
                comp.as_deref(),
                &kind,
                param,
            )?;
        }
        Commands::Trim {
            project,
            layer,
            in_frame,
            out_frame,
            comp,
        } => {
            cmd_trim(
                &project,
                &layer,
                comp.as_deref(),
                in_frame,
                out_frame,
            )?;
        }
        Commands::Transition {
            project,
            layer,
            kind,
            duration,
            comp,
        } => {
            cmd_transition(
                &project,
                &layer,
                comp.as_deref(),
                &kind,
                duration,
            )?;
        }
        Commands::Parent {
            project,
            layer,
            parent,
            comp,
        } => {
            cmd_parent(
                &project,
                &layer,
                &parent,
                comp.as_deref(),
            )?;
        }
        Commands::Matte {
            project,
            layer,
            mode,
            source,
            comp,
        } => {
            cmd_matte(
                &project,
                &layer,
                &mode,
                source.as_deref(),
                comp.as_deref(),
            )?;
        }
        Commands::Mask {
            project,
            layer,
            shape,
            coords,
            feather,
            inverted,
            comp,
        } => {
            cmd_mask(
                &project,
                &layer,
                &shape,
                &coords,
                feather,
                inverted,
                comp.as_deref(),
            )?;
        }
        Commands::Beats {
            file,
            fps,
            duration,
            json,
        } => {
            render::cmd_beats(&file, fps, duration, json)?;
        }
        Commands::Preview {
            project,
            comp,
            output,
            scale,
            step,
        } => {
            render::cmd_preview(&project, comp.as_deref(), &output, scale, step)?;
        }
        Commands::Bundle {
            project,
            output,
        } => {
            render::cmd_bundle(&project, &output)?;
        }
        Commands::Remap {
            project,
            layer,
            stretch,
            reverse,
            freeze,
            frame_blending,
            comp,
        } => {
            render::cmd_remap(
                &project,
                &layer,
                stretch,
                reverse,
                freeze,
                frame_blending,
                comp.as_deref(),
            )?;
        }
        Commands::Track {
            project,
            layer,
            tracker_idx,
            pos,
            search_size,
            apply_to,
            apply_pos,
            apply_rot,
            comp,
        } => {
            render::cmd_track(
                &project,
                &layer,
                tracker_idx,
                pos.as_deref(),
                search_size,
                apply_to.as_deref(),
                apply_pos,
                apply_rot,
                comp.as_deref(),
            )?;
        }
        Commands::TextAnim {
            project,
            layer,
            preset,
            duration,
            start_frame,
            shape,
            blur,
            tracking,
            comp,
        } => {
            render::cmd_text_anim(
                &project,
                &layer,
                &preset,
                duration,
                start_frame,
                &shape,
                blur,
                tracking,
                comp.as_deref(),
            )?;
        }
    }

    Ok(())
}

fn load_project(path: &str) -> Result<Project, Box<dyn std::error::Error>> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Failed to stat project file '{}': {}", path, e))?;
    if meta.len() > 100 * 1024 * 1024 {
        return Err(format!(
            "Project file too large ({} MB, limit 100 MB)",
            meta.len() / (1024 * 1024)
        )
        .into());
    }
    let json =
        kagari_vfx::core::project_migration::read_project_json_file(std::path::Path::new(path))
            .map_err(|e| format!("Failed to read project file '{}': {}", path, e))?;
    let project = kagari_vfx::core::production_document::ProductionDocument::from_json(&json)
        .map(|document| document.project().clone())
        .or_else(|_| kagari_vfx::core::project_migration::load_project_migrated(&json))
        .map_err(|e| format!("Failed to parse project JSON: {}", e))?;
    Ok(project)
}

fn find_comp<'a>(
    project: &'a Project,
    comp_ref: Option<&str>,
) -> Result<&'a Composition, Box<dyn std::error::Error>> {
    match comp_ref {
        None => {
            if project.compositions.is_empty() {
                return Err("Project has no compositions".into());
            }
            Ok(project.compositions.first().unwrap())
        }
        Some(name) => {
            if let Ok(idx) = name.parse::<usize>() {
                project.compositions.get(idx).ok_or_else(|| {
                    format!(
                        "Composition index {} out of range (project has {})",
                        idx,
                        project.compositions.len()
                    )
                    .into()
                })
            } else {
                project
                    .compositions
                    .iter()
                    .find(|c| c.name == name)
                    .ok_or_else(|| format!("Composition '{}' not found", name).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_comp() -> Composition {
        Composition::new(
            "qc-contract".to_string(),
            "QC Contract".to_string(),
            4,
            4,
            30,
            1,
        )
    }

    #[test]
    fn qc_report_flags_black_and_soft_frame() {
        let report = qc::build_qc_report(&test_comp(), 0, &[0; 4 * 4 * 4]).unwrap();
        assert!(report.shadow_clipping_percent > 99.0);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("Shadow")));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("soft-focus")));
    }

    #[test]
    fn qc_json_schema_is_versioned() {
        let report = qc::build_qc_report(&test_comp(), 0, &[255; 4 * 4 * 4]).unwrap();
        let batch = qc::QcBatchReport {
            schema_version: 1,
            composition: "QC Contract".to_string(),
            from_frame: 0,
            to_frame: 0,
            analyzed_frames: 1,
            warning_frames: 1,
            frames: vec![report],
        };
        let json = serde_json::to_value(batch).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["frames"].as_array().map(Vec::len), Some(1));
        assert!(json["frames"][0]["warnings"].as_array().is_some());
    }

    #[test]
    fn production_info_json_schema_exposes_cross_domain_contract() {
        let mut document =
            kagari_vfx::core::production_document::ProductionDocument::new(Project::default());
        document.audio.sample_rate = 44_100;
        let value = info::production_info_value(&document);
        assert_eq!(value["schema_version"], document.schema_version);
        assert_eq!(value["sample_rate"], 44_100);
        assert_eq!(value["tempo_changes"], 1);
        assert_eq!(
            value["automation_bindings"].as_array().map(Vec::len),
            Some(0)
        );
    }

    #[test]
    fn render_binding_values_parse_and_reject_non_finite_input() {
        assert_eq!(
            parse_binding_value("audio.bass=0.75").unwrap(),
            ("audio.bass".into(), 0.75)
        );
        assert!(parse_binding_value("audio.bass=NaN").is_err());
        assert!(parse_binding_value(" audio.bass=0.5").is_err());
    }
}
