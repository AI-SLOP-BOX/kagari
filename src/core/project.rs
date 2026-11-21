/// Project save/load — serializes compositions to/from JSON files.
use crate::core::timeline::Composition;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

fn write_project_json_atomic(path: &Path, json: &str) -> io::Result<()> {
    crate::core::atomic_file::write_atomic(path, json.as_bytes())
}

fn read_project_json(path: &Path) -> io::Result<String> {
    let file = fs::File::open(path)?;
    let max_bytes = crate::core::project_migration::MAX_PROJECT_JSON_BYTES;
    if file.metadata()?.len() > max_bytes as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Project file exceeds the {max_bytes} byte limit"),
        ));
    }
    let mut data = String::new();
    file.take(max_bytes.saturating_add(1) as u64)
        .read_to_string(&mut data)?;
    if data.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Project file exceeds the {max_bytes} byte limit"),
        ));
    }
    Ok(data)
}

/// Save a composition to a JSON file (.aeproj).
pub fn save_project(comp: &Composition, path: &Path) -> io::Result<()> {
    crate::core::production_document::ProductionDocument::validate_standalone_composition(comp)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let json = serde_json::to_string_pretty(comp)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    crate::core::project_migration::validate_project_json_size(&json)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_project_json_atomic(path, &json)
}

/// Load a composition from a JSON file (.aeproj).
pub fn load_project(path: &Path) -> io::Result<Composition> {
    let data = read_project_json(path)?;
    let composition: Composition =
        serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    crate::core::production_document::ProductionDocument::validate_standalone_composition(
        &composition,
    )
    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(composition)
}

/// Project metadata saved alongside the composition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub composition: Composition,
}

impl Default for ProjectFile {
    fn default() -> Self {
        Self {
            version: 1,
            composition: Composition::new(
                "untitled".into(),
                "Untitled".into(),
                1920,
                1080,
                30,
                300,
            ),
        }
    }
}

/// Save a full project file with version metadata.
pub fn save_project_file(proj: &ProjectFile, path: &Path) -> io::Result<()> {
    crate::core::production_document::ProductionDocument::validate_standalone_composition(
        &proj.composition,
    )
    .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let json = serde_json::to_string_pretty(proj)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    crate::core::project_migration::validate_project_json_size(&json)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_project_json_atomic(path, &json)
}

/// Load a full project file, handling version migration if needed.
pub fn load_project_file(path: &Path) -> io::Result<ProjectFile> {
    let data = read_project_json(path)?;
    let mut proj: ProjectFile =
        serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Future: migrate old versions here
    if proj.version == 0 {
        proj.version = 1;
    } else if proj.version > 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Project version {} is newer than supported version 1",
                proj.version
            ),
        ));
    }

    crate::core::production_document::ProductionDocument::validate_standalone_composition(
        &proj.composition,
    )
    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    Ok(proj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_save_load_roundtrip() {
        let comp = Composition::new("test".into(), "Test".into(), 1920, 1080, 30, 100);
        let dir = temp_dir();
        let path = dir.join("test_project.aeproj");

        save_project(&comp, &path).unwrap();
        let loaded = load_project(&path).unwrap();

        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.width, 1920);
        assert_eq!(loaded.height, 1080);
        assert_eq!(loaded.fps, 30);
        assert_eq!(loaded.duration_frames, 100);

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_project_file_roundtrip() {
        let proj = ProjectFile::default();
        let dir = temp_dir();
        let path = dir.join("test_project_file.aeproj");

        save_project_file(&proj, &path).unwrap();
        let loaded = load_project_file(&path).unwrap();

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.composition.name, "Untitled");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_project(Path::new("/nonexistent/path.aeproj"));
        assert!(result.is_err());
    }
}
