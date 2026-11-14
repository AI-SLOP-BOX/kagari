pub(crate) struct TextRasterParams {
    pub(crate) text: String,
    pub(crate) font_size: u32,
    pub(crate) color: [f32; 4],
    pub(crate) font_family: String,
    pub(crate) tracking: f32,
    pub(crate) leading: f32,
    pub(crate) align: usize,
    pub(crate) box_width: f32,
    pub(crate) box_height: f32,
    pub(crate) stroke_color: [f32; 4],
    pub(crate) stroke_width: f32,
    pub(crate) faux_style: crate::core::font_rasterizer::FauxTextStyle,
}
