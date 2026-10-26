use super::rasterizer::FontRasterizer;
use std::sync::OnceLock;

pub static GLOBAL_FONT_RASTERIZER: OnceLock<std::sync::Mutex<FontRasterizer>> = OnceLock::new();

/// Initialize the global font rasterizer and load default system fonts.
pub fn init_font_rasterizer() {
    let _ = GLOBAL_FONT_RASTERIZER.set(std::sync::Mutex::new({
        let mut rasterizer = FontRasterizer::new();
        rasterizer.load_all_system_fonts();
        rasterizer
    }));
}

/// Access the global font rasterizer immutably.
pub fn with_font_rasterizer<F, R>(f: F) -> R
where
    F: FnOnce(&FontRasterizer) -> R,
{
    let rasterizer = GLOBAL_FONT_RASTERIZER.get_or_init(|| {
        let mut r = FontRasterizer::new();
        r.load_all_system_fonts();
        std::sync::Mutex::new(r)
    });
    let mut lock = rasterizer.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut lock)
}

/// Access the global font rasterizer mutably.
pub fn with_font_rasterizer_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut FontRasterizer) -> R,
{
    let rasterizer = GLOBAL_FONT_RASTERIZER.get_or_init(|| {
        let mut r = FontRasterizer::new();
        r.load_all_system_fonts();
        std::sync::Mutex::new(r)
    });
    let mut lock = rasterizer.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut lock)
}
