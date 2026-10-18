use super::path_parser::parse_svg_path_data;
use super::types::SvgVectorPath;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SvgTransform {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl SvgTransform {
    pub const fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    pub fn concat(self, next: Self) -> Self {
        Self {
            a: self.a * next.a + self.c * next.b,
            b: self.b * next.a + self.d * next.b,
            c: self.a * next.c + self.c * next.d,
            d: self.b * next.c + self.d * next.d,
            e: self.a * next.e + self.c * next.f + self.e,
            f: self.b * next.e + self.d * next.f + self.f,
        }
    }

    pub fn apply_point(self, point: [f32; 2]) -> [f32; 2] {
        [
            self.a * point[0] + self.c * point[1] + self.e,
            self.b * point[0] + self.d * point[1] + self.f,
        ]
    }

    pub fn apply_vector(self, vector: [f32; 2]) -> [f32; 2] {
        [
            self.a * vector[0] + self.c * vector[1],
            self.b * vector[0] + self.d * vector[1],
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SvgStyle {
    pub fill: Option<[f32; 4]>,
    pub stroke: Option<[f32; 4]>,
    pub stroke_width: f32,
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
    pub fn with_attributes(self, attrs: &std::collections::HashMap<String, String>) -> Self {
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

    pub fn apply(&mut self, key: &str, value: &str) {
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

pub(crate) fn find_svg_tag_end(tag: &str) -> Option<usize> {
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

pub(crate) fn parse_svg_attributes(source: &str) -> std::collections::HashMap<String, String> {
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

pub(crate) fn parse_svg_numbers(source: &str) -> Vec<f32> {
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

pub(crate) fn parse_svg_transform(source: &str) -> SvgTransform {
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

pub(crate) fn parse_svg_color(value: &str) -> Option<[f32; 4]> {
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
    } else {
        let hex = value.strip_prefix('#')?;
        match hex.len() {
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
        }
    };
    Some([
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        1.0,
    ])
}

/// Extracts all vector shapes from an SVG file string.
pub fn parse_svg_document(svg_text: &str) -> Vec<SvgVectorPath> {
    let mut paths = Vec::new();
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
        if let Some(stripped_tag) = tag.strip_prefix('/') {
            let name = stripped_tag
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
