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

    let tokens = tokenize_svg_path(d);

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

fn tokenize_svg_path(source: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut number = String::new();
    let flush_number = |tokens: &mut Vec<String>, number: &mut String| {
        if !number.is_empty() {
            tokens.push(std::mem::take(number));
        }
    };
    for character in source.chars() {
        if is_svg_path_command(character) {
            flush_number(&mut tokens, &mut number);
            tokens.push(character.to_string());
        } else if character.is_ascii_whitespace() || character == ',' {
            flush_number(&mut tokens, &mut number);
        } else if matches!(character, '+' | '-')
            && !number.is_empty()
            && !number.ends_with('e')
            && !number.ends_with('E')
        {
            // SVG permits `10-20` without a comma or whitespace.
            flush_number(&mut tokens, &mut number);
            number.push(character);
        } else if character == '.'
            && number.contains('.')
            && !number.contains('e')
            && !number.contains('E')
        {
            // The grammar permits `1.2.3` as two adjacent numbers.
            flush_number(&mut tokens, &mut number);
            number.push(character);
        } else {
            number.push(character);
        }
    }
    flush_number(&mut tokens, &mut number);
    tokens
}

fn is_svg_path_command(character: char) -> bool {
    matches!(
        character,
        'M' | 'm'
            | 'L'
            | 'l'
            | 'H'
            | 'h'
            | 'V'
            | 'v'
            | 'C'
            | 'c'
            | 'S'
            | 's'
            | 'Q'
            | 'q'
            | 'T'
            | 't'
            | 'Z'
            | 'z'
    )
}

/// Extracts all vector shapes from an SVG file string.
pub fn parse_svg_document(svg_text: &str) -> Vec<SvgVectorPath> {
    let mut paths = Vec::new();

    // SVG files generated by Illustrator commonly put a path on several
    // lines and put its transform/style on an ancestor <g>. A line-based
    // parser silently loses that information, which makes an imported logo
    // land at the wrong position or become invisible. Scan tags while keeping
    // the inherited affine transform and style instead.
    let mut transform_stack = vec![SvgTransform::identity()];
    let mut style_stack = vec![SvgStyle::default()];
    let mut cursor = 0;
    while let Some(relative_start) = svg_text[cursor..].find('<') {
        let start = cursor + relative_start;
        let Some(relative_end) = find_svg_tag_end(&svg_text[start..]) else {
            break;
        };
        let end = start + relative_end;
        let tag = svg_text[start + 1..end].trim();
        cursor = end + 1;

        if tag.is_empty() || tag.starts_with('!') || tag.starts_with('?') {
            continue;
        }
        if tag.starts_with('/') {
            let name = tag[1..]
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if matches!(name.as_str(), "svg" | "g" | "defs" | "symbol" | "a")
                && transform_stack.len() > 1
            {
                transform_stack.pop();
                style_stack.pop();
            }
            continue;
        }

        let self_closing = tag.ends_with('/');
        let tag = tag.trim_end_matches('/').trim();
        let mut tag_parts = tag.splitn(2, char::is_whitespace);
        let element = tag_parts.next().unwrap_or_default().to_ascii_lowercase();
        let attrs = parse_svg_attributes(tag_parts.next().unwrap_or_default());
        let parent_transform = transform_stack
            .last()
            .copied()
            .unwrap_or_else(SvgTransform::identity);
        let own_transform = attrs
            .get("transform")
            .map(|value| parse_svg_transform(value))
            .unwrap_or_else(SvgTransform::identity);
        let transform = parent_transform.concat(own_transform);
        let parent_style = style_stack.last().copied().unwrap_or_default();
        let style = parent_style.with_attributes(&attrs);

        if element == "path" {
            if let Some(d) = attrs.get("d") {
                if let Ok(mut vertices) = parse_svg_path_data(d) {
                    if !vertices.is_empty() {
                        for vertex in &mut vertices {
                            vertex.position = transform.apply_point(vertex.position);
                            vertex.tangent_in = transform.apply_vector(vertex.tangent_in);
                            vertex.tangent_out = transform.apply_vector(vertex.tangent_out);
                        }
                        let name = attrs
                            .get("id")
                            .or_else(|| attrs.get("inkscape:label"))
                            .cloned()
                            .unwrap_or_else(|| format!("Path {}", paths.len() + 1));
                        paths.push(SvgVectorPath {
                            name,
                            is_closed: d.chars().any(|command| command == 'Z' || command == 'z'),
                            fill_color: style.fill,
                            stroke_color: style.stroke,
                            stroke_width: style.stroke_width,
                            vertices,
                        });
                    }
                }
            }
        }

        if matches!(element.as_str(), "svg" | "g" | "defs" | "symbol" | "a") && !self_closing {
            transform_stack.push(transform);
            style_stack.push(style);
        }
    }

    paths
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SvgTransform {
    // SVG affine matrix: x' = a*x + c*y + e, y' = b*x + d*y + f.
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl SvgTransform {
    const fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    fn concat(self, next: Self) -> Self {
        Self {
            a: self.a * next.a + self.c * next.b,
            b: self.b * next.a + self.d * next.b,
            c: self.a * next.c + self.c * next.d,
            d: self.b * next.c + self.d * next.d,
            e: self.a * next.e + self.c * next.f + self.e,
            f: self.b * next.e + self.d * next.f + self.f,
        }
    }

    fn apply_point(self, point: [f32; 2]) -> [f32; 2] {
        [
            self.a * point[0] + self.c * point[1] + self.e,
            self.b * point[0] + self.d * point[1] + self.f,
        ]
    }

    fn apply_vector(self, vector: [f32; 2]) -> [f32; 2] {
        [
            self.a * vector[0] + self.c * vector[1],
            self.b * vector[0] + self.d * vector[1],
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SvgStyle {
    fill: Option<[f32; 4]>,
    stroke: Option<[f32; 4]>,
    stroke_width: f32,
}

impl Default for SvgStyle {
    fn default() -> Self {
        Self {
            fill: Some([1.0, 1.0, 1.0, 1.0]),
            stroke: None,
            stroke_width: 1.0,
        }
    }
}

impl SvgStyle {
    fn with_attributes(self, attrs: &std::collections::HashMap<String, String>) -> Self {
        let mut style = self;
        if let Some(value) = attrs.get("style") {
            for declaration in value.split(';') {
                let Some((key, value)) = declaration.split_once(':') else {
                    continue;
                };
                style.apply(key.trim(), value.trim());
            }
        }
        for key in ["fill", "stroke", "stroke-width"] {
            if let Some(value) = attrs.get(key) {
                style.apply(key, value);
            }
        }
        style
    }

    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "fill" => self.fill = parse_svg_color(value),
            "stroke" => self.stroke = parse_svg_color(value),
            "stroke-width" => {
                if let Ok(width) = value.trim_end_matches("px").parse::<f32>() {
                    if width.is_finite() {
                        self.stroke_width = width.max(0.0);
                    }
                }
            }
            _ => {}
        }
    }
}

fn find_svg_tag_end(tag: &str) -> Option<usize> {
    let mut quote = None;
    for (index, character) in tag.char_indices() {
        match (quote, character) {
            (None, '\'' | '"') => quote = Some(character),
            (Some(open), character) if open == character => quote = None,
            (None, '>') => return Some(index),
            _ => {}
        }
    }
    None
}

fn parse_svg_attributes(source: &str) -> std::collections::HashMap<String, String> {
    let mut attrs = std::collections::HashMap::new();
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let key_start = index;
        while index < bytes.len() && !bytes[index].is_ascii_whitespace() && bytes[index] != b'=' {
            index += 1;
        }
        if key_start == index {
            index += 1;
            continue;
        }
        let key = source[key_start..index].to_ascii_lowercase();
        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b'=') {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let quote = matches!(bytes[index], b'\'' | b'"').then(|| bytes[index]);
        if quote.is_some() {
            index += 1;
        }
        let value_start = index;
        if let Some(quote) = quote {
            while index < bytes.len() && bytes[index] != quote {
                index += 1;
            }
        } else {
            while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
                index += 1;
            }
        }
        attrs.insert(key, source[value_start..index].to_string());
        if quote.is_some() && index < bytes.len() {
            index += 1;
        }
    }
    attrs
}

fn parse_svg_numbers(source: &str) -> Vec<f32> {
    source
        .split(|character: char| character == ',' || character.is_ascii_whitespace())
        .filter_map(|value| {
            value
                .parse::<f32>()
                .ok()
                .filter(|number| number.is_finite())
        })
        .collect()
}

fn parse_svg_transform(source: &str) -> SvgTransform {
    let mut result = SvgTransform::identity();
    let mut cursor = 0;
    while cursor < source.len() {
        while cursor < source.len()
            && (source.as_bytes()[cursor].is_ascii_whitespace()
                || source.as_bytes()[cursor] == b',')
        {
            cursor += 1;
        }
        let name_start = cursor;
        while cursor < source.len() && source.as_bytes()[cursor].is_ascii_alphabetic() {
            cursor += 1;
        }
        let name = source[name_start..cursor].to_ascii_lowercase();
        while cursor < source.len() && source.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= source.len() || source.as_bytes()[cursor] != b'(' {
            cursor += 1;
            continue;
        }
        let args_start = cursor + 1;
        let Some(relative_end) = source[args_start..].find(')') else {
            break;
        };
        let args = parse_svg_numbers(&source[args_start..args_start + relative_end]);
        cursor = args_start + relative_end + 1;
        let transform = match name.as_str() {
            "translate" if !args.is_empty() => SvgTransform {
                e: args[0],
                f: args.get(1).copied().unwrap_or(0.0),
                ..SvgTransform::identity()
            },
            "scale" if !args.is_empty() => {
                let sy = args.get(1).copied().unwrap_or(args[0]);
                SvgTransform {
                    a: args[0],
                    d: sy,
                    ..SvgTransform::identity()
                }
            }
            "rotate" if !args.is_empty() => {
                let angle = args[0].to_radians();
                let rotation = SvgTransform {
                    a: angle.cos(),
                    b: angle.sin(),
                    c: -angle.sin(),
                    d: angle.cos(),
                    ..SvgTransform::identity()
                };
                if args.len() >= 3 {
                    SvgTransform {
                        e: args[1],
                        f: args[2],
                        ..SvgTransform::identity()
                    }
                    .concat(rotation)
                    .concat(SvgTransform {
                        e: -args[1],
                        f: -args[2],
                        ..SvgTransform::identity()
                    })
                } else {
                    rotation
                }
            }
            "skewx" if !args.is_empty() => SvgTransform {
                c: args[0].to_radians().tan(),
                ..SvgTransform::identity()
            },
            "skewy" if !args.is_empty() => SvgTransform {
                b: args[0].to_radians().tan(),
                ..SvgTransform::identity()
            },
            "matrix" if args.len() >= 6 => SvgTransform {
                a: args[0],
                b: args[1],
                c: args[2],
                d: args[3],
                e: args[4],
                f: args[5],
            },
            _ => SvgTransform::identity(),
        };
        result = result.concat(transform);
    }
    result
}

fn parse_svg_color(value: &str) -> Option<[f32; 4]> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return None;
    }
    let named = match value.to_ascii_lowercase().as_str() {
        "black" => Some([0, 0, 0]),
        "white" => Some([255, 255, 255]),
        "red" => Some([255, 0, 0]),
        "green" => Some([0, 128, 0]),
        "blue" => Some([0, 0, 255]),
        "yellow" => Some([255, 255, 0]),
        _ => None,
    };
    let rgb = if let Some(rgb) = named {
        rgb
    } else if let Some(hex) = value.strip_prefix('#') {
        let rgb = match hex.len() {
            3 => [
                u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?,
                u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?,
                u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?,
            ],
            6 => [
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
            ],
            _ => return None,
        };
        rgb
    } else {
        return None;
    };
    Some([
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        1.0,
    ])
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
    fn test_parse_svg_path_accepts_signed_numbers_without_separators() {
        let verts = parse_svg_path_data("M10-20 L30-40").expect("signed SVG numbers should parse");
        assert_eq!(verts.len(), 2);
        assert_eq!(verts[0].position, [10.0, -20.0]);
        assert_eq!(verts[1].position, [30.0, -40.0]);
    }

    #[test]
    fn test_parse_svg_smooth_cubic_reflects_previous_handle() {
        let verts = parse_svg_path_data("M 0 0 C 10 0 10 10 20 10 S 30 20 40 10")
            .expect("smooth cubic command should parse");
        assert_eq!(verts.len(), 3);
        assert_eq!(verts[1].tangent_out, [10.0, 0.0]);
        assert_eq!(verts[2].tangent_in, [-10.0, 10.0]);
    }

    #[test]
    fn test_parse_svg_document_inherits_group_transform_and_style() {
        let svg = r##"
            <svg viewBox="0 0 100 100">
              <g transform="translate(10 20) scale(2)" fill="#ff0000" stroke="#00ff00" stroke-width="3">
                <path id="hero" d="M 1 2 C 2 3 4 5 6 7 Z" />
              </g>
            </svg>
        "##;
        let paths = parse_svg_document(svg);
        assert_eq!(paths.len(), 1);
        let path = &paths[0];
        assert_eq!(path.name, "hero");
        assert_eq!(path.vertices[0].position, [12.0, 24.0]);
        assert_eq!(path.vertices[1].position, [22.0, 34.0]);
        assert_eq!(path.vertices[0].tangent_out, [2.0, 2.0]);
        assert_eq!(path.fill_color, Some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(path.stroke_color, Some([0.0, 1.0, 0.0, 1.0]));
        assert_eq!(path.stroke_width, 3.0);
        assert!(path.is_closed);
    }

    #[test]
    fn test_parse_svg_document_accepts_multiline_path_and_none_fill() {
        let svg = r#"
            <svg>
              <path
                style="fill:none; stroke: #ffffff; stroke-width: 2px"
                d="M 0 0
                   L 10 0
                   L 10 10" />
            </svg>
        "#;
        let paths = parse_svg_document(svg);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].vertices.len(), 3);
        assert_eq!(paths[0].fill_color, None);
        assert_eq!(paths[0].stroke_color, Some([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(paths[0].stroke_width, 2.0);
        assert!(!paths[0].is_closed);
    }
}
