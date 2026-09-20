use eframe::egui;

fn kagari_app_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/kagari_logo.webp"
    )))
    .ok()?;
    let rgba = image.to_rgba8();

    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width: image.width(),
        height: image.height(),
    })
}

fn main() -> eframe::Result<()> {
    env_logger::init();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1536.0, 1024.0])
        .with_min_inner_size([300.0, 220.0])
        .with_title("Kagari VFX");
    if let Some(icon) = kagari_app_icon() {
        viewport = viewport.with_icon(icon);
    } else {
        log::warn!("Unable to decode the embedded Kagari application icon");
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Kagari VFX",
        options,
        Box::new(|cc| {
            let mut app = kagari_vfx::KagariApp::default();

            let (frame_tx, frame_rx) = std::sync::mpsc::channel();
            let (conn_tx, conn_rx) = std::sync::mpsc::channel();
            if let Err(e) =
                kagari_vfx::core::integration::start_sync_server(9000, frame_tx, conn_tx)
            {
                log::warn!("Dynamic Link sync server unavailable on port 9000: {}", e);
            }

            app.rx_frame = Some(frame_rx);
            app.rx_connection = Some(conn_rx);

            #[cfg(feature = "wgpu")]
            if let Some(wgpu_state) = &cc.wgpu_render_state {
                let renderer = kagari_vfx::core::renderer::WgpuRenderer::new(
                    wgpu_state.device.clone(),
                    wgpu_state.queue.clone(),
                );
                app.renderer = Some(renderer);
                app.wgpu_state = Some(wgpu_state.clone());
            }

            kagari_vfx::ui::theme::configure_ae_theme(&cc.egui_ctx);
            kagari_vfx::ui::icons::init_image_loaders(&cc.egui_ctx);

            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}
