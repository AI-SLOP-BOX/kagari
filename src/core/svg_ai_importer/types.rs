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
