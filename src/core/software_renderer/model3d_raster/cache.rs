use crate::core::obj_loader::Mesh3D;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileFingerprint {
    path: std::path::PathBuf,
    revision: u64,
}

type MeshFingerprint = Vec<FileFingerprint>;

struct CachedMesh {
    fingerprint: MeshFingerprint,
    mesh: Arc<Mesh3D>,
    byte_size: usize,
    lru_stamp: u64,
}

struct MeshCache {
    entries: HashMap<String, CachedMesh>,
    current_bytes: usize,
    max_bytes: usize,
    lru_clock: u64,
}

impl MeshCache {
    const MAX_BYTES: usize = 512 * 1024 * 1024;

    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            current_bytes: 0,
            max_bytes: Self::MAX_BYTES,
            lru_clock: 0,
        }
    }

    fn get(&mut self, path: &str) -> Option<(MeshFingerprint, Arc<Mesh3D>)> {
        self.lru_clock = self.lru_clock.wrapping_add(1);
        let stamp = self.lru_clock;
        let entry = self.entries.get_mut(path)?;
        entry.lru_stamp = stamp;
        Some((entry.fingerprint.clone(), entry.mesh.clone()))
    }

    fn remove(&mut self, path: &str) {
        if let Some(entry) = self.entries.remove(path) {
            self.current_bytes = self.current_bytes.saturating_sub(entry.byte_size);
        }
    }

    fn insert(
        &mut self,
        path: String,
        fingerprint: MeshFingerprint,
        mesh: Arc<Mesh3D>,
        byte_size: usize,
    ) {
        self.remove(&path);
        // Keep the cache within its advertised memory ceiling. The caller
        // still owns the loaded Arc for this render; oversized assets simply
        // remain uncached and can be released afterwards.
        if byte_size > self.max_bytes {
            return;
        }
        self.lru_clock = self.lru_clock.wrapping_add(1);
        while self.current_bytes.saturating_add(byte_size) > self.max_bytes {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.lru_stamp)
                .map(|(path, _)| path.clone())
            else {
                break;
            };
            self.remove(&oldest);
        }
        self.current_bytes += byte_size;
        self.entries.insert(
            path,
            CachedMesh {
                fingerprint,
                mesh,
                byte_size,
                lru_stamp: self.lru_clock,
            },
        );
    }
}

fn reported_mesh_errors() -> &'static Mutex<HashSet<String>> {
    static REPORTED_ERRORS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    REPORTED_ERRORS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn report_mesh_load_error_once(path: &str, error: &str) {
    let should_report = reported_mesh_errors().lock().is_ok_and(|mut paths| {
        if paths.len() >= 1024 {
            paths.clear();
        }
        paths.insert(path.to_owned())
    });
    if should_report {
        log::warn!("[Renderer] Could not load 3D model '{}': {error}", path);
    }
}

fn clear_mesh_load_error(path: &str) {
    if let Ok(mut paths) = reported_mesh_errors().lock() {
        paths.remove(path);
    }
}

fn reported_texture_errors() -> &'static Mutex<HashSet<String>> {
    static REPORTED_ERRORS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    REPORTED_ERRORS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub(super) fn report_texture_load_error_once(path: &str) {
    let should_report = reported_texture_errors().lock().is_ok_and(|mut paths| {
        if paths.len() >= 1024 {
            paths.clear();
        }
        paths.insert(path.to_owned())
    });
    if should_report {
        log::warn!(
            "[Renderer] Could not load 3D material texture '{}'; using its base color",
            path
        );
    }
}

pub(super) fn clear_texture_load_error(path: &str) {
    if let Ok(mut paths) = reported_texture_errors().lock() {
        paths.remove(path);
    }
}

fn mesh_cache() -> &'static Mutex<MeshCache> {
    static CACHE: OnceLock<Mutex<MeshCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(MeshCache::new()))
}

pub(crate) fn load_mesh(path: &str) -> Option<Arc<Mesh3D>> {
    let source = Path::new(path);
    let extension = source
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase);
    let cached = mesh_cache()
        .lock()
        .ok()
        .and_then(|mut cache| cache.get(path));
    if let Some((cached_fingerprint, mesh)) = cached {
        if fingerprint_is_current(&cached_fingerprint) {
            return Some(mesh);
        }
        if let Ok(mut cache) = mesh_cache().lock() {
            cache.remove(path);
        }
    }
    let obj_source = if extension.as_deref() == Some("obj") {
        match crate::core::obj_loader::read_obj_file_source(source) {
            Ok(source) => Some(source),
            Err(error) => {
                report_mesh_load_error_once(path, &error);
                return None;
            }
        }
    } else {
        None
    };
    let mut dependencies = vec![source.to_path_buf()];
    if matches!(extension.as_deref(), Some("gltf" | "glb")) {
        let Some(external_paths) = gltf_external_buffer_paths(source) else {
            report_mesh_load_error_once(path, "could not resolve glTF external buffers");
            return None;
        };
        dependencies.extend(external_paths);
    } else if extension.as_deref() == Some("obj") {
        if let Some(obj_source) = obj_source.as_ref() {
            dependencies.extend(obj_source.material_paths.iter().cloned());
        }
    }
    let Some(fingerprint) = dependencies
        .iter()
        .map(|dependency| file_fingerprint(dependency))
        .collect::<Option<Vec<_>>>()
    else {
        report_mesh_load_error_once(path, "source or external buffer file is missing");
        return None;
    };
    let loaded_mesh = if matches!(extension.as_deref(), Some("glb" | "gltf")) {
        crate::core::gltf_loader::load_gltf_asset(source)
    } else if let Some(obj_source) = obj_source {
        crate::core::obj_loader::parse_obj_file_source(obj_source)
    } else {
        crate::core::obj_loader::parse_obj_file(source)
    };
    let mesh = match loaded_mesh {
        Ok(mesh) => Arc::new(mesh),
        Err(error) => {
            report_mesh_load_error_once(path, &error);
            return None;
        }
    };
    clear_mesh_load_error(path);
    if let Ok(mut cache) = mesh_cache().lock() {
        let byte_size = mesh_approximate_bytes(&mesh);
        cache.insert(path.to_string(), fingerprint, mesh.clone(), byte_size);
    }
    Some(mesh)
}

fn mesh_approximate_bytes(mesh: &Mesh3D) -> usize {
    let mut bytes = mesh
        .vertices
        .capacity()
        .saturating_mul(std::mem::size_of::<[f32; 3]>())
        .saturating_add(
            mesh.normals
                .capacity()
                .saturating_mul(std::mem::size_of::<[f32; 3]>()),
        )
        .saturating_add(
            mesh.uvs
                .capacity()
                .saturating_mul(std::mem::size_of::<[f32; 2]>()),
        )
        .saturating_add(
            mesh.colors
                .capacity()
                .saturating_mul(std::mem::size_of::<[f32; 4]>()),
        )
        .saturating_add(
            mesh.indices
                .capacity()
                .saturating_mul(std::mem::size_of::<u32>()),
        )
        .saturating_add(
            mesh.material_indices
                .capacity()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(
            mesh.materials
                .capacity()
                .saturating_mul(std::mem::size_of::<crate::core::obj_loader::PbrMaterial3D>()),
        )
        .saturating_add(mesh.name.capacity())
        .saturating_add(
            mesh.uv_sets
                .capacity()
                .saturating_mul(std::mem::size_of::<Vec<[f32; 2]>>()),
        )
        .saturating_add(
            mesh.embedded_textures
                .capacity()
                .saturating_mul(std::mem::size_of::<
                    Arc<crate::core::image_cache::CachedImage>,
                >()),
        );
    for set in &mesh.uv_sets {
        bytes = bytes.saturating_add(
            set.capacity()
                .saturating_mul(std::mem::size_of::<[f32; 2]>()),
        );
    }
    for material in &mesh.materials {
        bytes = bytes.saturating_add(material.name.capacity());
        if let Some(path) = &material.albedo_texture {
            bytes = bytes.saturating_add(path.capacity());
        }
        if let Some(path) = &material.normal_map {
            bytes = bytes.saturating_add(path.capacity());
        }
    }
    for texture in &mesh.embedded_textures {
        bytes = bytes.saturating_add(texture.pixels.capacity());
    }
    bytes
}

fn file_fingerprint(path: &Path) -> Option<FileFingerprint> {
    Some(FileFingerprint {
        path: path.to_path_buf(),
        revision: crate::core::media_revision::existing_media_file_revision(path)?,
    })
}

fn fingerprint_is_current(fingerprint: &[FileFingerprint]) -> bool {
    !fingerprint.is_empty()
        && fingerprint.iter().all(|expected| {
            crate::core::media_revision::existing_media_file_revision(&expected.path)
                == Some(expected.revision)
        })
}

fn gltf_external_buffer_paths(source: &Path) -> Option<Vec<std::path::PathBuf>> {
    let bytes = crate::core::gltf_loader::read_gltf_file(source).ok()?;
    let json = if source
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("glb"))
    {
        crate::core::gltf_loader::glb_json_chunk(&bytes).ok()?
    } else {
        bytes.as_slice()
    };
    let root: crate::core::gltf_loader::GltfRootJson = serde_json::from_slice(json).ok()?;
    let mut paths = root
        .buffers
        .iter()
        .filter_map(|buffer| buffer.uri.as_deref())
        .filter(|uri| !uri.starts_with("data:"))
        .map(|uri| crate::core::gltf_loader::resolve_gltf_buffer_path(source, uri).ok())
        .collect::<Option<Vec<_>>>()?;
    for image in &root.images {
        if let Some(uri) = image.uri.as_deref().filter(|uri| !uri.starts_with("data:")) {
            paths.push(crate::core::gltf_loader::resolve_gltf_buffer_path(source, uri).ok()?);
        }
    }
    Some(paths)
}
