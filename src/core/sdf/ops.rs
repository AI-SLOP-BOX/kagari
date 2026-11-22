pub fn sdf_boolean_union(d1: f32, d2: f32) -> f32 {
    d1.min(d2)
}
pub fn sdf_boolean_subtract(d1: f32, d2: f32) -> f32 {
    d1.max(-d2)
}
pub fn sdf_boolean_intersect(d1: f32, d2: f32) -> f32 {
    d1.max(d2)
}
pub fn sdf_boolean_exclude(d1: f32, d2: f32) -> f32 {
    d1.abs().min(d2.abs()).copysign(d1)
}

pub fn sdf_boolean_op(op: u32, d1: f32, d2: f32) -> f32 {
    match op {
        0 => sdf_boolean_union(d1, d2),
        1 => sdf_boolean_subtract(d1, d2),
        2 => sdf_boolean_intersect(d1, d2),
        3 => sdf_boolean_exclude(d1, d2),
        _ => sdf_boolean_union(d1, d2),
    }
}
