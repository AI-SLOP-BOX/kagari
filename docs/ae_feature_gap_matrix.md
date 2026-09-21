# Kagari VFX / After Effects feature gap matrix

This is the working feature inventory for the replacement effort. It separates
the existence of a Rust type or panel from an end-to-end feature that a user
can operate, render, save, and undo.

Status:

- `✅` implemented and connected to the normal workflow
- `🟡` exists, but is partial, narrow, or not yet production-safe
- `🔴` missing or only represented by a placeholder/API surface

The After Effects side is the capability baseline, not a promise to clone
Adobe's proprietary effect implementations. Effect-name and parameter parity
is intentionally tracked separately from the core workflow.

## Feature inventory

| Area | After Effects baseline | Kagari status | Evidence / gap |
|---|---|---:|---|
| Project files | Project save/open, recovery, autosave | ✅ | `core/project.rs`, `core/autosave.rs`, `ui/project_io.rs` |
| Multiple compositions | Nested compositions and composition settings | ✅ | `core/timeline.rs`, precomp workflow and cache |
| Media import | Still images, image sequences, video, audio, vector assets | 🟡 | Image/video/audio paths exist; relink now recurses through PreComps and rebuilds missing WebP caches, layer proxies work in preview, and common compressed audio/video extensions enter through both project import and drag/drop; image-sequence authoring, vector import, and background proxy generation still need hardening |
| Layer model | Solids, footage, text, shapes, cameras, lights, nulls, audio | ✅ | `LayerType` and `Layer` in `core/timeline.rs` |
| Layer timing | In/out, trimming, time stretch, reverse, freeze, time remap | ✅ | `Layer` time-remap operations and timeline actions |
| Keyframes | Linear, hold, Bezier, spatial animation, value/speed graph | ✅ | `core/keyframe.rs`, `ui/graph_editor.rs`; pointer drag + one-step Undo is now covered by an egui integration test |
| Expressions | Property expressions, time/value helpers, wiggle and loops | 🟡 | Rhai engine is connected; transaction/timeout/error UX needs more hardening |
| Parenting | Parent chains and transform propagation | ✅ | `core/parenting_engine.rs`, timeline pick-whip path |
| Pre-composition | Move/leave attributes, nested render, open/return navigation | ✅ | `Composition::precompose_layers`, `core/software_renderer/precomp.rs` |
| Masks | Bezier paths, vertices, tangents, feather/expansion, animated masks | 🟡 | Core and viewport editing exist; mask-tracker workflow and complex path UX need deeper E2E coverage |
| Mattes | Alpha/luma track mattes, set-matte style operations | ✅ | `core/software_renderer/matte.rs`, `core/set_matte.rs` |
| Blend/composite | Layer blending, opacity, alpha, linear-light option | ✅ | software/GPU composite paths |
| 2D transforms | Position, scale, rotation, anchor, opacity, direct manipulation | ✅ | viewport + inspector + timeline |
| Text | Fonts, tracking, leading, alignment, text-on-path, text animation | 🟡 | Rasterizer and text animator exist; advanced typography/layout compatibility is incomplete |
| Shapes | Shape layers, fills/strokes, boolean paths, repeater/modifiers, extrusion | 🟡 | Shape engines exist; shape editing and renderer parity need more end-to-end tests |
| 3D layers | 3D transforms, cameras, lights, depth, shadows, DOF | 🟡 | `advanced_3d_engine.rs`, camera/light UI; model import and scene workflows are narrower than AE |
| Motion tracking | Point tracking, planar tracking, camera solve, stabilization | 🟡 | Point/quad tracking, animated Corner Pin, target-aware stabilization, and 3D camera solving are connected to the tracker panel; real-footage workflow coverage, planar confidence UX, and production-quality solve accuracy still need work |
| Roto / paint | Roto Brush, paint, clone, eraser, puppet | 🟡 | Tools and engines are present; temporal propagation/brush quality are not AE-level |
| Keying | Chroma/linear key, matte cleanup, spill-like workflows | 🟡 | Keying modules and controls exist; production-grade edge handling still needs validation |
| Color | Curves, levels, LUT, color management, scopes, HDR paths | 🟡 | Broad core coverage; live Lumetri histogram now samples the current rendered frame and ramp presets write Color Balance values; consistent 8/16/32-bit and GPU path parity remains a gap |
| Effects | Searchable effect library, animated parameters, presets | 🟡 | Large `EffectType` registry and controls; effect-standard parity is explicitly out of scope for exact cloning |
| Particles / procedural | Particles, lightning, star field, audio spectrum and generators | 🟡 | Engines and render paths exist; authoring, caching, and interaction depth vary by feature |
| Audio | Import, playback sync, mixing, meters, audio-to-keyframes | 🟡 | In-process Symphonia decoding now covers WAV/MP3/FLAC/Ogg/AIFF/CAF/MP4-family audio, project assets can be inserted as real audio layers, and the master VU/32-band display uses the live mix buffer; per-track metering and correction/interchange workflow still need work |
| Preview | Cached frames, RAM preview, adaptive quality, audio sync | 🟡 | Cache and playback exist; true real-time GPU effect processing is still limited |
| Render queue | Multiple items, progress, cancellation/failure reporting | ✅ | `core/render_queue.rs`, `ui/render_queue.rs` |
| Export | FFmpeg video, image/EXR paths, GIF, Lottie, MLT/OTIO-style interchange | 🟡 | Several exporters exist; codec/metadata/alpha compatibility needs broader fixture testing |
| Plugin/integration | Plug-ins, scripting, interchange with external tools | 🟡 | OFX/SDK bridges and Rhai exist; third-party plug-in compatibility is not complete |
| Workspaces | Dockable panels, resize, saved workspace layouts | 🟡 | SavedWorkspace persists panel widths, timeline height, graph mode, viewer state, and compact drawers; restore now runs before panel layout, while full dock visibility/custom panel topology remains narrower than AE |
| UI operations | Search/command palette, shortcuts, inspector, graph editor, undo/redo | 🟡 | Core paths exist; more real egui event/E2E tests are required for production confidence |
| Reliability | Bounded caches, crash recovery, deterministic renders, safe file I/O | 🟡 | Strong unit/integration coverage; sanitizer/stress and hostile-project coverage still need expansion |

## Current largest gaps outside exact effect parity

1. **Preview/render architecture** — GPU preview and CPU/export still have
   paths that do not share the same effect execution and cache behavior.
2. **Real footage workflows** — relink is now connected through nested PreComps
   with cache rebuilds, and preview proxy selection is connected, but
   image-sequence authoring, background proxy generation, and tracker/roto
   propagation still need complete user journeys rather than only engine tests.
3. **3D production workflow** — imported models, cameras, lights, depth,
   shadows, DOF, and compositing need one connected authoring path.
4. **Text and shape authoring** — the data model is broad, but direct editing,
   typography fidelity, path editing, and modifier interaction are behind AE.
5. **Workspace persistence** — saved workspaces must restore panel geometry,
   timeline height, graph mode, and viewer state, not only tab indices.
6. **Roto/paint temporal behavior** — propagation, caching, and correction
   tools need to survive multi-frame edits and real footage.
7. **Audio correction and interchange** — production audio formats and the
   correction workflow need a complete non-ML baseline before model support.

## Implementation order

The next work should follow user-visible leverage and verification cost:

1. Add a real footage workflow test covering import → relink/proxy → tracking →
   render/export, including a compressed audio layer inserted from the project bin.
2. Close the GPU preview/export semantic differences with pixel-contract tests.
3. Expand 3D model/camera/light authoring and its render fixtures.
4. Strengthen roto/paint propagation and audio correction without requiring an
   ML model.

Workspace layout persistence has now been implemented for the geometry and
compact drawer state listed above; the remaining workspace gap is a richer dock
topology/visibility model.

This file is intentionally conservative: a module or enum alone does not turn
the row green.
