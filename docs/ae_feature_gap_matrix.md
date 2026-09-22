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
| Media import | Still images, image sequences, video, audio, vector assets | 🟡 | Image/video/audio paths exist; relink now recurses through PreComps and rebuilds missing WebP caches, layer proxies can be generated from image/video media and work in preview, common compressed audio/video extensions enter through both project import and drag/drop, numbered image sequences can be imported as WebP footage, SVG paths can enter as editable mask geometry, and OBJ files can enter the project bin as real 3D model assets; Illustrator feature coverage and vector styling still need hardening |
| Layer model | Solids, footage, text, shapes, cameras, lights, nulls, audio | ✅ | `LayerType` and `Layer` in `core/timeline.rs` |
| Layer timing | In/out, trimming, time stretch, reverse, freeze, time remap | ✅ | `Layer` time-remap operations and timeline actions |
| Keyframes | Linear, hold, Bezier, spatial animation, value/speed graph | ✅ | `core/keyframe.rs`, `ui/graph_editor.rs`; pointer drag + one-step Undo is now covered by an egui integration test |
| Expressions | Property expressions, time/value helpers, wiggle and loops | 🟡 | Rhai engine is connected; transaction/timeout/error UX needs more hardening |
| Parenting | Parent chains and transform propagation | ✅ | `core/parenting_engine.rs`, timeline pick-whip path |
| Pre-composition | Move/leave attributes, nested render, open/return navigation | ✅ | `Composition::precompose_layers`, `core/software_renderer/precomp.rs` |
| Masks | Bezier paths, vertices, tangents, feather/expansion, animated masks | 🟡 | Core and viewport editing exist; CPU feather now ramps symmetrically across the boundary and mask opacity is applied in final renders. GPU preview falls back to software for enabled masks with expansion or Wiggle Paths because those geometries are not implemented in the GPU collector; mask-tracker workflow and complex path UX still need deeper E2E coverage |
| Mattes | Alpha/luma track mattes, set-matte style operations | ✅ | `core/software_renderer/matte.rs`, `core/set_matte.rs` |
| Blend/composite | Layer blending, opacity, alpha, linear-light option | ✅ | software/GPU composite paths |
| 2D transforms | Position, scale, rotation, anchor, opacity, direct manipulation | ✅ | viewport + inspector + timeline |
| Text | Fonts, tracking, leading, alignment, text-on-path, text animation | 🟡 | Rasterizer and text animator exist; advanced typography/layout compatibility is incomplete |
| Shapes | Shape layers, fills/strokes, boolean paths, repeater/modifiers, extrusion | 🟡 | Shape engines exist; shape editing and renderer parity need more end-to-end tests |
| 3D layers | 3D transforms, cameras, lights, depth, shadows, DOF | 🟡 | `advanced_3d_engine.rs`, camera/light UI, and software-rasterized OBJ layers with nested PreComp coverage; model materials, scene authoring, GPU mesh rendering, and broader interchange remain narrower than AE |
| Scene cameras/lights | Camera and light layers in the timeline, animated scene objects, active-camera switching | 🟡 | Creation from the timeline, settings, menu, and 3D camera solve now creates linked scene rows; deletion and pre-compose preserve/remove the linked objects, and animated row transforms drive software-render camera/light values; richer first-class layer controls and GPU parity remain |
| Motion tracking | Point tracking, planar tracking, camera solve, stabilization | 🟡 | Point/quad tracking, animated Corner Pin, target-aware stabilization, and 3D camera solving are connected to the tracker panel; real-footage workflow coverage, planar confidence UX, and production-quality solve accuracy still need work |
| Roto / paint | Roto Brush, paint, clone, eraser, puppet | 🟡 | Roto Brush source strokes persist and remain isolated by layer identity; segmentation samples the selected layer alone on transparent background, ignoring its existing mask and track-matte inputs so other layers cannot contaminate the foreground/background model. Foreground/background strokes are frame-scoped (legacy strokes without frame metadata remain global), their painted disks are enforced as hard alpha seeds, and sparse pointer samples are interpolated to prevent gaps; corrected contours are stored as undoable Hold keyframes so differing polygon vertex counts never interpolate into invalid geometry. Tracker propagation uses the selected point; a separate cancellable optical-flow path warps boundary and foreground/background strokes through adjacent frames, applies frame-specific user corrections, re-segments each frame, retains the component seeded by the propagated foreground stroke, and resamples contours to keep keyframe vertex counts consistent. The occlusion-correction path passes a synthetic subject/occluder test; the optical-flow search allows ±8 processing pixels at a 160px maximum processing dimension and passes a ±6px synthetic bidirectional motion test. Puppet pin deformation runs in the software renderer and the viewport now selects that path when pins are present because the GPU renderer does not yet deform the layer; production-footage validation, matte review/cache, color decontamination, paint/clone quality, and overall puppet parity remain well below AE |
| Content-Aware Fill | Remove an object across a sequence with generated replacement frames | 🟡 | Object (nearest-boundary propagation), Surface (boundary-average patch), and Edge Blend (masked-region smoothing) produce distinct results. UI generation isolates the selected layer, ignores the removal mask while sampling, renders the chosen Work Area or full layer-overlap range in a background job, stores one WebP patch per frame under project-adjacent `Assets/KagariGenerated`, and inserts a time-remapped fill sequence; cancellation, worker failure, and changed source masks are handled, while temporal coherence, moving-camera/occlusion quality, and real-footage acceptance remain unproven |
| Motion blur | Per-layer and comp motion blur with shutter controls | 🟡 | Layer/comp switches and shutter controls exist; temporal sampling quality, GPU/export parity, and representative fast-motion image tests remain incomplete |
| Keying | Chroma/linear key, matte cleanup, spill-like workflows | 🟡 | Keying modules and controls exist; production-grade edge handling still needs validation |
| Color | Curves, levels, LUT, color management, scopes, HDR paths | 🟡 | Broad core coverage; live Lumetri histogram now samples the current rendered frame and ramp presets write Color Balance values; consistent 8/16/32-bit and GPU path parity remains a gap |
| Effects | Searchable effect library, animated parameters, presets | 🟡 | Large `EffectType` registry and controls; effect-standard parity is explicitly out of scope for exact cloning |
| Particles / procedural | Particles, lightning, star field, audio spectrum and generators | 🟡 | Engines and render paths exist; authoring, caching, and interaction depth vary by feature |
| Audio | Import, playback sync, mixing, meters, audio-to-keyframes | 🟡 | In-process Symphonia decoding covers WAV/MP3/FLAC/Ogg/AIFF/CAF/MP4-family audio; project-bin insertion, save/reload, undo/redo, and mixer output have an egui workflow test. Decoded buffers now share a sample-rate-aware LRU capped at 256 MiB/4096 entries; per-track metering, correction/interchange, long-session playback sync, and broader device testing remain |
| Preview | Cached frames, RAM preview, adaptive quality, audio sync | 🟡 | Cache and playback exist; GPU preview falls back for Puppet Pins, unsupported blend modes, Shape Freeform/Stroke/3D geometry, layer styles, multi-effect stacks, every effect whose shader math/parameter mapping is not yet verified against export, and Lens Flare linked to a scene light (the shader currently receives authored UV rather than the CPU-resolved light position). An actual GPU texture readback now compares Color Tint, Levels, unlinked Lens Flare, and RGB-only Invert against CPU output at representative pixels within 2/255; broader frame-wide/alpha/blend/scale coverage remains unfinished, so preview correctness takes precedence over GPU acceleration |
| Render queue | Multiple items, progress, cancellation/failure reporting | ✅ | `core/render_queue.rs`, `ui/render_queue.rs` |
| Export | FFmpeg video, image/EXR paths, GIF, Lottie, MLT/OTIO-style interchange | 🟡 | Several exporters exist; codec/metadata/alpha compatibility needs broader fixture testing |
| Essential Graphics / templates | Exposed controls, reusable motion-graphics templates, host-side overrides | 🟡 | Native MOGRT-style package and import/export UI exist; Adobe-host compatibility, richer property binding, and template validation remain narrower |
| Plugin/integration | Plug-ins, scripting, interchange with external tools | 🟡 | OFX/SDK bridges and Rhai exist; third-party plug-in compatibility is not complete |
| Team collaboration | Shared projects, presence, merge/sync, review comments | 🔴 | No equivalent shared-project/presence/review workflow found; Adobe documents Team Projects and Frame.io integration |
| Adobe ecosystem links | Dynamic Link, Creative Cloud Libraries, Cinema 4D integration | 🟡 | Kagari has internal project assets and limited interchange, but these Adobe-host integrations are not equivalent or broadly compatible |
| Workspaces | Dockable panels, resize, saved workspace layouts | 🟡 | SavedWorkspace persists panel widths, timeline height, graph mode, viewer state, and compact drawers; restore now runs before panel layout, while full dock visibility/custom panel topology remains narrower than AE |
| UI operations | Search/command palette, shortcuts, inspector, graph editor, undo/redo | 🟡 | Core paths exist; more real egui event/E2E tests are required for production confidence |
| Reliability | Bounded caches, crash recovery, deterministic renders, safe file I/O | 🟡 | Strong unit/integration coverage; sanitizer/stress and hostile-project coverage still need expansion |

## Current largest gaps outside exact effect parity

1. **Preview/render architecture** — GPU preview and CPU/export still have
   paths that do not share the same effect execution and cache behavior.
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
8. **Roto/paint temporal behavior** — a user can now apply geometry smoothing,
   feather, and expansion to the actual stored matte with Undo, and can choose
   which tracker drives its translation. This is not temporal segmentation:
   optical-flow propagation, propagation review, correction caching, and
   real-footage quality remain substantially behind AE.
9. **Audio correction and interchange** — production audio formats and the
   correction workflow need a complete non-ML baseline before model support.

## Next implementation slices and acceptance criteria

These are user workflows, not module checkboxes. A slice remains partial until
the authored project can be saved, reopened, previewed, and rendered without
losing its meaning.

| Priority | AE workflow to match | Kagari gap to close | Acceptance evidence |
|---:|---|---|---|
| 1 | Roto Brush / paint a subject over time | Brush input, per-frame matte propagation, edge refinement, correction/review, cache invalidation, and undo are not yet one reliable workflow | Import a short real clip, create and propagate foreground/background strokes across a frame range, correct a later frame, save/reopen, and compare preview/export mattes at several frames |
| 2 | Consistent composition preview and final render | GPU preview is intentionally restricted to an allowlist; supported paths still need measured pixel contracts and unsupported paths must visibly use software fallback | For every allowlisted effect and representative alpha/blend/scale cases, compare GPU preview to the software reference within a documented tolerance; assert unsupported stacks take the fallback path |
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

1. Complete the Roto/paint real-footage journey and fix the first measured
   propagation, correction, cache, or undo failure.
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
