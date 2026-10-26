use super::*;

impl Composition {
    pub fn resolve_world_transform(
        &self,
        layer: &Layer,
        frame: u32,
    ) -> ([f32; 2], [f32; 2], f32, f32) {
        self.resolve_world_transform_limited(layer, frame, 0)
    }

    /// Depth-limited transform resolution: guards against cyclic parent_id chains
    /// (e.g. from hand-edited or corrupted project files) that would recurse forever.
    fn resolve_world_transform_limited(
        &self,
        layer: &Layer,
        frame: u32,
        depth: u32,
    ) -> ([f32; 2], [f32; 2], f32, f32) {
        const MAX_PARENT_DEPTH: u32 = 32;
        if depth > MAX_PARENT_DEPTH {
            // Cycle detected — fall back to the layer's own local transform
            let fps = self.fps;
            return (
                layer.transform.eval_position(frame, fps),
                layer.transform.eval_scale(frame, fps),
                layer.transform.eval_rotation(frame, fps),
                layer.transform.eval_opacity(frame, fps),
            );
        }
        let fps = self.fps;

        // Layer expressions with composition context (thisComp.layer(...) / thisLayer)
        let has_exprs = layer.transform.position_expression.is_some()
            || layer.transform.rotation_expression.is_some()
            || layer.transform.scale_expression.is_some()
            || layer.transform.opacity_expression.is_some();
        let (pos, scale, rot, opa) = if has_exprs {
            let comp_snap = expression_engine::build_comp_snapshot(self, frame);
            let this_snap = comp_snap.layers.get(&layer.name).cloned();
            fn raw_script(e: &Option<Expression>) -> Option<&String> {
                match e {
                    Some(Expression::Raw(s)) => Some(s),
                    _ => None,
                }
            }
            let base_pos = layer.transform.eval_position(frame, fps);
            let pos = raw_script(&layer.transform.position_expression)
                .map(|s| {
                    expression_engine::eval_v2_with_comp_and_property(
                        s,
                        base_pos,
                        frame,
                        fps,
                        &comp_snap,
                        this_snap.as_ref(),
                        &layer.transform.position,
                    )
                })
                .unwrap_or(base_pos);
            let base_scale = layer.transform.eval_scale(frame, fps);
            let scale = raw_script(&layer.transform.scale_expression)
                .map(|s| {
                    expression_engine::eval_v2_with_comp_and_property(
                        s,
                        base_scale,
                        frame,
                        fps,
                        &comp_snap,
                        this_snap.as_ref(),
                        &layer.transform.scale,
                    )
                })
                .unwrap_or(base_scale);
            let base_rot = layer.transform.eval_rotation(frame, fps);
            let rot = raw_script(&layer.transform.rotation_expression)
                .map(|s| {
                    expression_engine::eval_f32_with_comp_and_property(
                        s,
                        base_rot,
                        frame,
                        fps,
                        &comp_snap,
                        this_snap.as_ref(),
                        &layer.transform.rotation,
                    )
                })
                .unwrap_or(base_rot);
            let base_opa = layer.transform.eval_opacity(frame, fps);
            let opa = raw_script(&layer.transform.opacity_expression)
                .map(|s| {
                    expression_engine::eval_f32_with_comp_and_property(
                        s,
                        base_opa,
                        frame,
                        fps,
                        &comp_snap,
                        this_snap.as_ref(),
                        &layer.transform.opacity,
                    )
                })
                .unwrap_or(base_opa);
            (pos, scale, rot, opa)
        } else {
            (
                layer.transform.eval_position(frame, fps),
                layer.transform.eval_scale(frame, fps),
                layer.transform.eval_rotation(frame, fps),
                layer.transform.eval_opacity(frame, fps),
            )
        };

        // ── Auto-Orient: override rotation from motion path / target point ──
        let rot = match layer.auto_orient {
            crate::core::auto_orient::AutoOrientMode::Off => rot,
            mode => crate::core::auto_orient::evaluate_auto_orient_rotation(layer, frame, mode)
                .unwrap_or(rot),
        };

        if let Some(pid) = &layer.parent_id {
            if let Some(parent) = self.layers.iter().find(|l| &l.id == pid) {
                let (ppos, pscale, prot, popa) =
                    self.resolve_world_transform_limited(parent, frame, depth + 1);
                let rot_rad = prot.to_radians();
                let (s, c) = rot_rad.sin_cos();
                let world_x =
                    pos[0] * pscale[0] / 100.0 * c - pos[1] * pscale[1] / 100.0 * s + ppos[0];
                let world_y =
                    pos[0] * pscale[0] / 100.0 * s + pos[1] * pscale[1] / 100.0 * c + ppos[1];
                return (
                    [world_x, world_y],
                    [scale[0] * pscale[0] / 100.0, scale[1] * pscale[1] / 100.0],
                    rot + prot,
                    opa * popa / 100.0,
                );
            }
        }
        (pos, scale, rot, opa)
    }

    /// O(1) Layer ID to index lookup map generator.
    pub fn build_layer_id_index_map(&self) -> std::collections::HashMap<&str, usize> {
        let mut map = std::collections::HashMap::with_capacity(self.layers.len());
        for (idx, layer) in self.layers.iter().enumerate() {
            map.insert(layer.id.as_str(), idx);
        }
        map
    }

    /// Calculate and cache all layer world transforms in O(N) linear topological order with cycle protection and O(1) layer lookups.
    pub fn resolve_all_world_transforms_cached(
        &self,
        frame: u32,
    ) -> std::collections::HashMap<String, ([f32; 2], [f32; 2], f32, f32)> {
        let mut cache = std::collections::HashMap::with_capacity(self.layers.len());
        let mut visited = std::collections::HashSet::new();
        let id_map = self.build_layer_id_index_map();

        for layer in &self.layers {
            if !cache.contains_key(&layer.id) {
                visited.clear();
                self.resolve_layer_transform_recursive(
                    layer,
                    frame,
                    &mut cache,
                    &mut visited,
                    &id_map,
                );
            }
        }
        cache
    }

    fn resolve_layer_transform_recursive(
        &self,
        layer: &Layer,
        frame: u32,
        cache: &mut std::collections::HashMap<String, BoundingBox>,
        visited: &mut std::collections::HashSet<String>,
        id_map: &std::collections::HashMap<&str, usize>,
    ) -> BoundingBox {
        if let Some(cached) = cache.get(&layer.id) {
            return *cached;
        }

        let fps = self.fps;
        let pos = layer.transform.eval_position(frame, fps);
        let scale = layer.transform.eval_scale(frame, fps);
        let rot = layer.transform.eval_rotation(frame, fps);
        let opa = layer.transform.eval_opacity(frame, fps);

        if !visited.insert(layer.id.clone()) {
            // Cycle circuit breaker!
            return (pos, scale, rot, opa);
        }

        let res = if let Some(pid) = &layer.parent_id {
            if let Some(&parent_idx) = id_map.get(pid.as_str()) {
                let parent = &self.layers[parent_idx];
                let (ppos, pscale, prot, popa) =
                    self.resolve_layer_transform_recursive(parent, frame, cache, visited, id_map);
                let rot_rad = prot.to_radians();
                let (s, c) = (rot_rad.sin(), rot_rad.cos());
                let world_x =
                    pos[0] * pscale[0] / 100.0 * c - pos[1] * pscale[1] / 100.0 * s + ppos[0];
                let world_y =
                    pos[0] * pscale[0] / 100.0 * s + pos[1] * pscale[1] / 100.0 * c + ppos[1];
                (
                    [world_x, world_y],
                    [scale[0] * pscale[0] / 100.0, scale[1] * pscale[1] / 100.0],
                    rot + prot,
                    opa * popa / 100.0,
                )
            } else {
                (pos, scale, rot, opa)
            }
        } else {
            (pos, scale, rot, opa)
        };

        visited.remove(&layer.id);
        cache.insert(layer.id.clone(), res);
        res
    }

    pub fn resolve_world_transform_3d(&self, layer: &Layer, frame: u32) -> [[f32; 4]; 4] {
        let pos3d = layer.transform_3d.position.evaluate(frame);
        let rot3d = layer.transform_3d.rotation.evaluate(frame);
        let scale3d = layer.transform_3d.scale.evaluate(frame);

        // Perspective Projection Matrix for Camera
        let render_camera = self.resolve_camera_at();
        let fov_rad = render_camera.fov_at(frame).to_radians();
        let aspect = self.width as f32 / self.height.max(1) as f32;
        // Guard against degenerate FOV (e.g. fov_degrees=0 → tan(0)=0 → division by zero)
        let f = if fov_rad.abs() < 1e-4 {
            1e6
        } else {
            1.0 / (fov_rad * 0.5).tan()
        };
        let near = 0.1f32;
        let far = 10000.0f32;

        let proj_3d = [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, -f, 0.0, 0.0],
            [0.0, 0.0, far / (far - near), 1.0],
            [0.0, 0.0, -(far * near) / (far - near), 0.0],
        ];

        let rad_x = rot3d[0].to_radians();
        let rad_y = rot3d[1].to_radians();
        let rad_z = rot3d[2].to_radians();

        let cx = rad_x.cos();
        let sx = rad_x.sin();
        let cy = rad_y.cos();
        let sy = rad_y.sin();
        let cz = rad_z.cos();
        let sz = rad_z.sin();

        // Euler rotation matrix Z * Y * X
        let rot_matrix = [
            [cy * cz, cy * sz, -sy, 0.0],
            [sx * sy * cz - cx * sz, sx * sy * sz + cx * cz, sx * cy, 0.0],
            [cx * sy * cz + sx * sz, cx * sy * sz - sx * cz, cx * cy, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let scale_matrix = [
            [scale3d[0] / 100.0, 0.0, 0.0, 0.0],
            [0.0, scale3d[1] / 100.0, 0.0, 0.0],
            [0.0, 0.0, scale3d[2] / 100.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let pos_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [
                pos3d[0] / (self.width as f32 * 0.5),
                pos3d[1] / (self.height as f32 * 0.5),
                pos3d[2] / 1000.0,
                1.0,
            ],
        ];

        fn m4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
            let mut out = [[0.0; 4]; 4];
            for r in 0..4 {
                for c in 0..4 {
                    out[r][c] = a[r][0] * b[0][c]
                        + a[r][1] * b[1][c]
                        + a[r][2] * b[2][c]
                        + a[r][3] * b[3][c];
                }
            }
            out
        }

        m4_mul(
            proj_3d,
            m4_mul(pos_matrix, m4_mul(rot_matrix, scale_matrix)),
        )
    }
}
