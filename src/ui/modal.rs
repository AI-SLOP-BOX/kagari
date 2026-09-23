pub fn window<'a>(title: impl Into<eframe::egui::WidgetText>) -> eframe::egui::Window<'a> {
    eframe::egui::Window::new(title).collapsible(false)
}
