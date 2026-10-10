//! RigidBody, shapes, spring constraints, and contact manifolds.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RigidBodyType {
    Dynamic,
    Static,
    Kinematic,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ColliderShape {
    Box { half_extents: [f32; 2] },
    Circle { radius: f32 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RigidBody {
    pub id: usize,
    pub layer_id: Option<usize>,
    pub body_type: RigidBodyType,
    pub shape: ColliderShape,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub rotation_deg: f32,
    pub angular_velocity_deg: f32,
    pub mass: f32,
    pub inv_mass: f32,
    pub inertia: f32,
    pub inv_inertia: f32,
    pub restitution: f32, // Bounciness (0.0 to 1.0)
    pub friction: f32,    // Surface friction
}

impl RigidBody {
    pub fn new_box(
        layer_id: Option<usize>,
        position: [f32; 2],
        width: f32,
        height: f32,
        mass: f32,
        body_type: RigidBodyType,
    ) -> Self {
        let half_extents = [width * 0.5, height * 0.5];
        let (inv_mass, inertia, inv_inertia) = if body_type == RigidBodyType::Dynamic && mass > 0.0
        {
            let i = (mass * (width * width + height * height)) / 12.0;
            (1.0 / mass, i, 1.0 / i)
        } else {
            (0.0, 0.0, 0.0)
        };

        Self {
            id: 0,
            layer_id,
            body_type,
            shape: ColliderShape::Box { half_extents },
            position,
            velocity: [0.0, 0.0],
            rotation_deg: 0.0,
            angular_velocity_deg: 0.0,
            mass,
            inv_mass,
            inertia,
            inv_inertia,
            restitution: 0.5,
            friction: 0.3,
        }
    }

    pub fn new_circle(
        layer_id: Option<usize>,
        position: [f32; 2],
        radius: f32,
        mass: f32,
        body_type: RigidBodyType,
    ) -> Self {
        let (inv_mass, inertia, inv_inertia) = if body_type == RigidBodyType::Dynamic && mass > 0.0
        {
            let i = 0.5 * mass * radius * radius;
            (1.0 / mass, i, 1.0 / i)
        } else {
            (0.0, 0.0, 0.0)
        };

        Self {
            id: 0,
            layer_id,
            body_type,
            shape: ColliderShape::Circle { radius },
            position,
            velocity: [0.0, 0.0],
            rotation_deg: 0.0,
            angular_velocity_deg: 0.0,
            mass,
            inv_mass,
            inertia,
            inv_inertia,
            restitution: 0.6,
            friction: 0.2,
        }
    }

    pub fn corners(&self) -> Vec<[f32; 2]> {
        match self.shape {
            ColliderShape::Box { half_extents } => {
                let rad = self.rotation_deg.to_radians();
                let cos = rad.cos();
                let sin = rad.sin();
                let hx = half_extents[0];
                let hy = half_extents[1];

                let offsets = [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]];

                offsets
                    .iter()
                    .map(|&[ox, oy]| {
                        let rx = ox * cos - oy * sin;
                        let ry = ox * sin + oy * cos;
                        [self.position[0] + rx, self.position[1] + ry]
                    })
                    .collect()
            }
            ColliderShape::Circle { radius } => {
                vec![
                    [self.position[0] - radius, self.position[1]],
                    [self.position[0] + radius, self.position[1]],
                    [self.position[0], self.position[1] - radius],
                    [self.position[0], self.position[1] + radius],
                ]
            }
        }
    }
}

/// Spring constraint connecting two bodies or a body to a fixed point.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DistanceSpring {
    pub body_a: usize,
    pub body_b: Option<usize>, // None = attached to fixed world anchor
    pub anchor_a_local: [f32; 2],
    pub anchor_b_local_or_world: [f32; 2],
    pub rest_length: f32,
    pub stiffness: f32, // Spring constant k
    pub damping: f32,   // Damping constant c
}

#[derive(Debug, Clone)]
pub struct ContactManifold {
    pub body_a: usize,
    pub body_b: usize,
    pub normal: [f32; 2], // From A to B
    pub penetration: f32,
    pub contact_point: [f32; 2],
}
