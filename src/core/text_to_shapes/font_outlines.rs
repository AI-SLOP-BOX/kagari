use crate::core::font_rasterizer::{with_font_rasterizer_mut, FontRasterizer};
use ab_glyph::{OutlineCurve, Point};

const MAX_TEXT_SHAPE_CHARS: usize = 25_000;
const MAX_OUTLINE_POINTS: usize = 2_000_000;
const MAX_CONTOURS_PER_GLYPH: usize = 256;
type OutlineGeneration = Result<Option<(Vec<super::MaskedContour>, (f32, f32))>, ()>;

pub(super) struct FontOutlineLayout {
    pub font_size: f32,
    pub tracking: f32,
    pub leading: f32,
    pub alignment: usize,
    pub box_width: f32,
}

pub(super) fn apply_faux_outline_style(
    contours: &mut [super::MaskedContour],
    bounds: &mut (f32, f32),
    style: crate::core::font_rasterizer::FauxTextStyle,
) {
    if style.small_caps {
        for (points, _) in contours.iter_mut() {
            for point in points {
                point[0] *= 0.84;
                point[1] *= 0.84;
            }
        }
        bounds.0 *= 0.84;
        bounds.1 *= 0.84;
    }
    if style.italic {
        let shear = bounds.1 * 0.22;
        let centering = shear * 0.5;
        for (points, _) in contours.iter_mut() {
            for point in points {
                point[0] += (point[1] + bounds.1 * 0.5) * 0.22 - centering;
            }
        }
        bounds.0 += shear;
    }
}

pub(super) fn generate(text: &str, family: &str, options: FontOutlineLayout) -> OutlineGeneration {
    if text.chars().count() > MAX_TEXT_SHAPE_CHARS {
        return Err(());
    }
    with_font_rasterizer_mut(|rasterizer| {
        if !rasterizer.has_font(family) {
            rasterizer.load_system_font(family);
        }
        let resolved_family = rasterizer.resolve_family(family);
        generate_with_rasterizer(rasterizer, &resolved_family, text, options)
    })
}

fn generate_with_rasterizer(
    rasterizer: &FontRasterizer,
    family: &str,
    text: &str,
    options: FontOutlineLayout,
) -> OutlineGeneration {
    let alignment_mode = match options.alignment {
        1 => crate::core::text_layout::TextAlign::Center,
        2 => crate::core::text_layout::TextAlign::Right,
        3 => crate::core::text_layout::TextAlign::Justify,
        _ => crate::core::text_layout::TextAlign::Left,
    };
    let paragraph_layout = crate::core::text_layout::layout_text(
        text,
        options.font_size,
        options.tracking,
        options.leading,
        options.box_width,
        alignment_mode,
    );
    let mut line_glyphs = Vec::new();
    let mut line_widths = Vec::new();
    line_glyphs
        .try_reserve_exact(paragraph_layout.lines.len())
        .map_err(|_| ())?;
    line_widths
        .try_reserve_exact(paragraph_layout.lines.len())
        .map_err(|_| ())?;
    let mut ascent = 0.0f32;
    for line in &paragraph_layout.lines {
        let Some((glyphs, line_ascent)) =
            rasterizer.text_glyph_outlines(family, &line.text, options.font_size)
        else {
            return Ok(None);
        };
        let width = glyphs
            .iter()
            .map(|glyph| glyph.advance + glyph.kerning_after)
            .sum::<f32>()
            + options.tracking * glyphs.len().saturating_sub(1) as f32;
        if !width.is_finite() || !line_ascent.is_finite() {
            return Ok(None);
        }
        ascent = ascent.max(line_ascent);
        line_widths.push(width.max(0.0));
        line_glyphs.push(glyphs);
    }

    let max_width = line_widths.iter().copied().fold(
        paragraph_layout.total_width.max(options.box_width),
        f32::max,
    );
    let line_height = options.font_size * options.leading.max(0.5);
    let total_height = paragraph_layout.total_height.max(line_height);
    if !total_height.is_finite() {
        return Ok(None);
    }
    let mut contours = Vec::new();
    let mut point_count = 0usize;
    for (row, glyphs) in line_glyphs.iter().enumerate() {
        let line = &paragraph_layout.lines[row];
        let line_width = line_widths[row];
        let alignment_offset = crate::core::text_layout::get_alignment_offset(
            line_width,
            options.box_width,
            alignment_mode,
        );
        let justify_space = crate::core::text_layout::justification_space_width(
            line,
            line_width,
            options.box_width,
            alignment_mode,
        );
        let start_x = alignment_offset - max_width * 0.5;
        let baseline = line.y_offset - total_height * 0.5 + ascent;
        let mut cursor_x = start_x;
        for (character, glyph) in line.text.chars().zip(glyphs) {
            let mut glyph_contours = Vec::new();
            append_glyph_contours(
                &glyph.curves,
                [cursor_x, baseline],
                &mut glyph_contours,
                &mut point_count,
            )?;
            let modes = super::classify_glyph_contours(&glyph_contours);
            contours.try_reserve(glyph_contours.len()).map_err(|_| ())?;
            contours.extend(glyph_contours.into_iter().zip(modes));
            cursor_x += glyph.advance
                + glyph.kerning_after
                + options.tracking
                + if character == ' ' { justify_space } else { 0.0 };
        }
    }
    Ok(Some((
        contours,
        (max_width.max(10.0), total_height.max(10.0)),
    )))
}

fn append_glyph_contours(
    curves: &[OutlineCurve],
    origin: [f32; 2],
    contours: &mut Vec<Vec<[f32; 2]>>,
    point_count: &mut usize,
) -> Result<(), ()> {
    let mut contour = Vec::new();
    let mut contour_start = None;
    for curve in curves {
        let (start, end) = curve_endpoints(curve);
        if contour_start.is_none() {
            contour_start = Some(start);
            contour.try_reserve(16).map_err(|_| ())?;
            contour.push(transform(start, origin));
        }
        let segment_count = curve_segment_count(curve);
        contour.try_reserve(segment_count).map_err(|_| ())?;
        for step in 1..=segment_count {
            let t = step as f32 / segment_count as f32;
            let point = transform(evaluate_curve(curve, t), origin);
            if !point[0].is_finite() || !point[1].is_finite() {
                return Err(());
            }
            contour.push(point);
            *point_count = point_count.checked_add(1).ok_or(())?;
            if *point_count > MAX_OUTLINE_POINTS {
                return Err(());
            }
        }
        if contour_start == Some(end) {
            if contour.first() == contour.last() {
                contour.pop();
                *point_count = point_count.saturating_sub(1);
            }
            if contour.len() >= 3 {
                if contours.len() >= MAX_CONTOURS_PER_GLYPH {
                    return Err(());
                }
                contours.try_reserve(1).map_err(|_| ())?;
                contours.push(std::mem::take(&mut contour));
            }
            contour_start = None;
        }
    }
    if contour.len() >= 3 {
        if contours.len() >= MAX_CONTOURS_PER_GLYPH {
            return Err(());
        }
        contours.try_reserve(1).map_err(|_| ())?;
        contours.push(contour);
    }
    Ok(())
}

fn curve_endpoints(curve: &OutlineCurve) -> (Point, Point) {
    match curve {
        OutlineCurve::Line(start, end) => (*start, *end),
        OutlineCurve::Quad(start, _, end) => (*start, *end),
        OutlineCurve::Cubic(start, _, _, end) => (*start, *end),
    }
}

fn curve_segment_count(curve: &OutlineCurve) -> usize {
    if matches!(curve, OutlineCurve::Line(_, _)) {
        return 1;
    }
    let length = match curve {
        OutlineCurve::Line(start, end) => distance(*start, *end),
        OutlineCurve::Quad(start, control, end) => {
            distance(*start, *control) + distance(*control, *end)
        }
        OutlineCurve::Cubic(start, control1, control2, end) => {
            distance(*start, *control1) + distance(*control1, *control2) + distance(*control2, *end)
        }
    };
    (length / 2.0).ceil().clamp(1.0, 512.0) as usize
}

fn distance(left: Point, right: Point) -> f32 {
    (left.x - right.x).hypot(left.y - right.y)
}

fn evaluate_curve(curve: &OutlineCurve, t: f32) -> Point {
    let u = 1.0 - t;
    match curve {
        OutlineCurve::Line(p0, p1) => mix(*p0, *p1, t),
        OutlineCurve::Quad(p0, p1, p2) => ab_glyph::point(
            u * u * p0.x + 2.0 * u * t * p1.x + t * t * p2.x,
            u * u * p0.y + 2.0 * u * t * p1.y + t * t * p2.y,
        ),
        OutlineCurve::Cubic(p0, p1, p2, p3) => ab_glyph::point(
            u * u * u * p0.x + 3.0 * u * u * t * p1.x + 3.0 * u * t * t * p2.x + t * t * t * p3.x,
            u * u * u * p0.y + 3.0 * u * u * t * p1.y + 3.0 * u * t * t * p2.y + t * t * t * p3.y,
        ),
    }
}

fn mix(start: Point, end: Point, t: f32) -> Point {
    ab_glyph::point(
        start.x + (end.x - start.x) * t,
        start.y + (end.y - start.y) * t,
    )
}

fn transform(point: Point, origin: [f32; 2]) -> [f32; 2] {
    [origin[0] + point.x, origin[1] - point.y]
}
