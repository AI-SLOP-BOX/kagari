use super::load_project;

pub(super) fn cmd_validate(project_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    if project.compositions.is_empty() {
        errors.push("Project has no compositions".to_string());
    }

    for comp in project.compositions.iter() {
        if comp.width == 0 || comp.height == 0 {
            errors.push(format!(
                "Composition '{}': invalid dimensions {}x{}",
                comp.name, comp.width, comp.height
            ));
        }
        if comp.width > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
            || comp.height > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
        {
            errors.push(format!(
                "Composition '{}': dimensions {}x{} exceed render limit {}",
                comp.name,
                comp.width,
                comp.height,
                kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
            ));
        }
        if comp.layers.len() > 10_000 {
            warnings.push(format!(
                "Composition '{}': very high layer count ({}), rendering may be slow",
                comp.name,
                comp.layers.len()
            ));
        }
        if comp.fps == 0 {
            errors.push(format!("Composition '{}': FPS is 0", comp.name));
        }
        if comp.duration_frames == 0 {
            warnings.push(format!("Composition '{}': duration is 0 frames", comp.name));
        }

        for layer in comp.layers.iter() {
            if layer.in_frame >= layer.out_frame {
                warnings.push(format!(
                    "Composition '{}' layer '{}': in_frame >= out_frame ({} >= {})",
                    comp.name, layer.name, layer.in_frame, layer.out_frame
                ));
            }

            if let Some(ref parent_id) = layer.parent_id {
                if !comp.layers.iter().any(|l| &l.id == parent_id) {
                    errors.push(format!(
                        "Composition '{}' layer '{}': parent '{}' not found",
                        comp.name, layer.name, parent_id
                    ));
                }
            }

            for effect in &layer.effects {
                if effect.name.is_empty() {
                    warnings.push(format!(
                        "Composition '{}' layer '{}': effect has empty name",
                        comp.name, layer.name
                    ));
                }
            }
        }
    }

    // ── Project-wide pre-comp graph analysis (references + cycle detection) ──
    {
        use std::collections::{HashMap, HashSet};
        // Collect every composition in the project (top-level + nested sub-comps)
        let mut all_comps: HashMap<&str, &kagari_vfx::core::timeline::Composition> = HashMap::new();
        fn collect<'a>(
            comp: &'a kagari_vfx::core::timeline::Composition,
            all: &mut HashMap<&'a str, &'a kagari_vfx::core::timeline::Composition>,
        ) {
            if all.insert(comp.id.as_str(), comp).is_some() {
                return;
            }
            for sub in &comp.sub_compositions {
                collect(sub, all);
            }
        }
        for comp in project.compositions.iter() {
            collect(comp, &mut all_comps);
        }

        // Build edges: comp id -> referenced pre-comp ids
        let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();
        for (id, comp) in &all_comps {
            let mut refs = Vec::new();
            for layer in &comp.layers {
                if let kagari_vfx::core::timeline::LayerType::PreComp { comp_id } =
                    &layer.layer_type
                {
                    if !all_comps.contains_key(comp_id.as_str()) {
                        errors.push(format!(
                            "Composition '{}': pre-comp '{}' referenced but not found",
                            comp.name, comp_id
                        ));
                    } else {
                        refs.push(comp_id.as_str());
                    }
                }
            }
            edges.insert(*id, refs);
        }

        // DFS cycle detection across all comps
        fn dfs(
            node: &str,
            edges: &HashMap<&str, Vec<&str>>,
            visiting: &mut Vec<String>,
            visited: &mut HashSet<String>,
            reported: &mut HashSet<String>,
            errors: &mut Vec<String>,
        ) {
            if reported.contains(node) {
                return;
            }
            if let Some(pos) = visiting.iter().position(|v| v == node) {
                let cycle_path: Vec<String> = visiting[pos..].to_vec();
                errors.push(format!(
                    "Pre-comp cycle detected: {}",
                    cycle_path.join(" -> ")
                ));
                reported.insert(node.to_string());
                return;
            }
            if visited.contains(node) {
                return;
            }
            visiting.push(node.to_string());
            if let Some(refs) = edges.get(node) {
                for r in refs.clone() {
                    dfs(r, edges, visiting, visited, reported, errors);
                }
            }
            visiting.pop();
            visited.insert(node.to_string());
        }
        let keys: Vec<&str> = edges.keys().copied().collect();
        let mut visiting = Vec::new();
        let mut visited = HashSet::new();
        let mut reported = HashSet::new();
        for k in keys {
            dfs(
                k,
                &edges,
                &mut visiting,
                &mut visited,
                &mut reported,
                &mut errors,
            );
        }
    }

    // ── Per-composition layer checks ──
    for comp in project.compositions.iter() {
        // Expression script sanity
        for layer in &comp.layers {
            let exprs = [
                ("position", &layer.transform.position_expression),
                ("rotation", &layer.transform.rotation_expression),
                ("scale", &layer.transform.scale_expression),
                ("opacity", &layer.transform.opacity_expression),
            ];
            for (prop, expr) in exprs {
                if let Some(kagari_vfx::core::timeline::Expression::Raw(script)) = expr {
                    if script.len() > 10_000 {
                        errors.push(format!(
                            "Composition '{}' layer '{}': {} expression too long ({} bytes)",
                            comp.name,
                            layer.name,
                            prop,
                            script.len()
                        ));
                    }
                }
            }
        }

        // Extreme property values (NaN / out-of-range)
        for layer in &comp.layers {
            let opacity = layer.transform.opacity.evaluate(0);
            if !opacity.is_finite() || !(-1.0..=1e6).contains(&opacity) {
                warnings.push(format!(
                    "Composition '{}' layer '{}': suspicious opacity value {}",
                    comp.name, layer.name, opacity
                ));
            }
            let scale = layer.transform.scale.evaluate(0);
            if !scale.iter().all(|v| v.is_finite()) || scale.iter().any(|v| v.abs() > 1e6) {
                warnings.push(format!(
                    "Composition '{}' layer '{}': suspicious scale value {:?}",
                    comp.name, layer.name, scale
                ));
            }
        }

        // Check for parent cycles
        for layer in &comp.layers {
            if let Some(ref parent_id) = layer.parent_id {
                let mut visited = std::collections::HashSet::new();
                visited.insert(&layer.id);
                let mut current = parent_id;
                loop {
                    if visited.contains(current) {
                        errors.push(format!(
                            "Composition '{}': parent cycle detected involving '{}'",
                            comp.name, current
                        ));
                        break;
                    }
                    visited.insert(current);
                    match comp.layers.iter().find(|l| &l.id == current) {
                        Some(parent) => {
                            if let Some(ref next_parent) = parent.parent_id {
                                current = next_parent;
                            } else {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        println!("✓ Project is valid");
    } else {
        for w in &warnings {
            println!("⚠ Warning: {}", w);
        }
        for e in &errors {
            println!("✗ Error: {}", e);
        }
        println!("\n{} warnings, {} errors", warnings.len(), errors.len());
        if !errors.is_empty() {
            std::process::exit(1);
        }
    }

    Ok(())
}
