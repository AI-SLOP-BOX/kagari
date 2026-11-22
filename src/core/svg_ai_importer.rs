//! Vector Graphics (SVG / Illustrator) Path & Anchor Point Importer.
//!
//! Parses vector commands (MoveTo, LineTo, Cubic Bezier, ClosePath) into fully animatable
//! AE MaskVertex and MaskPath structures.

#![allow(dead_code)]

pub mod document;
pub mod path_parser;
pub mod tokenizer;
pub mod types;

pub use document::*;
pub use path_parser::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_svg_cubic_bezier_path() {
        let svg_d = "M 10 20 C 30 40 50 60 70 80 Z";
        let verts = parse_svg_path_data(svg_d).expect("SVG path parsing succeeds");
        assert_eq!(verts.len(), 2);
        assert_eq!(verts[0].position, [10.0, 20.0]);
        assert_eq!(verts[0].tangent_out, [20.0, 20.0]);
        assert_eq!(verts[1].position, [70.0, 80.0]);
        assert_eq!(verts[1].tangent_in, [-20.0, -20.0]);
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
    fn test_parse_svg_path_expands_repeated_move_and_line_groups() {
        let verts = parse_svg_path_data("M0 0 10 0 10 10 L20 10 20 20")
            .expect("repeated SVG coordinate groups should parse");
        assert_eq!(
            verts
                .iter()
                .map(|vertex| vertex.position)
                .collect::<Vec<_>>(),
            vec![
                [0.0, 0.0],
                [10.0, 0.0],
                [10.0, 10.0],
                [20.0, 10.0],
                [20.0, 20.0]
            ]
        );
    }

    #[test]
    fn test_parse_svg_elliptical_arc_creates_editable_bezier_segments() {
        let verts =
            parse_svg_path_data("M 10 0 A 10 10 0 0 1 -10 0").expect("elliptical arc should parse");
        assert_eq!(verts.len(), 3);
        assert_eq!(verts[0].position, [10.0, 0.0]);
        assert_eq!(verts.last().expect("arc endpoint").position, [-10.0, 0.0]);
        assert!(verts
            .iter()
            .flat_map(|vertex| vertex.tangent_in.into_iter().chain(vertex.tangent_out))
            .all(f32::is_finite));
        assert!(verts[0].tangent_out[1] > 0.0);
        assert!(verts[2].tangent_in[1] > 0.0);
    }

    #[test]
    fn test_parse_svg_relative_arc_and_degenerate_radii() {
        let verts = parse_svg_path_data("M 10 0 a 10 10 0 0 1 -10 10")
            .expect("relative elliptical arc should parse");
        assert_eq!(
            verts.last().expect("relative arc endpoint").position,
            [0.0, 10.0]
        );

        let line = parse_svg_path_data("M 1 2 A 0 8 0 0 1 9 10")
            .expect("zero-radius arc should fall back to a line");
        assert_eq!(line.len(), 2);
        assert_eq!(line[1].position, [9.0, 10.0]);
    }

    #[test]
    fn test_parse_svg_arc_accepts_compact_flag_pair() {
        let verts = parse_svg_path_data("M 10 0 A 10 10 0 01 -10 0")
            .expect("compact arc flags should parse");
        assert_eq!(verts.len(), 3);
        assert_eq!(
            verts.last().expect("compact arc endpoint").position,
            [-10.0, 0.0]
        );
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
