//! Vector Graphics (SVG / Illustrator) Path & Anchor Point Importer.
//!
//! Parses vector commands (MoveTo, LineTo, Cubic Bezier, ClosePath) into fully animatable
//! AE MaskVertex and MaskPath structures.

#![allow(dead_code)]

use crate::core::mask::{MaskPath, MaskVertex};

/// Imported vector path with style information.
#[derive(Debug, Clone)]
pub struct SvgVectorPath {
    pub name: String,
    pub vertices: Vec<MaskVertex>,
    pub is_closed: bool,
    pub fill_color: Option<[f32; 4]>,
    pub stroke_color: Option<[f32; 4]>,
    pub stroke_width: f32,
}

impl SvgVectorPath {
    /// Converts imported vector path into an animatable AE MaskPath.
    pub fn to_mask_path(&self) -> MaskPath {
        let mut path = MaskPath::new_closed(self.vertices.iter().map(|v| v.position).collect());
        let tangents = self
            .vertices
            .iter()
            .map(|v| (v.tangent_in, v.tangent_out))
            .collect();
        path.tangents = Some(tangents);
        path.is_closed = self.is_closed;
        path
    }
}

/// Parses an SVG path definition string (`d="..."`) into a list of `MaskVertex` points.
pub fn parse_svg_path_data(d: &str) -> Result<Vec<MaskVertex>, String> {
    let mut vertices: Vec<MaskVertex> = Vec::new();
    let mut curr_pos = [0.0f32, 0.0f32];

    // Tokenize command letters and numbers
    let mut tokens = Vec::new();
    let mut curr_token = String::new();

    for c in d.chars() {
        if c.is_alphabetic() {
            if !curr_token.trim().is_empty() {
                tokens.push(curr_token.trim().to_string());
                curr_token.clear();
            }
            tokens.push(c.to_string());
        } else if c.is_whitespace() || c == ',' {
            if !curr_token.trim().is_empty() {
                tokens.push(curr_token.trim().to_string());
                curr_token.clear();
            }
        } else {
            curr_token.push(c);
        }
    }
    if !curr_token.trim().is_empty() {
        tokens.push(curr_token.trim().to_string());
    }

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
                    let x1: f32 = tokens[i + 1].parse().map_err(|e| format!("C.x1: {e}"))?;
                    let y1: f32 = tokens[i + 2].parse().map_err(|e| format!("C.y1: {e}"))?;
                    let x2: f32 = tokens[i + 3].parse().map_err(|e| format!("C.x2: {e}"))?;
                    let y2: f32 = tokens[i + 4].parse().map_err(|e| format!("C.y2: {e}"))?;
                    let x: f32 = tokens[i + 5].parse().map_err(|e| format!("C.x: {e}"))?;
                    let y: f32 = tokens[i + 6].parse().map_err(|e| format!("C.y: {e}"))?;

                    let c0 = if is_rel {
                        [curr_pos[0] + x1, curr_pos[1] + y1]
                    } else {
                        [x1, y1]
                    };
                    let c1 = if is_rel {
                        [curr_pos[0] + x2, curr_pos[1] + y2]
                    } else {
                        [x2, y2]
                    };
                    let dest = if is_rel {
                        [curr_pos[0] + x, curr_pos[1] + y]
                    } else {
                        [x, y]
                    };

                    // Update tangent_out of previous vertex
                    if let Some(prev) = vertices.last_mut() {
                        prev.tangent_out = [c0[0] - prev.position[0], c0[1] - prev.position[1]];
                    }

                    // Create destination vertex with tangent_in
                    let mut dest_vert = MaskVertex::new(dest[0], dest[1]);
                    dest_vert.tangent_in = [c1[0] - dest[0], c1[1] - dest[1]];
                    vertices.push(dest_vert);

                    curr_pos = dest;
                    last_cubic_control = Some(c1);
                    last_quadratic_control = None;
                    i += 7;
                } else {
                    break;
                }
            }
            "S" | "s" => {
                let is_rel = cmd == "s";
                if i + 4 < tokens.len() {
                    let x2: f32 = tokens[i + 1].parse().map_err(|e| format!("S.x2: {e}"))?;
                    let y2: f32 = tokens[i + 2].parse().map_err(|e| format!("S.y2: {e}"))?;
                    let x: f32 = tokens[i + 3].parse().map_err(|e| format!("S.x: {e}"))?;
                    let y: f32 = tokens[i + 4].parse().map_err(|e| format!("S.y: {e}"))?;
                    let c0 = last_cubic_control
                        .map(|previous| {
                            [
                                2.0 * curr_pos[0] - previous[0],
                                2.0 * curr_pos[1] - previous[1],
                            ]
                        })
                        .unwrap_or(curr_pos);
                    let c1 = if is_rel {
                        [curr_pos[0] + x2, curr_pos[1] + y2]
                    } else {
                        [x2, y2]
                    };
                    let dest = if is_rel {
                        [curr_pos[0] + x, curr_pos[1] + y]
                    } else {
                        [x, y]
                    };
                    if let Some(previous) = vertices.last_mut() {
                        previous.tangent_out =
                            [c0[0] - previous.position[0], c0[1] - previous.position[1]];
                    }
                    let mut dest_vertex = MaskVertex::new(dest[0], dest[1]);
                    dest_vertex.tangent_in = [c1[0] - dest[0], c1[1] - dest[1]];
                    vertices.push(dest_vertex);
                    curr_pos = dest;
                    last_cubic_control = Some(c1);
                    last_quadratic_control = None;
                    i += 5;
                } else {
                    break;
                }
            }
            "Q" | "q" => {
                let is_rel = cmd == "q";
                if i + 4 < tokens.len() {
                    let x1: f32 = tokens[i + 1].parse().map_err(|e| format!("Q.x1: {e}"))?;
                    let y1: f32 = tokens[i + 2].parse().map_err(|e| format!("Q.y1: {e}"))?;
                    let x: f32 = tokens[i + 3].parse().map_err(|e| format!("Q.x: {e}"))?;
                    let y: f32 = tokens[i + 4].parse().map_err(|e| format!("Q.y: {e}"))?;
                    let control = if is_rel {
                        [curr_pos[0] + x1, curr_pos[1] + y1]
                    } else {
                        [x1, y1]
                    };
                    let dest = if is_rel {
                        [curr_pos[0] + x, curr_pos[1] + y]
                    } else {
                        [x, y]
                    };
                    let c0 = [
                        curr_pos[0] + (control[0] - curr_pos[0]) * (2.0 / 3.0),
                        curr_pos[1] + (control[1] - curr_pos[1]) * (2.0 / 3.0),
                    ];
                    let c1 = [
                        dest[0] + (control[0] - dest[0]) * (2.0 / 3.0),
                        dest[1] + (control[1] - dest[1]) * (2.0 / 3.0),
                    ];
                    if let Some(previous) = vertices.last_mut() {
                        previous.tangent_out =
                            [c0[0] - previous.position[0], c0[1] - previous.position[1]];
                    }
                    let mut dest_vertex = MaskVertex::new(dest[0], dest[1]);
                    dest_vertex.tangent_in = [c1[0] - dest[0], c1[1] - dest[1]];
                    vertices.push(dest_vertex);
                    curr_pos = dest;
                    last_quadratic_control = Some(control);
                    last_cubic_control = None;
                    i += 5;
                } else {
                    break;
                }
            }
            "T" | "t" => {
                let is_rel = cmd == "t";
                if i + 2 < tokens.len() {
                    let x: f32 = tokens[i + 1].parse().map_err(|e| format!("T.x: {e}"))?;
                    let y: f32 = tokens[i + 2].parse().map_err(|e| format!("T.y: {e}"))?;
                    let control = last_quadratic_control
                        .map(|previous| {
                            [
                                2.0 * curr_pos[0] - previous[0],
                                2.0 * curr_pos[1] - previous[1],
                            ]
                        })
                        .unwrap_or(curr_pos);
                    let dest = if is_rel {
                        [curr_pos[0] + x, curr_pos[1] + y]
                    } else {
                        [x, y]
                    };
                    let c0 = [
                        curr_pos[0] + (control[0] - curr_pos[0]) * (2.0 / 3.0),
                        curr_pos[1] + (control[1] - curr_pos[1]) * (2.0 / 3.0),
                    ];
                    let c1 = [
                        dest[0] + (control[0] - dest[0]) * (2.0 / 3.0),
                        dest[1] + (control[1] - dest[1]) * (2.0 / 3.0),
                    ];
                    if let Some(previous) = vertices.last_mut() {
                        previous.tangent_out =
                            [c0[0] - previous.position[0], c0[1] - previous.position[1]];
                    }
                    let mut dest_vertex = MaskVertex::new(dest[0], dest[1]);
                    dest_vertex.tangent_in = [c1[0] - dest[0], c1[1] - dest[1]];
                    vertices.push(dest_vertex);
                    curr_pos = dest;
                    last_quadratic_control = Some(control);
                    last_cubic_control = None;
                    i += 3;
                } else {
                    break;
                }
            }
            "Z" | "z" => {
                last_cubic_control = None;
                last_quadratic_control = None;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(vertices)
}

/// Extracts all vector shapes from an SVG file string.
pub fn parse_svg_document(svg_text: &str) -> Vec<SvgVectorPath> {
    let mut paths = Vec::new();

    for line in svg_text.lines() {
        if line.contains("<path") {
            if let Some(d_start) = line.find("d=\"") {
                let rest = &line[d_start + 3..];
                if let Some(d_end) = rest.find('"') {
                    let d = &rest[..d_end];
                    if let Ok(verts) = parse_svg_path_data(d) {
                        if !verts.is_empty() {
                            paths.push(SvgVectorPath {
                                name: format!("Path {}", paths.len() + 1),
                                vertices: verts,
                                is_closed: line.contains('Z') || line.contains('z'),
                                fill_color: Some([1.0, 1.0, 1.0, 1.0]),
                                stroke_color: None,
                                stroke_width: 1.0,
                            });
                        }
                    }
                }
            }
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_svg_cubic_bezier_path() {
        let svg_d = "M 10 20 C 30 40 50 60 70 80 Z";
        let verts = parse_svg_path_data(svg_d).expect("SVG path parsing succeeds");
        assert_eq!(verts.len(), 2);
        assert_eq!(verts[0].position, [10.0, 20.0]);
        assert_eq!(verts[0].tangent_out, [20.0, 20.0]); // (30 - 10, 40 - 20)
        assert_eq!(verts[1].position, [70.0, 80.0]);
        assert_eq!(verts[1].tangent_in, [-20.0, -20.0]); // (50 - 70, 60 - 80)
    }

    #[test]
    fn test_parse_svg_line_and_quadratic_commands() {
        let verts = parse_svg_path_data("M 0 0 H 10 V 20 Q 15 25 20 20 T 30 20")
            .expect("common SVG commands should parse");
        assert_eq!(verts.len(), 5);
        assert_eq!(verts[1].position, [10.0, 0.0]);
        assert_eq!(verts[2].position, [10.0, 20.0]);
        assert_eq!(verts[3].position, [20.0, 20.0]);
        assert!((verts[4].tangent_in[0] + 10.0 / 3.0).abs() < 1e-5);
        assert!((verts[4].tangent_in[1] + 10.0 / 3.0).abs() < 1e-5);
        assert_eq!(verts[4].position, [30.0, 20.0]);
    }

    #[test]
    fn test_parse_svg_smooth_cubic_reflects_previous_handle() {
        let verts = parse_svg_path_data("M 0 0 C 10 0 10 10 20 10 S 30 20 40 10")
            .expect("smooth cubic command should parse");
        assert_eq!(verts.len(), 3);
        assert_eq!(verts[1].tangent_out, [10.0, 0.0]);
        assert_eq!(verts[2].tangent_in, [-10.0, 10.0]);
    }
}
