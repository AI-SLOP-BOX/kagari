use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn shade(
    layer: &Layer,
    comp: &crate::core::timeline::Composition,
    world: [f32; 3],
    normal: [f32; 3],
    base: [f32; 4],
    imported_material: Option<&crate::core::obj_loader::PbrMaterial3D>,
    imported_emission: [f32; 3],
    frame: u32,
) -> [u8; 4] {
    let material = &layer.material;
    let metalness = imported_material
        .map_or(material.metalness, |material| material.metallic)
        .clamp(0.0, 1.0);
    let specular_exponent = imported_material.map_or(material.specular_exponent, |material| {
        let roughness = material.roughness.clamp(0.0, 1.0);
        (2.0 / (roughness.powi(2).max(1.0e-4)) - 2.0).clamp(1.0, 256.0)
    });
    let base_illumination =
        material.ambient.clamp(0.0, 1.0) * (1.0 - metalness) + material.emission.clamp(0.0, 1.0);
    let mut illumination = [base_illumination; 3];
    let mut specular_reflection = [0.0f32; 3];
    if material.accepts_lights {
        let camera_position = comp.resolve_camera_at().transform.position.evaluate(frame);
        let view_direction = normalize(sub(camera_position, world));
        for source in &comp.lights {
            let intensity = source.intensity_at(frame) / 100.0;
            if matches!(&source.light_type, LightType::Ambient) {
                let contribution = material.ambient.clamp(0.0, 1.0) * (1.0 - metalness) * intensity;
                for (channel, value) in illumination.iter_mut().enumerate() {
                    *value += source.color[channel] * contribution;
                }
                continue;
            }

            let (direction, attenuation, cone) = match &source.light_type {
                LightType::Ambient => unreachable!("ambient lights were handled above"),
                LightType::Parallel => (
                    normalize(super::super::shadow::PARALLEL_LIGHT_DIRECTION),
                    1.0,
                    1.0,
                ),
                LightType::Point | LightType::Spot { .. } => {
                    let light_position = comp.light_position_at(source, frame);
                    let direction = normalize(sub(light_position, world));
                    let attenuation = super::super::shadow::light_attenuation(
                        source,
                        light_position,
                        world,
                        frame,
                    );
                    let cone = super::super::shadow::spot_cone_factor(
                        source,
                        light_position,
                        world,
                        frame,
                    );
                    (direction, attenuation, cone)
                }
            };
            let direct_scale = intensity * attenuation * cone;
            let diffuse = dot(normal, direction).max(0.0)
                * material.diffuse.clamp(0.0, 1.0)
                * (1.0 - metalness)
                * direct_scale;
            let half_direction = normalize([
                direction[0] + view_direction[0],
                direction[1] + view_direction[1],
                direction[2] + view_direction[2],
            ]);
            let specular = dot(normal, half_direction)
                .max(0.0)
                .powf(specular_exponent.clamp(1.0, 256.0))
                * direct_scale;
            for (channel, value) in illumination.iter_mut().enumerate() {
                *value += source.color[channel] * diffuse;
                let dielectric_reflectance = if imported_material.is_some() {
                    0.04
                } else {
                    material.specular.clamp(0.0, 1.0)
                };
                let reflectance = dielectric_reflectance * (1.0 - metalness)
                    + base[channel].clamp(0.0, 1.0) * metalness;
                specular_reflection[channel] += source.color[channel] * reflectance * specular;
            }
        }
    }
    [
        ((base[0] * illumination[0].clamp(0.0, 2.0)
            + specular_reflection[0]
            + imported_emission[0].clamp(0.0, 1.0))
            * 255.0)
            .clamp(0.0, 255.0) as u8,
        ((base[1] * illumination[1].clamp(0.0, 2.0)
            + specular_reflection[1]
            + imported_emission[1].clamp(0.0, 1.0))
            * 255.0)
            .clamp(0.0, 255.0) as u8,
        ((base[2] * illumination[2].clamp(0.0, 2.0)
            + specular_reflection[2]
            + imported_emission[2].clamp(0.0, 1.0))
            * 255.0)
            .clamp(0.0, 255.0) as u8,
        (base[3].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

pub(super) fn sample_texture(
    texture: &crate::core::image_cache::CachedImage,
    uv: [f32; 2],
    wrap_s: u32,
    wrap_t: u32,
    nearest: bool,
) -> [f32; 4] {
    if texture.width == 0 || texture.height == 0 || uv.iter().any(|value| !value.is_finite()) {
        return [1.0; 4];
    }
    let width = texture.width as i64;
    let height = texture.height as i64;
    let coordinate = |value: f32, mode| match mode {
        33071 => value.clamp(0.0, 1.0),
        33648 => {
            let value = value.rem_euclid(2.0);
            if value <= 1.0 {
                value
            } else {
                2.0 - value
            }
        }
        _ => value.rem_euclid(1.0),
    };
    let x = coordinate(uv[0], wrap_s) * texture.width as f32 - 0.5;
    let y = coordinate(uv[1], wrap_t) * texture.height as f32 - 0.5;
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let tx = x - x.floor();
    let ty = y - y.floor();
    let address = |coordinate: i64, size: i64, mode| match mode {
        33071 => coordinate.clamp(0, size - 1),
        33648 => {
            let reflected = coordinate.rem_euclid(size * 2);
            if reflected < size {
                reflected
            } else {
                size * 2 - 1 - reflected
            }
        }
        _ => coordinate.rem_euclid(size),
    } as usize;
    let texel = |x: i64, y: i64| {
        let x = address(x, width, wrap_s);
        let y = address(y, height, wrap_t);
        let index = (y * texture.width as usize + x) * 4;
        if index + 3 < texture.pixels.len() {
            [
                texture.pixels[index] as f32 / 255.0,
                texture.pixels[index + 1] as f32 / 255.0,
                texture.pixels[index + 2] as f32 / 255.0,
                texture.pixels[index + 3] as f32 / 255.0,
            ]
        } else {
            [1.0; 4]
        }
    };
    if nearest {
        return texel((x + 0.5).floor() as i64, (y + 0.5).floor() as i64);
    }
    let c00 = texel(x0, y0);
    let c10 = texel(x0 + 1, y0);
    let c01 = texel(x0, y0 + 1);
    let c11 = texel(x0 + 1, y0 + 1);
    let interpolate = |channel: usize| {
        let top = c00[channel] + (c10[channel] - c00[channel]) * tx;
        let bottom = c01[channel] + (c11[channel] - c01[channel]) * tx;
        top + (bottom - top) * ty
    };
    let alpha = interpolate(3);
    let interpolate_premultiplied = |channel: usize| {
        let top = c00[channel] * c00[3] + (c10[channel] * c10[3] - c00[channel] * c00[3]) * tx;
        let bottom = c01[channel] * c01[3] + (c11[channel] * c11[3] - c01[channel] * c01[3]) * tx;
        top + (bottom - top) * ty
    };
    if alpha <= f32::EPSILON {
        [0.0, 0.0, 0.0, 0.0]
    } else {
        [
            interpolate_premultiplied(0) / alpha,
            interpolate_premultiplied(1) / alpha,
            interpolate_premultiplied(2) / alpha,
            alpha,
        ]
    }
}
