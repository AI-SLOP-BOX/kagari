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
    kagari_vfx::ui::localization::load_custom_texts();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1240.0, 780.0])
        .with_min_inner_size([480.0, 320.0])
        .with_decorations(true)
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

            let (frame_tx, frame_rx) = kagari_vfx::core::integration::latest_frame_channel();
            let (conn_tx, conn_rx) = std::sync::mpsc::sync_channel(
                kagari_vfx::core::integration::DYNAMIC_LINK_CONNECTION_QUEUE_CAPACITY,
            );
            let mut dynamic_link_port = None;
            for candidate_port in 9000..=9010 {
                match kagari_vfx::core::integration::start_sync_server(
                    candidate_port,
                    frame_tx.clone(),
                    conn_tx.clone(),
                ) {
                    Ok(port) => {
                        dynamic_link_port = Some(port);
                        break;
                    }
                    Err(e) => {
                        log::debug!("Port {candidate_port} busy or unavailable: {e}");
                    }
                }
            }
            if let Some(port) = dynamic_link_port {
                log::info!("Dynamic Link sync server successfully active on port {port}");
            } else {
                log::warn!("Dynamic Link sync server could not bind to ports 9000-9010");
                app.toasts.warning("Dynamic Link: ポート9000〜9010が使用中のため、同期サーバーを起動できませんでした");
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

            kagari_vfx::ui::preferences_dialog::apply_loaded(&mut app);
            kagari_vfx::ui::home_screen::trigger_recent_projects_scan(&mut app, &cc.egui_ctx);
            kagari_vfx::ui::theme::configure_ae_theme(&cc.egui_ctx);
            kagari_vfx::ui::icons::init_image_loaders(&cc.egui_ctx);
            kagari_vfx::core::font_rasterizer::init_font_rasterizer();

            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}
