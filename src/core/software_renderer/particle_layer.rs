use super::ParticleSimCacheEntry;
use crate::core::timeline::{Camera3D, Composition, Layer, LayerType};

pub(crate) struct ParticleLayerCtx<'a> {
    pub comp: &'a Composition,
    pub layer: &'a Layer,
    pub frame: u32,
    pub sorted_idx: usize,
    pub source_frame: u32,
    pub position: [f32; 2],
    pub opacity: f32,
    pub width: u32,
    pub height: u32,
    pub camera: &'a Camera3D,
}

pub(crate) fn render_particle_layer(buffer: &mut [u8], ctx: ParticleLayerCtx<'_>) -> bool {
    let ParticleLayerCtx {
        comp,
        layer,
        frame,
        sorted_idx,
        source_frame,
        position: pos,
        opacity: l_opacity,
        width,
        height,
        camera: render_camera,
    } = ctx;
    // then render particles directly into the composite buffer.
    if let LayerType::Particle { emitter } = &layer.layer_type {
        let mut em = emitter.clone();
        // Bake layer opacity into particle colors
        em.color_start[3] *= l_opacity;
        em.color_end[3] *= l_opacity;

        // Simulation is deterministic from frame 0, so memoize the simulated
        // state per (version, frame, emitter) — playback then costs O(1) per
        // particle layer instead of O(frame).
        thread_local! {
            static PARTICLE_SIM_CACHE: std::cell::RefCell<Option<ParticleSimCacheEntry>> =
                const { std::cell::RefCell::new(None) };
        }

        let em_bits: [u32; 15] = [
            em.rate.to_bits(),
            em.lifetime.to_bits(),
            em.speed.to_bits(),
            em.spread_degrees.to_bits(),
            em.gravity[0].to_bits(),
            em.gravity[1].to_bits(),
            em.color_start[0].to_bits(),
            em.color_end[3].to_bits(),
            em.emitter_size[0].to_bits(),
            em.emitter_size[1].to_bits(),
            em.max_particles,
            (em.shape as u32),
            em.depth_enabled as u32,
            em.depth_range[0].to_bits(),
            em.depth_range[1].to_bits(),
        ];
        let sim_key = (
            crate::core::frame_cache::current_version(),
            layer.id.clone(),
            frame.min(2000),
            em_bits,
        );
        let dt = 1.0 / comp.fps.max(1) as f32;
        let depth_enabled = em.depth_enabled;

        let ps = PARTICLE_SIM_CACHE.with(|cache| {
            let mut cache_ref = cache.borrow_mut();
            if let Some((v, ref id, f, ref bits, ref mut cached_ps)) = *cache_ref {
                if v == sim_key.0 && *id == sim_key.1 && *bits == sim_key.3 {
                    if f == sim_key.2 {
                        return cached_ps.clone();
                    }
                    // Sequential playback forward: step incrementally from cached frame!
                    if sim_key.2 > f && sim_key.2 <= f + 30 {
                        for _ in (f + 1)..=sim_key.2 {
                            cached_ps.update(dt, pos[0], pos[1]);
                        }
                        let updated = cached_ps.clone();
                        *cache_ref = Some((
                            sim_key.0,
                            sim_key.1.clone(),
                            sim_key.2,
                            sim_key.3,
                            updated.clone(),
                        ));
                        return updated;
                    }
                }
            }
            // Fallback: new simulation from frame 0 up to current frame
            let mut ps = crate::core::particle_system::ParticleSystem::new(em);
            let sim_frames = frame.min(2000);
            for _ in 0..=sim_frames {
                ps.update(dt, pos[0], pos[1]);
            }
            *cache_ref = Some((
                sim_key.0,
                sim_key.1.clone(),
                sim_key.2,
                sim_key.3,
                ps.clone(),
            ));
            ps
        });
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
            ps.render_projected(buffer, width, height, frame as f32 * dt, Some(&proj));
        } else {
            ps.render(buffer, width, height, frame as f32 * dt);
        }

        // Apply the layer's CPU effect stack to the full frame
        if layer.effects_enabled {
            crate::core::cpu_effects::apply_layer_effects(
                Some(comp),
                Some(sorted_idx),
                buffer,
                width,
                height,
                &layer.effects,
                source_frame,
                comp.fps,
            );
        }
        return true;
    }
    false
}
