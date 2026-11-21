use clap::Parser;
use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::{Composition, Project};

#[path = "cli/args.rs"]
mod args;
#[path = "cli/mocap.rs"]
mod mocap;
#[path = "cli/project.rs"]
mod project;
#[path = "cli/qc.rs"]
mod qc;
use args::{Cli, Commands, OutputFormat, RenderCommands};
pub(crate) use project::{find_comp, load_project};
use qc::{cmd_qc, QcArgs};
#[path = "cli/bindings.rs"]
mod bindings;
#[path = "cli/dispatch.rs"]
mod dispatch;
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
use bindings::{
    cmd_add_sample_binding, cmd_bindings, parse_binding_value, parse_sample_point,
    parse_string_override,
};
use edit::{
    cmd_add_effect, cmd_add_layer, cmd_animate, cmd_init, cmd_mask, cmd_matte, cmd_parent,
    cmd_transition, cmd_trim,
};
use frame::{cmd_frame, cmd_lottie, cmd_otio};
use info::{cmd_effects, cmd_info, cmd_production_info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();
    dispatch::dispatch(cli.command)
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

    #[test]
    fn doctor_and_presets_cli_reports_expose_subsystems_and_bundles() {
        let doc = info::doctor_report_value();
        assert_eq!(doc["subsystems"]["aces_color_management"], true);
        assert_eq!(doc["subsystems"]["deep_compositing"], true);
        assert_eq!(doc["subsystems"]["sfm_camera_tracker"], true);

        let presets = info::presets_report_value();
        assert!(!presets["timeline_presets"].as_array().unwrap().is_empty());
        assert!(!presets["composer_bundles"].as_array().unwrap().is_empty());

        assert!(Cli::try_parse_from(["kagari", "doctor", "--json"]).is_ok());
        assert!(Cli::try_parse_from(["kagari", "presets", "--json"]).is_ok());
    }
}
