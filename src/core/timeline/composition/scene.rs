use super::*;

impl Composition {
    /// Move camera/light scene objects referenced by `layers` into a nested
    /// composition. Timeline layers keep their stable object IDs, so moving
    /// the matching scene entries is enough to preserve render semantics.
    pub fn move_scene_objects_to(&mut self, destination: &mut Composition, layers: &[Layer]) {
        let camera_ids: std::collections::HashSet<&str> = layers
            .iter()
            .filter_map(|layer| match layer.scene_object.as_ref() {
                Some(SceneObjectRef::Camera { id }) => Some(id.as_str()),
                _ => None,
            })
            .collect();
        let light_ids: std::collections::HashSet<&str> = layers
            .iter()
            .filter_map(|layer| match layer.scene_object.as_ref() {
                Some(SceneObjectRef::Light { id }) => Some(id.as_str()),
                _ => None,
            })
            .collect();

        let shared_camera_ids: std::collections::HashSet<&str> = camera_ids
            .iter()
            .copied()
            .filter(|id| {
                self.layers.iter().any(|layer| {
                    matches!(
                        layer.scene_object.as_ref(),
                        Some(SceneObjectRef::Camera { id: layer_id }) if layer_id == id
                    )
                })
            })
            .collect();
        let mut moved_cameras = Vec::new();
        let mut copied_cameras = Vec::new();
        self.cameras.retain(|camera| {
            if camera_ids.contains(camera.id.as_str()) {
                if shared_camera_ids.contains(camera.id.as_str()) {
                    copied_cameras.push(camera.clone());
                    true
                } else {
                    moved_cameras.push(camera.clone());
                    false
                }
            } else {
                true
            }
        });
        let mut incoming_cameras = moved_cameras;
        incoming_cameras.extend(copied_cameras);
        if incoming_cameras.iter().any(|camera| camera.active) {
            for camera in &mut destination.cameras {
                camera.active = false;
            }
        }
        for camera in incoming_cameras {
            destination
                .cameras
                .retain(|existing| existing.id != camera.id);
            destination.cameras.push(camera);
        }

        let shared_light_ids: std::collections::HashSet<&str> = light_ids
            .iter()
            .copied()
            .filter(|id| {
                self.layers.iter().any(|layer| {
                    matches!(
                        layer.scene_object.as_ref(),
                        Some(SceneObjectRef::Light { id: layer_id }) if layer_id == id
                    )
                })
            })
            .collect();
        let mut moved_lights = Vec::new();
        let mut copied_lights = Vec::new();
        self.lights.retain(|light| {
            if light_ids.contains(light.id.as_str()) {
                if shared_light_ids.contains(light.id.as_str()) {
                    copied_lights.push(light.clone());
                    true
                } else {
                    moved_lights.push(light.clone());
                    false
                }
            } else {
                true
            }
        });
        moved_lights.extend(copied_lights);
        for light in moved_lights {
            destination
                .lights
                .retain(|existing| existing.id != light.id);
            destination.lights.push(light);
        }
    }

    /// Resolve the active camera with the transform animated on its timeline
    /// scene layer, while leaving the serialized scene object unchanged.
    pub fn resolve_camera_at(&self) -> Camera3D {
        let mut camera = self.resolve_camera().clone();
        if let Some(layer) = self.layers.iter().find(|layer| {
            matches!(
                layer.scene_object.as_ref(),
                Some(SceneObjectRef::Camera { id }) if id == &camera.id
            )
        }) {
            camera.transform.position = layer.transform_3d.position.clone();
            camera.transform.rotation = layer.transform_3d.rotation.clone();
        }
        camera
    }

    /// Resolve a light position from its timeline scene layer when one exists.
    pub fn light_position_at(&self, light: &Light3D, frame: u32) -> [f32; 3] {
        self.layers
            .iter()
            .find(|layer| {
                matches!(
                    layer.scene_object.as_ref(),
                    Some(SceneObjectRef::Light { id }) if id == &light.id
                )
            })
            .map(|layer| layer.transform_3d.position.evaluate(frame))
            .unwrap_or_else(|| light.position.evaluate(frame))
    }

    /// Look up a sub-composition by id (recursive search).
    /// The camera currently driving the render: first entry of `cameras`
    /// with `active == true`, else the legacy `active_camera` field.
    pub fn resolve_camera(&self) -> &Camera3D {
        self.cameras
            .iter()
            .find(|c| c.active)
            .unwrap_or(&self.active_camera)
    }

    pub fn resolve_camera_mut(&mut self) -> &mut Camera3D {
        if let Some(i) = self.cameras.iter().position(|c| c.active) {
            return &mut self.cameras[i];
        }
        &mut self.active_camera
    }

    /// Activate the camera at `idx` (deactivating others). Passing an index
    /// into `cameras`; out-of-range clears all flags (legacy camera resumes).
    pub fn set_active_camera(&mut self, idx: Option<usize>) {
        for (i, c) in self.cameras.iter_mut().enumerate() {
            c.active = Some(i) == idx;
        }
    }

    pub fn find_sub_comp(&self, comp_id: &str) -> Option<&Composition> {
        Self::find_sub_comp_limited(self, comp_id, 0)
    }

    /// Depth-limited recursive lookup: guards against cyclic sub_compositions graphs.
    fn find_sub_comp_limited<'a>(
        comp: &'a Composition,
        comp_id: &str,
        depth: u32,
    ) -> Option<&'a Composition> {
        const MAX_SUB_COMP_DEPTH: u32 = 32;
        if depth > MAX_SUB_COMP_DEPTH {
            return None;
        }
        for sub in &comp.sub_compositions {
            if sub.id == comp_id {
                return Some(sub);
            }
            if let Some(found) = Self::find_sub_comp_limited(sub, comp_id, depth + 1) {
                return Some(found);
            }
        }
        None
    }
}
