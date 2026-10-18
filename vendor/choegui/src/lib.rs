//! Motion and surface primitives for applications that already use egui.
//!
//! ```ignore
//! fn ui(ui: &mut egui::Ui) {
//!     let response = choegui::Button::new("Play").show(ui);
//!     ui.label("Ordinary egui stays right beside it.");
//!     let _ = response;
//! }
//! ```

use std::time::Duration;

use egui::{vec2, Color32, Context, Id, Response, Rounding, Sense, Shadow, Stroke, Ui, Vec2};

pub mod design;

/// Per-call motion preference, suitable for an app-level accessibility setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionPolicy {
    /// Animate with the supplied spring.
    #[default]
    Full,
    /// Jump to the target and avoid animation repaint requests.
    Reduced,
}

/// Default repaint cadence while a spring is settling (frames per second).
const DEFAULT_ANIMATION_FPS: f64 = 120.0;

/// egui temporary-memory key holding the app-wide animation repaint rate.
fn animation_fps_id() -> Id {
    Id::new("choegui/animation_fps")
}

/// Set how often animations request repaints while a spring is settling.
///
/// Defaults to 120 Hz. Lower it (e.g. `60.0`) on standard monitors or in
/// power-saving environments — springs stay smooth because the solver is
/// time-based, only the repaint cadence drops.
pub fn set_animation_fps(ctx: &Context, fps: f32) {
    let fps = fps.clamp(30.0, 240.0) as f64;
    ctx.data_mut(|data| data.insert_temp(animation_fps_id(), fps));
}

fn animation_fps(ctx: &Context) -> f64 {
    ctx.data(|data| {
        data.get_temp::<f64>(animation_fps_id())
            .unwrap_or(DEFAULT_ANIMATION_FPS)
    })
}

/// Repaint after one animation frame interval at the configured cadence.
fn request_animation_frame(ctx: &Context) {
    ctx.request_repaint_after(Duration::from_secs_f64(1.0 / animation_fps(ctx)));
}

/// Damped spring configuration. Frequency is measured in oscillations per second.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub frequency: f32,
    pub damping: f32,
}

impl Spring {
    pub const fn new(frequency: f32, damping: f32) -> Self {
        Self { frequency, damping }
    }
    pub const fn snappy() -> Self {
        Self::new(7.0, 0.88)
    }
    pub const fn gentle() -> Self {
        Self::new(4.0, 1.0)
    }
}

impl Default for Spring {
    fn default() -> Self {
        Self::snappy()
    }
}

#[derive(Clone, Copy, Debug)]
struct SpringState {
    value: f32,
    velocity: f32,
    target: f32,
    time: f64,
    initialized: bool,
}

/// Animate a scalar toward `target`, retaining velocity when the target changes.
///
/// Pass a stable, unique [`Id`] for each independent motion. State lives in egui's
/// temporary memory and repaint is requested only while the spring is settling.
pub fn animate(ctx: &Context, id: Id, target: f32, spring: Spring) -> f32 {
    animate_with_policy(ctx, id, target, spring, MotionPolicy::Full)
}

/// Animate from `initial` on the first call. Later calls preserve egui's stored state.
pub fn animate_from(ctx: &Context, id: Id, initial: f32, target: f32, spring: Spring) -> f32 {
    animate_inner(ctx, id, initial, target, spring, MotionPolicy::Full)
}

/// Animate a scalar with an explicit reduced-motion policy.
///
/// A retarget takes effect at the current frame boundary: elapsed time is first
/// advanced toward the previous target, then the spring follows the new one.
pub fn animate_with_policy(
    ctx: &Context,
    id: Id,
    target: f32,
    spring: Spring,
    policy: MotionPolicy,
) -> f32 {
    animate_inner(ctx, id, target, target, spring, policy)
}

fn animate_inner(
    ctx: &Context,
    id: Id,
    initial: f32,
    target: f32,
    spring: Spring,
    policy: MotionPolicy,
) -> f32 {
    let now = ctx.input(|i| i.time);
    let mut state = ctx
        .data_mut(|data| data.get_temp::<SpringState>(id))
        .unwrap_or(SpringState {
            value: target,
            velocity: 0.0,
            target,
            time: now,
            initialized: false,
        });
    if !state.initialized || policy == MotionPolicy::Reduced {
        state.value = if policy == MotionPolicy::Reduced {
            target
        } else if state.initialized {
            state.value
        } else {
            initial
        };
        state.target = target;
        state.velocity = 0.0;
        state.initialized = true;
        state.time = now;
        if policy == MotionPolicy::Full && (state.value - target).abs() >= 0.001 {
            request_animation_frame(ctx);
        }
    } else {
        let dt = (now - state.time).clamp(0.0, 0.1) as f32;
        integrate_spring(&mut state, dt, spring);
        state.target = target;
        if (state.target - state.value).abs() < 0.001 && state.velocity.abs() < 0.01 {
            state.value = state.target;
            state.velocity = 0.0;
        } else {
            request_animation_frame(ctx);
        }
        state.time = now;
    }
    if policy == MotionPolicy::Reduced {
        state.time = now;
    }
    ctx.data_mut(|data| data.insert_temp(id, state));
    state.value
}

// Closed-form damped oscillator solution: stable across irregular frame times,
// with no iteration count tied to refresh rate or spring frequency.
fn integrate_spring(state: &mut SpringState, dt: f32, spring: Spring) {
    if dt <= 0.0 {
        return;
    }
    let omega = std::f32::consts::TAU * spring.frequency.clamp(0.1, 30.0);
    let zeta = spring.damping.clamp(0.0, 8.0);
    let displacement = state.value - state.target;

    if zeta < 0.9999 {
        let damped = omega * (1.0 - zeta * zeta).sqrt();
        let angle = damped * dt;
        let envelope = (-zeta * omega * dt).exp();
        let b = (state.velocity + zeta * omega * displacement) / damped;
        let cos = angle.cos();
        let sin = angle.sin();
        let next_displacement = envelope * (displacement * cos + b * sin);
        state.velocity = envelope
            * (-zeta * omega * (displacement * cos + b * sin)
                + damped * (-displacement * sin + b * cos));
        state.value = state.target + next_displacement;
    } else if zeta <= 1.0001 {
        let c = state.velocity + omega * displacement;
        let envelope = (-omega * dt).exp();
        state.value = state.target + envelope * (displacement + c * dt);
        state.velocity = envelope * (state.velocity - omega * c * dt);
    } else {
        let root = (zeta * zeta - 1.0).sqrt();
        let r1 = -omega * (zeta - root);
        let r2 = -omega * (zeta + root);
        let c1 = (state.velocity - r2 * displacement) / (r1 - r2);
        let c2 = displacement - c1;
        let e1 = (r1 * dt).exp();
        let e2 = (r2 * dt).exp();
        state.value = state.target + c1 * e1 + c2 * e2;
        state.velocity = c1 * r1 * e1 + c2 * r2 * e2;
    }
}

/// Smoothstep helper for simple normalized transitions.
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A rounded, translucent surface with a soft egui shadow.
///
/// `Surface` paints a tinted panel; it does not perform live backdrop blur.
pub struct Surface {
    pub fill: Color32,
    pub stroke: Stroke,
    pub radius: u8,
    pub shadow: Shadow,
    pub opacity: f32,
}

impl Default for Surface {
    fn default() -> Self {
        Self {
            fill: Color32::from_rgb(34, 40, 55),
            stroke: Stroke::new(1.0_f32, Color32::from_white_alpha(30)),
            radius: 18,
            shadow: Shadow {
                offset: vec2(0.0, 8.0),
                blur: 24.0,
                spread: 0.0,
                color: Color32::from_black_alpha(90),
            },
            opacity: 1.0,
        }
    }
}

impl Surface {
    pub fn new() -> Self {
        Self::default()
    }
    /// Build a surface whose colours follow a [`Theme`](crate::design::Theme),
    /// so cards adopt the active light/dark palette instead of fixed defaults.
    pub fn from_theme(theme: &crate::design::Theme) -> Self {
        Self {
            fill: theme.surface,
            stroke: Stroke::new(1.0_f32, theme.border),
            radius: theme.radius,
            ..Self::default()
        }
    }
    /// Adopt a theme's colours after construction.
    pub fn theme(mut self, theme: &crate::design::Theme) -> Self {
        self.fill = theme.surface;
        self.stroke = Stroke::new(1.0_f32, theme.border);
        self.radius = theme.radius;
        self
    }
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = fill;
        self
    }
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let frame = egui::Frame::none()
            .fill(self.fill)
            .stroke(self.stroke)
            .rounding(Rounding::same(self.radius as f32))
            .shadow(self.shadow)
            .multiply_with_opacity(self.opacity)
            .inner_margin(egui::Margin::same(18.0));
        // Frame::multiply_with_opacity already scales the whole frame (fill, stroke,
        // shadow *and* every widget painted inside it), so the contents inherit the
        // same opacity. Multiplying again on the inner Ui would apply it twice.
        frame.show(ui, add_contents)
    }
}

/// A button with spring-driven hover and press feedback.
pub struct Button {
    text: egui::WidgetText,
    id: Option<Id>,
    fill: Color32,
    size: Vec2,
    motion: MotionPolicy,
}

impl Button {
    pub fn new(text: impl Into<egui::WidgetText>) -> Self {
        Self {
            text: text.into(),
            id: None,
            fill: Color32::from_rgb(111, 104, 255),
            size: vec2(132.0, 44.0),
            motion: MotionPolicy::Full,
        }
    }
    pub fn id(mut self, id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = Some(Id::new(id));
        self
    }
    /// Build a button whose fill follows a [`Theme`](crate::design::Theme)'s accent,
    /// so it adopts the active light/dark palette instead of the fixed default.
    pub fn from_theme(theme: &crate::design::Theme, text: impl Into<egui::WidgetText>) -> Self {
        Self::new(text).theme(theme)
    }
    /// Adopt a theme's accent fill and text colour after construction.
    pub fn theme(mut self, theme: &crate::design::Theme) -> Self {
        self.fill = theme.accent;
        self.motion = theme.motion;
        self
    }
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = fill;
        self
    }
    /// Apply an app-level reduced-motion preference to hover and press feedback.
    pub fn motion_policy(mut self, policy: MotionPolicy) -> Self {
        self.motion = policy;
        self
    }
    pub fn show(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::click());
        let id = self.id.unwrap_or(response.id);
        let hovered = response.hovered() || response.has_focus();
        let pressed = response.is_pointer_button_down_on();
        let hover = animate_with_policy(
            ui.ctx(),
            id.with("hover"),
            f32::from(hovered),
            Spring::gentle(),
            self.motion,
        );
        let press = animate_with_policy(
            ui.ctx(),
            id.with("press"),
            f32::from(pressed),
            Spring::snappy(),
            self.motion,
        );
        let scale = 1.0 + hover * 0.035 - press * 0.055;
        let size = rect.size() * scale;
        let paint_rect = egui::Rect::from_center_size(rect.center(), size);
        let color = self.fill.gamma_multiply(0.82 + hover * 0.18 - press * 0.10);
        ui.painter()
            .rect(paint_rect, Rounding::same(13.0), color, Stroke::NONE);
        let accessibility_text = self.text.text().to_owned();
        let galley = self.text.into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            size.x - 20.0,
            egui::TextStyle::Button,
        );
        ui.painter().galley(
            paint_rect.center() - galley.size() * 0.5,
            galley,
            Color32::WHITE,
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                &accessibility_text,
            )
        });
        response
    }
}

/// Return the spring progress for an animated visibility flag.
pub fn animated_visibility(ctx: &Context, id: Id, visible: bool, spring: Spring) -> f32 {
    animated_visibility_with_policy(ctx, id, visible, spring, MotionPolicy::Full)
}

/// Visibility progress with an app-supplied reduced-motion preference.
pub fn animated_visibility_with_policy(
    ctx: &Context,
    id: Id,
    visible: bool,
    spring: Spring,
    policy: MotionPolicy,
) -> f32 {
    animate_inner(
        ctx,
        id,
        0.0,
        if visible { 1.0 } else { 0.0 },
        spring,
        policy,
    )
}

/// Small toast message rendered in an egui foreground area.
#[derive(Clone, Copy, Debug, Default)]
struct ToastTimer {
    was_visible: bool,
    expires_at: f64,
}

pub struct Toast<'a> {
    pub id: Id,
    pub message: &'a str,
    pub visible: bool,
    duration: Duration,
    motion: MotionPolicy,
}

impl<'a> Toast<'a> {
    pub fn new(
        id: impl std::hash::Hash + std::fmt::Debug,
        message: &'a str,
        visible: bool,
    ) -> Self {
        Self {
            id: Id::new(id),
            message,
            visible,
            duration: Duration::from_millis(2400),
            motion: MotionPolicy::Full,
        }
    }
    /// Set how long the toast stays fully visible before fading out.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
    /// Apply an app-level reduced-motion preference to the toast transition.
    pub fn motion_policy(mut self, policy: MotionPolicy) -> Self {
        self.motion = policy;
        self
    }
    pub fn show(self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        let timer_id = self.id.with("timer");
        let mut timer = ctx
            .data_mut(|data| data.get_temp::<ToastTimer>(timer_id))
            .unwrap_or_default();
        if self.visible && !timer.was_visible {
            timer.expires_at = now + self.duration.as_secs_f64();
        }
        timer.was_visible = self.visible;
        let remaining = timer.expires_at - now;
        let is_visible = self.visible && remaining > 0.0;
        if is_visible {
            ctx.request_repaint_after(Duration::from_secs_f64(remaining));
        }
        ctx.data_mut(|data| data.insert_temp(timer_id, timer));
        let opacity = animated_visibility_with_policy(
            ctx,
            self.id.with("visibility"),
            is_visible,
            Spring::gentle(),
            self.motion,
        );
        if opacity < 0.01 {
            return;
        }
        egui::Area::new(self.id)
            .anchor(
                egui::Align2::CENTER_BOTTOM,
                vec2(0.0, -28.0 + (1.0 - opacity) * 14.0),
            )
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                Surface::new()
                    .fill(Color32::from_rgb(24, 29, 41))
                    .opacity(opacity)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(self.message).color(Color32::WHITE));
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::{integrate_spring, smoothstep, Spring, SpringState};

    #[test]
    fn smoothstep_clamps_and_eases() {
        assert_eq!(smoothstep(-1.0), 0.0);
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(0.5), 0.5);
        assert_eq!(smoothstep(1.0), 1.0);
        assert_eq!(smoothstep(2.0), 1.0);
    }

    #[test]
    fn spring_converges_to_target() {
        let mut state = SpringState {
            value: 0.0,
            velocity: 0.0,
            target: 1.0,
            time: 0.0,
            initialized: true,
        };
        for _ in 0..120 {
            integrate_spring(&mut state, 1.0 / 120.0, Spring::gentle());
        }
        assert!((state.value - 1.0).abs() < 0.01);
        assert!(state.velocity.abs() < 0.1);
    }
}
