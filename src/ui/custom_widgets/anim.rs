use eframe::egui;

/// Animated hover state for smooth transitions
pub struct HoverAnim {
    pub progress: f32,
    pub target: f32,
}

impl Default for HoverAnim {
    fn default() -> Self {
        Self::new()
    }
}

impl HoverAnim {
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            target: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, is_hovered: bool) {
        self.target = if is_hovered { 1.0 } else { 0.0 };
        let speed = 8.0;
        self.progress += (self.target - self.progress) * speed * dt;
        if (self.progress - self.target).abs() < 0.01 {
            self.progress = self.target;
        }
    }
}

/// Smooth color transition animation
pub struct ColorAnim {
    pub current: egui::Color32,
    pub target: egui::Color32,
    pub speed: f32,
}

impl ColorAnim {
    pub fn new(initial: egui::Color32) -> Self {
        Self {
            current: initial,
            target: initial,
            speed: 8.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        let r = self.current.r() as f32;
        let g = self.current.g() as f32;
        let b = self.current.b() as f32;
        let a = self.current.a() as f32;

        let tr = self.target.r() as f32;
        let tg = self.target.g() as f32;
        let tb = self.target.b() as f32;
        let ta = self.target.a() as f32;

        let new_r = r + (tr - r) * self.speed * dt;
        let new_g = g + (tg - g) * self.speed * dt;
        let new_b = b + (tb - b) * self.speed * dt;
        let new_a = a + (ta - a) * self.speed * dt;

        self.current = egui::Color32::from_rgba_premultiplied(
            new_r as u8,
            new_g as u8,
            new_b as u8,
            new_a as u8,
        );
    }

    pub fn set_target(&mut self, target: egui::Color32) {
        self.target = target;
    }
}

/// Pulse animation for active states
pub struct PulseAnim {
    pub phase: f32,
    pub speed: f32,
    pub min_alpha: u8,
    pub max_alpha: u8,
}

impl Default for PulseAnim {
    fn default() -> Self {
        Self::new()
    }
}

impl PulseAnim {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            speed: 2.0,
            min_alpha: 180,
            max_alpha: 255,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.phase += self.speed * dt;
        if self.phase > std::f32::consts::TAU {
            self.phase -= std::f32::consts::TAU;
        }
    }

    pub fn alpha(&self) -> u8 {
        let t = (self.phase.sin() + 1.0) / 2.0;
        let range = (self.max_alpha - self.min_alpha) as f32;
        (self.min_alpha as f32 + t * range) as u8
    }

    pub fn color(&self, base: egui::Color32) -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(base.r(), base.g(), base.b(), self.alpha())
    }
}
