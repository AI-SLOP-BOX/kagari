use crate::core::timeline::EffectType;
use eframe::egui;

use super::common::draw_prop;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::AudioSpectrum {
            enabled: _,
            bands,
            opacity,
            color_start: _,
            color_end: _,
            position_x,
            position_y,
            width: spec_w,
            height: spec_h,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bands",
                bands,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=5.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Opacity",
                opacity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Position X",
                position_x,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Position Y",
                position_y,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Width",
                spec_w,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.01..=1.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Height",
                spec_h,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.01..=1.0, "");
                },
            );
        }
        EffectType::BassTreble {
            bass_gain,
            treble_gain,
            crossover_freq,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bass (dB)",
                bass_gain,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -24.0..=24.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Treble (dB)",
                treble_gain,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -24.0..=24.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Crossover (Hz)",
                crossover_freq,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 80.0..=5000.0, "");
                },
            );
        }
        EffectType::Flanger {
            max_delay_ms,
            lfo_rate,
            feedback,
            wet_dry,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Max Delay (ms)",
                max_delay_ms,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=10.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "LFO Rate (Hz)",
                lfo_rate,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.1..=10.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Feedback",
                feedback,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=0.95, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wet/Dry",
                wet_dry,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
        }
        EffectType::Chorus {
            delay_ms,
            depth_ms,
            rate_hz,
            voices,
            feedback,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Delay (ms)",
                delay_ms,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 1.0..=30.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Depth (ms)",
                depth_ms,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.5..=10.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Rate (Hz)",
                rate_hz,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.1..=6.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Voices",
                voices,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 2.0..=8.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Feedback",
                feedback,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=0.9, "");
                },
            );
        }
        EffectType::ParametricEQ {
            freq_hz,
            gain_db,
            q_factor,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Frequency (Hz)",
                freq_hz,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 60.0..=18000.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Gain (dB)",
                gain_db,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -24.0..=24.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Q Factor",
                q_factor,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.5..=12.0, "");
                },
            );
        }
        EffectType::Echo {
            echo_time_seconds,
            num_echoes,
            starting_intensity,
            decay,
            operator,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Echo Time (s)",
                echo_time_seconds,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, -2.0..=2.0, " s");
                },
            );
            ui.horizontal(|ui| {
                if crate::ui::kui::property(ui, "Number of Echoes", |ui| {
                    crate::ui::kui::slider_integer(ui, num_echoes, 1..=30)
                })
                .changed()
                {
                    *project_changed = true;
                }
            });
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Starting Intensity",
                starting_intensity,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=2.0, "");
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Decay",
                decay,
                |ui, v| {
                    crate::ui::kui::slider(ui, v, 0.0..=1.0, "");
                },
            );
            let operators = [
                (crate::core::echo_effect::EchoOperator::Add, "Add"),
                (crate::core::echo_effect::EchoOperator::Screen, "Screen"),
                (crate::core::echo_effect::EchoOperator::Maximum, "Maximum"),
                (crate::core::echo_effect::EchoOperator::Minimum, "Minimum"),
                (
                    crate::core::echo_effect::EchoOperator::CompositeInBack,
                    "Composite In Back",
                ),
                (
                    crate::core::echo_effect::EchoOperator::CompositeInFront,
                    "Composite In Front",
                ),
                (crate::core::echo_effect::EchoOperator::Blend, "Blend"),
            ];
            let grid = crate::ui::kui::PropertyGrid::new(ui, &["Echo Operator"]);
            *project_changed |= grid.row(ui, "Echo Operator", |ui| {
                crate::ui::kui::combo(ui, "echo_operator", operator, &operators)
            });
        }
        _ => {}
    }
}
