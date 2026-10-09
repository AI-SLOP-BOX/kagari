use crate::core::mask::MaskVertex;
use super::tokenizer::{expand_repeated_svg_commands, tokenize_svg_path};

/// Parses an SVG path definition string (`d="..."`) into a list of `MaskVertex` points.
pub fn parse_svg_path_data(d: &str) -> Result<Vec<MaskVertex>, String> {
    let mut vertices: Vec<MaskVertex> = Vec::new();
    let mut curr_pos = [0.0f32, 0.0f32];

    let tokens = expand_repeated_svg_commands(tokenize_svg_path(d));

    let mut i = 0;
    let mut last_cubic_control: Option<[f32; 2]> = None;
    let mut last_quadratic_control: Option<[f32; 2]> = None;
    while i < tokens.len() {
        let cmd = &tokens[i];
        match cmd.as_str() {
            "M" | "m" => {
                let is_rel = cmd == "m";
                if i + 2 < tokens.len() {
                    let x: f32 = tokens[i + 1].parse().map_err(|e| format!("M.x: {e}"))?;
                    let y: f32 = tokens[i + 2].parse().map_err(|e| format!("M.y: {e}"))?;
                    curr_pos = if is_rel {
                        [curr_pos[0] + x, curr_pos[1] + y]
                    } else {
                        [x, y]
                    };
                    vertices.push(MaskVertex::new(curr_pos[0], curr_pos[1]));
                    last_cubic_control = None;
                    last_quadratic_control = None;
                    i += 3;
                } else {
                    break;
                }
            }
            "L" | "l" => {
                let is_rel = cmd == "l";
                if i + 2 < tokens.len() {
                    let x: f32 = tokens[i + 1].parse().map_err(|e| format!("L.x: {e}"))?;
                    let y: f32 = tokens[i + 2].parse().map_err(|e| format!("L.y: {e}"))?;
                    curr_pos = if is_rel {
                        [curr_pos[0] + x, curr_pos[1] + y]
                    } else {
                        [x, y]
                    };
                    vertices.push(MaskVertex::new(curr_pos[0], curr_pos[1]));
                    last_cubic_control = None;
                    last_quadratic_control = None;
                    i += 3;
                } else {
                    break;
                }
            }
            "H" | "h" => {
                let is_rel = cmd == "h";
                if i + 1 < tokens.len() {
                    let x: f32 = tokens[i + 1].parse().map_err(|e| format!("H.x: {e}"))?;
                    curr_pos[0] = if is_rel { curr_pos[0] + x } else { x };
                    vertices.push(MaskVertex::new(curr_pos[0], curr_pos[1]));
                    last_cubic_control = None;
                    last_quadratic_control = None;
                    i += 2;
                } else {
                    break;
                }
            }
            "V" | "v" => {
                let is_rel = cmd == "v";
                if i + 1 < tokens.len() {
                    let y: f32 = tokens[i + 1].parse().map_err(|e| format!("V.y: {e}"))?;
                    curr_pos[1] = if is_rel { curr_pos[1] + y } else { y };
                    vertices.push(MaskVertex::new(curr_pos[0], curr_pos[1]));
                    last_cubic_control = None;
                    last_quadratic_control = None;
                    i += 2;
                } else {
                    break;
                }
            }
            "C" | "c" => {
                let is_rel = cmd == "c";
                if i + 6 < tokens.len() {
                    let cp1x: f32 = tokens[i + 1].parse().map_err(|e| format!("C.cp1x: {e}"))?;
                    let cp1y: f32 = tokens[i + 2].parse().map_err(|e| format!("C.cp1y: {e}"))?;
                    let cp2x: f32 = tokens[i + 3].parse().map_err(|e| format!("C.cp2x: {e}"))?;
                    let cp2y: f32 = tokens[i + 4].parse().map_err(|e| format!("C.cp2y: {e}"))?;
                    let end_x: f32 = tokens[i + 5].parse().map_err(|e| format!("C.x: {e}"))?;
                    let end_y: f32 = tokens[i + 6].parse().map_err(|e| format!("C.y: {e}"))?;

                    let p_cp1 = if is_rel {
                        [curr_pos[0] + cp1x, curr_pos[1] + cp1y]
                    } else {
                        [cp1x, cp1y]
                    };
                    let p_cp2 = if is_rel {
                        [curr_pos[0] + cp2x, curr_pos[1] + cp2y]
                    } else {
                        [cp2x, cp2y]
                    };
                    let p_end = if is_rel {
                        [curr_pos[0] + end_x, curr_pos[1] + end_y]
                    } else {
                        [end_x, end_y]
                    };

                    if let Some(prev) = vertices.last_mut() {
                        prev.tangent_out =
                            [p_cp1[0] - prev.position[0], p_cp1[1] - prev.position[1]];
                    }

                    let mut dest_vert = MaskVertex::new(p_end[0], p_end[1]);
                    dest_vert.tangent_in = [p_cp2[0] - p_end[0], p_cp2[1] - p_end[1]];
                    vertices.push(dest_vert);

                    curr_pos = p_end;
                    last_cubic_control = Some(p_cp2);
                    last_quadratic_control = None;
                    i += 7;
                } else {
                    break;
                }
            }
            "S" | "s" => {
                let is_rel = cmd == "s";
                if i + 4 < tokens.len() {
                    let cp2x: f32 = tokens[i + 1].parse().map_err(|e| format!("S.cp2x: {e}"))?;
                    let cp2y: f32 = tokens[i + 2].parse().map_err(|e| format!("S.cp2y: {e}"))?;
                    let end_x: f32 = tokens[i + 3].parse().map_err(|e| format!("S.x: {e}"))?;
                    let end_y: f32 = tokens[i + 4].parse().map_err(|e| format!("S.y: {e}"))?;

                    let p_cp1 = if let Some(last_cp2) = last_cubic_control {
                        [
                            2.0 * curr_pos[0] - last_cp2[0],
                            2.0 * curr_pos[1] - last_cp2[1],
                        ]
                    } else {
                        curr_pos
                    };

                    let p_cp2 = if is_rel {
                        [curr_pos[0] + cp2x, curr_pos[1] + cp2y]
                    } else {
                        [cp2x, cp2y]
                    };
                    let p_end = if is_rel {
                        [curr_pos[0] + end_x, curr_pos[1] + end_y]
                    } else {
                        [end_x, end_y]
                    };

                    if let Some(prev) = vertices.last_mut() {
                        prev.tangent_out =
                            [p_cp1[0] - prev.position[0], p_cp1[1] - prev.position[1]];
                    }

                    let mut dest_vert = MaskVertex::new(p_end[0], p_end[1]);
                    dest_vert.tangent_in = [p_cp2[0] - p_end[0], p_cp2[1] - p_end[1]];
                    vertices.push(dest_vert);

                    curr_pos = p_end;
                    last_cubic_control = Some(p_cp2);
                    last_quadratic_control = None;
                    i += 5;
                } else {
                    break;
                }
            }
            "Q" | "q" => {
                let is_rel = cmd == "q";
                if i + 4 < tokens.len() {
                    let cpx: f32 = tokens[i + 1].parse().map_err(|e| format!("Q.cpx: {e}"))?;
                    let cpy: f32 = tokens[i + 2].parse().map_err(|e| format!("Q.cpy: {e}"))?;
                    let end_x: f32 = tokens[i + 3].parse().map_err(|e| format!("Q.x: {e}"))?;
                    let end_y: f32 = tokens[i + 4].parse().map_err(|e| format!("Q.y: {e}"))?;

                    let cp = if is_rel {
                        [curr_pos[0] + cpx, curr_pos[1] + cpy]
                    } else {
                        [cpx, cpy]
                    };
                    let end = if is_rel {
                        [curr_pos[0] + end_x, curr_pos[1] + end_y]
                    } else {
                        [end_x, end_y]
                    };

                    let cp1 = [
                        curr_pos[0] + 2.0 / 3.0 * (cp[0] - curr_pos[0]),
                        curr_pos[1] + 2.0 / 3.0 * (cp[1] - curr_pos[1]),
                    ];
                    let cp2 = [
                        end[0] + 2.0 / 3.0 * (cp[0] - end[0]),
                        end[1] + 2.0 / 3.0 * (cp[1] - end[1]),
                    ];

                    if let Some(prev) = vertices.last_mut() {
                        prev.tangent_out = [cp1[0] - prev.position[0], cp1[1] - prev.position[1]];
                    }

                    let mut dest_vert = MaskVertex::new(end[0], end[1]);
                    dest_vert.tangent_in = [cp2[0] - end[0], cp2[1] - end[1]];
                    vertices.push(dest_vert);

                    curr_pos = end;
                    last_quadratic_control = Some(cp);
                    last_cubic_control = None;
                    i += 5;
                } else {
                    break;
                }
            }
            "T" | "t" => {
                let is_rel = cmd == "t";
                if i + 2 < tokens.len() {
                    let end_x: f32 = tokens[i + 1].parse().map_err(|e| format!("T.x: {e}"))?;
                    let end_y: f32 = tokens[i + 2].parse().map_err(|e| format!("T.y: {e}"))?;

                    let cp = if let Some(last_cp) = last_quadratic_control {
                        [
                            2.0 * curr_pos[0] - last_cp[0],
                            2.0 * curr_pos[1] - last_cp[1],
                        ]
                    } else {
                        curr_pos
                    };

                    let end = if is_rel {
                        [curr_pos[0] + end_x, curr_pos[1] + end_y]
                    } else {
                        [end_x, end_y]
                    };

                    let cp1 = [
                        curr_pos[0] + 2.0 / 3.0 * (cp[0] - curr_pos[0]),
                        curr_pos[1] + 2.0 / 3.0 * (cp[1] - curr_pos[1]),
                    ];
                    let cp2 = [
                        end[0] + 2.0 / 3.0 * (cp[0] - end[0]),
                        end[1] + 2.0 / 3.0 * (cp[1] - end[1]),
                    ];

                    if let Some(prev) = vertices.last_mut() {
                        prev.tangent_out = [cp1[0] - prev.position[0], cp1[1] - prev.position[1]];
                    }

                    let mut dest_vert = MaskVertex::new(end[0], end[1]);
                    dest_vert.tangent_in = [cp2[0] - end[0], cp2[1] - end[1]];
                    vertices.push(dest_vert);

                    curr_pos = end;
                    last_quadratic_control = Some(cp);
                    last_cubic_control = None;
                    i += 3;
                } else {
                    break;
                }
            }
            "A" | "a" => {
                let is_rel = cmd == "a";
                if i + 7 < tokens.len() {
                    let rx: f32 = tokens[i + 1].parse().map_err(|e| format!("A.rx: {e}"))?;
                    let ry: f32 = tokens[i + 2].parse().map_err(|e| format!("A.ry: {e}"))?;
                    let x_axis_rotation: f32 =
                        tokens[i + 3].parse().map_err(|e| format!("A.rot: {e}"))?;
                    let large_arc_flag = tokens[i + 4] == "1";
                    let sweep_flag = tokens[i + 5] == "1";
                    let end_x: f32 = tokens[i + 6].parse().map_err(|e| format!("A.x: {e}"))?;
                    let end_y: f32 = tokens[i + 7].parse().map_err(|e| format!("A.y: {e}"))?;
                    let end = if is_rel {
                        [curr_pos[0] + end_x, curr_pos[1] + end_y]
                    } else {
                        [end_x, end_y]
                    };

                    append_svg_arc(
                        &mut vertices,
                        curr_pos,
                        end,
                        rx,
                        ry,
                        x_axis_rotation,
                        large_arc_flag,
                        sweep_flag,
                    );
                    curr_pos = end;
                    last_cubic_control = None;
                    last_quadratic_control = None;
                    i += 8;
                } else {
                    break;
                }
            }
            "Z" | "z" => {
                if let Some(first) = vertices.first() {
                    curr_pos = first.position;
                }
                last_cubic_control = None;
                last_quadratic_control = None;
                i += 1;
            }
            _ => {
                i += 1;
            }
        };
    }

    Ok(vertices)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_svg_arc(
    vertices: &mut Vec<MaskVertex>,
    start: [f32; 2],
    end: [f32; 2],
    rx: f32,
    ry: f32,
    rotation_degrees: f32,
    large_arc: bool,
    sweep: bool,
) {
    let rx = rx.abs();
    let ry = ry.abs();
    if !rx.is_finite() || !ry.is_finite() || !rotation_degrees.is_finite() {
        vertices.push(MaskVertex::new(end[0], end[1]));
        return;
    }
    if (start[0] - end[0]).abs() <= f32::EPSILON && (start[1] - end[1]).abs() <= f32::EPSILON {
        return;
    }
    if rx <= f32::EPSILON || ry <= f32::EPSILON {
        vertices.push(MaskVertex::new(end[0], end[1]));
        return;
    }

    let phi = rotation_degrees.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let dx = (start[0] - end[0]) * 0.5;
    let dy = (start[1] - end[1]) * 0.5;
    let x_prime = cos_phi * dx + sin_phi * dy;
    let y_prime = -sin_phi * dx + cos_phi * dy;

    let radii_ratio = x_prime * x_prime / (rx * rx) + y_prime * y_prime / (ry * ry);
    let scale = radii_ratio.sqrt().max(1.0);
    let rx = rx * scale;
    let ry = ry * scale;
    let rx_sq = rx * rx;
    let ry_sq = ry * ry;
    let x_prime_sq = x_prime * x_prime;
    let y_prime_sq = y_prime * y_prime;
    let denominator = rx_sq * y_prime_sq + ry_sq * x_prime_sq;
    let numerator = (rx_sq * ry_sq - rx_sq * y_prime_sq - ry_sq * x_prime_sq).max(0.0);
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let coefficient = if denominator <= f32::EPSILON {
        0.0
    } else {
        sign * (numerator / denominator).sqrt()
    };
    let center_x_prime = coefficient * rx * y_prime / ry;
    let center_y_prime = coefficient * -ry * x_prime / rx;
    let center = [
        cos_phi * center_x_prime - sin_phi * center_y_prime + (start[0] + end[0]) * 0.5,
        sin_phi * center_x_prime + cos_phi * center_y_prime + (start[1] + end[1]) * 0.5,
    ];

    let vector_start = [
        (x_prime - center_x_prime) / rx,
        (y_prime - center_y_prime) / ry,
    ];
    let vector_end = [
        (-x_prime - center_x_prime) / rx,
        (-y_prime - center_y_prime) / ry,
    ];
    let angle_between =
        |a: [f32; 2], b: [f32; 2]| (a[0] * b[1] - a[1] * b[0]).atan2(a[0] * b[0] + a[1] * b[1]);
    let start_angle = angle_between([1.0, 0.0], vector_start);
    let mut sweep_angle = angle_between(vector_start, vector_end);
    if !sweep && sweep_angle > 0.0 {
        sweep_angle -= std::f32::consts::TAU;
    } else if sweep && sweep_angle < 0.0 {
        sweep_angle += std::f32::consts::TAU;
    }
    let segment_count = (sweep_angle.abs() / (std::f32::consts::FRAC_PI_2))
        .ceil()
        .max(1.0) as usize;
    let segment_angle = sweep_angle / segment_count as f32;
    let rotate_point = |angle: f32| {
        let (sin_angle, cos_angle) = angle.sin_cos();
        [
            center[0] + cos_phi * rx * cos_angle - sin_phi * ry * sin_angle,
            center[1] + sin_phi * rx * cos_angle + cos_phi * ry * sin_angle,
        ]
    };
    let derivative = |angle: f32| {
        let (sin_angle, cos_angle) = angle.sin_cos();
        [
            -cos_phi * rx * sin_angle - sin_phi * ry * cos_angle,
            -sin_phi * rx * sin_angle + cos_phi * ry * cos_angle,
        ]
    };

    let mut previous_position = start;
    for segment in 0..segment_count {
        let angle_start = start_angle + segment as f32 * segment_angle;
        let angle_end = angle_start + segment_angle;
        let kappa = (4.0 / 3.0) * (segment_angle * 0.25).tan();
        let control_start = {
            let tangent = derivative(angle_start);
            [
                previous_position[0] + kappa * tangent[0],
                previous_position[1] + kappa * tangent[1],
            ]
        };
        let destination = if segment + 1 == segment_count {
            end
        } else {
            rotate_point(angle_end)
        };
        let tangent_end = derivative(angle_end);
        let control_end = [
            destination[0] - kappa * tangent_end[0],
            destination[1] - kappa * tangent_end[1],
        ];

        if let Some(previous) = vertices.last_mut() {
            previous.tangent_out = [
                control_start[0] - previous.position[0],
                control_start[1] - previous.position[1],
            ];
        }
        let mut destination_vertex = MaskVertex::new(destination[0], destination[1]);
        destination_vertex.tangent_in = [
            control_end[0] - destination[0],
            control_end[1] - destination[1],
        ];
        vertices.push(destination_vertex);
        previous_position = destination;
    }
}
