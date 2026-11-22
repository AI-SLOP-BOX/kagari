use crate::core::property::Animatable;

/// Mutable borrow of one keyframeable parameter track.
pub enum ParamRef<'a> {
    Scalar(&'a mut Animatable<f32>),
    Vec2(&'a mut Animatable<[f32; 2]>),
    Vec3(&'a mut Animatable<[f32; 3]>),
    Vec4Color(&'a mut Animatable<[f32; 4]>),
}

/// Immutable borrow of one keyframeable parameter track.
pub enum ParamRefRef<'a> {
    Scalar(&'a Animatable<f32>),
    Vec2(&'a Animatable<[f32; 2]>),
    Vec3(&'a Animatable<[f32; 3]>),
    Vec4Color(&'a Animatable<[f32; 4]>),
}
