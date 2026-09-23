include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

pub fn bytes_for_path(path: &std::path::Path) -> Option<&'static [u8]> {
    if let Ok(relative) = path.strip_prefix(env!("CARGO_MANIFEST_DIR")) {
        return bytes(&relative.to_string_lossy().replace('\\', "/"));
    }
    bytes(&path.to_string_lossy().replace('\\', "/"))
}

pub fn open_image(path: impl AsRef<std::path::Path>) -> image::ImageResult<image::DynamicImage> {
    let path = path.as_ref();
    match image::open(path) {
        Ok(image) => Ok(image),
        Err(file_error) => match bytes_for_path(path) {
            Some(data) => image::load_from_memory(data),
            None => Err(file_error),
        },
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_logo_and_font_are_present() {
        assert!(super::bytes("assets/kagari_logo.webp").is_some());
        assert!(super::bytes("assets/fonts/Inter.ttf").is_some());
    }

    #[test]
    fn embedded_image_fallback_decodes_without_source_tree() {
        let image = super::open_image("assets/kagari_logo.webp").unwrap();
        assert!(image.width() > 0 && image.height() > 0);
    }
}
