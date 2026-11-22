use super::{
    composite_layer_buffer, render_matte_buffer, CompositeCtx, CpuMaskEntry, ParticleSimCacheEntry,
};
use crate::core::timeline::{Camera3D, Composition, Layer, LayerType};
use std::hash::{Hash, Hasher};

const PARTICLE_SIM_CHECKPOINT_INTERVAL: u32 = 120;
const PARTICLE_SIM_CHECKPOINT_BUDGET_BYTES: usize = 8 * 1024 * 1024;
const PARTICLE_SIM_CACHE_BUDGET_BYTES: usize = 64 * 1024 * 1024;
const PARTICLE_SIM_CACHE_MAX_ENTRIES: usize = 16;

type ParticleSimCacheKey = (u64, String, usize, String, u32, u32, u64);

fn particle_sim_cache_key_matches(
    entry: &ParticleSimCacheEntry,
    key: &ParticleSimCacheKey,
) -> bool {
    entry.version == key.0
        && entry.composition_id == key.1
        && entry.composition_instance_id == key.2
        && entry.layer_id == key.3
        && entry.fps == key.4
        && entry.emitter_fingerprint == key.6
}

fn same_particle_sim_cache_identity(
    left: &ParticleSimCacheEntry,
    right: &ParticleSimCacheEntry,
) -> bool {
    left.version == right.version
        && left.composition_id == right.composition_id
        && left.composition_instance_id == right.composition_instance_id
        && left.layer_id == right.layer_id
        && left.fps == right.fps
        && left.emitter_fingerprint == right.emitter_fingerprint
}

fn particle_sim_cache_entry_memory_bytes(entry: &ParticleSimCacheEntry) -> usize {
    std::mem::size_of_val(entry)
        .saturating_add(entry.composition_id.capacity())
        .saturating_add(entry.layer_id.capacity())
        .saturating_add(
            entry
                .checkpoints
                .capacity()
                .saturating_mul(std::mem::size_of::<(
                    u32,
                    crate::core::particle_system::ParticleSystem,
                )>()),
        )
        .saturating_add(particle_system_memory_bytes(&entry.system))
        .saturating_add(entry.checkpoints.iter().fold(0usize, |total, (_, state)| {
            total.saturating_add(particle_system_memory_bytes(state))
        }))
}

fn insert_particle_sim_cache_entry(
    cache: &mut Vec<ParticleSimCacheEntry>,
    entry: ParticleSimCacheEntry,
) {
    if particle_sim_cache_entry_memory_bytes(&entry) > PARTICLE_SIM_CACHE_BUDGET_BYTES {
        return;
    }
    cache.retain(|existing| !same_particle_sim_cache_identity(existing, &entry));
    cache.push(entry);
    while cache.len() > PARTICLE_SIM_CACHE_MAX_ENTRIES
        || cache.iter().fold(0usize, |total, cached| {
            total.saturating_add(particle_sim_cache_entry_memory_bytes(cached))
        }) > PARTICLE_SIM_CACHE_BUDGET_BYTES
    {
        cache.remove(0);
    }
}

fn particle_emitter_fingerprint(emitter: &crate::core::particle_system::ParticleEmitter) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    macro_rules! hash_float {
        ($value:expr) => {
            $value.to_bits().hash(&mut hasher)
        };
    }

    hash_float!(emitter.rate);
    emitter.max_particles.hash(&mut hasher);
    hash_float!(emitter.lifetime);
    hash_float!(emitter.lifetime_variance);
    hash_float!(emitter.speed);
    hash_float!(emitter.speed_variance);
    hash_float!(emitter.spread_degrees);
    (emitter.shape as u8).hash(&mut hasher);
    for value in emitter.emitter_size {
        hash_float!(value);
    }
    for value in emitter.gravity {
        hash_float!(value);
    }
    for value in emitter.wind {
        hash_float!(value);
    }
    hash_float!(emitter.turbulence);
    for value in emitter.color_start {
        hash_float!(value);
    }
    for value in emitter.color_end {
        hash_float!(value);
    }
    hash_float!(emitter.size_start);
    hash_float!(emitter.size_end);
    hash_float!(emitter.opacity_start);
    hash_float!(emitter.opacity_end);
    hash_float!(emitter.rotation_speed);
    hash_float!(emitter.rotation_start);
    hash_float!(emitter.rotation_speed_variance);
    (emitter.fade_curve as u8).hash(&mut hasher);
    emitter.blend_mode.hash(&mut hasher);
    emitter.gravity_curve.0.len().hash(&mut hasher);
    for value in &emitter.gravity_curve.0 {
        hash_float!(*value);
    }
    hash_float!(emitter.wind_gust_strength);
    hash_float!(emitter.wind_gust_frequency);
    hash_float!(emitter.drag);
    emitter.collision_enabled.hash(&mut hasher);
    for value in emitter.collision_bounds {
        hash_float!(value);
    }
    hash_float!(emitter.restitution);
    hash_float!(emitter.surface_friction);
    emitter.particle_collisions.hash(&mut hasher);
    hash_float!(emitter.particle_diameter);
    emitter.trail_length.hash(&mut hasher);
    hash_float!(emitter.trail_taper);
    hash_float!(emitter.vortex_strength);
    for value in emitter.vortex_center {
        hash_float!(value);
    }
    hash_float!(emitter.attract_strength);
    for value in emitter.attract_center {
        hash_float!(value);
    }
    emitter.depth_enabled.hash(&mut hasher);
    for value in emitter.depth_range {
        hash_float!(value);
    }
    emitter.death_spawn_count.hash(&mut hasher);
    hash_float!(emitter.death_spawn_speed_scale);
    hash_float!(emitter.death_spawn_life_scale);
    hasher.finish()
}

fn particle_system_memory_bytes(system: &crate::core::particle_system::ParticleSystem) -> usize {
    std::mem::size_of_val(system)
        .saturating_add(
            system
                .particles
                .capacity()
                .saturating_mul(std::mem::size_of::<crate::core::particle_system::Particle>()),
        )
        .saturating_add(
            system
                .emitter
                .gravity_curve
                .0
                .capacity()
                .saturating_mul(std::mem::size_of::<f32>()),
        )
}

fn remember_particle_checkpoint(
    checkpoints: &mut Vec<(u32, crate::core::particle_system::ParticleSystem)>,
    frame: u32,
    system: &crate::core::particle_system::ParticleSystem,
) {
    let checkpoint_bytes = particle_system_memory_bytes(system);
    if checkpoint_bytes > PARTICLE_SIM_CHECKPOINT_BUDGET_BYTES {
        return;
    }
    if let Some(index) = checkpoints
        .iter_mut()
        .position(|(checkpoint_frame, _)| *checkpoint_frame == frame)
    {
        // Replace the old snapshot before accounting so a growing state cannot
        // bypass the aggregate budget on the duplicate-frame fast path.
        checkpoints.remove(index);
    }

    let mut total_bytes = checkpoints.iter().fold(0usize, |total, (_, state)| {
        total.saturating_add(particle_system_memory_bytes(state))
    });
    while total_bytes.saturating_add(checkpoint_bytes) > PARTICLE_SIM_CHECKPOINT_BUDGET_BYTES
        && !checkpoints.is_empty()
    {
        let removed = checkpoints.remove(0);
        total_bytes = total_bytes.saturating_sub(particle_system_memory_bytes(&removed.1));
    }
    checkpoints.push((frame, system.clone()));
    checkpoints.sort_by_key(|(checkpoint_frame, _)| *checkpoint_frame);
}

fn advance_particle_system(
    system: &mut crate::core::particle_system::ParticleSystem,
    first_frame: u32,
    last_frame: u32,
    dt: f32,
    comp: &Composition,
    layer: &Layer,
    mut checkpoints: Option<&mut Vec<(u32, crate::core::particle_system::ParticleSystem)>>,
) -> bool {
    if first_frame > last_frame {
        return true;
    }
    for frame in first_frame..=last_frame {
        if frame % 16 == 0 && super::render_cancelled() {
            return false;
        }
        let emitter_position = comp.resolve_world_transform(layer, frame).0;
        system.update(dt, emitter_position[0], emitter_position[1]);
        if frame % PARTICLE_SIM_CHECKPOINT_INTERVAL == 0 {
            if let Some(states) = checkpoints.as_deref_mut() {
                remember_particle_checkpoint(states, frame, system);
            }
        }
    }
    true
}

pub(crate) struct ParticleLayerCtx<'a> {
    pub comp: &'a Composition,
    pub composition_instance_id: usize,
    pub layer: &'a Layer,
    pub frame: u32,
    pub sorted_idx: usize,
    pub source_frame: u32,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
    pub opacity: f32,
    pub masks: &'a [CpuMaskEntry],
    pub blend_linear: bool,
    pub width: u32,
    pub height: u32,
    pub camera: &'a Camera3D,
}

pub(crate) fn render_particle_layer(
    buffer: &mut [u8],
    ctx: ParticleLayerCtx<'_>,
) -> Result<bool, ()> {
    let ParticleLayerCtx {
        comp,
        composition_instance_id,
        layer,
        frame,
        sorted_idx,
        source_frame,
        position: pos,
        scale,
        rotation,
        opacity: l_opacity,
        masks,
        blend_linear,
        width,
        height,
        camera: render_camera,
    } = ctx;
    // Render into an isolated layer buffer so particle effects and blend
    // modes cannot alter pixels belonging to layers underneath.
    if let LayerType::Particle { emitter } = &layer.layer_type {
        let em = emitter.clone();

        // Simulation is deterministic from frame 0, so retain a bounded LRU
        // across several particle layers instead of thrashing one cache slot.
        thread_local! {
            static PARTICLE_SIM_CACHE: std::cell::RefCell<Vec<ParticleSimCacheEntry>> =
                const { std::cell::RefCell::new(Vec::new()) };
        }

        let emitter_fingerprint = particle_emitter_fingerprint(&em);
        let sim_key = (
            crate::core::frame_cache::current_version(),
            comp.id.clone(),
            composition_instance_id,
            layer.id.clone(),
            comp.fps.max(1),
            source_frame,
            emitter_fingerprint,
        );
        let dt = 1.0 / comp.fps.max(1) as f32;
        let depth_enabled = em.depth_enabled;

        let ps = PARTICLE_SIM_CACHE.with(|cache| {
            let mut cache_ref = cache.borrow_mut();
            let cached_index = cache_ref
                .iter()
                .position(|entry| particle_sim_cache_key_matches(entry, &sim_key));
            if let Some(index) = cached_index {
                let mut entry = cache_ref.remove(index);
                let result = if entry.frame == source_frame {
                    entry.system.clone()
                } else {
                    if source_frame < entry.frame {
                        remember_particle_checkpoint(
                            &mut entry.checkpoints,
                            entry.frame,
                            &entry.system,
                        );
                    }
                    let mut nearest =
                        (entry.frame <= source_frame).then(|| (entry.frame, entry.system.clone()));
                    for (checkpoint_frame, state) in &entry.checkpoints {
                        if *checkpoint_frame <= source_frame
                            && nearest
                                .as_ref()
                                .is_none_or(|(nearest_frame, _)| checkpoint_frame > nearest_frame)
                        {
                            nearest = Some((*checkpoint_frame, state.clone()));
                        }
                    }
                    match nearest {
                        Some((checkpoint_frame, state)) if checkpoint_frame == source_frame => {
                            entry.frame = checkpoint_frame;
                            entry.system = state.clone();
                            state
                        }
                        nearest => {
                            let (mut system, first_frame) = match nearest {
                                Some((checkpoint_frame, state)) => {
                                    (state, checkpoint_frame.saturating_add(1))
                                }
                                None => (
                                    crate::core::particle_system::ParticleSystem::new(em.clone()),
                                    0,
                                ),
                            };
                            if advance_particle_system(
                                &mut system,
                                first_frame,
                                source_frame,
                                dt,
                                comp,
                                layer,
                                Some(&mut entry.checkpoints),
                            ) {
                                entry.frame = source_frame;
                                entry.system = system.clone();
                            }
                            system
                        }
                    }
                };
                insert_particle_sim_cache_entry(&mut cache_ref, entry);
                return result;
            }

            let mut entry = ParticleSimCacheEntry {
                version: sim_key.0,
                composition_id: sim_key.1.clone(),
                composition_instance_id: sim_key.2,
                layer_id: sim_key.3.clone(),
                fps: sim_key.4,
                emitter_fingerprint: sim_key.6,
                frame: source_frame,
                system: crate::core::particle_system::ParticleSystem::new(em.clone()),
                checkpoints: Vec::new(),
            };
            if !advance_particle_system(
                &mut entry.system,
                0,
                source_frame,
                dt,
                comp,
                layer,
                Some(&mut entry.checkpoints),
            ) {
                return entry.system;
            }
            let result = entry.system.clone();
            insert_particle_sim_cache_entry(&mut cache_ref, entry);
            result
        });
        if super::render_cancelled() {
            return Ok(true);
        }
        let mut ps = ps;
        let source_transform = comp.resolve_world_transform(layer, source_frame);
        transform_particle_system_between_layer_transforms(
            &mut ps,
            source_transform,
            (pos, scale, rotation, l_opacity),
        );
        let Some(layer_size) = super::rgba_buffer_size(width, height) else {
            log::error!("[Renderer] Invalid particle layer dimensions {width}x{height}");
            return Err(());
        };
        let Some(mut layer_buf) = super::try_zeroed_buffer(layer_size) else {
            log::error!(
                "[Renderer] Could not allocate {layer_size} bytes for particle layer {}",
                layer.id
            );
            return Err(());
        };
        if depth_enabled {
            // Project particles through the active camera: Z drives
            // screen position and size scaling.
            let cam = &render_camera;
            let cpos = cam.transform.position.evaluate(frame);
            let crot = cam.transform.rotation.evaluate(frame);
            let rad = crot[2].to_radians();
            let fov = cam.fov_at(frame).max(1.0).to_radians();
            let focal = (height as f32 * 0.5) / (fov * 0.5).tan();
            let proj = crate::core::particle_system::CameraProjection {
                cam_x: cpos[0],
                cam_y: cpos[1],
                cam_z: cpos[2],
                focal,
                cos_rz: rad.cos(),
                sin_rz: rad.sin(),
            };
            ps.render_projected(
                &mut layer_buf,
                width,
                height,
                source_frame as f32 * dt,
                Some(&proj),
            );
        } else {
            ps.render(&mut layer_buf, width, height, source_frame as f32 * dt);
        }
        if super::render_cancelled() {
            return Ok(true);
        }

        // Particle Add stores premultiplied RGB while the compositor consumes
        // straight-alpha pixels. Convert only nontransparent pixels.
        if ps.emitter.blend_mode == 1 {
            for (index, pixel) in layer_buf.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                if index % 4096 == 0 && super::render_cancelled() {
                    return Ok(true);
                }
                let alpha = pixel[3] as f32 / 255.0;
                if alpha > 0.0 {
                    for channel in &mut pixel[..3] {
                        *channel = ((*channel as f32 / alpha).round()).clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }

        if super::render_cancelled() {
            return Ok(true);
        }
        if !layer.paint_strokes.is_empty() {
            let rad = rotation.to_radians();
            let paint_transform = crate::core::software_renderer::precomp::paint::PaintTransform {
                scale,
                cos_r: rad.cos(),
                sin_r: rad.sin(),
                cx: pos[0],
                cy: pos[1],
                min_x: 0,
                min_y: 0,
            };
            crate::core::software_renderer::precomp::paint::apply_paint_strokes(
                layer,
                source_frame,
                &paint_transform,
                width,
                height,
                &mut layer_buf,
            );
            if super::render_cancelled() {
                return Ok(true);
            }
        }

        // Apply the layer's CPU effect stack to the full frame
        if layer.effects_enabled {
            crate::core::cpu_effects::apply_layer_effects(
                Some(comp),
                Some(sorted_idx),
                &mut layer_buf,
                width,
                height,
                &layer.effects,
                source_frame,
                comp.fps,
            );
            if super::render_cancelled() {
                return Ok(true);
            }
        }

        // Clip effect spill to the layer masks, matching the renderer's
        // post-effect mask pass for raster layers.
        if !masks.is_empty() {
            for y in 0..height {
                if super::render_cancelled() {
                    return Ok(true);
                }
                for x in 0..width {
                    let alpha = &mut layer_buf[((y * width + x) * 4 + 3) as usize];
                    if *alpha == 0 {
                        continue;
                    }
                    let coverage = super::mask::compute_combined_mask_coverage(
                        x as f32 + 0.5,
                        y as f32 + 0.5,
                        masks,
                    );
                    *alpha = ((*alpha as f32 * coverage).round()).clamp(0.0, 255.0) as u8;
                }
            }
        }

        let matte = match render_matte_buffer(comp, layer, frame, width, height) {
            Ok(matte) => matte,
            Err(()) => return Err(()),
        };
        if super::render_cancelled() {
            return Ok(true);
        }
        composite_layer_buffer(
            buffer,
            &layer_buf,
            &CompositeCtx {
                layer,
                matte: matte.as_deref(),
                min_x: 0,
                min_y: 0,
                bw: width,
                bh: height,
                width,
                height,
                blend_linear,
                l_opacity,
            },
        );
        return Ok(true);
    }
    Ok(false)
}

fn transform_particle_system_between_layer_transforms(
    system: &mut crate::core::particle_system::ParticleSystem,
    source: ([f32; 2], [f32; 2], f32, f32),
    current: ([f32; 2], [f32; 2], f32, f32),
) {
    let (source_position, source_scale, source_rotation, _) = source;
    let (current_position, current_scale, current_rotation, _) = current;
    let source_angle = source_rotation.to_radians();
    let (source_sin, source_cos) = source_angle.sin_cos();
    let current_angle = current_rotation.to_radians();
    let (current_sin, current_cos) = current_angle.sin_cos();
    let source_sx = source_scale[0] / 100.0;
    let source_sy = source_scale[1] / 100.0;
    let current_sx = current_scale[0] / 100.0;
    let current_sy = current_scale[1] / 100.0;
    let same_transform = source_position == current_position
        && source_scale == current_scale
        && source_rotation == current_rotation;

    // Particle sizes are authored in layer space while the CPU particle pass
    // draws directly into composition pixels. Apply the current layer scale
    // even when source and current transforms are identical. A circular
    // particle cannot represent non-uniform scaling, so preserve its area with
    // the geometric mean of the absolute axis scales.
    let size_scale = (current_sx.abs() * current_sy.abs()).sqrt();

    let transform_point = |point: &mut (f32, f32)| {
        let dx = point.0 - source_position[0];
        let dy = point.1 - source_position[1];
        let local_x = source_cos * dx + source_sin * dy;
        let local_y = -source_sin * dx + source_cos * dy;
        let local_x = if source_sx.abs() > f32::EPSILON {
            local_x / source_sx
        } else {
            0.0
        };
        let local_y = if source_sy.abs() > f32::EPSILON {
            local_y / source_sy
        } else {
            0.0
        };
        let scaled_x = local_x * current_sx;
        let scaled_y = local_y * current_sy;
        point.0 = current_position[0] + current_cos * scaled_x - current_sin * scaled_y;
        point.1 = current_position[1] + current_sin * scaled_x + current_cos * scaled_y;
    };

    for particle in &mut system.particles {
        particle.size *= size_scale;
        if same_transform {
            continue;
        }
        let mut point = (particle.x, particle.y);
        transform_point(&mut point);
        (particle.x, particle.y) = point;
        for trail in particle.trail.iter_mut().take(particle.trail_len as usize) {
            transform_point(trail);
        }
    }
}

#[cfg(test)]
mod cache_tests {
    use super::{
        insert_particle_sim_cache_entry, particle_sim_cache_key_matches,
        PARTICLE_SIM_CACHE_MAX_ENTRIES,
    };
    use crate::core::software_renderer::ParticleSimCacheEntry;

    fn entry(layer_id: &str) -> ParticleSimCacheEntry {
        ParticleSimCacheEntry {
            version: 7,
            composition_id: "comp".to_owned(),
            composition_instance_id: 11,
            layer_id: layer_id.to_owned(),
            fps: 24,
            emitter_fingerprint: 19,
            frame: 0,
            system: crate::core::particle_system::ParticleSystem::new(
                crate::core::particle_system::ParticleEmitter::default(),
            ),
            checkpoints: Vec::new(),
        }
    }

    #[test]
    fn cache_retains_simulations_for_multiple_particle_layers() {
        let mut cache = Vec::new();
        insert_particle_sim_cache_entry(&mut cache, entry("fire"));
        insert_particle_sim_cache_entry(&mut cache, entry("smoke"));

        assert_eq!(cache.len(), 2);
        let fire_key = (7, "comp".to_owned(), 11, "fire".to_owned(), 24, 83, 19);
        let smoke_key = (7, "comp".to_owned(), 11, "smoke".to_owned(), 24, 83, 19);
        assert!(cache
            .iter()
            .any(|cached| particle_sim_cache_key_matches(cached, &fire_key)));
        assert!(cache
            .iter()
            .any(|cached| particle_sim_cache_key_matches(cached, &smoke_key)));
    }

    #[test]
    fn cache_evicts_oldest_entries_at_its_bound() {
        let mut cache = Vec::new();
        for index in 0..=PARTICLE_SIM_CACHE_MAX_ENTRIES {
            insert_particle_sim_cache_entry(&mut cache, entry(&format!("layer-{index}")));
        }

        assert_eq!(cache.len(), PARTICLE_SIM_CACHE_MAX_ENTRIES);
        let oldest_key = (7, "comp".to_owned(), 11, "layer-0".to_owned(), 24, 0, 19);
        let newest_key = (
            7,
            "comp".to_owned(),
            11,
            format!("layer-{PARTICLE_SIM_CACHE_MAX_ENTRIES}"),
            24,
            0,
            19,
        );
        assert!(!cache
            .iter()
            .any(|cached| particle_sim_cache_key_matches(cached, &oldest_key)));
        assert!(cache
            .iter()
            .any(|cached| particle_sim_cache_key_matches(cached, &newest_key)));
    }
}
