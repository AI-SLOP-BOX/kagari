use crate::core::timeline::Project;
use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROJECT_JSON_BYTES: usize = 100 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedProjectFile {
    pub schema_version: u32,
    pub project_data: serde_json::Value,
}

/// Load a project JSON string safely, executing schema migrations if needed.
pub fn load_project_migrated(json_str: &str) -> Result<Project, String> {
    if json_str.len() > MAX_PROJECT_JSON_BYTES {
        return Err(format!(
            "Project JSON exceeds the {} MiB limit",
            MAX_PROJECT_JSON_BYTES / (1024 * 1024)
        ));
    }

    let mut proj: Project = if let Ok(proj) = serde_json::from_str::<Project>(json_str) {
        proj
    } else if let Ok(wrapper) = serde_json::from_str::<VersionedProjectFile>(json_str) {
        let migrated_val = migrate_schema_json(wrapper.schema_version, wrapper.project_data)?;
        serde_json::from_value::<Project>(migrated_val)
            .map_err(|e| format!("Failed to parse project after migration: {}", e))?
    } else {
        let val: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid JSON project file: {}", e))?;
        let schema_ver = val
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let migrated = migrate_schema_json(schema_ver, val)?;
        serde_json::from_value::<Project>(migrated)
            .map_err(|e| format!("Schema migration error: {}", e))?
    };

    for comp in &mut proj.compositions {
        comp.sanitize_parent_cycles();
    }

    crate::core::production_document::ProductionDocument::new(proj.clone())
        .validate()
        .map_err(|e| format!("Project validation failed after migration: {e}"))?;

    Ok(proj)
}

/// Serialize a Project to versioned JSON string.
pub fn save_project_versioned(proj: &Project) -> Result<String, String> {
    let proj_val = serde_json::to_value(proj).map_err(|e| e.to_string())?;
    let wrapper = VersionedProjectFile {
        schema_version: CURRENT_SCHEMA_VERSION,
        project_data: proj_val,
    };
    serde_json::to_string_pretty(&wrapper).map_err(|e| e.to_string())
}

/// Migrate JSON schema from `from_version` to `CURRENT_SCHEMA_VERSION`.
pub fn migrate_schema_json(
    from_version: u32,
    mut data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if from_version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "Project schema version {} is newer than supported version {}",
            from_version, CURRENT_SCHEMA_VERSION
        ));
    }

    let mut version = from_version;
    while version < CURRENT_SCHEMA_VERSION {
        match version {
            0 => {
                if let Some(obj) = data.as_object_mut() {
                    if !obj.contains_key("name") {
                        obj.insert("name".to_string(), serde_json::json!("Untitled Project"));
                    }
                    if !obj.contains_key("active_composition_idx") {
                        obj.insert("active_composition_idx".to_string(), serde_json::json!(0));
                    }
                    if !obj.contains_key("assets") {
                        obj.insert("assets".to_string(), serde_json::json!([]));
                    }
                }
                version = 1;
            }
            _ => break,
        }
    }

    Ok(data)
}
