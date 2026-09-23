use eframe::egui;

pub fn window<'a>(title: impl Into<egui::WidgetText>) -> egui::Window<'a> {
    egui::Window::new(title).collapsible(false)
}

pub struct DialogWindow<'open> {
    window: egui::Window<'open>,
    open: Option<&'open mut bool>,
}

pub fn dialog<'open>(title: impl Into<egui::WidgetText>) -> DialogWindow<'open> {
    DialogWindow {
        window: egui::Window::new(title)
            .collapsible(false)
            .order(egui::Order::Foreground),
        open: None,
    }
}

impl<'open> DialogWindow<'open> {
    pub fn open(mut self, open: &'open mut bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.window = self.window.collapsible(collapsible);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.window = self.window.resizable(resizable);
        self
    }

    pub fn default_width(mut self, width: f32) -> Self {
        self.window = self.window.default_width(width);
        self
    }

    pub fn default_height(mut self, height: f32) -> Self {
        self.window = self.window.default_height(height);
        self
    }

    pub fn default_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.window = self.window.default_size(size);
        self
    }

    pub fn fixed_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.window = self.window.fixed_size(size);
        self
    }

    pub fn min_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.window = self.window.min_size(size);
        self
    }

    pub fn min_width(mut self, width: f32) -> Self {
        self.window = self.window.min_width(width);
        self
    }

    pub fn anchor(mut self, align: egui::Align2, offset: impl Into<egui::Vec2>) -> Self {
        self.window = self.window.anchor(align, offset);
        self
    }

    pub fn fixed_pos(mut self, pos: impl Into<egui::Pos2>) -> Self {
        self.window = self.window.fixed_pos(pos);
        self
    }

    pub fn title_bar(mut self, title_bar: bool) -> Self {
        self.window = self.window.title_bar(title_bar);
        self
    }

    pub fn frame(mut self, frame: egui::Frame) -> Self {
        self.window = self.window.frame(frame);
        self
    }

    pub fn show<R>(
        mut self,
        ctx: &egui::Context,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<egui::InnerResponse<Option<R>>> {
        let text_input_focused = crate::ui::focus::is_text_input_focused(ctx);
        let escape_pressed = !text_input_focused
            && ctx.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
            });
        if escape_pressed {
            if let Some(open) = self.open.as_deref_mut() {
                *open = false;
            }
        }
        if let Some(open) = self.open {
            self.window = self.window.open(open);
        }

        let screen_rect = ctx.screen_rect();
        egui::Area::new(egui::Id::new("kagari_modal_backdrop"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                ui.set_min_size(screen_rect.size());
                let (rect, _) =
                    ui.allocate_exact_size(screen_rect.size(), egui::Sense::click_and_drag());
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(156));
            });

        self.window.show(ctx, add_contents)
    }
}
