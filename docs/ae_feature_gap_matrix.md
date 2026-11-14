# Kagari VFX / After Effects feature gap matrix

This is the working feature inventory for the replacement effort. It separates
the existence of a Rust type or panel from an end-to-end feature that a user
can operate, render, save, and undo.

The baseline follows Adobe's documented workflow: import and organize footage,
create and composite layers, animate properties and expressions, apply effects,
preview with color management, and render/export. A row is not considered
complete merely because a panel or enum exists: the acceptance path is operate
the feature, save and reload the project, preview it, and render/export the
result without a silent fallback to a different meaning.

Unless a row explicitly says otherwise, operation coverage means the default
AmeUI `kagari-studio` workflow. Improvements available only in the optional
legacy `kagari-studio-egui` binary do not count as primary-product coverage.

Reference baseline: <https://helpx.adobe.com/after-effects/desktop/get-started/understand-after-effects-workflow/workflows.html>

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
| Media import | Still images, image sequences, video, audio, vector assets | 🟡 | Image/video/audio paths exist; imported video extraction cadence is persisted with decoded-frame caches and mapped consistently through rendering, audio, tracking, and thumbnails when composition FPS changes; relink recurses through PreComps and rebuilds missing WebP caches, layer proxies can be generated from image/video media and work in preview, common compressed audio/video extensions enter through project import and drag/drop, numbered image sequences can be imported as WebP footage, SVG paths can enter as editable mask geometry, and OBJ files can enter the project bin as real 3D model assets; Illustrator feature coverage and vector styling still need hardening |
| Layer model | Solids, footage, text, shapes, cameras, lights, nulls, audio | ✅ | `LayerType` and `Layer` in `core/timeline.rs` |
| Layer timing | In/out, trimming, time stretch, reverse, freeze, time remap | ✅ | `Layer` time-remap operations and timeline actions |
| Video frame blending | Frame Mix and motion-compensated Pixel Motion | 🟡 | Frame Mix now interpolates RGBA in premultiplied-alpha space in the CPU render and video-matte paths; luma mattes multiply luminance by source alpha. The standalone Pixel Motion helper ranks all offsets on a 4×4 sample, refines four candidates in parallel by block row, handles partial edge blocks with at least 50% overlap, and prefers zero displacement for ambiguous matches. A warmed 1080p random-texture probe measured 48.2 ms median on the Linux builder; production layer selection/render wiring and representative footage/hardware latency remain unverified |
| Optical-flow timewarp | Motion-compensated intermediate-frame synthesis | 🟡 | Shared engine uses bidirectional inverse warps, including iterative source-coordinate solving for spatially varying motion; translation and expansion regressions cover the mapping. Studio workflow wiring, occlusion behavior, and real-footage quality remain unverified |
| Keyframes | Linear, hold, Bezier, spatial animation, value/speed graph | ✅ | `core/keyframe.rs`, `ui/graph_editor.rs`; pointer drag + one-step Undo is now covered by an egui integration test |
| Expressions | Property expressions, time/value helpers, wiggle and loops | 🟡 | Rhai engine is connected; transaction/timeout/error UX needs more hardening |
| Parenting | Parent chains and transform propagation | ✅ | `core/parenting_engine.rs`, timeline pick-whip path |
| Pre-composition | Move/leave attributes, nested render, open/return navigation | ✅ | `Composition::precompose_layers`, `core/software_renderer/precomp.rs` |
| Masks | Bezier paths, vertices, tangents, feather/expansion, animated masks | 🟡 | Core and viewport editing exist; CPU feather now ramps symmetrically across the boundary and mask opacity is applied in final renders. GPU preview falls back to software for enabled masks with expansion or Wiggle Paths because those geometries are not implemented in the GPU collector; mask-tracker workflow and complex path UX still need deeper E2E coverage |
| Mattes | Alpha/luma track mattes, set-matte style operations | ✅ | `core/software_renderer/matte.rs`, `core/set_matte.rs` |
| Blend/composite | Layer blending, opacity, alpha, linear-light option | ✅ | software/GPU composite paths |
| 2D transforms | Position, scale, rotation, anchor, opacity, direct manipulation | ✅ | viewport + inspector + timeline |
| Text | Fonts, tracking, leading, alignment, text-on-path, text animation | 🟡 | Rasterizer and text animator exist; text-on-path toggle is now exposed in the text properties panel; advanced typography/layout compatibility is incomplete |
| Shapes | Shape layers, fills/strokes, boolean paths, repeater/modifiers, extrusion | 🟡 | Shape engines exist; shape editing and renderer parity need more end-to-end tests |
| 3D layers | 3D transforms, cameras, lights, depth, shadows, DOF | 🟡 | `camera_dof.rs` (simple + thin-lens CoC), camera/light UI, and software-rasterized OBJ/glTF layers with nested PreComp coverage; imported base color, metallic, and roughness now affect software shading (roughness uses a Blinn-Phong approximation), while GPU mesh rendering, metallic-roughness textures, scene authoring, and broader interchange remain narrower than AE |
| Scene cameras/lights | Camera and light layers in the timeline, animated scene objects, active-camera switching | 🟡 | Creation from the timeline, settings, menu, and 3D camera solve now creates linked scene rows; deletion and pre-compose preserve/remove the linked objects, and animated row transforms drive software-render camera/light values; richer first-class layer controls and GPU parity remain |
| Motion tracking | Point tracking, planar tracking, camera solve, stabilization | 🟡 | Point/quad tracking, animated Corner Pin, target-aware stabilization, and 3D camera solving are connected to the tracker panel; real-footage workflow coverage, planar confidence UX, and production-quality solve accuracy still need work |
| Roto / paint | Roto Brush, paint, clone, eraser, puppet | 🟡 | Default AmeUI Studio has mouse foreground/background strokes, saved per-frame strokes, asynchronous current-frame matte generation, and optical-flow propagation for image/video/precomp layers. Matte analysis polls cancellation during its main sampling and raster passes and is bounded to 8.4 megapixel (4K UHD) frames before render/analysis allocation. On a 2-second 1280×720 pedestrian clip, initial propagation leaked badly; area-growth and motion-prior constraints reduced the tested corrected result to 140,049 opaque pixels at frame 0, 181,609 at frame 12, and 175,989 at frame 24, but visual review showed it still selected other pedestrians/scenery and lost the target. Subject-isolation acceptance failed with and without the tested midpoint correction. The motion prior remains active on corrected frames, with local expansion only around foreground correction strokes. The prior uses Euclidean expansion, and flow input uses area averaging plus a 320-pixel maximum analysis dimension to retain more narrow-subject detail than the earlier 160-pixel cap. Dense-flow ties now prefer zero motion in flat regions; synthetic propagation tests pass, but these changes have not been re-rendered on the pedestrian probe, so the existing real-footage failure remains the acceptance baseline. The repository does not include that pedestrian probe; `tests/roto_footage_workflow.rs` generates a three-frame synthetic shape/occluder clip, and the only bundled video is `assets/library/av1_forest.webm`, a 73×67 forest illustration loop rather than subject footage. The synthetic workflow and this small artwork clip are not evidence of real-footage subject isolation. Review/cache, GPU preview/export parity, paint/clone, and full puppet parity remain incomplete. Kagari preserves per-point pressure and coalesced samples in Roto strokes and applies pressure to brush radius. AmeUI API delivery is partially implemented (REQ-353); macOS AppKit stylus delivery is implemented in the current AmeUI working tree but awaits hardware acceptance; Windows/Linux native stylus backends, OS coalesced source samples, and real-footage Roto acceptance remain open |
| Content-Aware Fill | Remove an object across a sequence with generated replacement frames | 🟡 | Object (nearest-boundary propagation), Surface (boundary-average patch), and Edge Blend (masked-region smoothing) produce distinct results. UI generation isolates the selected layer, ignores the removal mask while sampling, renders the chosen Work Area or full layer-overlap range in a background job, stores one WebP patch per frame under project-adjacent `Assets/KagariGenerated`, and inserts a time-remapped fill sequence; cancellation, worker failure, and changed source masks are handled, while temporal coherence, moving-camera/occlusion quality, and real-footage acceptance remain unproven |
| Motion blur | Per-layer and comp motion blur with shutter controls | 🟡 | Layer/comp switches and shutter controls exist; temporal sampling quality, GPU/export parity, and representative fast-motion image tests remain incomplete |
| Keying | Chroma/linear key, matte cleanup, spill-like workflows | 🟡 | Keying modules and controls exist; production-grade edge handling still needs validation |
| Color | Curves, levels, LUT, color management, scopes, HDR paths | 🟡 | Broad core coverage; live Lumetri histogram now samples the current rendered frame and ramp presets write Color Balance values; consistent 8/16/32-bit and GPU path parity remains a gap |
| Effects | Searchable effect library, animated parameters, presets | 🟡 | Large `EffectType` registry and controls; the searchable catalog now wires many more scalar-parameter entries (LightSweep, Radial Fast Blur, Bend It, Venetian Blinds, Glow Pro, etc.) and reflects new selectable entries in the properties panel; effect-standard parity is explicitly out of scope for exact cloning |
| Particles / procedural | Particles, lightning, star field, audio spectrum and generators | 🟡 | Engines and render paths exist; authoring, caching, and interaction depth vary by feature |
| Audio | Import, playback sync, mixing, meters, audio-to-keyframes | 🟡 | In-process Symphonia decoding covers WAV/MP3/FLAC/Ogg/AIFF/CAF/MP4-family audio; project-bin insertion, save/reload, undo/redo, and mixer output have an egui workflow test. Decoded buffers now share a sample-rate-aware LRU capped at 256 MiB/4096 entries; the audio mixer exposes an "音声をキーフレームへ変換" action that runs the core Audio Amplitude assistant for the selected audio/video-with-audio layer; per-track metering, correction/interchange, long-session playback sync, and broader device testing remain |
| Preview | Cached frames, RAM preview, adaptive quality, audio sync | 🟡 | Cache and playback exist; GPU preview falls back for Puppet Pins, unsupported blend modes, Shape Freeform/Stroke/3D geometry, layer styles, unverified effect stacks, every effect whose shader math/parameter mapping is not yet verified against export (including spatial distortion effects until their animated parameters pass parity), and Lens Flare linked to a scene light (the CPU now resolves a projected comp-space light into the cropped layer buffer; GPU still lacks the corresponding transformed local UV). GPU texture readback compares Color Tint, Levels, unlinked Lens Flare, RGB-only Invert, and Posterize against CPU output across full frames within 2/255 for opaque solids and patterned alpha images with 63.7% layer opacity over translucent backgrounds in Normal and Screen modes; the alpha contract compares GPU premultiplied target RGB with the software renderer's straight RGBA. A patterned transparent image with subpixel position, nonuniform scale, rotation, and 73.4% opacity matches within 3/255. Posterize combined with other effects falls back to software because the GPU's single-pass intermediate precision differs from CPU 8-bit stage rounding and caused a measured 43/255 threshold error. Broader blend/scale/sequence coverage remains unfinished, so preview correctness takes precedence over GPU acceleration |
| Render queue | Multiple items, progress, cancellation/failure reporting | ✅ | `core/render_queue.rs`, `ui/render_queue.rs` |
| Export | FFmpeg video, image/EXR paths, GIF, Lottie, MLT/OTIO-style interchange | 🟡 | The export screen now exposes MLT XML, MOGRT, Lottie (precomps flattened), OTIO JSON and SRT/VTT alongside FFmpeg video and a still-frame PNG; unsupported layer types and blend modes are rejected with explicit errors rather than silently dropped. Fixture coverage and codec/alpha metadata parity still need broader testing |
| Essential Graphics / templates | Exposed controls, reusable motion-graphics templates, host-side overrides | 🟡 | The export screen can save a MOGRT package that exposes every text layer as an editable essential text property, and the project dialog can open `.mogrt` files as unsaved projects. Adobe-host compatibility, richer property binding, and template validation remain narrower |
| Plugin/integration | Plug-ins, scripting, interchange with external tools | 🟡 | OFX/SDK bridges and Rhai exist; third-party plug-in compatibility is not complete |
| Team collaboration | Shared projects, presence, merge/sync, review comments | 🔴 | No equivalent shared-project/presence/review workflow found; Adobe documents Team Projects and Frame.io integration |
| Adobe ecosystem links | Dynamic Link, Creative Cloud Libraries, Cinema 4D integration | 🟡 | Kagari has internal project assets and limited interchange, but these Adobe-host integrations are not equivalent or broadly compatible |
| Workspaces | Dockable panels, resize, saved workspace layouts | 🟡 | SavedWorkspace persists panel widths, timeline height, graph mode, viewer state, and compact drawers; restore now runs before panel layout, while full dock visibility/custom panel topology remains narrower than AE |
| UI operations | Search/command palette, shortcuts, inspector, graph editor, undo/redo | 🟡 | Core paths exist; more real egui event/E2E tests are required for production confidence |
| Reliability | Bounded caches, crash recovery, deterministic renders, safe file I/O | 🟡 | Strong unit/integration coverage; sanitizer/stress and hostile-project coverage still need expansion |

## Current largest gaps outside exact effect parity

1. **Preview/render architecture** — GPU preview and CPU/export still have
   paths that do not share the same effect execution and cache behavior. The
   The software preview now renders ordinary 2D layers directly at the requested
   viewport dimensions, scaling transforms and masks before rasterization.
   Compositions with effects, mattes, precomps, particles, paint, 3D, or layer
   styles still use full-resolution rendering followed by resize to preserve
   behavior. Expand direct target-resolution rendering to those paths and
   measure peak memory and frame time. AmeUI supplies the viewport dimensions;
   renderer scaling and parity remain Kagari-owned.
2. **Real footage workflows** — relink is now connected through nested PreComps
   with cache rebuilds, preview proxy generation and selection are connected,
   and numbered image-sequence authoring now enters the normal WebP footage
   path; tracker/roto propagation still need complete user journeys rather
   than only engine tests.
3. **Vector/text production workflow** — SVG path geometry can now enter the
   editable mask system, but Illustrator styling, shape fills, and robust SVG
   transform parsing are still behind AE.
4. **3D production workflow** — imported OBJ models now render in top-level and
   nested compositions, but materials, scene authoring, GPU mesh rendering,
   interchange, depth, shadows, and DOF still need one connected authoring path.
5. **Scene object authoring** — camera/light controls now affect the real scene,
   and linked timeline transforms survive deletion and pre-compose, but richer
   first-class layer controls, per-object selection UX, and GPU parity are still
   behind AE.
6. **Text and shape authoring** — the data model is broad, but direct editing,
   typography fidelity, path editing, and modifier interaction are behind AE.
7. **Workspace persistence** — saved workspaces must restore panel geometry,
   timeline height, graph mode, and viewer state, not only tab indices.
8. **Roto/paint temporal behavior** — a user can apply geometry smoothing,
   feather, and expansion to the stored matte with Undo, choose which tracker
   drives translation, and propagate a roto matte by warping foreground and
   background strokes, applying frame-specific corrections, and re-segmenting
   each frame. Synthetic occlusion behavior is covered; real-footage quality,
   propagation review, correction caching, and paint/clone remain substantially
   behind AE.
9. **Audio correction and interchange** — production audio formats and the
   correction workflow need a complete non-ML baseline before model support.

## Next implementation slices and acceptance criteria

These are user workflows, not module checkboxes. A slice remains partial until
the authored project can be saved, reopened, previewed, and rendered without
losing its meaning.

| Priority | AE workflow to match | Kagari gap to close | Acceptance evidence |
|---:|---|---|---|
| 1 | Roto Brush / paint a subject over time | Optical-flow/color re-segmentation leaks into surrounding people and scenery by frames 12–24 on the pedestrian probe, even with a tested midpoint correction; area-growth protection does not preserve subject identity. The re-segmentation prior margin has since been narrowed from 2% to 0.5% of frame size and single-foreground spatial falloff corrected, but neither change has been measured on that footage. The local foreground-correction motion-prior guard remains an unproven mitigation | In the standard Studio, import annotated real subject clips including crossings/occlusions, paint strokes with pen or mouse, propagate and correct later frames, save/reopen, and compare preview/export mattes against references; reject scene leakage and verify edge quality, stroke undo/cancel, and bounded latency |
| 2 | Consistent composition preview and final render | The default AmeUI viewport requests a smaller output buffer, but Kagari's `core/software_renderer::render_frame_to_pixels_preview` first renders at full composition dimensions and resizes afterward. Preview resolution therefore does not lower Kagari's core raster workload or composition-buffer peak; compositions above Kagari's dimension/buffer limits can also yield no preview despite a smaller requested output. Preview-to-export pixel contracts across scaling, proxies, color transforms, and effect stacks remain unmeasured; legacy egui GPU allowlist fallback is a separate path | In the standard AmeUI Studio workflow, compare preview pixels and exported reference pixels at matching dimensions across proxy/scaling, color-transform, alpha/blend, and representative effect cases; document tolerances and make failures visible. Separately, profile 4K/8K compositions at Full/Half/Quarter and show lower raster work and peak memory at reduced settings without changing preview/export pixel tolerances. For legacy GPU preview, assert unsupported stacks take the software fallback path |
| 3 | 3D camera, lights, and imported models in a composition | OBJ/software coverage exists, but authored scenes, material controls, broader model interchange, and GPU parity are incomplete | Build a scene with a model, camera, and light using UI controls; animate it; save/reopen; render nested and top-level compositions; compare against reference frames |
| 4 | Content-Aware Fill over a footage range | The mask-driven path exists, but temporal coherence and failure/review UX are not established | Run on representative static-camera and moving-camera clips; measure fill-region temporal flicker and boundary error, support manual replacement frames, and persist the result |
| 5 | Motion tracking and stabilization on production footage | Point/quad/camera-solve controls exist, but confidence review, correction, and full footage journeys remain thin | Track a supplied clip, inspect/reject low-confidence frames, adjust a track, apply it to a layer, save/reopen, and validate overlay drift against annotated points |
| 6 | Text and shape motion-graphics authoring | Engines and data models exceed the depth of direct manipulation and typography/geometry feedback | Create/edit text and shape paths in the viewer, animate them, reopen the project, and assert matching preview/export frames |
| 7 | Reliable project interchange and reusable graphics | MOGRT/OTIO/MLT and plugin paths have narrower host compatibility than AE | Maintain fixture projects from supported external tools, round-trip supported fields, and report unsupported fields instead of silently dropping them |

Do not count “the panel opens”, “the enum exists”, “does not panic”, or “the
buffer dimensions are unchanged” as acceptance evidence for these workflows.

## Implementation order

The first import/audio round-trip and graph-keyframe drag paths have already
been exercised; do not repeat them as the next pass. Continue by user-visible
leverage, and update the acceptance evidence above only after each slice closes:

1. Complete the Roto/paint real-footage journey; measure edge quality and
   preview/export agreement, then fix the first measured propagation,
   correction, cache, or undo failure.
2. Audit the GPU allowlist against the software renderer, add representative
   pixel-contract tests, and remove any unproven effect from the allowlist.
3. Complete one UI-authored 3D model/camera/light project through save/reopen
   and top-level plus nested rendering.
4. Measure Content-Aware Fill on real clips, then address temporal flicker and
   correction/review before expanding model-backed approaches.
5. Close tracking/stabilization review and correction UX, followed by direct
   text/shape authoring and interchange fixture coverage.

## Adobe reference pages

- [After Effects workflows](https://helpx.adobe.com/after-effects/desktop/get-started/understand-after-effects-workflow/workflows.html)
- [After Effects features](https://www.adobe.com/products/aftereffects/features.html)
- [Motion tracking and stabilization](https://helpx.adobe.com/after-effects/desktop/animate-in-after-effects/track-motion/tracking-stabilizing-motion-cs5.html)
- [Rendering and export](https://helpx.adobe.com/after-effects/desktop/render-and-export/basics-of-rendering-and-exporting/basics-rendering-exporting.html)
- [Team Projects](https://helpx.adobe.com/after-effects/desktop/collaboration-with-others/team-projects/collaborate-using-team-projects.html)

Workspace layout persistence has now been implemented for the geometry and
compact drawer state listed above; the remaining workspace gap is a richer dock
topology/visibility model.

This file is intentionally conservative: a module or enum alone does not turn
the row green.

PNG and EXR sequence exporters now pass cooperative cancellation into software
rendering and clean up previously written frames on cancellation or a render
panic. The export workflow remains partial pending end-to-end cancellation and
broader codec/alpha metadata acceptance.

Active Studio edits now advance the shared render generation immediately, so
nested PreComp caches update during a drag, before its undo action is committed.
Installing another project also advances the generation, preventing cached
nested frames from crossing project loads. The preview row remains partial
because wider preview/export pixel parity and real-media workflows are still
incomplete.
