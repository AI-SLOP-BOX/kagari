use super::*;

pub(crate) fn cmd_mask(
    project_path: &str,
    layer_ref: &str,
    shape: &str,
    coords: &str,
    feather: f32,
    inverted: bool,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::mask::Mask;
    use kagari_vfx::core::property::Animatable;

    let mut project = load_project(project_path)?;
    let comp = find_comp_mut(&mut project, comp_ref)?;

    let layer = comp
        .layers
        .iter_mut()
        .find(|l| l.name == layer_ref || l.id == layer_ref)
        .ok_or_else(|| {
            format!(
                "Layer '{layer_ref}' not found in composition '{}'",
                comp.name
            )
        })?;

    let parts = parse_finite_components(coords, "Mask coordinates")?;
    let mask_id = format!("mask_{}", layer.masks.len());
    let mask_name = format!("Mask {}", layer.masks.len() + 1);

    let mut mask = match shape.to_lowercase().as_str() {
        "rect" | "rectangle" => {
            if parts.len() < 4 {
                return Err("Rect mask requires coords 'x,y,w,h'".into());
            }
            Mask::new_rect(mask_id, mask_name, parts[0], parts[1], parts[2], parts[3])
        }
        "ellipse" | "circle" => {
            if parts.len() < 4 {
                return Err("Ellipse mask requires coords 'cx,cy,rx,ry'".into());
            }
            Mask::new_ellipse(mask_id, mask_name, parts[0], parts[1], parts[2], parts[3])
        }
        "polygon" | "poly" | "path" => {
            if parts.len() < 6 || parts.len() % 2 != 0 {
                return Err(
                    "Polygon mask requires at least 3 points 'x1,y1,x2,y2,x3,y3...'".into(),
                );
            }
            let mut verts = Vec::with_capacity(parts.len() / 2);
            for chunk in parts.as_chunks::<2>().0 {
                verts.push([chunk[0], chunk[1]]);
            }
            Mask::new_closed(mask_id, mask_name, verts)
        }
        other => {
            return Err(format!(
                "Unknown mask shape '{}'. Valid: rect, ellipse, polygon",
                other
            )
            .into())
        }
    };

    mask.feather = Animatable::new_constant(feather.max(0.0));
    mask.inverted = inverted;

    let layer_name = layer.name.clone();
    layer.masks.push(mask);

    save_project(project_path, &project)?;
    eprintln!(
        "Added {} mask (feather: {}px, inverted: {}) to layer '{}' in {}",
        shape, feather, inverted, layer_name, project_path
    );
    Ok(())
}
