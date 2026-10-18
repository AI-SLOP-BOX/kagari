# choegui

**Make egui 超えぐい.**

Motion and polished interaction primitives for apps that already use egui. choegui keeps the immediate-mode model and returns ordinary egui responses; it does not add an app runtime, windowing layer, or application-wide theme.

```toml
[dependencies]
choegui = { path = "../choegui" }
egui = "0.29"
```

```rust,ignore
fn ui(ui: &mut egui::Ui) {
    if choegui::Button::new("Play").show(ui).clicked() {
        // update app state
    }
    ui.label("Ordinary egui widgets work alongside it.");
}
```

The library also includes an optional, app-neutral design layer: semantic dark and light palettes, measured responsive property grids, consistent fields, buttons and icon buttons, selectable rows, sections, compact typography, combos, color fields, progress bars, and meters. The public theme tokens can be customized or replaced. The host application owns localization, icons, and application state; pass localized labels and provide custom icon painting through `Theme::icon_button_painted`. These primitives work for editors, dashboards, creative tools, and other egui applications without imposing a domain or app runtime.

The motion primitives remain independently usable: an ID-keyed spring, animated visibility progress, a tinted surface with shadow, a spring-driven button, and a timed toast. See [`DESIGN.md`](DESIGN.md) for ecosystem research, technical constraints, and roadmap; run the native comparison app with `cargo run --example showcase`.

The spring uses an exact damped-oscillator solution, egui temporary memory, and repaint requests only while settling. Its target can change mid-motion without clearing velocity; use `animate_from` to set the first-frame value for enter transitions. `MotionPolicy::Reduced` snaps to targets and suppresses motion repainting; applications can connect this to their accessibility preference. `Surface` provides a translucent/material-like appearance, not live backdrop blur: that requires explicit renderer integration. See `egui_glass` or `backdrop-blur-egui` when an application can opt into their renderer-specific paths.

Licensed under MIT OR Apache-2.0.
