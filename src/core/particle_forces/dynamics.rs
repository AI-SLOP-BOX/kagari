/// Wind including a sinusoidal gust component travelling along the base wind
/// direction. When the base wind is zero the gust blows along +X.
pub fn wind_with_gust(
    base: [f32; 2],
    gust_strength: f32,
    gust_frequency_hz: f32,
    time: f32,
) -> [f32; 2] {
    if gust_strength == 0.0 {
        return base;
    }
    let mag = (base[0] * base[0] + base[1] * base[1]).sqrt();
    let dir = if mag > 1e-6 {
        [base[0] / mag, base[1] / mag]
    } else {
        [1.0, 0.0]
    };
    let gust = (std::f32::consts::TAU * gust_frequency_hz * time).sin() * gust_strength;
    [base[0] + dir[0] * gust, base[1] + dir[1] * gust]
}

/// Exponential air drag: stable for any dt (v *= 1 / (1 + drag*dt)).
pub fn apply_drag(vx: &mut f32, vy: &mut f32, drag: f32, dt: f32) {
    if drag <= 0.0 || dt <= 0.0 {
        return;
    }
    let damp = 1.0 / (1.0 + drag * dt);
    *vx *= damp;
    *vy *= damp;
}

/// Vortex orbital force: pulls particles inward while accelerating them tangentially.
pub fn vortex_force(
    pos: [f32; 2],
    center: [f32; 2],
    inward_pull: f32,
    tangential_speed: f32,
) -> [f32; 2] {
    let dx = pos[0] - center[0];
    let dy = pos[1] - center[1];
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1.0 {
        return [0.0, 0.0];
    }
    let ndx = dx / dist;
    let ndy = dy / dist;

    // Radial inward acceleration: -dir * inward_pull
    let fx_rad = -ndx * inward_pull;
    let fy_rad = -ndy * inward_pull;

    // Tangential perpendicular acceleration: [-ndy, ndx] * tangential_speed
    let fx_tan = -ndy * tangential_speed;
    let fy_tan = ndx * tangential_speed;

    [fx_rad + fx_tan, fy_rad + fy_tan]
}

/// Divergence-free 2D Curl Noise turbulence force for natural fluid/smoke motion.
pub fn curl_noise_turbulence(pos: [f32; 2], frequency: f32, amplitude: f32) -> [f32; 2] {
    if amplitude <= 0.0 || frequency <= 0.0 {
        return [0.0, 0.0];
    }
    let px = pos[0] * frequency;
    let py = pos[1] * frequency;

    // Numerical gradient of potential field: Psi = sin(px)*cos(py) + cos(px*1.3)*sin(py*1.3)*0.5
    let eps = 0.01f32;
    let psi_up = (px).sin() * (py + eps).cos() + (px * 1.3).cos() * ((py + eps) * 1.3).sin() * 0.5;
    let psi_down =
        (px).sin() * (py - eps).cos() + (px * 1.3).cos() * ((py - eps) * 1.3).sin() * 0.5;
    let psi_right =
        (px + eps).sin() * (py).cos() + ((px + eps) * 1.3).cos() * (py * 1.3).sin() * 0.5;
    let psi_left =
        (px - eps).sin() * (py).cos() + ((px - eps) * 1.3).cos() * (py * 1.3).sin() * 0.5;

    let d_psi_dy = (psi_up - psi_down) / (2.0 * eps);
    let d_psi_dx = (psi_right - psi_left) / (2.0 * eps);

    // Curl in 2D: [dPsi/dy, -dPsi/dx] (guaranteed div=0)
    [d_psi_dy * amplitude, -d_psi_dx * amplitude]
}
