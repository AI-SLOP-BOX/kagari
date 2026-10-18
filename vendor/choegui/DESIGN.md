# choegui: design and ecosystem notes

Date checked: 2026-10-05. Prototype targets egui/eframe 0.29.1; downstream apps should use a compatible egui version.

## Goal and boundaries

choegui combines motion and transient surfaces with a small optional design layer for ordinary application UI. It remains a library over `egui::Ui`, `Context`, `Id`, and `Response`: callers own app state, frame loop, windowing, renderer, localization, and icon assets. It is not a runtime or a comprehensive replacement widget kit. The design layer provides reusable layout and interaction patterns while letting each app choose its palette, labels, and visual identity. Kagari's KUI informed the initial extraction, but choegui is intended to work across application domains.

## Ecosystem map

| Area | Existing work | choegui stance |
| --- | --- | --- |
| Theme, semantic colors, spacing, typography | modern-egui, egui-thematic, egui_colors, catppuccin-egui | Provide an optional semantic palette and compact typography/layout tokens; callers can supply their own values or keep their existing widgets. |
| Shadcn/Radix-style controls and overlays | egui-shadcn (large component set including dialogs, popovers, toast, command, skeleton); shadcn-rs also names a separate Yew/web library | Do not duplicate static component breadth. Add motion/surface behavior that can coexist with other crates. |
| Polished design systems and platform materials | ouroboros-ui, fluent-egui | Complement their visual systems; avoid prescribing an application-wide design language. |
| Tween/easing/collapse | egui built-in Context animation methods; egui_animation (easing, collapse, position/translation helpers) | Prefer built-ins for simple scalar timing. Provide a small velocity-preserving spring core where interruption continuity matters. |
| Glass/backdrop | egui_glass and backdrop-blur-egui | Optional renderer-specific integrations can provide true blur. Core offers tinted/translucent surfaces, never promises framebuffer sampling. |

The ecosystem changes quickly; egui and eframe are pinned to the same minor (0.29.1), with the immediate-mode API kept visible. `egui_glass` requires an egui-wgpu render state and an explicit backdrop (or an extra off-screen render); `backdrop-blur-egui` has renderer-specific glow/wgpu paths. These are not renderer-neutral drop-ins.

## egui constraints that shape the design

- `Context` already provides keyed scalar/bool animations and `request_repaint_after`. Use those for straightforward tweens; a custom spring only requests another frame while it is still settling.
- A spring needs persistent `(value, velocity, target, last_time)` keyed by caller `Id`. A target change updates the destination but preserves current value and velocity. State stays in egui temporary data, with no app-global registry or per-frame heap allocation.
- Frame time can be irregular. Advance the damped oscillator with its closed-form solution (under-damped, critical, or over-damped), cap pathological pauses, and snap when both position and velocity are within tolerances. A retarget takes effect at the current frame boundary after advancing toward the previous target. An app-supplied reduced-motion policy snaps immediately.
- egui paints in a single pass and a normal widget cannot sample already-painted pixels behind itself. Translucent fills and layered shadows are portable; true backdrop blur requires explicit renderer cooperation and has ordering/performance implications.
- Animated content still needs layout every frame. Visibility transitions can animate opacity/offset/clip while preserving egui's normal layout and interaction/accessibility behavior; removing content immediately at the closed endpoint is a caller choice.

## Initial scope (small, composable primitives)

1. ID-keyed damped spring for `f32`, with exact continuity on retarget and idle-aware repaint.
2. Animated visibility progress; callers keep contents in the egui pass while transitioning.
3. `Surface`/glass-like panel: rounded fill, border, and shadow, with configurable alpha; blur is explicitly out of core.
4. Animated button and toast primitives with ordinary egui responses and caller-owned lifetime.
5. Optional semantic theme tokens and shared field/button styling.
6. Measured property grids that stack labels above controls in narrow panels.
7. Selectable rows, icon buttons with custom painter hooks, tabs, sections, status text, and typography helpers.

Popup, modal/dialog, tooltip, command palette, context menu, sliders, skeletons, and hover-reveal patterns are follow-on work. Use egui built-ins or established component crates until a focused motion gap is demonstrated.

## Recommended architecture and public API direction

- A small `choegui` crate with `egui` re-exported only if it helps avoid version ambiguity; no runtime or mandatory renderer features.
- `Spring` is a copyable configuration (`frequency`/`damping`), with named presets. `animate(ctx, id, target, spring) -> f32` is deliberately direct and keeps egui's `Id` explicit. `animate_from` seeds the initial value for enter transitions.
- Surface/button/toast builders take `&mut Ui` or `&Context` only when their work requires it and return egui responses where appropriate. They do not hide application state in a DSL.
- Core dependency set: egui only. Optional renderer integrations stay separate. Existing tween crate reuse is unnecessary for a velocity-aware spring; don't add a dependency merely to wrap egui's existing animations.
- Reduced motion is an app-supplied per-call or per-widget policy (`Full`, `Reduced`) that snaps motion without changing widget interaction or semantics. egui does not supply a renderer-independent system preference, so the app owns connecting any platform setting.

Prototype API is intentionally smaller than this roadmap. See `examples/showcase.rs` for ordinary egui widgets mixed with choegui primitives.

## References checked

- [egui Context docs](https://docs.rs/egui/latest/egui/struct.Context.html)
- [egui_animation 0.13](https://docs.rs/egui_animation/latest/egui_animation/)
- [egui third-party crate list](https://github.com/emilk/egui/wiki/3rd-party-egui-crates)
- [egui-thematic](https://docs.rs/egui-thematic/latest/egui_thematic/)
- [egui-shadcn](https://docs.rs/egui-shadcn/latest/egui_shadcn/)
- [egui_glass](https://docs.rs/egui_glass/latest/egui_glass/)
- [backdrop-blur-egui](https://docs.rs/backdrop-blur-egui/latest/backdrop_blur_egui/)
