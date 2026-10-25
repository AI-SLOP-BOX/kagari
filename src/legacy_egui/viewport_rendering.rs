pub fn software_preview_required(comp: &crate::core::timeline::Composition, frame: u32) -> bool {
    crate::core::render_capability::software_preview_required(comp, frame)
}
