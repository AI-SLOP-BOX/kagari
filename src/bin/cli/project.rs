use super::*;

pub(crate) fn load_project(path: &str) -> Result<Project, Box<dyn std::error::Error>> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Failed to stat project file '{}': {}", path, e))?;
    if meta.len() > 100 * 1024 * 1024 {
        return Err(format!(
            "Project file too large ({} MB, limit 100 MB)",
            meta.len() / (1024 * 1024)
        )
        .into());
    }
    let json =
        kagari_vfx::core::project_migration::read_project_json_file(std::path::Path::new(path))
            .map_err(|e| format!("Failed to read project file '{}': {}", path, e))?;
    let (project, _) = kagari_vfx::core::project_migration::load_project_document_or_legacy(&json)
        .map_err(|error| format!("Failed to parse project JSON: {error}"))?;
    Ok(project)
}

pub(crate) fn find_comp<'a>(
    project: &'a Project,
    comp_ref: Option<&str>,
) -> Result<&'a Composition, Box<dyn std::error::Error>> {
    match comp_ref {
        None => {
            if project.compositions.is_empty() {
                return Err("Project has no compositions".into());
            }
            Ok(project.compositions.first().unwrap())
        }
        Some(name) => {
            if let Ok(idx) = name.parse::<usize>() {
                project.compositions.get(idx).ok_or_else(|| {
                    format!(
                        "Composition index {} out of range (project has {})",
                        idx,
                        project.compositions.len()
                    )
                    .into()
                })
            } else {
                project
                    .compositions
                    .iter()
                    .find(|c| c.name == name)
                    .ok_or_else(|| format!("Composition '{}' not found", name).into())
            }
        }
    }
}
