# Code review — 2026-11-11

## Screen blend equation did not match the CPU renderer

The WGPU Screen pipeline used `One + OneMinusSrc` after the fragment shader premultiplied source RGB by source alpha. That computes `S_premul + D * (1 - S_premul)`, which is not Screen for a partially transparent source and caused the recorded GPU/CPU mismatch. The correct premultiplied Screen equation is `D + S_premul * (1 - D)`. The pipeline now uses `OneMinusDst + One`; alpha remains standard source-over. Static inspection confirms the shader still premultiplies RGB before this pipeline.

The first hardware comparison after the blend fix still failed at pixel (0, 0), with GPU value 13 versus CPU 187. Inspection showed a deeper transform mismatch: the GPU used AE-style anchor coordinates (position denotes the anchor), while the software renderer always treated position as the visual center and ignored the animated anchor. A temporary GPU-only center adjustment made the simple CLI case pass but broke the full-frame parity tests for centered anchors, so it was discarded. The final fix keeps the GPU's local-pixel-plus-anchor transform and updates the software renderer's raster center to `position + R*S*(layer_center - anchor)`. Both top-level and nested precomp layer paths use the same helper; the helper accounts for rotation and signed scale.

Validation: `cargo fmt --all --check` passed. In an isolated Linux worktree using `--no-default-features --features cli-gpu`, `cargo test --test gpu_preview_parity -- --nocapture` passed both full-frame GPU/software parity cases, including five allowlisted effects and a multi-effect chain. `cargo test --test bridge_acceptance cli_gpu_test_proves_the_hardware_backend_ran -- --nocapture` also passed; its CLI log asserted WGPU was used and all channels were within tolerance 2. `git diff --check` passed locally. The first targeted run before the CPU anchor fix failed at maximum error 174; the final two GPU suites passed.

## Follow-up: first Intersect/Darken/Difference mask erased the image

Mask composition started from transparent coverage and only initialized a full-frame matte for a leading Subtract mask. As a result, a first Intersect or Darken mask always combined against zero and erased the layer; Difference also had no meaningful first-mask baseline. The CPU reference and GPU mask rasterizer now treat the first effective non-Subtract mask as the initial matte, while preserving the full-frame baseline for a leading Subtract mask. This fixes the underlying shared behavior rather than routing the case to a slower renderer.

Validation: `cargo fmt --all --check`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Runtime rendering was not exercised; tests were not run. The GUI check reports the existing future-incompatibility notice for `block v0.1.6`.

## Follow-up: one oversized GPU mask bypassed the cache byte budget

The mask cache evicted entries while over 128 MiB only when more than one entry existed. A single large raster could therefore remain cached above the declared budget. Oversized rasters are now excluded from the cache, and an older entry with the same key is removed if encountered. The current render can still use its raster; only retention is skipped.

Validation: `cargo fmt --all --check`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Cache behavior was not runtime-exercised; tests were not run. The GUI check reports the existing future-incompatibility notice for `block v0.1.6`.

## Follow-up: replacing a particle checkpoint bypassed its memory budget

Particle checkpoint insertion returned immediately when a frame already existed, replacing that snapshot without accounting for its new size or evicting older snapshots. A checkpoint that grew during deterministic replay could push retained state past the 8 MiB budget. The old same-frame snapshot is now removed before aggregate size calculation, after which normal oldest-first eviction enforces the budget before reinsertion.

Validation: `cargo fmt --all --check`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Checkpoint growth/eviction was not runtime-exercised; tests were not run. The GUI check reports the existing future-incompatibility notice for `block v0.1.6`.

## Follow-up: Roto Brush matte preview filled the convex hull

The Roto Brush matte preview passed its boundary to egui's `convex_polygon`, which fills the convex hull and misrepresented concave subjects. It also sampled only keyframe vertices, omitting Bezier handles used by the actual rendered mask. Preview now samples the animated mask path with the renderer's curve segment count, triangulates simple concave polygons, and draws the true boundary. Degenerate or excessively detailed paths that cannot be triangulated show the boundary without an invented fill; self-intersecting paths remain outside the triangulator's supported input.

Validation: `cargo fmt --all --check`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. No on-screen concave roto preview was visually exercised; tests were not run. The GUI check reports the existing future-incompatibility notice for `block v0.1.6`.

## Follow-up: matte preview allocated the full renderer curve before applying its UI cap

`MaskPath::to_polygon` permits up to four million samples for render paths, while the UI triangulator accepts at most 4,096 points. The preview previously built the full polygon before the triangulator rejected an oversized path, potentially allocating millions of points on every repaint. It now checks the source vertex count first and lowers Bezier subdivisions to keep preview sampling within the triangulation cap; paths with more than 4,096 authored vertices are skipped without cloning or sampling them.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Large-path repaint allocation was not runtime-profiled; tests were not run. The GUI check reports the existing future-incompatibility notice for `block v0.1.6`.

## Follow-up: Roto Brush accepted strokes from letterbox margins and the tool strip

The interaction widget covers the full viewport, including letterbox margins. Roto Brush converted every pointer coordinate directly to composition space, so a stroke begun in the margin could write out-of-composition points. The floating tool strip also suppressed clicks and drags but left `drag_started` active. Roto samples now convert only finite coordinates inside the displayed composition rectangle, and the viewport clears drag-start events while the pointer is over the tool strip.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Pointer interaction was not exercised in the running UI; tests were not run. The GUI check reports the existing future-incompatibility notice for `block v0.1.6`.

## Follow-up: Roto Brush controls allowed conflicting async jobs

While frame segmentation was still running, Clear could delete the committed strokes and matte; the late worker would then restore its stale temporary stroke list after detecting a source-layer mismatch. Tracker propagation could also start against the pre-analysis project state. Clear and both propagation actions are now disabled while segmentation or optical-flow propagation owns the matte. Optical-flow propagation remains cancellable through its existing Cancel button.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Concurrent UI actions were not exercised; tests were not run. The GUI check reports the existing future-incompatibility notice for `block v0.1.6`.

## Follow-up: live Roto Brush feedback did not match the saved stroke

The live preview used a fixed six-point line, regardless of brush radius or viewer zoom. It also read foreground/background mode and radius again on release, so changing either during a drag changed the entire committed stroke without the preview showing that change. Live strokes now capture their radius and mode at pointer-down, retain both through completion, and draw at the corresponding composition-to-screen diameter.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Brush feedback was not visually measured at multiple zoom levels; tests were not run. The GUI check reports the existing future-incompatibility notice for `block v0.1.6`.

## Follow-up: tracker propagation read the wrong base shape at frame zero

`bake_tracked_mask` evaluated its base mask at `max(start_frame, 1)`, while tracker motion was anchored at `start_frame`. For a propagation starting at frame zero, an animated mask therefore used frame one's shape as the origin and shifted that wrong shape across the entire range. It now evaluates the base polygon at the requested start frame.

Validation: `cargo fmt --all --check`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Animated-mask propagation from frame zero was not runtime-exercised; tests were not run. The GUI check reports the existing future-incompatibility notice for `block v0.1.6`.

## Follow-up: DMG was not revalidated after signing or stapling

The packager verified the disk image before applying its optional code signature and notarization ticket. Those operations mutate the image after the check, so the exact file atomically published to the release path was not verified. It now runs `hdiutil verify` after all configured signing/notarization steps and immediately before publication.

Validation: `bash -n scripts/package_macos.sh bundle_dmg.sh` and `git diff --check` passed. End-to-end packaging was not run because `target/release/kagari-studio` is absent; the signing/stapling path was not exercised.

## Review slice: export integrity and honest capability reporting

### Changes reviewed

- EXR image construction now rejects zero or out-of-range dimensions, mismatched RGBA sample counts, and size overflow instead of accepting malformed buffers.
- EXR, PNG sequence, and text-based exports write through unique temporary files. EXR and PNG sequence outputs are published without replacing a file that appeared during export; EXR and text writes are finalized atomically.
- Sequence preflight checks canonical frame names across the whole output directory, including stale frames beyond the current duration. Project names are sanitized before being used as sequence prefixes.
- OpenShot and MLT export paths reject unsupported project features rather than silently dropping those features from the result.
- Video export finalization honors the overwrite decision and protects a destination that appears concurrently when overwrite was not confirmed.
- The `.kgtpl` format is presented as a Kagari template rather than implying compatibility with Adobe MOGRT; legacy loading remains supported.
- Embedded video audio gain is editable in the legacy inspector, and export controls were adjusted to avoid presenting unsupported options as functional.

### Why these changes

Malformed image buffers and partially written files can produce corrupt deliverables. Replacing an existing destination or mixing stale sequence frames can destroy or misrepresent prior work. Exporting unsupported timeline features without an error silently changes the project, so the affected exporters now fail closed for the features they detect.

### Validation

- `cargo fmt --check`: passed.
- `cargo check --no-default-features --features cli,wgpu --tests --lib`: passed; this compiled test targets but did not execute tests.
- `cargo check --no-default-features --features cli --bin kagari`: passed.
- `cargo check --no-default-features --features gui --bin kagari-studio-egui`: passed.
- `cargo check --no-default-features --features ame --bin kagari-studio`: passed after the AmeUI owner fixed an upstream type mismatch. The AmeUI owner separately reported 282 library tests passing and the related AmeUI regression test passing; those tests were not run by this Kagari review.
- `git diff --check`: passed.

### Remaining review limits

No tests were executed by this review. GUI workflows and interoperability in external EXR readers, OpenShot, and MLT consumers still need hands-on validation. Exporter rejection coverage should continue to be reviewed as the project model adds features.

## Follow-up: malformed WAV input could panic

`AudioBuffer::load_wav` derived bytes per sample before restricting the bit depth. A malformed PCM WAV with a nonzero bit depth below 8 (for example, 1 bit) therefore produced a zero-sized sample and frame, causing division by zero while calculating the output buffer capacity. The loader now validates PCM bit depth before arithmetic, checks RIFF/chunk bounds and block alignment, rejects incomplete sample frames and empty data, bounds decoded allocation, and uses fallible reservation.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. No tests were executed; malformed-input regression coverage remains to be added and run.

## Follow-up: GIF destination race could overwrite an existing file

The Studio checked that the selected GIF path was absent before starting export, but the GIF worker later finalized with `rename`, which replaces an existing destination on common platforms. A file created during rendering could therefore be silently overwritten. Added an overwrite-aware GIF entry point; Studio uses no-overwrite hard-link finalization, while the existing API keeps its prior overwrite behavior for CLI callers.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --bin kagari`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `git diff --check` passed. The first compile exposed an unused RAII guard binding, which was fixed and rechecked. Tests were not run. GUI race behavior and filesystem hard-link support still need runtime verification.

## Follow-up: shared PNG sequence API could overwrite or publish corrupt frames

The reusable PNG sequence API wrote directly to final filenames, so a collision overwrote files and an encoding failure could leave a truncated destination. It also accepted invalid dimensions, partial pixel buffers, zero-frame jobs, and overflowing frame ranges. It now validates the frame range and image buffer size, rejects unsafe filename stems, preflights existing frame names, encodes to a unique temporary file, and publishes with no-overwrite hard-link semantics by default. An explicit overwrite option preserves a caller's ability to replace frames. The Studio and legacy egui sequence paths call the protected default; the AmeUI Studio has its own writer with the same no-overwrite policy.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `git diff --check` passed. A final library check after tightening temporary-file ownership also passed. Tests were not run. Frames render to a per-export staging directory and publish only after rendering completes. A later follow-up adds rollback for ordinary publication failures; a process or machine crash during the multi-file commit can still leave a partial sequence. External consumer behavior and hard-link support on removable/network filesystems still need runtime validation.

## Follow-up: CLI GIF export rejected projects with embedded video audio

The CLI populated `ExportConfig.audio_wav` from the composition before calling the GIF-specific exporter. Shared validation correctly rejects audio for GIF, so any project containing embedded video audio failed even though GIF should simply omit audio. The CLI GIF configuration now passes no audio and identifies its codec as GIF; video and audio export paths are unchanged.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --bin kagari`, and `git diff --check` passed. Tests and an end-to-end GIF export with an audio-bearing project were not run.

## Follow-up: OTIO import could silently change track type or truncate content

The standard OTIO parser defaulted a missing track kind to Video and treated every unknown kind as Video. It also trusted Kagari `duration_frames` metadata even when it was shorter than parsed track content, and ignored malformed duration metadata. The parser now accepts only explicit Audio/Video track kinds, validates the optional duration value, rejects zero, and rejects a declared timeline duration shorter than its tracks.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. OTIO fixtures and external reference applications were not run.

## Follow-up: failed project autosave disabled retries and unsaved-change prompts

The Studio derived project dirtiness from the visible `save_status` string. After a project autosave failure, that string changed to an error, so future autosaves stopped retrying; project-open/new-project confirmation also only recognized the exact unsaved-status text. A manual save failure had the same confirmation bypass. Project dirtiness is now tracked independently: failed autosaves retry after the configured interval, failed manual saves remain dirty, and project switching prompts based on the actual dirty state. Successful autosaves remain dirty relative to the main project file; only a successful manual save or loading a project clears the flag. Creating a new project marks it dirty.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `git diff --check` passed. Save-failure retry timing, project-switch confirmation, and actual filesystem failure recovery were not exercised end to end. An autosave written directly to the main project path clears dirty state; a separate recovery-folder snapshot does not.

## Follow-up: backup pruning could delete another project's generations

`prune_backup_files` selected snapshots using `starts_with(stem)`, so pruning backups for project `A` could include files belonging to `A2`; different project folders with the same filename stem also shared one backup series. Snapshot names now include a stable hash of the canonical project path plus process and sequence identifiers, and pruning matches that project-specific prefix and recognized timestamp shapes. New-project setup also displays the unsaved state that its dirty flag already represented.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `git diff --check` passed after the project-scoped backup naming change. The first Studio compile was blocked by an unrelated AmeUI `layout.rs` stale-name error; the AmeUI owner fixed it, and a fresh Studio check passed against the corrected shared source. No tests were run. Existing backups made with the old unscoped naming are left untouched by the new pruning rule.

## Follow-up: exit confirmation told users to save but offered no save action

The AmeUI Studio's exit dialog warned users to save unsaved work, but only offered Cancel and Exit. Added a bound dirty-state indicator and distinct Save and Exit, Exit Without Saving, and Cancel actions. Save and Exit calls the existing save flow and exits only after a successful save; cancelling the save dialog or a write error leaves the exit dialog open. Clean projects retain a simple Exit confirmation.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `git diff --check` passed. Save-dialog cancellation and write-failure behavior were not exercised interactively. No tests were run.

## Follow-up: saving a restored project could overwrite its valid backup with corrupt data

When the main project file was corrupt but its `.bak` was valid, loading from that backup and then saving would first copy the corrupt primary over the valid `.bak`. If the new save then failed, both copies could be unusable. `save_project_atomic` now refreshes the backup only when the existing primary parses and validates as a Kagari project or ProductionDocument; corrupt or unsupported primaries no longer replace a recovery backup. Review also found that the Studio's Project-only save path replaced a valid ProductionDocument and discarded its audio, tempo, and automation metadata. Saves now update the embedded Project while preserving that metadata; when the primary is invalid, a valid ProductionDocument backup supplies the metadata template. Backup loading accepts either file format.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `git diff --check` passed after the ProductionDocument preservation change. The AmeUI owner had already changed the unused parameters to underscore-prefixed names, so the two warnings from the previous check no longer appeared. The existing `block 0.1.6` future-incompatibility notice remains. No tests were run; corrupt-primary recovery and save-failure behavior still need regression coverage and runtime validation.

## Follow-up: cancelling Preferences could leave runtime settings changed

The GPU-effects toggle and undo-history cap were applied while editing the Preferences draft, so Cancel restored the displayed values but left the runtime using the abandoned values. Settings were also persisted one key at a time, allowing a disk error to leave a partially applied preference set. GPU/undo runtime changes now apply only after the full preference set is atomically written. The cache directory is checked before commit and its runtime change is rolled back if the atomic preference write fails. Cancel restores runtime GPU, undo, and cache-directory settings after a failed commit.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `git diff --check` passed. The check emitted only the existing `block 0.1.6` future-incompatibility notice. Tests were not run. Preferences cancellation and simulated write/cache failures were not exercised interactively.

## Follow-up: variable-size compressed audio packets could panic during decode

`load_audio` created a Symphonia `SampleBuffer` sized from the first decoded packet and reused it without checking later packet capacities. Symphonia asserts that the destination fits each source packet, so valid streams with a larger later packet could panic. The decoder now enforces the decoded-size limit per packet, grows the reusable buffer when needed, rejects sample-rate/channel-layout changes mid-stream, and reserves accumulated samples fallibly.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `git diff --check` passed. The check emitted only the existing `block 0.1.6` future-incompatibility notice. No tests were run; variable-size packet behavior was verified by reviewing Symphonia's `SampleBuffer` capacity assertion and is not covered by a runtime fixture here.

## Follow-up: image import could multiply decoder memory without a bound

`image::open` limits decoder allocation, but EXR/PNG/TIFF import then expands pixels into three or four `f32` planes. That second allocation had no limit and used infallible `Vec::with_capacity`, so large but decodable images could exhaust app memory. Import now checks the expanded channel-plane size against a 512 MiB budget, checks dimension arithmetic, and uses fallible reservations for every plane. The reader docs now describe this limit.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `git diff --check` passed. The check emitted only the existing `block 0.1.6` future-incompatibility notice. No tests were run; large-image rejection and allocation-failure paths were not exercised with fixtures.

## Follow-up: Studio EXR export requested compression unavailable in UI builds

`ExrImage::rgba_f32` defaults to ZIP compression, but `zip_deflate` was compiled only with the `cli` feature. AmeUI Studio and legacy egui builds therefore failed on every normal EXR export with “EXR ZIP compression requires the cli feature.” Added a shared `exr-compression` feature and enabled it for CLI and both UI builds, keeping the default ZIP output and compression support consistent across supported entry points. Updated the disabled-feature error to name the actual feature rather than implying CLI is required.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, `cargo check --no-default-features --features cli --bin kagari`, and `git diff --check` passed. The UI checks emitted only the existing `block 0.1.6` future-incompatibility notice. Tests and an end-to-end EXR sequence export were not run.

## Follow-up: GIF export could leave palette intermediates behind

The two-pass GIF worker manually removed its temporary palette on most failures, but cancellation during palette generation and malformed-frame or pipe-write failures could return without cleanup. Added an RAII cleanup guard for the palette file so every return path removes the intermediate, including unwinding. The encoded GIF continues to use its separate output guard.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. The compile check updated `Cargo.lock` while resolving optional AmeUI dependencies; that generated lockfile change was restored. No tests or GIF export runtime checks were run.

## Follow-up: disk-cache allocation failure could abort frame loading

Disk-cache reads validated dimensions and capped decoded pixels at 512 MiB, but allocated the buffer with `vec![0; size]`. Under memory pressure, the infallible allocation could panic instead of treating the cached frame as unavailable. The reader now uses `try_reserve_exact` and returns a cache miss when allocation fails.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. The compile check updated `Cargo.lock` while resolving optional AmeUI dependencies; that generated lockfile change was restored. No tests or memory-pressure runtime checks were run.

## Follow-up: generated glTF normals broke across UV seams

When a glTF primitive omitted `NORMAL`, the loader accumulated face normals by expanded vertex index. UV seams commonly duplicate a geometric vertex into separate indices, so those duplicates received different normals and rendered as a visible lighting split. Generated normals now accumulate across vertices at the same transformed position within the same primitive instance. Separate primitive instances remain isolated to preserve authored boundaries.

The expanded-size preflight now accounts conservatively for generated-normal keys and hash buckets. Reservations for those structures are fallible, so memory pressure returns an import error rather than relying on an infallible growth path.

Validation: `cargo fmt --all` and `git diff --check` passed. On the Linux builder, the CLI library check and AmeUI Studio check passed. Clippy passed with the existing `needless_borrows_for_generic_args`, `approx_constant`, and `useless_conversion` lint classes allowed; strict Clippy still reports seven instances of those unrelated warnings elsewhere in the workspace. No tests or runtime renders were run. Position-based smoothing cannot infer intentional hard edges represented only by duplicated positions inside a single primitive; assets that need those hard edges should provide `NORMAL`.

## Follow-up: strict Clippy findings in timeline, OTIO, and disk-space code

The review's strict Clippy run identified four needless borrows of the copyable OTIO frame-rate closure, two hand-written golden-ratio approximations, and one redundant `u64::from` conversion on Linux's `statvfs` field types. The OTIO mappings now pass the closure directly, expression evaluation uses `f32::consts::GOLDEN_RATIO`, and the disk-space conversion keeps portable field-width conversion with a narrowly scoped lint allowance for platforms where the source type is already `u64`.

Validation: `cargo fmt --all`, `git diff --check`, strict CLI-library Clippy (`-D warnings`), and the AmeUI Studio `cargo check` passed on the Linux builder. No tests were run.

## Follow-up: FFmpeg error formatting could panic on multibyte text

Video and GIF export error paths sliced stderr at byte 500. Since stderr is decoded as UTF-8, a non-ASCII diagnostic crossing that byte could panic while reporting the original encoder failure. All three formatting sites now use a shared UTF-8-boundary-aware truncation helper.

Validation: `cargo fmt --all`, `git diff --check`, the Linux CLI-library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were added or run; the multibyte truncation boundary was verified by code inspection, not a runtime fixture.

## Follow-up: FFmpeg stderr-reader thread creation could orphan encoders

The export paths spawned a stderr-draining thread with infallible `thread::spawn` after launching FFmpeg. If thread creation failed, the export worker could panic while the encoder remained alive and its temporary output was being cleaned up. `drain_stderr` now uses fallible thread creation, kills and waits for FFmpeg on failure, and returns the error for video and both GIF passes to report.

Validation: `cargo fmt --all`, `git diff --check`, Linux CLI-library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run; thread-creation failure was not induced at runtime.

## Follow-up: relative image-sequence paths were rejected

When a sequence source was passed as a bare filename such as `shot_0001.png`, directory scanning returned `./shot_0001.png` while the selected path remained `shot_0001.png`. Exact `Path` equality therefore rejected valid sequences in the current directory. The source membership check now compares the filename within the already-scanned source directory.

Validation: `cargo fmt --all`, `git diff --check`, Linux CLI-library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run; relative-path import was not exercised end to end.

## Follow-up: mask raster cache was bounded only by item count

The renderer retained up to six mask rasters regardless of their resolution; six 512 MiB rasters could consume 3 GiB. The FIFO cache now also has a 128 MiB byte budget, updates byte accounting when a key is replaced, and retains a single oversized raster by itself to avoid re-rasterizing it on every frame.

Validation: `cargo fmt --all`, `git diff --check`, and the Linux AmeUI Studio `cargo check` passed. The cache's byte-budget eviction behavior was reviewed statically; no tests or runtime memory-pressure checks were run.

## Follow-up: project validation did not limit mask-path complexity

Project JSON had a total file-size limit but no cap on mask path vertices. Each anchor expands to 16 samples during raster preparation, so a compact, high-vertex mask could allocate a very large polygon before rasterization. Validation now caps each path at 65,536 anchors and the combined maximum path size per layer at 262,144 anchors, including animated paths.

Validation: `cargo fmt --all`, `git diff --check`, Linux CLI-library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run; oversized mask rejection was not exercised with a fixture.

## Follow-up: mask polygon sampling could overflow or abort on allocation

`MaskPath::to_polygon_into` multiplied the anchor count by a caller-provided segment count without checked arithmetic, then used infallible `Vec::reserve`. It now checks the sample-count calculation, caps a polygon at 4,194,304 points, and leaves the output empty if fallible reservation fails.

Validation: `cargo fmt --all`, `git diff --check`, Linux CLI-library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run; extreme segment-count rejection and allocation failure were not induced at runtime.

## Follow-up: legacy project APIs read unbounded JSON and accepted future versions

The public `.aeproj` compatibility loaders bypassed the bounded reader used by Studio and CLI, allowing arbitrarily large files to allocate memory during `read_to_string`. Both legacy loaders now enforce the shared 100 MiB limit while reading. The versioned loader also rejects versions newer than 1 instead of silently interpreting them as version 1.

Validation: `cargo fmt --all`, `git diff --check`, Linux CLI-library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run; oversized-file and future-version rejection were not exercised with fixtures.

## Follow-up: legacy project saves could truncate the only copy

The public composition and versioned-project save functions wrote directly to their destination with `fs::write`, which truncates an existing project before serialization reaches disk. They now write and sync a uniquely reserved temporary file beside the destination, then atomically replace the target. Existing destination permissions and symlink targets are preserved when available.

Validation: `cargo fmt --all`, `git diff --check`, Linux CLI-library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run; interrupted-write recovery and symlink destinations were not exercised end to end.

## Follow-up: FFmpeg polling errors could leave worker processes behind

The video importer returned immediately when `Child::try_wait` reported an operating-system error. That path did not kill or reap FFmpeg and detached the stderr reader thread. Export finalization had the same child-process leak: its guard was taken after polling failed, so its destructor could no longer reap FFmpeg. Both paths now kill and wait for the child before returning the original error; the import path also joins its stderr reader, and export callers join theirs after the helper returns.

Validation: after both changes, `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. The check updated `Cargo.lock` while resolving optional AmeUI dependencies; that generated change was restored. The OS-error cleanup path was not induced at runtime, and no tests were run.

## Follow-up: compressed audio decoding could silently return truncated audio

The Symphonia loader skipped packet decode and I/O errors, and treated a stream reset request as end-of-file. Callers therefore received a successful but potentially truncated audio buffer. Decode, packet I/O, and stream-configuration reset failures now return explicit errors so preview and exports cannot mistake partial audio for a complete decode.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. No damaged or chained-stream fixture was exercised, and no tests were run.

## Follow-up: project save staging could overwrite an unowned temporary path

Project saves and backup refreshes used predictable per-process temporary names with non-exclusive creation. A stale or pre-existing path could be overwritten, and a later failure could delete that path even if this save did not create it. Both staging paths now use exclusive `create_new`; cleanup only removes a temporary file after this operation successfully created it.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. The first compile caught an ambiguous closure result type, which was annotated before the successful recheck. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. No collision/symlink scenario was exercised, and no tests were run.

## Follow-up: peak analysis could overflow on invalid time inputs

`AudioBuffer::peak_at` converted arbitrary floating-point time values to `usize`, then multiplied the start index by the channel count without saturation. Infinite or otherwise extreme public API inputs could panic in checked builds or wrap in optimized builds. The method now rejects non-finite/negative windows and invalid audio metadata, calculates in `f64`, and clamps both frame bounds to the available samples.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. Boundary inputs were not executed as tests.

## Follow-up: waveform peak requests could trigger unbounded allocation

`waveform_peaks` used the caller's requested bin count directly for allocation and index arithmetic. Extremely large requests could panic or exhaust memory, including for an empty buffer. It now caps output at one million bins and at the available sample count, reserves fallibly, and partitions samples using overflow-safe proportional boundaries.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. Oversized-bin requests were not executed as tests.

## Follow-up: cache staging could truncate or remove an unrelated temporary file

Frame-cache writes used `File::create` on predictable process/counter paths, and cache-directory validation attempted to remove its probe even when exclusive creation had failed. A stale path could therefore be truncated or deleted. Both paths now track ownership from successful exclusive creation; frame cache files are only published and cleaned up when owned, and a successfully created probe must be removed for validation to pass.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. Temporary-name collision scenarios were not exercised.

## Follow-up: EXR HALF output could silently turn large finite values into infinity

EXR channels were checked for non-finite `f32` values, but finite values outside the HALF range passed validation and converted to HALF infinity during writing. HALF channels now reject values whose conversion is non-finite before any output is written.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. HALF overflow values were not run through an EXR reader fixture.

## Follow-up: production-document and video-manifest staging could overwrite stale files

Production Document saves and video frame-cache manifest writes also used predictable temporary names with truncating creation and unconditional cleanup. Both now require exclusive temporary-file creation and remove a file only when the current operation created it.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. Stale temporary-name collisions were not exercised.

## Follow-up: Dynamic Link cache startup could delete active temporary files

The persistent cache used shared, key-only `.tmp` names and deleted every temporary file during startup or `clear()`. A second application process could remove another writer's in-progress cache entry. Writes now use per-process sequence names with exclusive creation and sync both staged files; startup and clearing only remove temporary files older than 24 hours.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. The first compile caught a borrowed temporary path in `clear()`; binding the path before inspecting its extension fixed it. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. Multi-process cache writes were not exercised.

## Follow-up: CLI MP4/GIF silently ignored sparse frame selections

The CLI accepted explicit frame lists and `--step` for every output format, but MP4/GIF workers rendered `from..=to` and ignored the selected frame vector. This could produce a different video from the requested frames and misalign audio. Video formats now reject non-contiguous selections before rendering or audio preparation; CLI help describes the format limitation, while PNG/EXR retain sparse selection support.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --bin kagari`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. No sparse MP4/GIF command was executed.

## Follow-up: CLI PNG failures could truncate existing outputs

The CLI PNG helper opened final paths with `File::create`, destroying any previous frame before encoding had succeeded. It now validates the RGBA byte count, writes and syncs a uniquely named exclusive temporary file, then atomically renames it over the requested output. On encoding errors, only the owned temporary is removed.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --bin kagari`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. No encoder-failure or overwrite scenario was exercised.

## Follow-up: CLI OTIO import bypassed atomic project save and document preservation

OTIO import serialized the updated `Project` directly over the source file. A write failure could corrupt the project, and importing into a ProductionDocument could discard its audio, tempo, and binding metadata. The command now saves through `save_project_atomic`, which retains a valid ProductionDocument wrapper and its recovery behavior.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --bin kagari`, and `git diff --check` passed. The first check attempt hit a full shared build volume; cleaning only `kagari-vfx` build artifacts freed 3.3 GiB, after which the check passed. Cargo's generated `Cargo.lock` update was restored. OTIO import against a ProductionDocument was not run end to end.

## Follow-up: CLI Lottie/OTIO exports could overwrite their source project

Lottie and OTIO exports wrote directly to the selected output path, including when it resolved to the input project, and could leave a partial export on failure. Both now reject output paths resolving to the source project and write through an exclusive same-directory temporary file before atomic replacement.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --bin kagari`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. Same-path and write-failure scenarios were not executed.

## Follow-up: CLI edit commands bypassed project recovery and document preservation

Layer, effect, animation, trim, transition, parenting, matte, and mask commands serialized `Project` directly to the input path. A failed write could corrupt the project, and ProductionDocument metadata could be lost. All project-mutating CLI edit commands, including initialization, now use the atomic project migration save path, which preserves a valid ProductionDocument wrapper and refreshes its backup.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --bin kagari`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. The individual edit commands were not executed against a ProductionDocument fixture.

## Follow-up: render CLI project transformations still wrote project files directly

Project bundling, layer/camera/animation modifications, presets, and generated music-video projects still used `fs::write` on project paths. Those paths could truncate a source project on failure or discard ProductionDocument metadata. All project serialization in the render CLI now goes through the atomic migration save helper; bundle and generated outputs use the same path for consistent publication.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --bin kagari`, and `git diff --check` passed. Cargo refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. Render transformations were not executed against project fixtures.

## Follow-up: image cache validated dimensions before reopening the image

The image cache checked header dimensions, reopened the path for decoding, then converted the decoded image to RGBA before checking its actual dimensions and cache budget. If the file changed between the two opens, the RGBA conversion could allocate beyond the configured cache limit. It now validates decoded dimensions and required RGBA bytes before conversion, then verifies the converted buffer length.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. The compile check updated `Cargo.lock` while resolving optional AmeUI dependencies; that generated lockfile change was restored. No tests or concurrent-file-replacement runtime checks were run.

## Follow-up: SVG source size check was vulnerable to a growing file

SVG loading checked file metadata and then used `std::fs::read`, which could allocate the whole file if it grew after the metadata check. The loader now reads at most the 32 MiB source limit plus one byte and rejects oversized content before parsing.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. The compile check updated `Cargo.lock` while resolving optional AmeUI dependencies; that generated lockfile change was restored. No oversized-file race fixture was run.

## Follow-up: WAV size limit could be bypassed between stat and read

WAV loading checked metadata through the path, then reopened the path with `std::fs::read`. A file replacement or growth between those calls could bypass the 512 MiB check and allocate the full contents. Loading now opens once, checks that handle's metadata, and reads at most the limit plus one byte before rejecting oversized data.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. The compile check updated `Cargo.lock` while resolving optional AmeUI dependencies; that generated lockfile change was restored. No file-growth race fixture was run.

## Follow-up: video import could silently lose audio

When FFmpeg detected source audio but failed to extract a usable WAV, video import still reported success without telling the user. Long clips could also exceed the audio engine's 512 MiB decoded PCM cap, leaving an extracted WAV that could not be loaded later. `VideoAsset` now carries an optional audio warning; video import omits audio when the decoded size would exceed the engine limit, excludes skipped audio from its disk estimate, reports extraction failures, and removes invalid partial WAVs when possible. AmeUI Studio, legacy egui import/relink, and project cache recovery surface the warning.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, a subsequent CLI library check after partial-WAV cleanup, `cargo check --no-default-features --features ame --bin kagari-studio`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `git diff --check` passed. CLI checks updated `Cargo.lock` while resolving optional AmeUI dependencies; generated lockfile changes were restored. No FFmpeg audio failure or long-duration media fixture was run.

## AmeUI handoff: dependency compile warnings

The Kagari AmeUI Studio check reported 17 warnings originating in AmeUI's `src/view.rs` and `src/view/controls.rs`: ambiguous glob re-exports, an unused glob import, and dead code in duplicate named constructors. Added P3 REQ-348 to AmeUI's canonical request list using the repository's request format, then sent it to the AmeUI owner thread. The owner resolved it and moved it to `リクエスト履歴.md`; constructors are consolidated while their public paths remain available, and the targeted warnings no longer appear in Kagari's AmeUI Studio check.

Validation: AmeUI `cargo check --lib` and `cargo clippy --lib -- -D warnings` passed, and Kagari `cargo check --no-default-features --features ame --bin kagari-studio` passed without the reported warnings. The request was transferred from the open list to history. No AmeUI code was modified in Kagari.

## Follow-up: MLT and OpenShot exports could silently drop newer layer features

The exporters rejected many unsupported layer types and legacy compositing features, but accepted image/video layers with newer render-affecting state such as layer styles, paint strokes, puppet pins, posterize time, auto-orient, and preserve-transparency. MLT also silently skipped visible layers with invalid or out-of-composition frame ranges. Both exporters now reject those unsupported render features, and MLT rejects invalid visible-layer ranges instead of omitting them.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. The Cargo check refreshed optional dependency resolution in `Cargo.lock`; that generated change was restored. No OpenShot/MLT application import or export round-trip was run.

## Follow-up: PNG sequence publication could leave a mixed output after failure

PNG sequences render to a staging directory, but the final per-frame publication loop previously returned on the first filesystem error and left all earlier published frames in place. Cancelling during publication was also ignored. Publication now uses no-overwrite hard links, backs up the exact pre-existing frames when overwrite was explicitly requested, checks cancellation while committing, and rolls back files created by the current export after failure. Rollback checks file identity before removing outputs; if a concurrent replacement is observed, that path is left untouched. If the staged identity is missing while the destination still exists, rollback is reported incomplete instead of claiming success. If restoration cannot finish, the hidden staging directory is preserved and its location is included in the error.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli,wgpu --tests --lib` (test targets compiled, not executed), a subsequent `cargo check --no-default-features --features cli --lib`, and `git diff --check` passed. Cargo checks refreshed optional dependency resolution in `Cargo.lock`; generated changes were restored. No filesystem fault-injection or crash-during-commit test was run; a process/machine crash can still interrupt a multi-file commit.

## Follow-up: macOS package version and local/CI bundles could drift

The local DMG script and CI independently assembled the same `.app`, both with a hard-coded `0.1.0` bundle version while the Cargo package is `0.3.0`. The local script also omitted the native icon that CI included. Packaging is now centralized in `scripts/package_macos.sh`; both paths use it, bundle versions come from Cargo metadata, the icon and Applications shortcut are included, and the generated property list is linted before creating the DMG.

Validation: `bash -n` passed for both packaging scripts, the CI YAML parsed successfully, and `cargo metadata --no-deps` reported the expected `0.3.0` source version. End-to-end DMG creation was not run because no release binary exists in the current worktree. The package is still unsigned and unnotarized, so public macOS distribution requires signing credentials and a notarization step.

### Follow-up: prepare the macOS package for signed public distribution

`scripts/package_macos.sh` now supports Developer ID signing and `notarytool` submission through `KAGARI_CODESIGN_IDENTITY` and `KAGARI_NOTARY_PROFILE`. It verifies the app and disk image, staples and validates the notarization ticket, and only moves the staged DMG to its final path after every configured step succeeds. Unsigned and signed-but-unnotarized builds emit explicit Gatekeeper warnings. README documents the release setup. This worktree has no release binary, signing identity, or notarization profile, so the macOS package was not built or notarized.

Validation: `bash -n scripts/package_macos.sh bundle_dmg.sh` and `git diff --check` passed. End-to-end macOS signing and notarization remain unverified until release credentials are available.

## Follow-up: Finder-launched macOS Studio could miss Homebrew FFmpeg

Video import and export relied on process `PATH`. A Studio app launched from Finder may not inherit shell initialization that adds Homebrew or MacPorts directories, so FFmpeg could be installed yet reported missing. Added a shared executable resolver that honors the existing `PATH` first and then checks standard macOS Homebrew, MacPorts, and Nix locations for both `ffmpeg` and `ffprobe`. Video import, probes, MP4/GIF export, and availability checks now use it. README documents this launch behavior.

Validation: `cargo fmt --check`, `cargo check --no-default-features --features cli --lib`, `cargo check --no-default-features --features ame --bin kagari-studio`, `cargo check --no-default-features --features cli,wgpu --tests --lib` (test targets compiled, not executed), and `git diff --check` passed. The checks emitted only the existing `block 0.1.6` future-incompatibility notice. No runtime check with Finder or a Homebrew FFmpeg installation was performed.

## Follow-up: autosave could publish a partial rolling backup after a crash

Autosave copied the current project directly to the final rolling-backup filename. A process or machine failure during that copy could leave a truncated snapshot with a valid-looking final name. Snapshots now copy into an exclusive temporary file, sync the file, and publish it with a no-overwrite hard link; older generations are pruned only after publication succeeds. The temporary file is cleaned up on errors.

Validation: `cargo fmt --check`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, and a subsequent `cargo check --no-default-features --features ame --bin kagari-studio` passed. The initial Studio check hit 755 errors while AmeUI's `src/lang.rs` module split was in progress; the follow-up check passed after that working tree changed. AmeUI later moved P0 REQ-349 to `リクエスト履歴.md` as complete after its Linux CI passed (`cargo check` and 282 tests) and Kagari's Studio check passed. No tests or crash/fault-injection checks were run for the autosave snapshot change.

## Follow-up: FFmpeg could hang after producing more than 1 MiB of stderr

The stderr reader used a 1 MiB limited reader and stopped reading once its capture limit was reached. A verbose FFmpeg process could then fill the OS pipe and block forever while the exporter waited for it. The reader now retains at most 1 MiB but continues draining and discarding bytes until EOF, preserving the diagnostic bound without back-pressuring the child process.

Validation: `cargo check --no-default-features --features cli --lib` passed. No test or FFmpeg stress run exceeding the capture limit was performed.

## Follow-up: canceling during FFmpeg finalization did not stop the export

Video and GIF exports checked cancellation while rendering and writing frames, but then used a blocking `wait()` after closing FFmpeg's input. During a long final encode or container flush, cancel requests were ignored and the completed file could still be published. The workers now poll process completion while observing the cancel flag, kill and reap FFmpeg on cancellation, wait for stderr draining to finish, and skip publishing the temporary output.

Validation: `cargo check --no-default-features --features cli --lib` passed. No long-running FFmpeg cancellation scenario was exercised, and no tests were run.

## Follow-up: video export cancellation did not interrupt audio preparation

The Studio prepares a temporary WAV before starting FFmpeg, but the mix loop had no cancellation input. A cancel request during this potentially long phase was ignored until the entire soundtrack finished mixing. Added a cancelable mixer entry point that checks once per video frame and removes its partial WAV on cancellation; existing mixer callers retain their original API. Studio now uses the cancelable path and reports cancellation as such.

Validation: `cargo check --no-default-features --features ame --bin kagari-studio` passed. No long-duration audio mix cancellation scenario was exercised, and no tests were run.

## Follow-up: 3D camera and light drags created one undo entry per pointer move

`mark_project_changed` grouped timeline, graph, mask, and layer-position drags into one history action, but omitted `camera_nav_drag` and `light_gizmo_drag`. Their pointer-move handlers therefore committed a full project snapshot on every event and required repeated undo to reverse a single gesture. Both drag states now participate in the shared interaction snapshot, so the pointer-up path commits one undo entry for the complete gesture.

Validation: `cargo check --no-default-features --features ame --bin kagari-studio` passed. Undo/redo behavior was not exercised interactively, and no tests were run.

## Follow-up: undo could leave stale pointer gestures active

History restore replaced the project and cleared timeline, graph, layer, and mask drags, but left camera/light gizmo, viewport pan/pen, work-area, and flowchart-wire gestures alive. Continuing the old pointer sequence could immediately mutate the restored project or create a connection after undo. Restoring history now cancels every active canvas/timeline gesture and clears transient snap/scrub state before applying the restored project.

Validation: `cargo fmt --check`, `git diff --check`, and a follow-up `cargo check --no-default-features --features ame --bin kagari-studio` passed after AmeUI fixed and committed its duplicated `find_tree_disclosure` source as part of `d3c7cc99`. The AmeUI Linux CI log for that exact commit shows `cargo check` and 282 tests passing (`UNIT TEST OK`); REQ-350 was moved to `リクエスト履歴.md`. No interactive undo-during-drag scenario or Kagari tests have been run.

## Follow-up: lowering the undo limit did not release existing history immediately

Changing the Preferences undo limit only updated the configured cap; existing entries stayed resident and usable until another edit committed. The cap now trims history as soon as it changes. Trimming first drops the furthest redo states, then the oldest undo states, while keeping the active project entry and index coherent even when the user is currently at the beginning of a redo branch.

Validation: `cargo check --no-default-features --features cli --lib` passed. No runtime preference change or undo/redo branch scenario has been exercised, and no tests were run.

## Follow-up: Mocap and QC could truncate existing outputs on failure

The CLI's Mocap pose/track exporters and QC report writer wrote directly to user-selected output paths. A serialization or I/O failure after opening the file could leave an incomplete artifact and destroy the prior output. They now publish through the shared exclusive-temp, sync, rename helper already used by text exports, preserving existing outputs until serialization and writing complete.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No failure-injection or overwrite behavior was exercised; no tests were run.

## Follow-up: the script command still used a predictable project-save temporary

The CLI script command bypassed the project migration storage helper for plain JSON projects and wrote through a predictable `.kagari_save_<pid>` path. That could collide with a stale file or another same-process invocation, and it omitted the common project's durability and cleanup behavior. It now uses `save_project_atomic`, matching the other project-mutating CLI commands and retaining the ProductionDocument-specific save path when applicable.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No crash injection or script execution was performed; no tests were run.

## Follow-up: image-sequence directory read errors were silently ignored

Image-sequence discovery flattened `read_dir` results, discarding per-entry I/O errors. If enumeration failed partway through, the importer could validate and cache only the visible contiguous subset, presenting a shortened sequence as a successful import. Enumeration now propagates any entry error and aborts before creating the import cache.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. The directory-enumeration failure path was not fault-injected; no tests were run.

## Follow-up: video decode could count only a partially enumerated frame directory

After FFmpeg completed, the importer flattened `read_dir` entries while counting generated frames. A mid-enumeration I/O error could hide later files and let a shorter contiguous prefix pass cache verification as a successful import. The count now requires the directory enumeration to complete and reports entry errors before publishing the asset.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No filesystem fault injection or video import was run; no tests were run.

## Follow-up: nested precomposition audio mixing lacked a recursion guard

The renderer and structural audio-source scan limit precomposition traversal, but the actual audio mixer recursively followed nested precomp references without a depth bound. A cyclic or pathologically deep precomp graph could overflow the stack during preview or audio export. Audio traversal now uses the renderer's shared `MAX_PRECOMP_DEPTH` limit and stops consistently at that boundary.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No cyclic-precomp runtime scenario was exercised; no tests were run.

## Follow-up: malformed tracker coordinates silently changed or skipped tracking setup

The `kagari track --pos` handler ignored individual parse failures and accepted extra coordinate components. A malformed value could silently fall back to the composition center for a new tracker or leave an existing tracker's position unchanged while analysis proceeded. It now parses exactly two finite coordinates before loading or mutating the project; non-finite search-window sizes are rejected as well.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No tracking command was executed; no tests were run.

## Follow-up: CLI layer and keyframe edits silently dropped malformed numeric components

Several `edit` subcommands used `filter_map(parse)` for solid colors, custom Bezier handles, position/scale keyframes, and mask coordinates. An input containing one typo or non-finite value could be partially applied while the command still saved the project. These paths now parse every component, reject non-finite numbers, and enforce the supported component counts for colors and transform keyframes before saving.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No edit command was executed; no tests were run.

## Follow-up: camera edit inputs could be silently ignored or saved as non-finite values

The CLI camera command accepted `--position` and `--rotation` by filtering parse failures and only applying when at least three numbers remained; malformed values could therefore be silently ignored. Scalar camera values also accepted `NaN`/infinity into project state. Camera vectors now require exactly three finite components, and FOV, focus distance, aperture, and maximum blur reject non-finite inputs before project loading or mutation.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No camera edit command was executed; no tests were run.

## Follow-up: Shape Animation and particle CLI could silently ignore invalid vectors or presets

Shape repeater offsets and particle emitter positions filtered malformed numeric components, accepted extra values, and could silently preserve defaults when explicit inputs were invalid. The particle command also documented an `explosion` preset that fell through to the generic default emitter, while unknown preset names did the same. These arguments now require exact finite vector values, reject non-finite emission rates, reject unknown preset names, and provide the documented explosion configuration.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No CLI edit command was executed; no tests were run.

## Follow-up: Audio Spectrum CLI controls were disconnected from the renderer

The CLI claimed 64 bars and arbitrary frequency bounds, but the effect renderer read only five expression bands and discarded the configured frequency range. The effect model now stores backward-compatible start/end frequency fields, the renderer injects a 512-bin logarithmic spectrum alongside the existing five expression bands, and each visualizer samples the configured frequency interval at its requested 1–256 bar count. The legacy effect inspector exposes band count and frequency bounds. The CLI now restores the advertised 64-bar default and frequency controls, validates their ranges, and fails early when the source file is missing instead of silently creating an audio-less visualizer.

Validation: `cargo fmt --check`, `git diff --check`, `cargo check --no-default-features --features cli --bin kagari`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. The Studio `ame` feature check was attempted but is currently blocked by compilation errors in the shared AmeUI checkout (tracked separately as REQ-351). No audio-spectrum render or runtime FFT behavior was exercised; no tests were run.

## Follow-up: high-resolution audio analysis ran even without a visualizer

The new 512-bin spectrum path ran for every composition that had any audio, even when no Audio Spectrum effect was present. This added an unnecessary FFT and temporary allocations to ordinary audio playback and renders. The renderer now scans for a visible, active, enabled Audio Spectrum effect and only computes the high-resolution spectrum when one needs it; otherwise it clears the frame-local spectrum data. The five broad audio-expression bands remain available as before.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No render performance capture was run; no tests were run.

## Follow-up: UI could invert an Audio Spectrum frequency interval

The frequency fields are independently editable, so a user could set the lower bound at or above the upper bound; the renderer then returned zero spectrum bars. When either bound changes, the inspector now moves the opposite bound by at least 1 Hz while keeping the interval within 20–20000 Hz.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. No interactive edit was performed; no tests were run.

## Follow-up: Roto and Puppet CLI coordinates could be partially accepted

Roto stroke parsing discarded malformed components and ignored an unmatched final coordinate; unknown stroke kinds silently became foreground. Puppet pin creation/movement similarly accepted malformed coordinates as center-position fallbacks or no-ops. Both commands now validate all coordinates as finite numbers and require complete exact x,y pairs before loading/mutating the project. Roto also validates the stroke kind and finite radius explicitly.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No Roto/Puppet command was executed; no tests were run.

## Follow-up: invalid Chroma Key colors could silently become another key color

The Chroma Key CLI defaulted unknown color names to green and substituted channel-specific defaults for invalid hexadecimal digits, potentially saving an unintended key color. It now accepts only green, blue, or a valid `#RRGGBB`, rejects non-finite gain/clip thresholds and inverted clip ranges, and performs validation before loading or modifying the project.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No Chroma Key edit was executed; no tests were run.

## Follow-up: solid-color render overrides advertised hex support but ignored it

`kagari render --override Layer.color=...` claimed to accept hexadecimal colors, but only attempted comma-separated parsing and silently ignored invalid components. Solid-color overrides now accept valid `#RRGGBB`, `#RRGGBBAA`, or exactly three/four finite comma-separated channels; malformed values abort the render before any output is started.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No render override was executed; no tests were run.

## Follow-up: render overrides could silently skip invalid numeric values and targets

Template overrides for opacity, rotation, scale, blur radius, and glow parameters ignored parse failures; unknown properties on a matched layer also appeared successful. These recognized numeric properties now require one finite scalar, visibility requires a valid boolean, and a matched-but-unsupported target fails instead of rendering with stale values. `wiggle(...)` now requires exactly two finite components and malformed wiggle syntax is rejected rather than stored as a Raw expression.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No render override/expression was executed; no tests were run.

## Follow-up: Audio Track CLI could silently mute and overflow fade frame arithmetic

A non-finite volume could collapse to zero through `max(0.0)`, and invoking the command without any volume/fade option saved an unchanged project. Fade-in frame addition could overflow `u32` before clamping to the layer end. The command now requires at least one requested operation, rejects negative/non-finite volume before project loading, and uses saturating frame addition for the fade endpoint.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No Audio Track command was executed; no tests were run.

## Follow-up: particle emission could spin after reaching its cap

The simulation loop attempted one emission for every accumulated particle, even after the alive-particle cap was reached. A large configured rate or frame delta could therefore consume unbounded CPU while emitting nothing. The scheduler now emits only the integer particles that fit under the configured cap, keeps only the fractional remainder, and clears invalid scheduling state. The particle CLI accepts a zero rate, rejects negative/non-finite rates, and enforces the same 1–10,000,000 particle range as project validation instead of silently raising small values or accepting an unbounded cap.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No particle simulation was run; no tests were run.

## Follow-up: particle collisions compared every pair

Enabling particle collisions previously ran an O(n²) scan across all alive particles, even when most were far apart. The collision pass now uses a uniform grid to find only nearby candidates, updates cell membership after positional corrections, and preserves ascending pair resolution order. Dense clusters can still require quadratic work because many particles genuinely share the same neighborhood.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No collision simulation or performance capture was run; no tests were run.

## Follow-up: JSON render logs broke on quoted or escaped names

Render and batch-queue progress events interpolated composition names and output paths directly into JSON strings. A valid path containing a quote, backslash, or line break made the machine-readable log invalid. All dynamic render and queue events now use `serde_json` serialization, which escapes those values correctly.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No CLI render was executed; no tests were run.

## Follow-up: video export could hide audio-mix failures

When a composition contained unreadable or otherwise unmixable audio, the video CLI logged a notice and continued with no audio input, producing a silent video that looked successful. Audio mixing errors now abort the export with a clear error; compositions with no readable audio sources still follow the explicit `Ok(None)` path and can export without audio.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No audio/video export was executed; no tests were run.

## Follow-up: source-layer recursion depth could leak after a panic

The renderer incremented a thread-local recursion counter before rendering a referenced layer and restored it only on normal return. If rendering unwound, a reused worker thread kept the elevated depth and could reject later, otherwise valid source-layer effects. A drop guard now restores the previous depth during both normal return and unwinding.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. Panic recovery behavior was not exercised; no tests were run.

## Follow-up: render cancellation state could leak after a panic

MFR workers and deadline-limited renders installed cancellation flags in thread-local state and cleared them only after normal rendering. A panic could leave a stale flag on a reused worker, causing unrelated later frames to appear cancelled; a deadline watchdog could also outlive an unwinding render. Cancellation flags now use a scope guard that restores the previous thread-local value, and a watchdog guard signals and joins the timer during both normal return and unwinding.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. Panic recovery was not exercised; no tests were run.

## Follow-up: expression evaluation could reuse another layer's transform context

The `toComp`/`fromComp` helpers read thread-local layer position, scale, and rotation. Composition-aware vector evaluation updated that state but did not restore it, while scalar evaluation did not set it at all. A later expression without its own layer context could therefore inherit a previous layer's transform. Vector and scalar composition-aware evaluations now install the current layer transform (or the documented neutral default) in a scope guard and restore the prior context on return or unwinding.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. Expression output parity and panic recovery were not exercised; no tests were run.

## Follow-up: invalid audio binding values could retain a previous frame

`set_audio_from_binding_sources` skipped the entire update when amplitude or any band was non-finite. Since this state is thread-local, a malformed binding could leave the previous frame's audio values active. Each input is now normalized independently: finite values are clamped to 0–1, invalid values become zero, and the context is always replaced.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. Invalid binding behavior was not exercised; no tests were run.

## Follow-up: particle cache froze after 2,000 frames and reused incomplete settings

The particle simulation cache clamped its frame key and initial simulation to frame 2,000, so long compositions stopped advancing particles after roughly 83 seconds at 24 fps. The cache also keyed only 15 selected emitter fields and layer ID, allowing collisions across compositions and stale results after untracked parameter changes. The cap is removed, the cache uses composition and layer IDs plus a fingerprint of every emitter setting, and catch-up simulation samples the emitter's world position at each simulated frame. Layer opacity is now applied to the rendered copy after cache lookup, so animated opacity updates without contaminating the cached simulation state.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. Long-duration particle output, animated emitter position/opacity, and cache invalidation were not rendered; no tests were run. Rendering a late frame without a warm cache now simulates the full preceding frame range, so random access to very long simulations can take longer.

## Follow-up: precomposition recursion state could leak after a panic

Precomp rendering pushed the composition ID onto a thread-local cycle-detection stack and incremented a thread-local nesting depth, then restored both only on normal return. A panic could make later renders on that worker falsely report cycles or exceed the nesting limit. A scope guard now pops the stack and restores the prior depth on normal return, depth-limit exit, and unwinding.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. Panic recovery was not exercised; no tests were run.

## AmeUI dependency follow-up: REQ-351 resolved and independently rechecked

AmeUI's request log now marks REQ-351 complete and records its API restoration and acceptance evidence. Rechecked the shared worktrees: AmeUI `cargo check --lib` and Kagari `cargo check --no-default-features --features ame --bin kagari-studio` both pass. The dependency blocker is cleared; no AmeUI source was changed from Kagari.

## Review finding: particle simulation does not follow layer source time

`Layer::effective_render_frame` defines the remapped/posterized source frame, and the software renderer passes that frame to layer effects. Particle simulation had instead keyed, advanced, and rendered its state using composition `frame`, so freeze and reverse remaps did not rewind/freeze particle state. It now keys and simulates through `source_frame`, samples the emitter trajectory on that source timeline, and transforms the resulting world-space particles from the source-time layer transform into the current composition-time transform. This preserves the split between remapped source content and current layer transforms. Static and identity-remapped layers use the same source/current transform and therefore retain their prior coordinates.

Validation at implementation time: `cargo fmt --check`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-check-20261111 cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. A subsequent manual renderer probe is recorded below.

Manual renderer probe after implementation: a frozen source frame matched its unremapped source image and remained pixel-identical at two later composition frames; reverse-remapped samples matched their corresponding earlier source-frame renders; animated current position/scale/rotation changed output while source time stayed frozen. A zero-opacity emitter matched an empty composition render. Probe source lived under `/tmp` and was not added to the repository. Non-linear remap interpolation and real-project visual review remain unverified.

## Follow-up: particle cache could collide across separate projects with repeated IDs

Composition and layer IDs are only project-local. Rendering two separate compositions with the same IDs, frame rate, source frame, and emitter settings on a reused worker could return the first composition's particle history, even when their animated emitter trajectories differ. The cache now includes the address of the original composition instance (captured before collapsed-precomp flattening) alongside the global edit version, FPS, source frame, and full emitter fingerprint. This isolates independent projects without serializing transform tracks on every playback frame.

Validation: `cargo fmt --check`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-check-20261111 cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Cross-project cache behavior was not rendered; no tests were run.

## Follow-up: particle cache could reuse simulation across frame rates

The cache key distinguished composition and layer IDs, frame, emitter settings, and global invalidation version, but omitted composition FPS. Two same-ID compositions rendered without a cache-version bump could therefore reuse particle state produced with a different integration step. FPS is now part of the key and cache entry, so playback cadence and particle age cannot cross-contaminate.

Validation: `cargo fmt --check`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-check-20261111 cargo check --no-default-features --features cli --bin kagari` passed. No render or tests were run.

## Follow-up: particle opacity settings were ignored

`ParticleEmitter` stored and validated start/end opacity, but rendering only interpolated the alpha channel in the start/end color fields. The opacity controls now multiply the color alpha and fade curve, with non-finite values treated as transparent; both AmeUI and the legacy Inspector expose editable start/end opacity values. The AmeUI Studio build could not validate the new Ame bindings because the existing AmeUI dependency failure remains at `src/app/config.rs:143` and `src/app/traits.rs:375`, producing missing public exports and 83 errors. The egui Studio build passed, covering the shared renderer and legacy controls.

Validation: `cargo fmt --check`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-check-20261111 cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. The AmeUI Studio check was attempted and is blocked by the dependency errors above. No tests or particle renders were run.

## Follow-up: macOS packaging shared destructive intermediate paths

Concurrent or interrupted packaging runs shared `target/bundle/Kagari Studio.app` and `target/kagari_logo.png`; each run deleted or overwrote the other's intermediate output before the final DMG was ready. App and icon staging now lives in per-run temporary directories, and the generated DMG is checked with `hdiutil verify` before signing/notarization and publication. The script also rejects an output path resolving to the input release binary and rejects directory output paths, preventing an accidental overwrite or misleading destination. Shell cleanup is installed before temporary directories are created so partial setup is cleaned up.

Validation: `bash -n scripts/package_macos.sh bundle_dmg.sh` and `git diff --check` passed. End-to-end packaging could not run because `target/release/kagari-studio` is absent; signing and notarization were not exercised.

## Follow-up: particle layers disappeared inside precompositions

The dedicated precomp renderer returned early for layer types outside its supported raster list, which silently omitted Particle layers. Particle layers now render through the main renderer with the original composition context and a transparent background, then composite into the precomp buffer as a full-frame layer. This preserves transform, opacity, remap, and effect handling from the main path without applying the precomp layer transform twice.

Validation: `cargo fmt --check`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-check-20261111 cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Follow-up review found that the original particle draw path also bypassed the shared compositor and drew effects into the accumulated frame. Particle rendering now uses an isolated layer buffer, applies effects and layer masks there, then goes through the shared track-matte, layer-opacity, blend-mode compositor. Additive emitter pixels are converted back to straight alpha before compositing. The same-size direct-vs-precomp manual probe now reports pixel-identical output (0 differing channels; max error 0). The probe lived under `/tmp`; no tests were run.

## Follow-up: CLI OpenEXR export could exceed the float working-memory budget

The CLI checked the RGBA8 renderer's 512 MiB buffer limit, but EXR conversion simultaneously holds float RGBA data and then deinterleaves it into four full-size channels. At the renderer's maximum dimensions this could request multiple gigabytes and terminate the process under memory pressure. Float conversion now checks its own peak working set before rendering and uses fallible reservation; the EXR conversion limit is shared by the CLI and Studio preflight so oversized exports fail before frame rendering with a dimension-reduction message.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features 'gui cli' --bin kagari-studio-egui --bin kagari` passed. No tests were run.

## Follow-up: CLI `--skip-existing` trusted incomplete image files

The PNG and EXR render loops skipped any regular file at the expected frame path. A previous interrupted export or truncated file could therefore be reported as a completed frame on every retry. Skip decisions now require the expected image format, matching dimensions, and successful full decode/EXR validation; invalid frames are regenerated through the existing atomic writers.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No tests or renders were run.

## Follow-up: normal audio import duplicated large PCM buffers

`load_cached_audio` decoded a source and always called `resample`; when source and target rates already matched, `resample` cloned the entire PCM array. A 512 MiB decoded source therefore briefly required another 512 MiB copy, and both same-rate and converted paths used infallible capacity allocation. Matching-rate loads now move the decoded buffer directly into the cache, while explicit resampling uses checked sample counts and fallible reservations. WAV mixing also rejects zero or unsupported sample rates before writing an invalid or impractically large PCM stream.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No tests or audio renders were run.

## Follow-up: audio sample playback ignored layer time remapping

Audio and extracted Video audio were sampled from `(frame - in_frame) / fps` and advanced at normal speed even when the layer was frozen, reversed, or retimed. Sample timing now derives a source-time offset and playback rate from adjacent time-remap values. PreComp traversal carries the source-frame rate through nested compositions and combines it with each child layer's remap rate; Video speed multiplies the resulting sample rate. This keeps the audio path aligned with source-time operations instead of silently playing at normal speed.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No tests or audio renders were run; source-rate behavior remains unverified against an actual remapped project.

## Follow-up: persistent dynamic-link frames had no content-integrity check

Disk cache reload trusted file length and metadata identity. A damaged cache file with the same byte count could therefore be accepted as a valid rendered frame after restart. Cache metadata now stores an FNV-1a content checksum; disk reload verifies it before inserting bytes into memory. Older metadata without a checksum is discarded and regenerated. Reload allocation now also uses checked platform-size conversion and fallible reservation.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No tests were run.

## Follow-up: retimed audio used nearest-sample reads

Audio mixing converted every source time to an integer sample index, truncating fractional positions. This caused stepped playback during time stretching and remapping, including reverse playback and nested PreComps. A shared linear-interpolation reader now samples fractional positions for Audio, extracted Video audio, and nested PreComp sources; out-of-range and invalid positions return silence.

Validation: `cargo fmt`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No tests or audio renders were run.

## Follow-up: production-document save did not sync its directory entry

`ProductionDocument::save_atomic` synced the staged file contents before rename but did not sync the containing directory afterward. A sudden system interruption could therefore lose the replacement directory entry even after the save call succeeded. It now performs the same best-effort parent-directory sync used by the main project save path after a successful rename.

Validation: `cargo fmt --check` and `git diff --check` passed. No tests or forced-interruption save simulation were run.

## Follow-up: video export could overwrite its own audio input

When the configured WAV audio input and video destination resolved to the same path, overwrite-enabled export could finish encoding and replace the source WAV with the video. Export validation now resolves existing paths (including symlinked paths and relative parent directories) and rejects this destructive configuration before FFmpeg starts.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No FFmpeg export was run.

## Follow-up: long audio timelines lost fractional-sample precision

The shared interpolated reader still computed sample positions in `f32`. At 48 kHz, `f32` can no longer represent every adjacent sample after roughly 5.8 minutes, so long recordings would develop progressively coarser read positions. Audio time offsets, playback scales, and sample positions now use `f64`; stored samples and interpolation output remain `f32`.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No long-duration audio render was run.

## Follow-up: video audio-probe failures were silently treated as no audio

The `ffprobe` audio-stream check returned `false` both when a source had no audio and when probing failed. Import then succeeded without a track or warning. It now distinguishes probe errors and stops the import with a clear message. Probe subprocesses also discard stderr, which the import path never surfaced, instead of buffering unbounded diagnostics in memory.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No media import was run.

## Follow-up: image-sequence import could over-allocate and exhaust disk

Image-sequence discovery first materialized every directory entry, then filtered matching frames and only afterward enforced the frame limit. A large media folder could consume excessive memory before rejection. Discovery now streams entries, retains only matching numbered frames, enforces the cap as it goes, and uses fallible reservation. The import also preflights estimated output size and checks free space during encoding, cleaning its staging directory if the safety reserve is reached.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No high-volume image-sequence import was run.

## Follow-up: EQ filter history reset at every video-frame boundary

The EQ helper constructed fresh biquad filters for each audio block, so per-layer and master EQ lost its delay history at every composition frame. Continuous exports could therefore contain repeated filter transients. EQ now supports persistent filter state across sequential blocks; audio mixing keeps distinct state for each layer/effect and PreComp instance, updates coefficients while preserving delay history, and resets DSP state when playback jumps to a non-adjacent frame or a layer effect is bypassed.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No tone render or frame-boundary listening check was run.

## Follow-up: extracted video audio used a different source-time origin than video frames

Video rasterization indexes extracted frames from `remap_frame(frame) * speed`, while audio subtracted the layer's timeline `in_frame` before applying speed and also converted invalid/nonpositive speeds to normal speed. This caused audio/video source drift for offset, remapped, zero-speed, or invalid-speed clips. Extracted video audio now uses the same absolute remapped source-frame origin and nonnegative speed semantics as the renderer, including through nested PreComps; ordinary Audio layers retain their timeline-relative offset behavior.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No synchronized video/audio render was run.

## Follow-up: PreComp cache revision hashed the wrong video frames

The PreComp content revision hashed the time-remapped frame before applying Video speed and omitted the adjacent frame used by frame blending. If those actual source frames changed while the hashed frame did not, the cached PreComp pixels could be reused incorrectly. Revision calculation now mirrors the renderer's source-position, clamping, proxy, and frame-blending dependencies.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No cache invalidation render was run.

## Follow-up: repeated PreComp instances hid media dependencies

Media revision traversal used one global visited-composition set. When a parent composition referenced the same nested PreComp more than once at different remapped frames, only the first instance contributed its source-frame dependencies to the cache revision. The set now tracks only the active recursion path, so cycles still terminate while separate instances and frame positions are all hashed.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No multi-instance cache invalidation render was run.

## Follow-up: disk frame cache accepted same-length corruption

Frame-cache reload checked dimensions and byte length but could return corrupted pixels when the file retained its expected size. The cache format now stores an FNV-1a checksum covering dimensions and pixel bytes; reload verifies it before returning a frame. Older cache files without a checksum become misses and are regenerated. The checksum bytes are included in budget accounting.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No corruption/rebuild run was performed.

## Follow-up: cache read cleanup could delete a concurrent replacement

On a checksum or format mismatch, the cache reader removed the path it had opened. Another process could atomically replace that path with a valid frame between the read and removal, causing the cleanup to delete the new frame. Invalid entries are now treated as misses without unlinking from the read path; the normal atomic writer replaces them safely.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No concurrent multi-process cache stress run was performed.

## Follow-up: audio DSP state was not scoped to a composition or sample rate

Sequential-frame detection reset DSP state on seeks, but reusing a `MasterDspState` for another composition or output sample rate could carry EQ/filter and compressor history across unrelated audio. State now records its composition and sample rate and resets when either changes, as well as on non-adjacent frames.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No composition-switch audio render was run.

## Follow-up: particle scrubbing repeatedly simulated from frame zero

The particle cache kept only the latest state, so seeking backward or jumping more than 30 frames forced a full replay from frame zero. It now retains periodic deterministic simulation checkpoints and resumes from the nearest cached state before the requested source frame. Checkpoint snapshots have an 8 MiB per-render-thread budget and evict oldest snapshots when full; cache identity and source-time behavior remain unchanged.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. Long-timeline scrubbing performance and checkpoint memory under maximum particle counts were not measured.

## Follow-up: particle collision candidate search allocated repeatedly

The uniform-grid collision broad phase allocated a fresh candidate vector for every particle and every candidate-search iteration. With collisions enabled, this created repeated heap allocation during playback. Candidate selection now scans the neighboring cells and tracks the lowest eligible index directly, preserving deterministic pair order without those temporary vectors. The grid also no longer reserves one hash bucket per configured particle before knowing how many cells are occupied.

Validation: `cargo fmt --check`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-check-20261111 cargo check --no-default-features --features cli --bin kagari` passed. No collision simulation or performance benchmark was run.

## Follow-up: WAV input allocation failure could panic

The WAV loader bounds source files to 512 MiB and now reads fixed-size chunks, checking the limit and reserving each growth fallibly. This also handles a file that grows after its initial metadata check, without allowing `read_to_end` to request unchecked vector growth.

Validation: `cargo fmt --check`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-check-20261111 cargo check --no-default-features --features cli --bin kagari` passed. No large-file mutation or allocation-failure injection was run.

## Follow-up: Windows cross-build used unstable file identity APIs

A Windows cross-build exposed unstable `MetadataExt::volume_serial_number` and `file_index` calls in PNG rollback identity checks. These now use `GetFileInformationByHandle` via `windows-sys`, which is available on stable Rust. The first review also hypothesized that `std::fs::rename` could not replace existing files on Windows; that was incorrect. Rust documents `rename` as replacing an existing destination and as using `MoveFileExW` on Windows, so the unnecessary custom replacement helper was removed. [Rust `std::fs::rename` documentation](https://doc.rust-lang.org/stable/std/fs/fn.rename.html)

Validation: native CLI `cargo check` and `CARGO_TARGET_DIR=/tmp/kagari-codex-win-check cargo check --target x86_64-pc-windows-msvc --no-default-features --features cli --bin kagari` passed before adding the Windows identity API; the Windows Studio cross-check below passed after that change. Windows runtime rollback behavior was not executed.

## Follow-up: CI did not compile the Windows target

The repository CI compiled and tested on Linux and packaged on macOS, but had no Windows build gate. That gap allowed a Windows-only unstable API call to break compilation unnoticed. A `windows-latest` job now checks the Studio binary with the stable MSVC target on every push and pull request.

Validation: the updated workflow parses as YAML, `git diff --check` passed, and `CARGO_TARGET_DIR=/tmp/kagari-codex-win-studio-check cargo check --target x86_64-pc-windows-msvc --no-default-features --features ame --bin kagari-studio` passed locally. GitHub-hosted execution has not yet run.

## Follow-up: Kagari Hosted CI cannot resolve its AmeUI path dependency

Current workflows checkout only Kagari, but Cargo declares optional `ame-ui` at `../AmeUI`. Hosted runners do not have that sibling checkout, and Cargo must still resolve optional path dependencies before selecting features. The configured AmeUI remote is the private LAN host; anonymous access to the guessed GitHub URL returns `Repository not found`. Thus the existing Linux/macOS CI and the new Windows Studio gate cannot run from a clean hosted checkout until AmeUI has a CI-readable, version-pinned distribution path. Registered **REQ-352** in AmeUI's `リクエスト.md` with reproduction, ownership, and acceptance criteria. The Windows gate checks the Studio product build once dependency distribution is available.

Validation: `CARGO_TARGET_DIR=/tmp/kagari-codex-win-studio-check cargo check --target x86_64-pc-windows-msvc --no-default-features --features ame --bin kagari-studio` passed locally, and the workflow YAML parses. GitHub Actions cannot be verified until REQ-352 is resolved; current local sibling checkout masks the missing-dependency condition.

## Follow-up: multichannel audio was silently truncated to stereo

The audio decoders accepted multichannel input, but timeline sampling consumed only the first two channels. WAV documentation already described mono/stereo support, yet its parser accepted up to 32 channels. Both decoders now downmix layouts with known speaker positions to stereo using center, surround, height, and LFE coefficients. WAVE_FORMAT_EXTENSIBLE PCM parses and validates its subtype, valid-bit count, and speaker mask; legacy multichannel PCM without a speaker mask and layouts containing unknown positions return an explicit error rather than silently losing channels. PCM 32-bit containers are supported alongside 8/16/24-bit samples.

Validation: `cargo fmt --check`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-check-20261111 cargo check --no-default-features --features cli --bin kagari` passed. No multichannel sample file was decoded; downmix coefficients and WAVE_FORMAT_EXTENSIBLE parsing need per-channel impulse fixtures before release.

## Follow-up: local DMG wrapper assumed Cargo's default target directory

`bundle_dmg.sh` was committed without execute permission, so the documented `./bundle_dmg.sh` entry point failed immediately. After correcting the mode, an actual run exposed a second issue: Cargo honored the configured `/tmp/cargo-mac-ephemeral-target`, while the wrapper still looked in the repository's `target/release`. The wrapper is now executable and gets the target directory from `cargo metadata` before invoking the shared packager.

Validation: `./bundle_dmg.sh` completed a macOS release build and generated the 17 MiB DMG; `hdiutil verify` reported a valid checksum. The image mounted read-only, and its app executable, `org.kagari.studio` identifier, `0.3.0` version, and Applications symlink were verified. This artifact is unsigned and unnotarized, as expected without release credentials. macOS emitted deprecation warnings for the current `hdiutil` create/attach/detach commands; the commands still succeeded.

## Follow-up: replace deprecated macOS image creation command

The packaging script still used deprecated `hdiutil create`. The current `diskutil image create from` command supports the same UDZO format and volume name from a staging folder. The script now uses it for creation while retaining `hdiutil verify` for checksum validation; attachment checks use `diskutil image attach` and eject.

Validation: the current wrapper rebuilt and packaged the macOS release app with `./bundle_dmg.sh`; the updated DMG passed `hdiutil verify` and read-only attachment checks for its executable, Info.plist, identifier, version, and Applications symlink. The create/attach/eject deprecation warnings are gone. `bash -n`, `cargo fmt --check`, and `git diff --check` passed. The artifact remains unsigned and unnotarized.

## Follow-up: PreComp audio bypassed layer audio effects

Nested Audio and Video sources were summed directly into the parent stereo mix, so their Bass/Treble effects never ran. PreComp layers also lacked an isolated mix stage for applying effects on the PreComp itself. Each nested source now passes through its own layer effects before gain/pan mixing, and nested/top-level PreComps collect child audio into an isolated buffer, apply their own effects, then apply their gain and pan to the parent mix.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No tests or audio renders were run.

## Follow-up: Bell and shelf EQ accepted frequencies outside the audio band

The EQ parameter filter enforced the Nyquist limit for high-pass and low-pass filters, but bell and shelf bands were accepted at negative frequencies and at or above Nyquist. Biquad design then received an out-of-band frequency, producing a wrapped or unintended response. Bell and shelf filters now require a frequency above 1 Hz and below Nyquist, matching the safe pass-band constraints of the other filter types.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No frequency-sweep render or listening check was performed.

## Follow-up: right-panel input cloned the full project during unrelated drags

The effects panel cloned the entire current project whenever any pointer button was pressed anywhere in the application. Because the panel is drawn every frame, holding a drag in the timeline or viewport repeatedly cloned even large projects, adding avoidable UI latency and memory churn. Snapshot capture now runs only for the Effects, Properties, and fallback effects panels, and only for pointer or keyboard edits in those panels. A held pointer drag must have started inside the panel before it can trigger a snapshot, so a viewport/timeline drag crossing over the panel does not resume the cloning.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No interactive large-project latency profile was captured.

## Follow-up: 3D mesh cache ignored file size and missing timestamps

The OBJ/GLB mesh cache keyed freshness only by modification time. Replacing a model with different-length content while preserving its timestamp could reuse stale geometry; filesystems that could not provide a modification time also allowed `None == None` to count as a cache hit. The cache now compares both file length and modification time, and only treats a complete fingerprint as a hit.

Validation: the GUI library build and `cargo fmt --check` / `git diff --check` passed. No same-timestamp model replacement render was performed; same-length replacements with deliberately preserved timestamps still require explicit cache invalidation or content hashing to detect.

## Follow-up: video proxies were not tied to their source movie

Project opening validated extracted-frame integrity but did not compare a complete proxy cache with the current source movie. Replacing a movie at the same path could therefore keep showing old frames and extracted audio indefinitely. Video cache manifests now record the source's canonical path, byte size, and modification time, and project-load refresh re-decodes when an available source no longer matches. A complete proxy remains usable when the source is offline. Legacy manifests without a source fingerprint are refreshed when the source is available.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features gui --lib` passed. No source-replacement import was run; same-size replacements that preserve modification time remain undetected by this metadata fingerprint.

## Follow-up: confirmed still-PNG overwrite encoded directly to the destination

After the user confirmed overwriting a still PNG, Studio encoded directly into the existing file. An encoder or disk error could leave the old image truncated or partially replaced. Still exports now encode and sync a unique temporary file beside the destination, then atomically rename it over the old file only after encoding succeeds. The non-overwrite path continues to publish via a no-clobber hard link.

Validation: `cargo fmt --check`, `git diff --check`, and the AmeUI Studio compile check passed. No injected disk-full or encoder-failure run was performed.

## Follow-up: still PNG rendering blocked the Studio UI thread

The still-frame export rendered the selected frame and encoded the PNG synchronously in the app event handler. Complex frames could freeze transport, panel interaction, and window repaint until rendering completed. Rendering and writing now run on a named worker thread; the UI polls a completion channel, reports success/failure, and prevents another export from starting while it runs. The worker catches renderer panics and preserves the previously added atomic output behavior.

Validation: `cargo fmt --check`, `git diff --check`, and the AmeUI Studio compile check passed. No large-composition UI responsiveness measurement was run.

## Follow-up: video source metadata errors were mistaken for offline media

The source-fingerprint check treated every metadata failure like a missing source and trusted the proxy cache. Permission or filesystem errors could therefore suppress a required refresh and silently keep stale footage. Only `NotFound` now permits a valid cache to serve as an offline proxy; other metadata failures mark the cache as needing refresh, allowing the import path to report the underlying error.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features gui --lib` passed. No permission-denied filesystem scenario was run.

## Follow-up: footage relinking could choose the wrong same-named file

Project relinking previously treated relative paths as valid when they happened to resolve from the process working directory, even if a different matching file existed beside the project. Its fallback scan also selected the first same-named file from candidate folders. Relative paths now resolve against the project directory first; fallback filename matches are accepted only when exactly one unique file exists. Ambiguous matches are logged and left unresolved for explicit relinking.

Validation: `cargo fmt --check`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. No multi-match project fixture was exercised.

## Follow-up: PNG sequence rollback needed a Windows compile gate

The staged PNG sequence publisher uses file identity checks before removing newly published files during rollback. Those checks have separate Unix and Windows implementations; validating only on the macOS development host would leave the Windows Studio/CLI branch unchecked. The Windows identity implementation was reviewed against the enabled `windows-sys` filesystem API and the CLI was cross-compiled for `x86_64-pc-windows-msvc`.

Validation: `CARGO_TARGET_DIR=/tmp/kagari-codex-windows-check-20261112 cargo check --target x86_64-pc-windows-msvc --no-default-features --features cli --bin kagari` passed. This confirms compilation only; Windows runtime replacement/rollback behavior still needs a native filesystem exercise.

## Follow-up: PNG sequence publication now supports filesystems without hard links

The no-clobber publisher previously required hard links from a staging directory into the selected output directory, which rejects common removable and network filesystems such as exFAT shares. Publication now attempts the atomic hard-link path first and falls back to opening the destination with `create_new`, copying and syncing the staged file. Rollback tracks file identity from the open handle so it removes only the output created by this export; restoration copies a backup with the same no-replace behavior and verifies the restored path still names that file before deleting the recovery copy. This also avoids following a replacement symlink and discarding the only backup.

Validation: `cargo fmt --check`, `git diff --check`, and CLI `cargo check` passed for both the host and `x86_64-pc-windows-msvc`. Runtime behavior on exFAT/network shares and injected mid-copy failures was not exercised.

## Follow-up: canceling export released the UI lock before cleanup finished

The export dialog treated the cancel click as worker completion: it dropped the receiver and cleared `is_exporting` immediately, although a worker could still be rendering a frame or rolling back published sequence files. A second export could then race the first against the same destination. Cancel now requests cancellation but keeps the export active and drains its completion event; only after the worker reports cancellation and finishes cleanup can another export start. The final UI state distinguishes a completed cancellation from a render failure.

Validation: `cargo fmt --check`, `git diff --check`, and GUI/AmeUI Studio compile checks passed. No interactive cancellation during a large frame or sequence rollback was performed.

## Follow-up: canceled queue items were labeled as failures

After the worker-aware cancellation fix, the export dialog displayed a canceled result but still marked the corresponding render-queue item as `Failed`. Queue state now has a distinct `Cancelled` value, the composition queue summary counts canceled items separately, and both queue views display the canceled status. Canceled batch renders still stop the remaining batch.

Validation: `cargo fmt --check`, `git diff --check`, GUI/AmeUI Studio compile checks, and the Windows `kagari-studio` cross-compile passed. No interactive queue cancellation was performed.

## Follow-up: viewport flagged usable offline video proxies as missing media

The viewport warning checked only whether the original video source path existed, despite projects supporting offline playback from extracted frames. It also used `exists()` for video sources, which accepted directories as valid files. The warning now accepts the actual extracted frame needed by the current fractional remap/speed-adjusted frame (including posterize-time sampling) and requires real files for image, model, and video sources.

Validation: `cargo fmt --check`, `git diff --check`, GUI/AmeUI Studio compile checks, and the Windows `kagari-studio` cross-compile passed. No offline-media project was opened in the viewport.

## Follow-up: image-sequence imports were treated as videos on project reopen

Image sequences use the shared `Video` layer type and extracted-frame cache, but their cache manifest had no source-video fingerprint. Project-open refresh therefore attempted to decode the first numbered still as a movie and emitted a cache-refresh error on each open. New cache manifests identify image sequences explicitly; legacy caches are recognized from a numbered still-image source path. Complete sequence caches now remain intact, and a missing/invalid sequence cache is rebuilt with the sequence importer rather than the video decoder.

Validation: `cargo fmt --check`, `git diff --check`, and GUI/AmeUI Studio compile checks passed. No image-sequence project was opened and saved through the app.

## Follow-up: collected projects dropped image-sequence frames

Media collection copied each video layer's source file and intentionally marked normal video caches for regeneration. For image sequences the source is only the first numbered still, so regeneration from that one file can collapse the sequence to one frame. Collection now copies the complete identified sequence cache (including its manifest and numbered frames) into the bundle and rewrites the layer's cache path to the collected copy; ordinary video caches remain regenerated from their source movie.

Validation: `cargo fmt --check`, `git diff --check`, and GUI/AmeUI Studio compile checks passed. No collected image-sequence bundle was opened to verify its frame count.

## Follow-up: project-relative video cache paths were not resolved on open

Collected image-sequence caches use a relative `frames_dir` inside the bundle. Project open previously resolved footage source paths but left video cache and extracted-audio paths relative to the process working directory, so a valid collected cache could still appear missing and be rebuilt incorrectly. Video cache directories and optional audio paths are now anchored to the project directory when opening a project.

Validation: `cargo fmt --check`, `git diff --check`, and GUI/AmeUI Studio compile checks passed. No bundle was opened from a different current working directory.

## Follow-up: Render All could replace an active batch

The render-queue panel allowed `Render All Queue` while an export was active. The handler replaced `batch_queue` before the export runner's `is_exporting` guard returned, so the current composition could finish against a different batch and advance into items the user had not intended to add to the running job. The control is now visibly disabled while an export is active.

Validation: `cargo fmt --check`, `git diff --check`, and GUI/AmeUI Studio compile checks passed. No live queue interaction was performed.

## Follow-up: Cycle time-remap interpolated backwards at the wrap point

The baked Cycle loop placed the last source frame one timeline frame before the next cycle's first key and marked both keys Linear. Integer-frame playback hid the defect, but fractional-time evaluation interpolated from the source end back to zero during that boundary interval. Cycle endpoint keys now use Hold so the final source frame remains stable until the next cycle begins; PingPong keeps Linear interpolation because its turnarounds are continuous.

Validation: formatting, whitespace/diff checks, and the GUI library compile check passed. No fractional-frame render of a baked Cycle was performed.

## Follow-up: Time-remap panel used three incompatible source-frame conventions

The panel hand-built Enable and Reverse curves with a zero-based source range while the shared `Layer` API and normal remap path use the composition-frame identity range. Freeze subtracted the layer's in-point from the playhead even though `freeze_at` accepts a source frame, so a layer beginning after frame zero froze to the wrong source frame. Enable, Freeze, and Reverse now delegate to the shared layer operations, keeping UI and CLI/core semantics aligned.

Validation: formatting, whitespace/diff checks, and GUI library compile check passed. No interactive nonzero-in-point layer workflow was run.

## Follow-up: sliding a clip detached its animation from the clip

Normal timeline body-drag changed only `in_frame` / `out_frame`; transform, mask, effect, text-selector, shape, marker, paint, and roto keyframes stayed at their old composition times. Video and PreComp layers also used implicit composition-time source mapping, so moving them changed the viewed source frame. A slide now shifts the layer's animated tracks by the actual snapped delta and materializes the prior source-time mapping for Video / PreComp layers before the shift. Alt-drag slip behavior remains separate.

Validation: formatting, whitespace/diff checks, and GUI Studio compile check passed. Interactive drag with animated footage, masks, and effects was not performed.

## Follow-up: dragging a clip longer than its composition could underflow

The slide clamp computed `total_frames - clip_span` with unsigned arithmetic. Imported footage can be longer than the composition, making a normal drag panic in debug builds or wrap the allowed range in optimized builds. The maximum start frame now uses saturating subtraction, and drag position arithmetic uses a wider signed type before clamping.

Validation: formatting, whitespace/diff checks, and GUI Studio compile check passed. A long-footage timeline drag was not exercised interactively.

## Follow-up: Alt-slip did not change video or PreComp source content

Alt-drag previously shifted animated property keyframe times but left an unremapped Video / PreComp source on the same frames, so slipping a plain footage layer had no visible effect. Those layer types now materialize their current source-time mapping and offset remap values by the dragged source-frame amount while keeping the clip bounds and layer-space animation in place. Video slips are clamped to the decoded frame range and account for playback speed; non-media Alt-slip keeps its existing keyframe-timing behavior.

Validation: formatting, whitespace/diff checks, and GUI Studio compile check passed. Interactive slip of normal-speed, retimed, and end-of-source footage was not performed.

PreComp slip initially had no upper source bound because the clip handler lacked access to the nested composition duration. The timeline now builds an ID-to-duration map from the project composition tree and passes the selected PreComp's source length into the slip operation; missing/zero-length sources are left unchanged rather than shifted into an empty render range.

Validation: formatting, whitespace/diff checks, and GUI Studio compile check passed. Interactive PreComp slip at both source boundaries was not performed.

## Follow-up: production AmeUI timeline move bypassed the clip-timing fix

The default `kagari-studio` binary uses the AmeUI-hosted timeline, whose `timeline_ops` updated only clip in/out frames; the earlier egui fix therefore did not cover the production app. Layer range movement and animation-frame shifting are now shared by the core `Layer` API, and AmeUI Move drags call it. This shifts animated properties by the snapped movement and preserves Video / PreComp source time; trimming remains a separate operation and does not shift keyframes.

Validation: formatting, whitespace/diff checks, and AmeUI Studio / egui Studio compile checks passed. Interactive movement of an animated nested composition was not performed.

## Follow-up: moving a camera/light layer left scene-property keys behind

Camera lens/DoF tracks and light intensity are stored on composition scene objects rather than on their timeline layers. Moving a linked layer therefore shifted its transform but left those property keys at their original composition frames. A move now shifts the associated camera lens/DoF or light-intensity tracks when exactly one layer references that scene object. Shared scene objects are left untouched because moving one of their references has ambiguous ownership.

Validation: formatting, `git diff --check`, and the AmeUI Studio compile check passed. Interactive camera/lens and light/intensity movement was not performed.

## Follow-up: Save As could discard production-document metadata

The Studio loads a `ProductionDocument` but keeps only its VFX `Project` in editor state. Saving to a new path then had no target document to copy, so audio settings, tempo, and automation bindings were replaced by defaults. Added a source-aware atomic save path and routed Save As plus external autosave through it; metadata is carried from the currently open source document, with the target's existing document and backup as fallbacks. Ordinary callers retain the prior save API.

Validation: `cargo fmt --all`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-ame-studio-check-20261112 cargo check --no-default-features --features ame --bin kagari-studio` passed. Save As with non-default audio/tempo/binding data was not exercised interactively; no tests were run.

## Follow-up: expired particles could still collide on their death frame

Particle lifetime reached zero during the update loop, but pairwise collision resolution ran before dead particles were removed. An expired particle could therefore push or accelerate a live particle on the frame it disappeared. Pairwise resolution now builds its inputs from live particles only and writes results back to those original indices; expired positions remain available to the death-spawn pass.

Validation: `cargo fmt --all`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-particle-check-20261112 cargo check --no-default-features --features cli --lib` passed. No particle render or tests were run.

## Follow-up: layer out-points rendered one extra frame

Timeline sizing, layer duration, export frame counts, and project validation treat `out_frame` as an exclusive boundary, but `Layer::is_active` included it. Layers ending before the composition end could therefore render and play audio for one frame beyond their displayed/exported duration. Active-range evaluation now uses `[in_frame, out_frame)` so renderers, audio mixing, and timeline duration agree.

Validation: `cargo fmt --all`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-layer-range-check-20261112 cargo check --no-default-features --features cli --lib` passed. No full render/playback boundary check or tests were run.

## Follow-up: Time Reverse started one frame past the source clip

After aligning layer activity to an exclusive out-point, Time Reverse still mapped the first active frame to `out_frame`, which is outside the layer's source span. The reverse map now starts at `out_frame - 1` and maps active output frames through the source span in reverse order; zero-based clips end at source frame 0 without producing a negative source coordinate. Updated the existing endpoint expectation to the exclusive-span behavior.

Validation: `cargo fmt --all`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-timeremap-check-20261112 cargo check --no-default-features --features ame --bin kagari-studio` passed. The changed expectation and reverse audio/video output were not run or rendered.

## Follow-up: primary Studio Clippy gate had 27 errors

Running the primary AmeUI Studio Clippy gate with warnings denied exposed 13 library and 14 Studio errors. Simplified checked-size propagation and RGBA chunk iteration, made divisibility and collision-loop idioms explicit, and ensured completed FFmpeg children are waited on after ownership leaves their guards. Cleaned Studio export filename borrows, PNG/EXR sequence loop counters, project initialization, and duplicate playback-start branches without changing their intended behavior.

Validation: `cargo fmt --all`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-clippy-ame-20261112 cargo clippy --no-default-features --features ame --bin kagari-studio -- -D warnings` passed. This checks the primary Studio binary and its library dependencies, not all feature/target combinations or tests.

## Follow-up: full CI Clippy gate covered stale egui, CLI, examples, and event fixtures

The CI-equivalent `--all-features --all-targets` run found additional warnings outside the primary AmeUI Studio: legacy egui helpers, CLI functions and pixel loops, example particle initialization, and test fixtures missing AmeUI's new `CanvasPointerEvent::click_count`. Simplified iterator and initializer patterns, kept argument-count allowances local to existing multi-context helpers, made shortcut lock recovery poison-tolerant, and updated event fixtures to declare a single click. The CLI/story/preview behavior was not changed.

Validation: `cargo fmt --all`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-clippy-all-20261112 cargo clippy --all-features --all-targets --message-format short -- -D warnings` passed. Clippy compiled all targets, including test targets, but no tests were executed.

## Follow-up: Time Reverse's last active frame overshot the source range

The zero-based reverse map placed its final key at `out_frame - 1` with a value of `-1`. That made the last active frame extrapolate below source frame 0, so audio sampling returned silence there. The final key now lands on `in_frame`; zero-based and nonzero-based active spans both finish on their first source frame. Added regression coverage for the zero-based endpoints.

Validation: `cargo fmt --all` and `git diff --check` passed. No tests or audio/video renders were run.

## Follow-up: FFmpeg temporary output names could collide with stale files

Video and GIF exporters passed predictable process-and-sequence paths to FFmpeg, then used cleanup guards that removed those paths on failure. If a stale file remained after PID reuse, a new export could overwrite it or cleanup could delete a file this export never created. Exporters now reserve the temporary output with exclusive creation before FFmpeg starts; GIF also reserves its palette sidecar. A collision advances to another output name or reports an error, and cleanup guards are only installed for paths this export reserved.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. No FFmpeg export or collision scenario was run.

## Follow-up: DMG publication could report success without writing to the requested path

The packaging script checked that the destination was not a directory, then used `mv -f`. If a directory appeared at that path between the check and publication, `mv` could place the DMG inside it and the script would still print the requested path as complete. Publication now uses Python's same-filesystem `os.replace`, which atomically replaces a file or fails when the destination has become a directory; the script reports success only after the requested path is replaced.

Validation: `bash -n scripts/package_macos.sh` and `git diff --check` passed. macOS DMG packaging and the concurrent destination-directory case were not rerun.

## Follow-up: beat detection averaged signed samples and missed ordinary transients

The 10 ms beat-detection envelope summed signed channel samples and took the absolute value of their average. A centered waveform or phase-inverted stereo transient can cancel toward zero, so the detector could miss audible attacks. The envelope now uses RMS energy across decoded samples, accumulates squares in `f64` to avoid overflow from finite float audio, and treats non-finite samples as silence before onset-flux detection.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Beat detection was not evaluated against reference audio; no tests were added or run.

## Follow-up: collecting OBJ/MTL files duplicated whole model files in memory

Media collection read each OBJ/MTL companion into a full `String`, then allocated a second full-size rewritten string. Large geometry files could therefore multiply their memory footprint during collection. Reference scanning and path rewriting now stream one line at a time through buffered I/O, with a 1 MiB maximum line length so malformed single-line files fail with a clear error. The rewrite also calculates indentation from comment-free content; inline comments previously contaminated the indentation slice and could corrupt rewritten `mtllib` directives.

Validation: `cargo fmt --all --check`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Collection of large OBJ/MTL files and inline-comment fixtures was not run; lines longer than 1 MiB are explicitly rejected.

## Follow-up: project validation accepted PreComp graphs deeper than the renderer

Production-document validation rejected cyclic PreComp graphs but accepted acyclic chains up to 1024 nested compositions. The renderer stops at `MAX_PRECOMP_DEPTH` (16), silently omitting deeper content from otherwise valid projects. Graph validation now computes the longest referenced PreComp path while checking for cycles and rejects projects that exceed the renderer's actual nesting limit.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Deep-chain open/render behavior was not run; project validation now fails before rendering for over-limit references.

## Follow-up: blend modes altered source-only pixels over transparent backgrounds

The blend compositor applied the blend function to every source pixel, including areas where the backdrop alpha was zero. Multiply over a transparent RGBA canvas therefore multiplied source RGB by transparent black and produced black pixels with visible alpha. Standard color blend modes now interpolate the source color with the blend result according to backdrop alpha before source-over compositing. Stencil, silhouette, Behind, and Alpha Add retain their custom alpha/coverage operations.

Validation: `cargo fmt --all --check`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Transparent-background blend renders were not run. The formula follows the source-over blend model in the [W3C Compositing and Blending specification](https://www.w3.org/TR/compositing-1/).

## Follow-up: proxy generation could exhaust disk space or decode oversized frames

Video proxy generation accepted up to two million cached frames without estimating output size or preserving free disk space. Image and video proxy sources also reached full decode before a proxy-specific memory bound was checked. Proxy generation now validates source dimensions against the image cache pixel limit and a 512 MiB RGBA decode budget, preflights estimated WebP output plus at least 512 MiB or 15% free-space headroom, and periodically stops video proxy writes if that reserve is no longer available. Video frames must retain the first frame's dimensions; changed or unreadable frames clean up the partial proxy directory. Shared import disk-space helpers are now crate-visible for reuse.

Validation: `cargo fmt --all`, `git diff --check`, `cargo clippy --no-default-features --features cli --lib -- -D warnings`, and `CARGO_TARGET_DIR=/tmp/kagari-codex-proxy-win-check cargo check --target x86_64-pc-windows-msvc --no-default-features --features cli --bin kagari` passed. Actual proxy generation, disk-pressure interruption, and oversized image handling were not exercised; no tests were run.

## Follow-up: concurrent image proxies could overwrite the same output file

Single-image proxies selected an unused filename with an existence check, then encoded to it. Two concurrent jobs for the same layer could both choose that path and truncate or overwrite one another. Proxy output names are now reserved with exclusive file creation before encoding; collisions advance to another name, and failed encodes remove only the file reserved by that job.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Concurrent proxy generation was not exercised; no tests were run.

## Follow-up: overwrite exports used a replacement operation that fails on Windows

Video and GIF exports published successful temporary files with `std::fs::rename` when overwrite was enabled. Windows does not replace an existing destination with that operation, so a repeat export to the same filename could fail after rendering completed. Both exporters now share a platform-aware publish helper: Windows calls `MoveFileExW` with replace-existing semantics, while other platforms keep atomic same-filesystem rename behavior. The existing destination remains untouched if FFmpeg fails before publication.

Validation: `cargo fmt --all`, `git diff --check`, `cargo clippy --no-default-features --features cli --lib -- -D warnings`, and a Windows-target `cargo check` passed. Re-exporting over an existing file was not exercised through FFmpeg; no tests were run.

## Follow-up: projects with repeated video layers rehashed the same decoded cache

Opening or relinking a project validated each video layer's frame cache by opening every frame and hashing all encoded bytes. When multiple layers shared the same source, cache directory, and frame count, the same potentially multi-gigabyte cache was scanned once per layer. Validation results are now memoized for the duration of the project refresh using that exact identity tuple, preserving full integrity verification while limiting each shared cache to one scan per open.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Large-project open time was not benchmarked; no tests were run.

## Follow-up: video import buffered every frame directory entry to count frames

After FFmpeg extracted a video, the importer collected every directory entry into a `Vec` before counting numbered frames. At the supported two-million-frame limit this added a large, unnecessary allocation on top of the decoded media cache. Frame counting now streams `read_dir` entries and retains only the highest frame number, keeping memory usage independent of the number of files.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. A two-million-frame import was not run; no tests were run.

## Follow-up: GPU mask parity notes described behavior that the software renderer already supports

The mask rasterizer's parity-test documentation claimed the software renderer used only the first mask and ignored mask modes and opacity. The current CPU implementation combines all modes and opacity; only expansion and wiggle require the software path, while CPU and GPU feather edges use different continuous-versus-pixel-grid distance calculations. Corrected the stale comment so future reviews and render comparisons target the actual remaining edge-sampling difference.

Validation: `cargo fmt --all --check` and `git diff --check` passed. No mask render or tests were run.

## Follow-up: software layer masks sampled inconsistent pixel coordinates

The CPU renderer passed integer pixel origins to masks in image, solid, text, shape, 3D, precomp, and post-effect paths, while the compositor and particle path sampled at pixel centers (`+0.5`); the GPU mask rasterizer also samples pixel centers. This shifted mask boundaries by half a pixel depending on layer type and made CPU export differ from GPU preview. All software-renderer mask call sites now evaluate coverage at pixel centers. CPU polygon feathering and GPU distance-transform feathering can still differ at edges, but they now use the same coordinate convention.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. No visual mask parity render or tests were run.

## Follow-up: curved mask paths used different tessellation between CPU and GPU

The software renderer converted mask Bezier paths to 16 segments per curve while the GPU mask rasterizer used 12. Even with matching pixel-center sampling, curved masks could therefore have different silhouettes. Both paths now use the shared `MASK_CURVE_SEGMENTS` setting (16), retaining the smoother existing CPU outline for GPU preview as well.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Curved-mask preview/export parity was not rendered; no tests were run.

## Follow-up: reduced-size software previews cropped compositions instead of scaling them

The Studio requested viewport-sized frames from `render_frame_to_pixels_preview`, but the software renderer kept all layer positions, dimensions, masks, and camera coordinates in composition space. For a 1920×1080 composition rendered into a 640×360 buffer, layers were clipped against the smaller top-left buffer rather than downscaled with the composition. Preview rendering now renders in composition coordinates and resizes the completed frame to the requested dimensions, preserving the full composition and its mask/layout geometry. The full render and target sizes are checked against existing buffer and dimension limits before resizing.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Viewport rendering and large-composition performance were not exercised; no tests were run.

## Follow-up: PNG sequence collision checks missed case-only filename variants

The sequence exporter searched for exact `{stem}_{frame}.png` strings. On case-insensitive filesystems, an existing `.PNG` or differently-cased stem can occupy the destination path while escaping the overwrite preflight; a non-overwrite export then fails only during publication, and overwrite mode may omit the frame from its backup set. Existing-frame discovery now parses PNG frame indices case-insensitively and checks the exact generated destination with `symlink_metadata`, so the filesystem's own case rules determine whether the output path is already occupied. Dangling symlinks are also detected as occupied paths.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Case-insensitive filesystem sequence export was not exercised; no tests were run.

## Follow-up: imported motion-blur values could cause excessive work and corrupt trailing bytes

The CPU motion-blur kernel converted the configured distance directly into an `i32` loop bound. Extremely large or non-finite project values could make rendering impractically expensive or produce invalid averaging; fractional distances also made the divisor disagree with the inclusive sample loop. The kernel now caps radius at 64 pixels, rejects non-finite distances, normalizes non-finite angles to zero, and uses an exact sample count. It now allocates and copies only the validated RGBA region, preserving any trailing bytes supplied by callers.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Extreme-value and visual blur output were not exercised; no tests were run.

## Follow-up: replacing saved projects failed on Windows

Video and GIF export already used a Windows-specific replace-existing operation, but project saves, production-document saves, and project-backup replacement called `std::fs::rename` directly. On Windows, rename does not replace an existing destination, so saving over an existing project or backup could fail. The shared replacement helper now accepts `Path` values (preserving non-UTF paths) and is used by all of these save paths as well as media exports.

The PNG sequence overwrite path had the same gap in its per-frame atomic publish step; it now uses the shared replacement helper too, so a sequence can replace already-rendered frames on Windows.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Windows cross-compilation could not run because the `x86_64-pc-windows-gnu` Rust target is not installed in this environment; existing-file replacement was not runtime-tested, and no tests were run.

## Follow-up: composition settings accepted frames larger than the renderer can allocate

The Studio allowed width and height independently up to 16,384, while the software renderer rejects any single RGBA8 frame larger than 512 MiB. This made combinations such as 16,384×16,384 appear valid even though preview/render returned no frame. Width and height edits and preset application now check the combined frame-buffer budget and show a clear settings message before changing the composition.

Validation: `cargo fmt --all`, `git diff --check`, and `CARGO_TARGET_DIR=/tmp/kagari-studio-check-20261112 cargo check --bin kagari-studio` passed. UI interaction was not exercised; no tests were run.

## Follow-up: no-overwrite exports required hard-link support

Video, GIF, and PNG no-overwrite publication used `hard_link` as the only way to avoid replacing a destination. Some filesystems (including network or removable volumes) do not support hard links, causing otherwise valid exports to fail. The existing PNG sequence fallback was moved into a shared publisher: it tries an atomic hard link first, then uses `create_new` plus a synced copy, and removes a partial destination only after verifying it is still the file created by this operation. Video/GIF exports, individual PNG output, and sequence recovery now share this behavior.

Validation: `cargo fmt --all`, `git diff --check`, `CARGO_TARGET_DIR=/tmp/kagari-studio-check-20261112 cargo check --bin kagari-studio`, and `CARGO_TARGET_DIR=/tmp/kagari-studio-check-20261112 cargo clippy --no-default-features --features cli --lib -- -D warnings` passed. Hard-link-disabled filesystem behavior was not runtime-exercised; no tests were run.

## Follow-up: GLB/glTF import read unbounded files and trusted accessor arithmetic

Both interactive import and software-renderer mesh loading used `fs::read` with no file-size cap. The GLB parser also trusted chunk/accessor arithmetic and allowed reads to extend beyond a declared buffer view, which could panic or accept malformed geometry. GLB/glTF reads now stop at 128 MiB; chunk boundaries and accessor byte lengths use checked arithmetic; buffer-view bounds and buffer index are enforced; POSITION and index accessor formats and vertex references are validated; non-finite positions are rejected; and vertex reservation reports allocation errors rather than panicking.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Malformed-asset fixtures and large-file behavior were not exercised; no tests were run.

## Follow-up: glTF vertex attributes ignored interleaved byte strides

The hardened accessor reader still treated every attribute as tightly packed. Valid glTF buffer views may interleave POSITION, NORMAL, and TEXCOORD data using `byteStride`; the old reader silently decoded neighboring attributes as vertex coordinates. Accessor reads now compute checked strided spans, validate the declared stride and buffer-view bounds, and decode positions, normals, UVs, and indices per element. Invalid optional attribute accessors now return a descriptive import error instead of being silently skipped; non-finite normals and UVs are rejected. Mesh vectors reserve capacity fallibly before appending.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Interleaved glTF fixtures and render output were not exercised; no tests were run.

## Follow-up: advertised .gltf imports had no binary-buffer loading path

The import dialog advertised `.gltf`, but passed an empty binary payload to the parser, so standard JSON glTF files referencing an external `.bin` could not load. glTF loading now resolves external buffers or decodes data URIs, enforces the 128 MiB file and combined-buffer limits plus declared byte lengths, and shares one loader between import and preview. The parser uses each buffer view's declared buffer index. The model cache tracks every external buffer revision so changing a sidecar invalidates the cached mesh. Project collection copies referenced external buffer files and rewrites their URIs to the collected filenames.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Multiple external buffers, data URIs, sidecar invalidation, and collected-project round trips were not runtime-exercised; no tests were run.

## Follow-up: glTF normalized integer normals and UVs were rejected

The loader accepted only floating-point NORMAL and TEXCOORD_0 attributes, although glTF allows normalized signed BYTE/SHORT normals and normalized unsigned BYTE/SHORT texture coordinates. Accessors now preserve the `normalized` flag and decode these supported encodings to their normalized float ranges, including the signed minimum clamp. Unsupported component/normalization combinations are rejected explicitly; index accessors marked normalized are rejected as invalid.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Quantized attribute fixtures and render appearance were not exercised; no tests were run.

## Follow-up: glTF triangle strips and fans were imported as triangle lists

The importer assumed every primitive was `TRIANGLES`. Valid `TRIANGLE_STRIP` and `TRIANGLE_FAN` data was therefore assembled into incorrect faces. Primitive mode is now read (defaulting to glTF's `TRIANGLES`), strips are expanded with alternating winding, fans are expanded around their first vertex, and degenerate strip/fan triangles are skipped. Other point/line modes fail with an explicit unsupported-mode error because the current renderer only draws triangles. Material indices are now added only for emitted triangles.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Strip/fan image output and winding were not rendered against fixtures; no tests were run.

## Follow-up: imported 3D material colors were ignored by preview

The glTF importer preserved per-triangle PBR base colors, but the software preview shaded every face from the layer color alone. The rasterizer now applies the imported material's base-color factor per triangle, including alpha. Primitive material references outside the imported material table now fail with a descriptive error instead of leaving invalid indices in the mesh.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Material-color render comparisons and malformed material-index fixtures were not exercised; no tests were run.

## Follow-up: imported vertex normals and per-pixel light position were ignored

The model preview calculated one face normal and shaded an entire triangle with the light direction measured at its first vertex. Smooth normals imported from OBJ/glTF were unused, producing faceted shading and inaccurate point/spot light falloff. The renderer now inverse-transforms and rotates vertex normals, interpolates and normalizes them per covered pixel, and evaluates lighting at the interpolated world position. Meshes without a complete vertex-normal set retain the geometric face-normal fallback.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Smooth-shading visual comparisons, non-uniform/negative scale cases, and light falloff were not runtime-rendered; no tests were run.

## Follow-up: non-finite 3D transforms could poison raster bounds and depth

Animated model transforms or camera projection can produce non-finite world/screen coordinates. The model rasterizer now skips vertices with non-finite world or projected coordinates and skips triangles whose screen-space area is non-finite, preventing NaN arithmetic from entering bounds, barycentric, and depth calculations.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Malformed animation values and extreme projection cases were not runtime-exercised; no tests were run.

## Follow-up: triangles crossing the camera near plane disappeared

The projection helper returns no screen point for vertices at or behind the camera near plane, and the model rasterizer discarded a triangle whenever any one vertex had no projection. A visible polygon crossing the camera plane could therefore vanish. Model triangles are now clipped against the projection's 0.1-unit near threshold (with a small epsilon), preserving the visible polygon and interpolating its imported vertex normals at generated clipping vertices.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. A model intersecting the near plane was not runtime-rendered; no tests were run.

## Follow-up: glTF import silently dropped scene meshes and node transforms

The parser previously read only the first mesh resource and ignored scenes, node hierarchy, and object transforms. It now resolves the selected scene (or the root-node forest when scenes are omitted), flattens every referenced mesh instance, composes hierarchical translation/rotation/scale or affine matrix transforms, and applies inverse-transpose transforms to imported normals. Repeated instances preserve their primitive material assignments. Missing references, multiple parents, cyclic graphs, invalid transforms, and empty/non-triangle scenes now return explicit import errors. Scene expansion is capped at 100,000 instances and preflighted against a 256 MiB geometry bound to prevent repeated mesh instances from multiplying a small source file into unbounded allocations. Quaternion normalization rejects overflowed lengths instead of silently dropping the rotation.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Multi-mesh scene fixtures, nested transforms, repeated instances, and malformed-graph cases were not runtime-exercised; no tests were run. This is a Kagari importer fix, not an AmeUI request.

## Follow-up: glTF primitives without NORMAL used a fixed up-vector

After scene flattening, another shading defect was confirmed: primitives without an optional NORMAL accessor were padded with `[0, 1, 0]`, so lighting was wrong for most surfaces. Missing normals are now generated by accumulating transformed triangle normals at affected vertices; valid imported normals remain unchanged. Degenerate generated normals receive a finite fallback.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Lighting on normal-less multi-mesh assets was not visually compared; no tests were run.

## Follow-up: glTF alpha modes and emissive factors were discarded

Imported base-color alpha was previously treated as transparency for every glTF material, even when `alphaMode` was `OPAQUE` or `MASK`; emissive factors were retained in the data but not applied in preview. PBR materials now preserve alpha mode/cutoff with backward-compatible serde defaults, validate glTF mode/cutoff values, force opaque/mask fragments to opaque after the mask test, retain blend alpha, and add imported emissive color during shading. Albedo textures are not sampled yet.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Alpha-mode render comparisons and existing project material deserialization were not runtime-exercised; no tests were run.

## Follow-up: transparent 3D surfaces occluded one another

The model depth buffer retained only the nearest fragment, so a transparent front surface hid opaque or transparent geometry behind it. When a mesh uses `BLEND`, the software rasterizer now draws opaque/masked geometry first, then composites blended triangles from far to near without writing transparent depth. Per-pixel depth tests still discard blended fragments hidden by opaque geometry. Triangle ordering uses average camera depth, so intersecting transparent triangles can still need a more exact per-pixel ordering method.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Overlapping transparent model surfaces were not visually rendered; no tests were run.

## Follow-up: model preview failures hid importer diagnostics

The model renderer converted every read/parse failure to `None`, leaving the asset panel with only a generic format/path message. Model load failures now emit one detailed log entry per source path (including parser errors), and a successful reload clears the suppression so a later independent failure can be reported. The asset preview failure message now points users to the application log for the detailed reason.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Missing and malformed model previews were not exercised; no tests were run.

## Follow-up: OBJ face UV and normal indices were ignored

The OBJ parser used only the position component of each `f` token. Standard `v/vt/vn`, `v//vn`, and negative-relative references therefore lost hard/smooth normals and could not align UVs with split seams. Faces now resolve position/UV/normal indices independently, expand vertices by the full attribute tuple, preserve referenced UVs/normals, and generate area-weighted smooth normals only where OBJ omitted them. Out-of-range references and files with no triangles fail explicitly. This prepares UVs for texture sampling, which remains unsupported in the software renderer.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. OBJ seam fixtures, negative-index variants, and rendered lighting were not runtime-exercised; no tests were run.

## Follow-up: glTF base-color textures were ignored

glTF materials now retain `baseColorTexture` references and map texture indices through the glTF texture-to-image table. Data URI images and GLB bufferView images are decoded into shared image data for the software renderer; external `.gltf` image URIs resolve relative to the model, participate in cache dependency fingerprints, and are collected and rewritten when bundling a project. The rasterizer samples these using the existing perspective-correct bilinear/repeat path and material alpha handling.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. No runtime render was performed and no tests were run. External URI images referenced by GLB resolve, participate in renderer cache invalidation, and are now collected and rewritten in the GLB JSON chunk while preserving binary and extension chunks. Remaining follow-up: sampler wrap/filter settings and texture transform extensions are not honored. Embedded image dimensions and aggregate decoded memory are checked before pixel decoding.

## Follow-up: glTF texture sampler state was ignored

Imported texture materials now retain glTF sampler wrapS/wrapT and filter choices. The CPU sampler supports repeat, clamp-to-edge, mirrored-repeat, and nearest sampling while keeping bilinear filtering as the default for materials without a sampler. Unsupported wrap/filter enums and invalid referenced sampler indices fail import with a diagnostic rather than silently rendering the wrong address mode.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. No runtime image comparison was performed and no tests were run. Mipmap levels remain unsupported.

## Follow-up: glTF texture UV transforms were ignored

Base-color texture imports now read `KHR_texture_transform` offset, scale, and rotation and apply them after perspective-correct UV interpolation. The effective texCoord selector is validated; nonzero sets fail clearly because the mesh importer currently imports only `TEXCOORD_0`. Non-finite transform values are rejected instead of contaminating raster coordinates.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. No runtime render comparison was performed and no tests were run. UV sets beyond TEXCOORD_0 remain unsupported.

## Follow-up: bilinear texture filtering caused transparent edge halos

The software texture sampler previously interpolated RGB independently from alpha, so RGB stored in fully transparent texels could bleed into semi-transparent edge samples. Bilinear sampling now interpolates premultiplied RGB and alpha, then returns straight-alpha color for the existing material/compositing path. Nearest sampling remains unchanged.

Validation: `cargo fmt --all`, `git diff --check`, and Studio `cargo check --bin kagari-studio` passed. The updated sampler was not visually compared against a reference and no tests were run.

## Follow-up: glTF minification and magnification filters were conflated

The sampler previously collapsed minFilter and magFilter into one nearest/linear choice, producing incorrect results whenever a material specified different filters for enlargement and reduction. Filter choices are now stored independently, and the rasterizer estimates the local screen-space texel footprint using perspective-correct UV samples to choose minification or magnification filtering per pixel. Mipmap levels are still not generated, so mipmap filter enums use their nearest/linear within-level component only.

Validation: `cargo fmt --all`, `git diff --check`, and Studio `cargo check --bin kagari-studio` passed. No runtime filter comparison was performed and no tests were run.

## Follow-up: glTF importer accepted files without a compatible asset version

The mesh parser previously accepted any JSON containing mesh data, even without the required `asset.version` declaration. That could interpret a different glTF generation as 2.0 and produce plausible but incorrect geometry or materials. Loading now requires `asset.version` to be exactly `2.0` immediately after parsing the JSON, before opening external buffers, validates `minVersion` syntax, and rejects minimum versions newer than the supported importer.

Validation: `cargo fmt --all`, `git diff --check`, and Studio `cargo check --bin kagari-studio` passed. Version rejection cases were not run and no tests were added.

## Follow-up: sparse glTF accessors silently lost overridden vertices

The importer read only the dense bufferView portion of accessors and ignored sparse override records, which silently changed valid optimized meshes. Accessors now materialize only when sparse data (or a zero-initialized no-bufferView form) requires it, apply strictly increasing validated sparse indices, overlay raw values before normal attribute decoding, and preserve the zero-copy path for ordinary interleaved attributes. Sparse count, component types, bufferView stride, bounds, and byte alignment are checked.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Sparse fixtures were not executed and no tests were added.

## Follow-up: glTF materials could not select additional UV sets

TextureInfo `texCoord` and `KHR_texture_transform.texCoord` now select imported UV sets 0 through 7. glTF primitives decode each declared `TEXCOORD_n` accessor, and rendering picks the set required by the material before applying its UV transform. A textured primitive that lacks its selected set fails explicitly. Unsupported or malformed channel names are rejected, and expanded-memory estimation includes all allocated UV sets.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Multi-UV model rendering was not visually exercised and no tests were added.

## Follow-up: glTF vertex colors were discarded

`COLOR_0` attributes now import FLOAT and normalized unsigned VEC3/VEC4 data, with VEC3 alpha defaulting to one. Color values are perspective-correctly interpolated through rasterization and near-plane clipping, then multiplied with material factors and texture samples. Mixed primitives default missing vertex colors to white; meshes without color attributes retain the empty fast path. Expanded-memory checks account for color storage.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Vertex-color gradients and mixed colored/uncolored primitives were not visually rendered; no tests were added.

## Follow-up: glTF animation and skinning data was silently ignored

The static model layer previously flattened bind-pose meshes while silently ignoring glTF animation channels, skin joints/weights, and morph targets. These files now fail early with a feature-specific diagnostic instead of appearing as valid but incorrect static models. All indexed JOINTS/WEIGHTS sets are detected, not only set 0. Required extensions are checked explicitly; `KHR_texture_transform` is recognized and other required extensions are rejected before external buffers are opened. Core normal, occlusion, emissive, and metallic-roughness textures are also rejected clearly until the renderer supports them. Vertex-color components are validated to the glTF 0..1 range.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Unsupported-feature fixtures were not exercised and no tests were added.

## Follow-up: glTF accessor offsets could be misaligned

Accessor byte offsets were bounds-checked but not checked against the component width. A malformed float or short accessor could therefore be decoded from the wrong byte boundary and produce plausible but corrupt geometry. Base and sparse accessor offsets and their bufferView offsets now require component alignment before bytes are interpreted.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Misaligned fixtures were not exercised and no tests were added.

## Follow-up: GLB parser accepted malformed container framing

The GLB reader accepted bytes beyond the declared file length, allowed non-aligned chunk lengths, and overwrote earlier JSON/BIN chunk references when duplicates appeared. It now requires the declared length to match the file, a JSON first chunk, four-byte chunk alignment, and at most one JSON and BIN chunk.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Malformed container fixtures were not exercised and no tests were added.

## Follow-up: glTF materials inherited OBJ gray defaults

Materials with no PBR block, and primitives with no material at all, inherited the OBJ importer's 0.8 gray base color. glTF's base-color default is white; material initialization now uses white base color and the glTF metallic/roughness defaults even when those fields are omitted.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Default-material render output was not visually checked and no tests were added.

## Follow-up: glTF PBR factors were silently clamped during rendering

Base color, metallic, roughness, and emissive factors outside the glTF 0..1 range were accepted and later clamped in the renderer, hiding invalid source values. These factors are now validated during import and produce a material-specific diagnostic instead of changing the authored asset silently.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Invalid-factor fixtures were not exercised and no tests were added.

## Follow-up: empty glTF accessors were accepted

Zero-count accessors are invalid glTF and can produce empty or misleading mesh data. The importer now rejects them during JSON validation, before loading external buffers or constructing geometry.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Zero-count fixtures were not exercised and no tests were added.

## Follow-up: tiny glTF buffer declarations could trigger large reads

External buffers were read up to the general 128 MiB file limit and only then truncated to `byteLength`; the oversized vector capacity remained allocated. The loader now reads exactly the declared byte count from external files, reserves that bounded size fallibly, and bounds base64/percent-encoded data URIs against the declared length before decoding. Zero-length buffers and decoded-length mismatches fail explicitly.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Oversized dependency files and malformed data URIs were not exercised; no tests were added.

## Follow-up: glTF scene roots could omit parent transforms

The selected scene's root list was traversed without verifying that entries were actual graph roots or unique. Listing a child as a scene root discarded its parent transform; duplicate entries instantiated the same node twice. Selected roots are now checked against the validated parent table for existence, zero parents, and uniqueness before traversal.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Invalid root-graph fixtures were not exercised; no tests were added.

## Follow-up: imported meshes accumulated without a memory limit

The renderer's process-wide mesh cache retained every model path indefinitely, including up to 256 MiB of embedded textures per imported mesh. It now tracks approximate retained allocations and evicts least-recently-used entries to a 512 MiB cache budget. If one valid model exceeds that budget, it becomes the sole retained entry until another model is inserted, avoiding a full re-import on every frame. External image pixels remain owned by the separately bounded image cache.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. LRU eviction under multiple open models was not runtime-exercised; no tests were added.

## Follow-up: OBJ face expansion could exceed the mesh cache budget

OBJ source files are bounded to 128 MiB, but splitting vertices by UV/normal/smoothing-group tuples and triangulating large polygons can expand the in-memory mesh substantially. The parser now checks estimated geometry/index/material storage during expansion against a 256 MiB budget and caps one polygon at one million corners, rejecting oversized data before continuing allocations.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check --bin kagari-studio`, and CLI-library Clippy passed. Oversized OBJ inputs were not exercised and no tests were added.

## Follow-up: OBJ smoothing groups were ignored

Generated normals were previously shared across every face using a position/UV tuple, even when the OBJ declared `s off`; hard edges therefore appeared rounded. Vertex sharing now includes the smoothing group for generated normals, defaults to per-face flat groups, recognizes `s on` and named/numeric groups, and accumulates smooth normals independently of UV seams. Explicit `vn` indices continue to take precedence.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Flat/smooth group boundaries and UV-seam shading were not visually rendered; no tests were run.

## Follow-up: OBJ MTL materials and face assignments were ignored

`mtllib` and `usemtl` did not affect imported face colors or opacity. OBJ loading now resolves referenced MTL files, maps `usemtl` onto each emitted triangle, and imports `Kd`, `d`/`Tr`, `Ke`, and `Ns` into diffuse color, blend alpha, emission, and roughness. `map_Kd` is now sampled with repeating bilinear UV lookup from the shared image cache; OBJ V coordinates are converted to the image origin, and unavailable maps fall back to the material color with a logged diagnostic. The model cache fingerprints MTL dependencies so edits invalidate previews; project collection copies and rewrites OBJ/MTL and map references. Other MTL directives remain unsupported. glTF `baseColorTexture`, sampler address/filter choices, and `KHR_texture_transform` are handled in the follow-up entries above.

Validation: `cargo fmt --all`, `git diff --check`, Studio `cargo check`, and CLI-library Clippy passed. Multi-library material assignments, UV orientation, filtering, and rendered MTL shading were not runtime-exercised; no tests were run.


## Follow-up: project save paths could produce files rejected by the reader

The versioned project serializer, production-document serializer, and both legacy project serializers now validate the serialized JSON against the same 100 MiB limit enforced by readers. Oversized content fails before any target file is replaced, so a successful save cannot create a project that the corresponding bounded reader refuses solely because of size.

Validation: `cargo fmt --all`, `git diff --check`, remote CLI library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: versioned saves could write projects rejected by migration loading

`save_project_versioned` previously serialized a `Project` without the production-document validation that `load_project_migrated` applies after parsing. It now validates the project before serialization, so the primary versioned save path refuses invalid projects before creating or replacing a file.

Validation: `cargo fmt --all`, `git diff --check`, remote CLI library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: image-sequence collection used unbounded recursive directory traversal

Copying a collected image-sequence cache recursively consumed one Rust call frame per directory level. A deeply nested cache could exhaust the stack and abort the Studio process. The traversal now uses an explicit pending-directory stack, preserving file copying and symlink rejection without recursion.

Validation: `cargo fmt --all`, `git diff --check`, remote CLI library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: autosave slot replacement failed on Windows

Autosave snapshots reuse a bounded set of slot paths. The writer used `std::fs::rename`, which does not replace an existing destination on Windows, so a slot's second write could fail and leave autosave unable to advance. It now uses the shared cross-platform atomic replacement helper already used by project and export writers.

Validation: `cargo fmt --all`, `git diff --check`, remote CLI library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: autosave writer could exceed the recovery reader limit

Recovery loading rejects files over the shared 100 MiB JSON cap, but the snapshot writer previously streamed without a corresponding bound and could report success for a snapshot that recovery would always reject. Autosave now uses a bounded streaming writer and aborts before writing beyond the same cap; the incomplete temporary file is removed on failure.

Validation: `cargo fmt --all`, `git diff --check`, remote CLI library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: preset saves could truncate the last valid user settings

Export and effect preset writers serialized JSON and then called `fs::write` directly on the destination. An interrupted write or disk error could truncate the user's only preset file. A shared atomic file writer now stages and syncs the complete payload before replacing the destination, and the legacy project writer uses the same helper to keep replacement behavior consistent.

Validation: `cargo fmt --all`, `git diff --check`, remote CLI library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: modifying a malformed export-preset file could erase all presets

The display loader intentionally falls back to an empty list when user settings cannot be read, but add/delete also used that fallback and then saved it over the source file. A strict internal loader now distinguishes a missing file from read/parse errors; add/delete propagate those errors and leave malformed settings untouched. The existing display API remains compatible and logs the load failure.

Validation: `cargo fmt --all`, `git diff --check`, remote CLI library `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: Studio exports used Windows-incompatible replacement

The Studio's atomic text exporter and PNG overwrite path still called `std::fs::rename` directly. On Windows, exporting over an existing Lottie, MLT, subtitle, template, or PNG could fail even though the UI offered a save destination. Text exports now use the shared atomic writer, and PNG overwrite uses its cross-platform staged-file replacement helper.

Validation: `cargo fmt --all`, `git diff --check`, remote `cargo check --bin kagari-studio`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: remaining settings and image outputs used direct rename

A repository-wide rename scan found Windows-incompatible replacement in Studio preferences, CLI text/PNG output, and OpenEXR output. These paths now use the shared atomic writer or cross-platform staged-file replacement helper. This prevents existing preferences and exports from failing to update on Windows after a successful encode.

Validation: `cargo fmt --all`, `git diff --check`, remote Studio and CLI binary `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: persistent cache refreshes could fail on Windows

The render-frame cache, video-sequence manifest, and dynamic-link frame cache all refreshed existing files with `std::fs::rename`. On Windows, those refreshes fail when the destination already exists. They now use the shared cross-platform replacement helper, keeping cache regeneration available across supported platforms.

Validation: `cargo fmt --all`, `git diff --check`, remote Studio and CLI binary `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: legacy preferences and image caches still refreshed with rename

The remaining direct file refreshes included legacy preference writes and persistent render/video caches. Preference writes used truncating temporary-file creation and non-replacing rename; this could collide with a stale temp file and fail to update an existing file on Windows. Legacy preferences now use the shared atomic writer. Render frame cache, video sequence manifest, and dynamic-link cache updates now use cross-platform replacement as well.

Validation: `cargo fmt --all`, `git diff --check`, remote Studio and CLI binary `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: Windows-only OpenFX discovery warning blocked strict cross-Clippy

After adding the Windows GNU Rust target on the remote builder, CLI library, Studio, and CLI binary cross-checks passed. Strict Windows-target Clippy then exposed a Windows-only `Option` conversion warning in OpenFX search-path discovery; it now matches the environment lookup result directly.

Validation: Windows-target CLI library, Studio, and CLI binary `cargo check` passed; Windows-target strict CLI-library Clippy (`-D warnings`), `cargo fmt --all`, and `git diff --check` passed. No tests were run and no Windows executable was launched.


## Cross-platform verification: staged file replacement changes

The shared atomic writer and the Studio/CLI/cache replacement call sites were checked with the Windows GNU target after the platform-specific work landed. Windows-target CLI library, Studio, and CLI binary `cargo check` succeeded, followed by strict Windows-target CLI-library Clippy (`-D warnings`). The Windows Clippy pass also includes the OpenFX environment-lookup cleanup noted above.

No tests were run and no Windows binary was launched.


## Follow-up: Studio preference writes could replace unreadable settings with defaults

Preference reads returned an empty object for malformed, oversized, or inaccessible files. The write path reused that fallback and could then replace the existing settings with only the changed keys. Reads now distinguish missing files from invalid or unreadable files; the UI keeps its safe empty fallback with a warning, while preference writes abort on read/parse/size errors and preserve the original file.

Validation: `cargo fmt --all`, `git diff --check`, remote Linux and Windows-target Studio `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: partial PNG/EXR cleanup could delete files created by another process

Studio PNG and EXR sequence exporters removed partial outputs by filename alone after cancellation or a later-frame failure. If another process replaced an earlier frame during export, cleanup could delete that unrelated file. New PNG/EXR publication now retains the filesystem identity returned by the staged-file publisher; rollback removes a frame only while that exact file is still present, and reports any replacement without deleting it.

Validation: `cargo fmt --all`, `git diff --check`, remote Linux and Windows-target Studio `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: proxy cleanup deleted a reserved path without checking ownership

Image proxy generation reserves a unique output file, then removes that path if encoding fails. Cleanup now retains the reserved file identity and only removes the reservation if it still points to the same file. The shared identity-checked removal helper is also used by Studio sequence rollback.

Validation: `cargo fmt --all`, `git diff --check`, remote Linux and Windows-target Studio `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: import and proxy worker startup could panic the Studio

Video import, image-sequence import, and proxy generation used infallible thread spawning. If the OS could not allocate another thread, the UI event handler would panic instead of reporting a recoverable failure. These workers now use named fallible thread builders and leave the current editor state intact while displaying a localized startup error.

Validation: `cargo fmt --all`, `git diff --check`, remote Linux and Windows-target Studio `cargo check`, and strict CLI-library Clippy (`-D warnings`) passed. No tests were run.


## Follow-up: export worker startup could panic the Studio

GIF, PNG sequence, EXR sequence, and video exports used infallible thread spawning. If the operating system could not create a worker thread, starting an export could panic the UI. These workers now have descriptive thread names and report a localized startup error while leaving the export controls idle if thread creation fails.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --bin kagari-studio` passed. The Linux/Windows remote check did not run because syncing the large AmeUI checkout stalled. No tests were run.


## Follow-up: slow-motion preset could make a project unsavable

The 2x slow-motion preset doubles a layer's duration. When there was not enough remaining composition time, it pushed the layer out-point beyond the composition; project validation then rejected the edited document on save. Studio now explains that the composition must be extended before applying the preset, and the CLI rejects stretches that exceed the selected composition or use non-finite/too-small factors, instead of reporting an ignored stretch as successful. The shared time-stretch operation also ignores non-finite factors instead of converting them into a one-frame duration.

The shared operation also rejects finite stretch factors whose requested duration cannot fit in the layer's `u32` frame range, avoiding saturated out-points from extreme values.

Validation: local macOS `cargo check --bin kagari-studio`, `cargo check --features cli --bin kagari`, `cargo fmt --all`, and `git diff --check` passed. No tests were run.


## Follow-up: project validation allowed multi-gigabyte particle emitters

Production documents and the CLI accepted up to ten million live particles per emitter. CPU particle state includes a fixed trail buffer per particle, so rendering such a project could allocate gigabytes and terminate the editor. A shared 100,000-particle ceiling now applies at project validation, CLI input, and `ParticleSystem` construction, keeping invalid files from entering the project and protecting direct renderer callers. Project validation names the layer and configured count when the cap is exceeded so users can identify the setting to change.

Validation: local macOS `cargo check --bin kagari-studio`, `cargo check --features cli --bin kagari`, `cargo fmt --all`, and `git diff --check` passed. No tests were run.


## Follow-up: timeline canvas ignored wheel navigation and layer rows were not scrollable

Kagari renders its timeline as a host-owned AmeUI Canvas, but did not implement AmeUI's canvas scroll callback, so wheel/trackpad navigation over the timeline did nothing. The timeline now pans in time with horizontal scroll or Shift+vertical scroll, moves through layer rows with vertical scroll, and zooms around the pointer with the platform's primary modifier plus vertical scroll. Rendering and camera-keyframe overlays are limited to visible track rows, and pointer hit-testing maps the scrolled row back to its layer index. The change stays in Kagari because AmeUI already exposes the required callback.

Validation: local macOS `cargo check --bin kagari-studio`, `cargo fmt --all`, and `git diff --check` passed. No tests were run. Interactive mouse/trackpad behavior has not been exercised in the running app.


## Follow-up: timeline hierarchy filtering repeatedly searched every layer

The timeline's collapsed-parent filter looked up every ancestor by scanning the entire composition layer list for each layer. Deep hierarchies could make this preparation quadratic even though only a small number of rows were drawn. It now builds a parent-to-children index once and walks hidden descendants iteratively, bounding that hierarchy pass to linear work.

Validation: local macOS `cargo check --bin kagari-studio`, `cargo fmt --all`, and `git diff --check` passed. No tests were run.

## Follow-up: Dynamic Link server startup could panic on thread exhaustion

`start_sync_server` already returned `io::Result` for bind failures but used infallible thread spawning for its accept loop. If the operating system could not create the server thread, the public API panicked after binding a port. It now starts a named thread with `Builder::spawn` and propagates the startup error through the existing `Result`; the “listening” log is emitted only after successful startup.

Validation: local macOS CLI-library `cargo check`, `cargo fmt --all`, and `git diff --check` passed. Thread-creation failure injection and TCP synchronization were not run.

## Follow-up: dense particle collisions repeatedly rescanned candidate cells

The uniform-grid particle collision solver rescanned every index in each neighboring cell for every candidate pair just to find the next index. Dense cells therefore added a repeated linear scan inside the pair loop, driving collision-heavy frames toward cubic work. Grid buckets are now kept sorted as particles move, and `partition_point` locates the next candidate in logarithmic time while preserving ascending pair order and the existing dynamic grid updates.

Validation: local macOS `cargo check --lib`, `cargo fmt --all`, and `git diff --check` passed. No collision tests or performance benchmarks were run, so dense-overlap output and measured speedup still require runtime verification.

## Follow-up: asset preview worker failure left requests loading forever

The image/model/composition preview worker also discarded its thread-start result. A failed start or later worker exit left requested and queued preview keys pending, so the Editor kept polling indefinitely and the selected asset never received a useful failure state. Preview result polling now distinguishes an empty channel from a disconnected worker, clears and resolves outstanding keys, detects a disconnected request queue separately from temporary queue pressure, and stops resubmitting work after worker failure. The asset panel reports that its preview service stopped and asks for an app restart. The viewport preview now reports the same worker-stop condition in the composition panel instead of leaving a blank frame without explanation.

Validation: local macOS Studio `cargo check --bin kagari-studio`, `cargo fmt --all`, and `git diff --check` passed. No tests or worker-failure injection were run; interactive asset preview behavior still needs runtime verification.


## Follow-up: graph and timeline canvases shared a width measurement

Both canvases wrote to `timeline_width`. When both were visible, whichever Canvas drew last could skew the other's pointer-to-frame calculations and graph snap distance. Graph interactions now use a separate graph-canvas width; timeline scroll and editing keep the timeline width.

Validation: local macOS `cargo check --bin kagari-studio`, `cargo fmt --all`, and `git diff --check` passed. No tests were run.


## Follow-up: timeline requested waveforms for offscreen rows

Waveform decoding is asynchronous and bounded, but every timeline redraw scanned every row and queued audio assets that were not visible. The timeline now requests waveforms only for visible rows; scrolling a row into view triggers its decode request.

Validation: local macOS `cargo check --bin kagari-studio`, `cargo fmt --all`, and `git diff --check` passed. No tests were run.


## Follow-up: waveform decoder startup failure was silent

The waveform worker's fallible thread creation result was discarded. A disconnected request channel was treated like a full queue, so selected audio assets remained in a loading state forever. Requests now distinguish temporary queue pressure from a dead worker; the latter marks the waveform service unavailable and gives a restart instruction in the asset panel.

Validation: local macOS `cargo check --bin kagari-studio`, `cargo fmt --all`, and `git diff --check` passed. No tests were run.

## Follow-up: render pipeline worker startup could panic

The public `RenderPipeline::new` constructor used `expect` when its worker thread could not be created, turning an operating-system resource error into a process panic. It now retains the startup error, logs it, exposes availability and error-detail accessors, and ignores frame/prefetch/flush requests when no worker was started.

Validation: local macOS CLI-library `cargo check`, `cargo fmt --all`, and `git diff --check` passed. Thread-start failure injection was not run.

## Follow-up: Dynamic Link startup errors were all reported as port conflicts

After server startup began propagating thread-creation errors, the legacy GUI caller still retried every error across eleven ports and finally told users that all ports were occupied. It now retries only `AddrInUse`; other failures stop the search and display the actual port and operating-system error. The all-ports-in-use message remains specific to genuine bind conflicts.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. No port-conflict or thread-start failure was induced.

## Follow-up: GPU RAM preview could never start its first prepass

Playback pre-rendering was gated on the current frame already existing in the RAM cache. Starting playback clears that cache, so the prepass never began and playback stayed on the regular per-frame render path. The prepass now runs during playback whenever there are frames left to prepare; cached frames are still used opportunistically when the playhead reaches them.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Playback timing and cache hit behavior were not exercised interactively.

## Follow-up: project saves through a symlink replaced the link itself

The project serializer preserved symlink targets, but the document-aware atomic save path still passed an existing symlink pathname directly to atomic replacement. On platforms where rename replaces the directory entry, Save would replace the link instead of updating its target. Existing save paths are now resolved to their canonical target before reading the current document, refreshing its backup, and publishing the new file. A dangling or inaccessible existing symlink fails path resolution without replacing the link.

Validation: local macOS `cargo check --lib`, `cargo fmt --all`, and `git diff --check` passed. Symlink save behavior was not runtime-exercised; no tests were run.

## Follow-up: legacy exports and settings wrote directly to final files

The legacy Studio's OTIO, MLT XML, SRT, Lottie, MOGRT, render-farm manifest, and .cube LUT exports used `fs::write` on the selected destination. A process interruption or write failure after truncation could destroy the previous export. These text outputs now use the shared atomic writer, which stages and syncs the full file before replacing the destination. UI mode, custom translation, and shortcut JSON also used direct writes; those settings now use the same atomic path so a partial write does not corrupt their saved preferences.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Export interruption behavior was not runtime-exercised; no tests were run.

## Follow-up: stock asset import could overwrite a concurrently created file

Stock asset import checked whether a destination name existed and then used `fs::write`. Another process could create that path between the check and write, causing an unrelated file to be truncated; a failed copy could also leave a partial asset behind. Import now reserves each candidate with `create_new`, writes and syncs through the reserved handle, and tries the next suffix on name collisions. On write or image-inspection failure it removes the file only if its identity still matches the file created by this import.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Concurrent import and injected I/O failures were not runtime-exercised; no tests were run.

## Follow-up: invalid production documents silently lost production-only data

The Studio and CLI previously attempted `ProductionDocument::from_json(...).ok()` and then retried any failure as a legacy `Project`. Since serde accepts unknown fields, a production document whose audio, tempo, automation, or schema validation failed could still open as a plain project; saving it could discard that metadata. A shared loader now distinguishes the production envelope (`schema_version` plus `project`) from legacy project formats and returns the production validation error instead of downgrading. The Studio open path and CLI edit, render, and frame paths use the shared loader. Backup loading continues to try a valid backup when the primary is invalid.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui,cli --bin kagari-studio-egui --bin kagari` passed. No tests were run; malformed production-document recovery and legacy-format compatibility still need fixture-based verification.

## Follow-up: production document saves could replace a symlink instead of its target

A previous fix canonicalized existing project paths in the legacy atomic serializer, but Studio saves now call `ProductionDocument::save_atomic` directly. That path still atomically replaced the symlink directory entry, silently breaking linked project paths. Production-document saves now resolve existing targets before staging/replacement, and fail without replacing dangling or inaccessible links.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui,cli --bin kagari-studio-egui --bin kagari` passed. Symlink behavior was not runtime-exercised; no tests were run.

## Follow-up: atomic writes replaced dangling symlink entries

The shared `write_atomic` resolved normal symlinks, but treated `canonicalize` returning NotFound as a new path. For a dangling symlink, this let replacement overwrite the link itself. It now checks the directory entry with `symlink_metadata` first; existing entries must canonicalize successfully, while only truly absent entries use the requested path. This aligns exports and preferences with the production-document save behavior.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui,cli --bin kagari-studio-egui --bin kagari` passed. Symlink cases were not runtime-exercised; no tests were run.

## Follow-up: legacy fallback remained in shared CLI, preview, and template loaders

A cross-repository search found additional readers that still converted any production-document parse/validation failure into a legacy project: the CLI's common project loader, the external preview app's reload path, and the Studio's template project loader. All now call `load_project_document_or_legacy`, so production-only data errors remain visible instead of being silently dropped. The envelope detector also treats production-only fields as evidence even when the `schema_version` or `project` field itself is missing, preventing malformed production documents from masquerading as legacy projects.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui,cli,preview,ame --bin kagari-studio --bin kagari-studio-egui --bin kagari --bin kagari-preview` passed. No tests were run; malformed-document and old-format fixtures still need verification.

## Follow-up: thread exhaustion could panic media import and Home scanning

Video and image-sequence imports used `thread::spawn` after presenting a pending import job, while recent-project scanning used the infallible spawn API as well. If the OS could not create a thread, these actions could panic the UI process; imports could also leave a receiver registered with no worker. The three paths now use named fallible thread builders, register import jobs only after the worker starts, track successful handles, and report startup errors in the UI. Home keeps its immediately seeded recent-project list if scanning cannot start.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Thread-start failure handling and import flows were not runtime-exercised; no tests were run.

## Follow-up: proxy, tracking, camera solve, and collection workers could panic on startup failure

A second pass over UI worker creation found four more high-impact actions still using infallible `thread::spawn`: proxy rendering, quad tracking, 3D camera solving, and portable-footage collection. They now use named fallible builders, publish pending UI state only after successful startup, track started workers, and show an error if the OS refuses to create a thread. This prevents dead pending jobs and process panics under resource pressure.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Thread-start failure paths were not runtime-exercised; no tests were run.

## Follow-up: remaining UI workers and render watchdog used infallible thread creation

The final UI `thread::spawn` sites covered footage replacement, video export preparation, scripting, and Motion Sketch bake. They now use fallible named builders; pending state is committed after startup, and failed starts report errors. Motion Sketch restores its captured samples if the worker cannot start, avoiding loss of the recorded gesture. The software-renderer's deadline watchdog also used infallible spawn; it now reports watchdog startup failure and returns a timed-out frame result instead of panicking or rendering without the promised deadline protection.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Worker-start failure behavior and the watchdog fallback were not runtime-exercised; no tests were run. Search confirms no remaining infallible `thread::spawn` calls in the Studio UI or software-renderer deadline path.

## Follow-up: all remaining fallible UI worker starts now preserve app state

Finished the remaining `thread::spawn` review by converting footage replacement, video-export preparation, script execution, and Motion Sketch baking to named fallible builders. Worker receivers and loading state are installed only after successful startup; thread-start errors reset export state, show a user-facing error, and Motion Sketch retains its recorded samples for retry. The software-renderer deadline watchdog also returns an explicit timed-out result if its timer thread cannot start. PNG sequence export already reports its builder spawn error through its completion channel.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS Studio `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. No runtime failure injection or tests were run. A source search found no remaining direct `thread::spawn` calls in the reviewed Studio UI and render-watchdog areas.

## Follow-up: discarded media imports leaked generated frame and audio caches

Video and image-sequence imports correctly refused to attach results after a project switch, missing target composition, or failed insertion, but left the successfully decoded frames and optional audio in the shared temp media area. Those discarded results now clean up their generated import directory. Cleanup only accepts the expected `frames` layout and a real directory canonically contained under Kagari's temp `kagari_media` root, avoiding recursive deletion through an externally redirected path.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS Studio `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Project-switch cleanup was not runtime-exercised; no tests were run.

The same guarded cleanup is now reused when a video-footage replacement is abandoned because its target asset changed or the project switched, replacing the previous unvalidated `remove_dir_all` over each returned parent path.

## Follow-up: project serializers duplicated atomic publication and dropped file permissions

`ProductionDocument::save_atomic` and the legacy project serializer maintained separate temporary-file/rename implementations. Besides duplicating symlink handling, neither copied existing file permissions to the replacement, unlike the shared atomic writer; saving a restricted project could silently make it more broadly readable under the process umask. Both save paths now publish through `core::atomic_file::write_atomic`, retaining the existing project backup and save lock behavior while sharing unique staging, permission preservation, sync, and symlink-target handling.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui,cli,preview,ame --bin kagari-studio --bin kagari-studio-egui --bin kagari --bin kagari-preview` passed. Permission retention and symlink saves were not runtime-exercised; no tests were run.

## Follow-up: project backups did not inherit the source file permissions

The atomic project backup copied the project bytes into a newly created staging file but retained that file's default permissions. A restricted project could therefore have a more broadly readable `.bak`. Backup staging now copies the source project's permissions before writing its contents and publishing the backup.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --lib` passed. Backup permission behavior was not runtime-exercised; no tests were run.

## Follow-up: backup files inherited default permissions

Atomic project backup staging copied contents but did not carry the source project's permissions. It now applies source permissions before copying bytes, so a restricted project does not produce a more broadly readable backup.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --lib` passed. Permission behavior was not runtime-exercised; no tests were run.

## Follow-up: CLI MP4/GIF export added a redundant panic-prone thread

The CLI wrapped calls to the FFmpeg export API in an extra `thread::spawn`, then waited on a second completion channel and joined that wrapper. The export APIs already create their own fallible worker and return the startup result, so the outer thread only added an unhandled panic path. MP4 and GIF export now call the API directly, consume its startup result, and continue reading progress/completion events from the existing channel.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --bin kagari` passed. CLI exports were not run; no tests were run.

## Follow-up: clean shutdown removed crash marker before joining workers

`on_exit` removed the dirty-exit marker after autosave but before joining background workers. If a worker hung during shutdown and the user force-quit the app, the next launch could miss the interrupted session. Marker removal now happens only after all workers have joined, and only when autosave succeeded and no worker panicked. A force quit during a long join therefore leaves the marker for the next startup recovery check.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Shutdown interruption behavior was not runtime-exercised; no tests were run.

## Follow-up: completed workers stayed joinable until app exit

`track_background_thread` retained every handle for shutdown, but no code reaped completed workers during a long Studio session. Repeated short imports/scans could accumulate joinable OS thread resources. Each UI update now extracts handles whose threads have finished and joins them outside the mutex; active workers remain tracked for shutdown. Worker panics are remembered so shutdown keeps the recovery marker even after the handle has been reaped.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS Studio `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Long-session resource usage and panic recovery behavior were not runtime-exercised; no tests were run.

## Follow-up: shutdown cancel-flag registrations accumulated after jobs ended

The app stored a strong `Arc` for every registered CAF and roto-bake cancellation flag and never removed completed entries. The registry now stores `Weak` references, prunes expired entries during each UI update and before registration, and upgrades only live flags during shutdown. This keeps cancellation effective for active workers without retaining completed-job state indefinitely.

Validation: `cargo fmt --all`, `git diff --check`, and local macOS Studio `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Long-session registry growth and shutdown cancellation were not runtime-exercised; no tests were run.

## Follow-up: PNG-sequence export workers were not joined during Studio shutdown

The PNG sequence exporter started a worker internally but returned no handle, so Studio could cancel it during exit without waiting for staging cleanup and final publication to finish. Added tracked variants that return the worker handle while preserving the existing fire-and-forget API, then connected the Studio export path to the app's background-worker registry. Shutdown now joins this worker under the same recovery-marker policy as other tracked UI jobs.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features cli --bin kagari` passed. Shutdown during active PNG sequence export was not runtime-exercised; no tests were run.

## Follow-up: PNG overwrite mode could replace a file changed during rendering

Overwrite mode only remembered which frame numbers existed when rendering began. During a long sequence render, another process could replace one of those paths; publication would then move the new file into recovery storage and replace it with Kagari's output. The exporter now snapshots identities of the exact regular files selected for overwrite, rejects symlinks and other file types, verifies each file again before moving it, and verifies the moved recovery copy before proceeding. A frame that changes during rendering is left untouched and the sequence rolls back its already published frames. Frame discovery is also scoped to the requested filename stem.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features cli --bin kagari` passed. Concurrent replacement behavior was not runtime-exercised; no tests were run.

## Follow-up: PNG rollback could delete a recovery copy after a failed emergency restore

A final transaction-path review found that if a frame changed between identity verification and rename, and the immediate attempt to move the unexpected file back also failed, the recovery copy remained inside the staging directory. The normal cleanup guard could then delete that directory even though the error said recovery data was retained. This failure path now marks the staging directory for preservation. The filename scanner also now accepts only the exact canonical `{stem}_{frame:04}.png` pattern, avoiding false matches from similarly named or differently cased files.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features cli --bin kagari` passed. Emergency restore failure and filename edge cases were not runtime-exercised; no tests were run.

## Follow-up: DMG collision cleanup could delete a pre-existing temporary file

The packager derived a temporary DMG path from the process ID, checked whether it existed, and then installed a cleanup trap that unconditionally removed that path. If a collision was detected, the exit trap could delete the file it had just reported as pre-existing. Packaging now creates a unique temporary directory in the final DMG's parent and builds the image inside it; cleanup only removes that script-created directory. Keeping it beside the destination also guarantees that final `os.replace` stays on the same filesystem. The input binary is now required to be an executable regular file, preventing a directory from passing the executable-bit check.

Validation: `bash -n scripts/package_macos.sh bundle_dmg.sh`, `git diff --check`, and local `diskutil image create from --help` confirmed the packaging command's accepted syntax and options. Full DMG creation was not run because no release binary is present; signing and notarization were not exercised.

## AmeUI handoff: REQ-352 blocks clean Hosted CI

Rechecked the canonical AmeUI tracker: P0 REQ-352 remains open. Kagari's `Cargo.toml` still points `ame-ui` at `../AmeUI`, while every hosted workflow checks out only Kagari, so a clean runner cannot resolve the dependency before compilation. The request already contains reproduction evidence, expected behavior, and four acceptance conditions. Sent the AmeUI owner task the exact request and asked it to own the fixed distribution route plus Linux CLI, Windows Studio, and macOS Studio clean-checkout builds. The owner is evaluating crates.io as a public fixed-version route and found that the current crate package omits an Android starter template needed by `amec`; package validation is in progress. No distribution contract or hosted CI success is confirmed yet. Track completion in `/Users/dev/Desktop/AI-SLOP-Family/AmeUI/リクエスト.md` under REQ-352; Kagari must keep its CI gates marked blocked until hosted logs pass.

## Follow-up: AmeUI Studio exports detached workers at shutdown

The newer AmeUI-backed Studio uses its own export workers instead of the legacy app's tracked-worker registry. GIF, PNG/EXR sequence, video, and still-PNG actions retained only their channels and cancellation flags, so closing Studio could let workers continue rendering, publishing output, or cleaning up audio/temp files after the app had exited. Added join handles for all five export paths, periodic joining after completion, and shutdown cancellation followed by joins. Export controls remain busy until the worker has actually exited, preventing a completed event from replacing a still-joining handle. Still PNG now checks cancellation around frame rendering, and sequence exporters also check after each frame write so cancellation on the final frame rolls back the partial sequence.

Validation: `cargo fmt --all`, `git diff --check`, and local `cargo check --no-default-features --features ame --bin kagari-studio` passed. Active-export shutdown, cancellation during a final frame, and worker panic reporting were not runtime-exercised; no tests were run.

## Follow-up: media import and proxy workers were detached at shutdown

AmeUI Studio retained cancellation flags and result channels for video/image-sequence imports and proxy generation, but dropped each thread handle immediately. Closing the app could therefore leave FFmpeg extraction or proxy writes running without a way to wait for staging cleanup. These workers now retain named join handles; normal result handling joins them, and shutdown requests cancellation before joining each active worker. Thread creation errors are surfaced in the existing status UI.

Validation: `cargo fmt --all`, `git diff --check`, and local `cargo check --no-default-features --features ame --bin kagari-studio` passed. Shutdown during active media import/proxy generation and cancellation latency were not runtime-exercised; no tests were run.

## Follow-up: completed autosave workers remained unjoined during long sessions

The async autosave path only joined its thread during application shutdown. Once the UI consumed a completed result it cleared the receiver but kept the join handle until another save or shutdown; repeated saves could replace and drop already-finished handles. The manager now joins and clears the handle as soon as it receives a result or detects a disconnected worker, while leaving an active worker untouched when polling reports `Empty`.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Long-session OS thread resource behavior and worker panic paths were not runtime-exercised; no tests were run.

## Follow-up: Studio joined export relays but not the FFmpeg workers

The earlier export shutdown change tracked Studio's event-relay worker. Video and GIF core APIs spawned a second FFmpeg worker and returned `()`, so the relay could forward a terminal event and exit before the FFmpeg worker finished dropping its temporary-output guards. Added tracked video/GIF API variants that return the actual FFmpeg thread handle; Studio now joins that worker before its relay exits and before video-export audio WAV cleanup. Worker panics are reported through the Studio export event channel. Existing fire-and-forget APIs remain available and delegate to the tracked variants.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `cargo check --no-default-features --features cli --lib` passed. Shutdown during active FFmpeg export and panic reporting were not runtime-exercised; no tests were run.

## Follow-up: video-export audio WAV cleanup was not panic-safe

The video export relay removed its temporary mixed WAV on normal completion, cancellation, and FFmpeg startup failure, but a panic after mixing and before those explicit cleanup branches could leave the WAV in the system temp directory. Added an RAII guard immediately after mixing so every exit path attempts removal, including unwinding; cleanup failures other than an already-missing file are logged with the path.

Validation: `cargo fmt --all`, `git diff --check`, and local `cargo check --no-default-features --features ame --bin kagari-studio` passed. Panic-time cleanup and filesystem removal failures were not runtime-exercised; no tests were run.

## Follow-up: temporary FFmpeg output cleanup failures were silent

Reviewed GIF's two-pass cancellation and panic paths and confirmed the output and palette files already have RAII guards. The shared `TemporaryOutputGuard`, however, ignored removal failures, leaving failed/cancelled exports capable of stranding large temporary media with no diagnostic. It now warns with the exact path for cleanup errors while treating an already-missing file as normal (for example, after atomic publication).

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `cargo check --no-default-features --features cli --lib` passed. A forced filesystem removal failure was not runtime-exercised; no tests were run.

## Follow-up: clearing a restored legacy autosave could delete other process sessions

`clear_recovery` inferred a namespace from the restored filename and removed every JSON filename beginning with `{namespace}_`. For the legacy `recovery_0.json` format this namespace is just `recovery`, so a clean save after restoring an old snapshot could also delete active/newer `recovery_{pid}_{slot}.json` data from other sessions. Cleanup now requires the filename to match the exact inferred namespace plus a numeric slot and `.json` suffix; legacy cleanup therefore only targets `recovery_{number}.json`, while process-scoped cleanup remains confined to that process namespace.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Cross-session deletion was not exercised at runtime; no tests were run.

## Follow-up: autosave synced snapshot contents but not the published directory entry

The recovery writer flushed and `sync_all`'d the staged snapshot before atomic replacement, but did not sync the containing directory afterward. On Unix filesystems, a sudden power loss could therefore lose the rename even though the autosave path reported success. The writer now syncs the recovery directory after publishing on Unix; a sync failure is surfaced as an autosave error so the dirty snapshot remains eligible for retry. Windows replacement durability remains governed by its platform move semantics and is not strengthened by this change.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed on macOS. Power-loss durability and non-Unix behavior were not runtime-exercised; no tests were run.

## Follow-up: FFprobe checks could make video-import cancellation hang

Video import ran FFmpeg/FFprobe availability, duration, dimensions, and audio-stream probes with blocking `Command::output()`. A stalled or malformed-media probe ignored the cancellation flag, so Studio shutdown could block forever when joining the import worker. Video import now runs each small-output probe as a child process polled for completion, kills and reaps it when cancellation is requested, and uses that same cancellable path for the FFmpeg availability check. Probe errors preserve the existing user-facing diagnostics, with cancellation included in the error detail.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, and `cargo check --no-default-features --features ame --bin kagari-studio` passed. A deliberately stalled FFprobe cancellation was not runtime-exercised; no tests were run.

## FFprobe probe-output pipe follow-up

A second pass over the cancellable probe runner found it waited for FFprobe before reading stdout. A process whose output exceeded the OS pipe buffer could block before exit, defeating cancellation and shutdown joins. Probe stdout is now drained concurrently by a named reader, retained only up to 64 KiB while excess bytes are still discarded, and the reader is joined after normal exit or cancellation. This bounds memory and avoids pipe backpressure deadlocks.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, and `cargo check --no-default-features --features ame --bin kagari-studio` passed. High-volume probe output and stalled-child cancellation were not runtime-exercised; no tests were run.

## Follow-up: FFmpeg preflights still ran synchronously in editor UI callbacks

Although video import's worker probes had become cancellable, AmeUI Studio and legacy EGUI still called `ffmpeg_available()` before starting those workers. Video/GIF export UI handlers also called `is_ffmpeg_available()` synchronously, and the legacy export runner checked it on the UI thread. Removed these UI-thread preflights so the worker reports missing/unusable FFmpeg without freezing the editor. The legacy export runner now uses the tracked FFmpeg worker API and joins the actual encoder before cleanup; its audio mix also observes the export cancel flag, including during app shutdown.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features ame --bin kagari-studio` passed. No-FFmpeg UI responsiveness, stalled executable behavior, and cancellation during legacy audio mixing were not runtime-exercised; no tests were run.

## FFmpeg export API preflight follow-up

After removing editor-side availability calls, a final call-chain review found the core video/GIF API still ran a blocking `-version` subprocess before spawning its encoder thread. The API now reports a missing or unusable executable from the actual encoder launch event instead. This keeps the entire editor call chain nonblocking and lets the existing tracked worker finish through its normal error event path. CLI commands retain their explicit command-line availability checks.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features ame --bin kagari-studio` passed. Missing-FFmpeg event reporting and stalled executable behavior were not runtime-exercised; no tests were run.

## Linux build matrix after FFmpeg responsiveness changes

Copied the current dirty Kagari source (excluding `.git`, `target/`, and the large promo video) into an isolated Linux checkout with the available sibling AmeUI tree. Linux `cargo check` passed for `--no-default-features --features cli --lib`, `--no-default-features --features gui --bin kagari-studio-egui`, and `--no-default-features --features ame --bin kagari-studio`. The temporary checkout and its build outputs were removed afterward. No tests were run.

## Read-only audit of the existing macOS DMG artifact

The release binary is absent from the current checkout, but a pre-existing `Kagari-Studio-macOS.dmg` is present. Verified its image checksum with `hdiutil verify`, attached it read-only, and inspected the mounted app: bundle identifier `org.kagari.studio`, version `0.3.0`, executable `Kagari Studio`, executable permission set, and `/Applications` shortcut present. The image was detached after inspection and the artifact was left unchanged. This confirms the artifact's internal layout, not signing/notarization or that it was produced by the current script revision.

## Follow-up: OpenFX property-set pointers could move when the owner vector grew

`LoadedOfxPlugin::new_property_set` returned a raw pointer into `Vec<OfxPropertySet>` while documenting that it remains valid for the plugin lifetime. A later `push` could reallocate that vector and invalidate pointers already handed across the C ABI. Property sets are now individually boxed; moving the boxes in the vector does not move the pointed-to storage, and the owner still frees all sets on drop. OpenFX execution remains disabled until the host suites are complete.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Pointer stability under multiple allocations was not runtime-exercised; no tests were run.

## Follow-up: OpenFX property callbacks dereferenced unvalidated plugin handles

The property suite converted every non-null C handle directly into `&mut OfxPropertySet`. A plugin passing a stale or arbitrary handle could trigger undefined behavior before the suite could report a bad-handle status. Added a registry of host-created property-set addresses; callbacks validate membership and mutate under the registry lock, while `LoadedOfxPlugin::drop` unregisters each set before its Box is freed. This lock also prevents concurrent callback access from racing deallocation. The existing suite test fixture now registers its host-owned set explicitly. OpenFX remains disabled pending the full suite implementation.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Tests were not run; invalid-handle behavior and concurrent callback/unload were not runtime-exercised.

## Follow-up: OpenFX property indices could trigger oversized allocations

The C ABI property setters previously converted negative indices to zero and accepted any positive `i32` as a vector position. A malformed or hostile plugin could request a multi-billion-element resize and terminate the host through allocation failure. Setters now reject negative indices and cap accepted indices at 4095 before touching property storage. Also corrected the test module's imports for the crate-private handle registration helpers so the test target can resolve them when compiled.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Tests were not run; the upper-bound behavior was not runtime-exercised. The 4095 index cap is a defensive host limit and should be checked against the intended OFX compatibility profile before enabling third-party plugin execution.

## Follow-up: AE plugin dispatch trusted caller buffer dimensions

`AePluginHostContext::render_frame` passed dimensions and `width * 4` row bytes to a native entry point without validating them against the supplied slice. Invalid or overflowing dimensions could create inconsistent world metadata and let a plugin write outside the caller's buffer. The host now rejects non-positive dimensions/frame rates, checked-multiplies row bytes and required RGBA storage, and refuses undersized buffers before calling the entry point. This protects host-generated requests; an arbitrary in-process native plugin can still violate the ABI and corrupt its own process.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Tests were not run; rejection cases were not runtime-exercised.

## Follow-up: OpenFX status values did not match the official ABI

The bridge labeled `1` as `ReplyYes` and `2` as `ReplyDefault`, although the official OpenFX `ofxCore.h` defines those as `12` and `14`; `1` and `2` are generic failure and fatal error. The property callbacks also returned generic failure for invalid handles and indices. Corrected the status constants and now return `ErrBadHandle`, `ErrBadIndex`, or `ErrValue` for those input errors. Separately removed unsound unconditional `Send`/`Sync` impls from `PF_EffectWorld`, whose mutable pixel pointer must remain thread-confined. OpenFX loading/execution is still disabled until the complete host suites are implemented.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Tests were not run. Official ABI reference: [AcademySoftwareFoundation OpenFX `ofxCore.h`](https://github.com/AcademySoftwareFoundation/openfx/blob/main/include/ofxCore.h).

## Follow-up: AE pixel format enum could be invalidated by C plugin writes

`PF_EffectWorld` exposes a mutable C-ABI pointer to a plugin. Its `pixel_format` field used a Rust enum, whose valid-discriminant requirement does not match an arbitrary integer written by foreign code. Replaced it with an ABI-compatible `repr(transparent)` i32 newtype and named constants, preserving the four-byte C enum layout while making unknown values representable without violating Rust enum validity.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Tests were not run.

## Follow-up: particle frame catch-up ignored render cancellation

Particle cache misses and reverse seeks advance deterministically from the nearest checkpoint, but the frame-by-frame catch-up loop did not inspect the renderer's cooperative cancellation flag. A deadline or export cancel could therefore remain stuck inside particle simulation until a distant remapped source frame finished. The simulation now checks cancellation every 16 source frames, reports incomplete advancement to the cache path, and exits the particle layer before rasterizing or publishing partial simulation state as the requested frame.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Tests were not run; mid-simulation cancellation was not runtime-exercised.

## Follow-up: non-finite time-remap values leaked into source-time consumers

`remap_frame_f32` clamped negative values but allowed NaN and infinities through. Those values feed media sampling, nested composition time, and audio-rate calculations; positive infinity also saturated integer conversions to the largest frame and could make particle catch-up effectively unbounded. Non-finite evaluations now fall back to the current timeline frame (identity mapping), while finite negative values retain the existing clamp to frame zero.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features cli --lib` passed. Tests were not run; malformed non-finite keyframe values were not runtime-exercised.

## Follow-up: video playback speed was interpreted in separate render and audio paths

Software rendering, GPU preview, track mattes, nested precomps, media previews, timeline waveforms, and extracted video audio each applied video speed independently. Non-finite speed could therefore produce different source frames in the picture and sound. Added a shared finite, non-negative source-frame scaling helper and routed those call sites through it; video audio now derives its sampling rate from the same scaled source-frame delta used by video playback.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Tests were not run; A/V sync under reverse, fractional remap, and invalid speed was not runtime-exercised. Both checks report an existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: GPU preview ignored active video frame blending

The GPU video render pass selected one floored sequence frame, while the CPU path blends adjacent source frames for fractional positions. The shared GPU-to-CPU capability check did not account for that feature, so the legacy GPU viewport could silently ignore enabled frame blending. It now requests CPU rendering only when the active video frame actually has a fractional scaled source position and a following frame exists; the diagnostic reports why. Integer-frame playback keeps the GPU path.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features cli --lib` passed. Tests were not run; fractional playback fallback was not runtime-exercised. GUI check reports the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: nested GPU precomp cache tracked the wrong media frame

`composition_media_revision` hashed `frame - layer.in_frame`, but actual video rendering selects frames from the layer's effective time remap, posterize-time result, and playback speed. Its key also omitted the adjacent sequence frame used for fractional frame blending. A changed source frame could therefore leave a stale nested precomp texture cached. The revision now hashes the same one or two sequence files the CPU render samples, recurses into nested precomps at their effective source frame, and removes a composition from the cycle guard after each traversal so repeated instances are each evaluated.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features ame --bin kagari-studio` passed. Tests were not run; media replacement inside nested precomps was not runtime-exercised. Both checks report the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: top-level paint strokes were omitted from rendering

Paint strokes were applied by the precomp CPU renderer, but the main top-level software renderer never called that path. The GPU renderer also has no paint-stroke implementation, and the capability check did not force CPU fallback. Top-level CPU rendering now composites paint strokes after base rasterization and before layer effects. The same is now done for full-frame particle layers and parent precomp layers, both of which return before the generic layer path. The GPU capability check routes layers with paint strokes through the CPU renderer.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features ame --bin kagari-studio`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Tests were not run; paint rendering in top-level, particle, and parent-precomp compositions was not runtime-exercised. Both checks report the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: paint strokes did not follow non-uniform layer transforms

Reviewing the new paint call sites exposed a shared transform defect: stroke points were scaled after rotation, which is wrong for non-uniformly scaled, rotated layers. The visibility bounds also transformed only opposite rectangle corners and could miss the actual bounds of a rotated stroke. Layer-local points are now scaled per axis before rotation, stroke bounds are calculated from transformed points, and brush size follows the layer's largest scale axis.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features ame --bin kagari-studio` passed. Tests were not run; no runtime render comparison was performed. Both checks report the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: preview mode leaked across panic unwinding

`render_frame_to_pixels_preview` manually restored its thread-local preview flag only after rendering returned normally. If a render panicked and its worker caught the unwind, later non-preview renders on that worker could continue using proxy media. A drop guard now restores the previous flag during both normal return and unwinding, and preserves nested preview state.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features ame --bin kagari-studio` passed. Tests were not run; panic recovery behavior was not runtime-exercised. Both checks report the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: failed software previews reached the GUI image constructor

The legacy viewer passed renderer output directly to `ColorImage::from_rgba_unmultiplied`. The renderer intentionally returns an empty vector for invalid or over-limit dimensions and allocation failure, but the GUI image constructor requires exactly width × height × 4 bytes. The viewer now checks for a nonzero expected byte count and an exact buffer length, then paints a black fallback frame instead of passing malformed data to egui.

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Tests were not run; invalid-dimension UI behavior was not runtime-exercised.

## Follow-up: stale EXR staging files blocked future exports

EXR staging names use process ID plus an in-process counter. After a crash, a later process can reuse the same ID and encounter a stale staging file; the writer previously failed immediately. Both EXR file publishing paths now reserve a unique staging file with `create_new` and retry collisions up to 100 candidates, matching the collision-safe behavior used by other export paths.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features gui --bin kagari-studio-egui`, and `cargo check --no-default-features --features cli --lib` passed. Tests were not run; stale-file collision and exhaustion cases were not runtime-exercised. GUI check reports the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: EXR chunk sizes could overflow OpenEXR's signed size fields

The writer cast compressed chunk lengths and attribute payload lengths directly to `i32`, and scanline packing used unchecked capacity arithmetic. Large but otherwise structurally valid multi-channel images could therefore produce invalid size fields or panic during allocation. The writer now preflights maximum raw chunk size, checks compressed/attribute lengths before serialization, streams the zeroed offset table instead of allocating a second copy, and uses fallible reservations for chunk offsets and scanline data.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Tests were not run; overflow and allocation-failure cases were not runtime-exercised. GUI check reports the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: EXR staging-link cleanup failures were silent

The no-replace EXR writer publishes the completed file with a hard link, then removes its temporary staging link. A failed removal left a hidden extra file while the function returned success without any diagnostic. Successful publication remains successful, but cleanup failures other than `NotFound` are now logged with both output and staging paths so disk leaks can be traced without misreporting the output as failed.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Cleanup failure was not runtime-induced. GUI check reports the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: image import could approach twice its documented memory limit

Image decoding used `image::open` with its default 512 MiB decoder allocation limit, then kept the decoded image alive while allocating up to another 512 MiB of float channel planes. This allowed a single import to hold roughly 1 GiB before conversion overhead. Import now applies explicit decoder allocation and 16,384-pixel axis limits, caps decoder memory at 256 MiB, and caps converted channel planes at 256 MiB, bounding those two image buffers to about 512 MiB combined. Both EXR and general image import use the same limited reader.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Oversized/corrupt image behavior was not runtime-exercised. GUI check reports the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: frame-cache manifest staging names also failed on stale collisions

The frame-cache manifest writer had the same process-ID-plus-counter staging-name pattern as the EXR writer, but returned immediately when a stale file occupied the generated name. It now retries up to 100 `create_new` candidates and reports a clear collision-exhaustion error. Manifest replacement remains atomic and continues replacing the cache-local manifest path itself.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Stale manifest collision was not runtime-induced. GUI check reports the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: WAV decoding retained the whole source file while allocating PCM

The WAV path buffered up to 512 MiB of source bytes, then allocated up to another 512 MiB of decoded float samples. It now parses RIFF chunk headers with seeks, reads only the small `fmt` prefix needed for validation, and decodes the validated `data` chunk in frame-aligned 64 KiB windows. The existing 512 MiB source and decoded-sample limits remain, while peak input buffering drops from the full file to at most one decode window.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. PCM formats, multichannel downmix, malformed chunks, and short reads were not runtime-exercised. GUI check reports the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: renderer image cache bypassed bounded import decoding

The main raster renderer decodes through `ImageCache`, separate from `exr_reader`; that path inspected dimensions first but then used unrestricted `image::open` and could allocate the decoded source plus a second RGBA bitmap. It now applies strict 16,384-pixel axis limits and a 512 MiB decoder limit, then rejects RGBA conversions over 512 MiB or the configured cache budget. This covers ordinary image and video-frame raster decoding used during preview and export.

Validation: `cargo fmt --all`, `git diff --check`, `cargo check --no-default-features --features cli --lib`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Oversized raster imports and decoder allocation failures were not runtime-exercised. GUI check reports the existing future-incompatibility notice for dependency `block v0.1.6`.

## Follow-up: roto propagation included the exclusive out-point frame

Layer activity uses `frame < out_frame`, but both tracker-based and optical-flow roto propagation passed `out_frame` as an inclusive bake endpoint. This created an extra matte keyframe after the layer stopped rendering, which could unexpectedly become visible if the layer was later extended. Both UI workflows now stop at `out_frame - 1` (clamped to `in_frame`).

Validation: `cargo fmt --all`, `git diff --check`, and `cargo check --no-default-features --features gui --bin kagari-studio-egui` passed. Boundary behavior was not runtime-exercised. The GUI check reports the existing future-incompatibility notice for dependency `block v0.1.6`.
# Screen blend equation did not match the CPU renderer

The WGPU Screen pipeline used `One + OneMinusSrc` after the fragment shader premultiplied source RGB by source alpha. That computes `S_premul + D * (1 - S_premul)`, which is not Screen when the source has partial opacity and caused the recorded GPU/CPU mismatch. The correct premultiplied Screen equation is `D + S_premul * (1 - D)`. The pipeline now uses `OneMinusDst + One`; alpha remains standard source-over. Static inspection confirms the shader still premultiplies RGB before this pipeline. Hardware pixel parity is not yet reverified in this change.

## Follow-up: negative image scale and GPU texture sampling diverged

The software rasterizer computed UVs without applying the sign of layer scale, so negative Scale mirrored GPU quads but left CPU-rendered image/video frames and nested precomp textures unmirrored. The GPU quad UV assignment also disagreed with the top-left-origin image convention under the composition projection. The software paths now mirror U/V for signed scales, and the WGPU quad maps source rows upright. Image sampling uses pixel centers in top-level and nested texture paths so the software reference selects the same texels as GPU rasterization at boundaries.

Added a full-frame GPU/software regression using a four-color image, 15× upscaling, and negative X scale. Expanding the image also exposed that WGPU uses linear texture filtering while the software renderer chose the nearest texel. Added a shared RGBA8 bilinear sampler matching normalized WGPU texel-center coordinates and ClampToEdge behavior, and applied it to top-level images/video frames plus nested texture and precomp sampling. Validation: on the Linux WGPU builder, the scaled negative-scale parity case passed and the complete `gpu_preview_parity` integration suite passed (3 tests). `cargo fmt --all --check` passed in the Linux source snapshot; `cargo fmt --all --check` and `git diff --check` passed locally before this sampler follow-up. Regression coverage exercises scaled image layers; mask and 3D transform parity still need separate coverage.

## Follow-up: GPU video preview dropped frame blending

The WGPU path floored the remapped source position and bound only that frame, while the software renderer blended adjacent frames when `Layer::frame_blending` was enabled. The texture bind group and shader now sample both frames and interpolate by the fractional source position. Frame pairs are cached together, with both texture allocations included in the existing GPU frame-cache byte budget. Proxy playback and non-video texture layers keep blending disabled.

Added a full-frame GPU/software regression with distinct red and green frames, a 0.5-frame time remap, and frame blending enabled. Validation: targeted case passed and the complete Linux `gpu_preview_parity` suite passed (4 tests); `cargo fmt --all` passed on the Linux source snapshot, and local `cargo fmt --check` plus `git diff --check` passed.

## Follow-up: software text rasterization ignored layer transforms

Normal text rasterization placed glyphs directly in composition coordinates and ignored signed layer Scale and rotation. It now inverse-maps output pixels into the text bitmap using the layer transform, positions the bitmap using its rasterized dimensions for AE-style anchor behavior, and applies the same linear sampling as GPU text textures. Stroked text uses the renderer's shared stroke-baking routine before transformed sampling.

The negative-X-scale text parity test first reproduced a 247.35/255 maximum channel mismatch, then passed after the transform fix. The final test also uses 13° rotation and a visible stroke. Its tolerance is 3/255 because the software reference stores intermediate coverage in RGBA8 while the GPU target is RGBA16F; the measured rotated glyph-edge maximum was 2.414/255. The final Linux `gpu_preview_parity` suite passed all 5 tests. Text-on-path, masks attached to transformed layers, and 3D image planes remain outside this regression's coverage.

## Follow-up: CPU 3D image raster used the unrelated 2D layer center

The software renderer projected 3D layer corners to find raster bounds but centered the raster using the 2D transform. With a default 2D transform, a 3D image at the camera center was silently clipped at the composition origin. The CPU raster now centers on the projected bounds. GPU rendering had a separate root defect: the 3D matrix multiplication used row-major math while the shader consumes column-major matrices, so 3D layers were clipped. The GPU matrix math now follows the renderer's column-major convention, and the 3D projection now scales layer geometry from its source dimensions and normalizes X/Y/Z using composition dimensions. A regression confirms a 3D solid renders, and a 3D image is visible around camera center in both renderers.

The first full-frame comparison exposed a 159/255 difference: the CPU renderer applied the layer material's ambient/diffuse/specular lighting, while the GPU shader left the image unlit. The lighting evaluation is now shared between renderers, passed through the existing uniform padding, and applied in the shader before layer styles. The CPU image raster now inverts the GPU's 3D projective matrix per output pixel to recover perspective-correct UVs; its old axis-aligned UV mapping could not match a tilted plane. The full-frame parity fixture now uses a centered 64×64 image at depth 1000, rotated 18° around X and 27° around Y, with non-uniform 120% × 80% scale, and asserts a maximum error of 3/255. The 3D solid test also compares its center pixel against the CPU render.

The render view matrix also ignored the active camera's position and rotation, even though camera controls and point-projection helpers exposed both. The GPU transform now composes the inverse camera transform before the layer transform; the shared point-projection function uses the same pitch/yaw/roll convention. X, Y, and Z world units now use half the composition height as their common scale before aspect-ratio projection. The camera regression moves and yaws the camera, checks that a stationary layer shifts left, and verifies full-frame CPU/GPU parity both before and after the move.

Validation: `cargo fmt --check`, `git diff --check`, Linux `cargo test --no-default-features --features cli-gpu --lib projection_tests` (6 tests), and Linux `cargo test --no-default-features --features cli-gpu --test gpu_preview_parity` (all 8 tests) passed. Transformed masks, planes crossing the near plane, and non-square 3D camera parity remain unverified.

## Follow-up: camera navigation refreshes the interactive preview while dragging

Reviewed the AmeUI canvas camera-navigation path after the render-view fix. Orbit, pan, and dolly update the active camera on pointer movement and call `mark_project_changed`, which increments `preview_revision`. The composition preview request and result cache both include that revision, so an in-flight old frame is discarded and a render for the latest camera state is requested. Pointer release commits the drag history transaction. This was verified by tracing the event, revision, and preview-worker paths; GUI runtime behavior and perceived refresh latency were not exercised.

## Follow-up: 3D planes now clip consistently at the camera frustum

The software renderer previously discarded a projected quad if any corner was behind a clip plane. Fully invisible 3D layers then fell through to the 2D raster path and could appear at an unrelated composition-space position. The CPU projection now clips the quad in homogeneous coordinates against all six GPU clip planes, derives its raster bounds from surviving vertices, and skips a layer when no visible polygon remains. CPU 3D lighting also evaluates at the projected layer origin, matching the GPU uniform calculation when the clipped raster bounds become asymmetric.

Added unit coverage for a plane crossing the near plane, geometry fully behind the camera, and projected-point-to-UV round trips. Added full-frame GPU/software cases for a visible near-plane crossing with scene shadows enabled and a layer behind the camera. The near-plane case builds and uploads the same composition-space shadow map used by the editor preview.

Validation: local `cargo fmt --all --check` and `git diff --check` passed. On Linux, `cargo test --no-default-features --features cli-gpu --lib projective` passed all 3 projective tests, and `cargo test --no-default-features --features cli-gpu --test gpu_preview_parity` passed all 10 tests. The first full-frame run hit the remote temporary-disk quota while linking unrelated binary targets; after clearing only the temporary build directory and rebuilding with reduced debug output, the suite passed.

## Follow-up: GPU scene shadows sampled in layer space instead of composition space

The editor already builds the composition-space shadow map and uploads it before rendering in `src/legacy_egui/viewport/gpu_preview.rs`. The shader then sampled that map with `in.tex_coords`, which only describes the layer's source UVs; transformed and perspective-projected layers therefore read the wrong part of the shadow map. Shadow sampling now uses normalized fragment output coordinates (`in.clip_position.xy / globals.viewport_size`).

The near-plane image regression now uploads the CPU-built map before GPU rendering. With the old layer-UV sampling, the full-frame CPU/GPU comparison failed by 41.055/255 at pixel `(45, 0)`. With output-space sampling, the shadow-enabled case passes within 3/255. Validation: Linux `cargo test --no-default-features --features cli-gpu --test gpu_preview_parity` passed all 10 tests; local formatting and diff checks passed.

Remaining coverage: translated 2D receivers, multiple shadow casters, reduced-resolution preview, and visual GUI refresh/latency. This is a Kagari shader issue; the existing AmeUI request path is not involved.

## Follow-up: scene shadows used the layer's 2D transform for 3D casters

`build_shadow_map` projected 3D casters using `resolve_world_transform`, which evaluates the independent 2D transform. The renderer draws those planes from `transform_3d`, so changing the 3D position, scale, or Euler rotation could leave the shadow behind or give it the wrong shape. Shadow geometry now transforms each caster outline point with the animated 3D transform and the same Z*Y*X rotation convention as the 3D composition transform. Shadow fixtures now set/animate their 3D scale and position explicitly, and a regression deliberately separates the 2D and 3D transforms.

Validation: the two `shadow_map_*` unit cases passed, including the differing-transform regression. Linux `cargo test --no-default-features --features cli-gpu --test gpu_preview_parity` passed all 10 tests. An initial wider `--lib shadow_tests` run had 11 failures; investigation traced them to test fixtures that positioned full-composition layers as if `position` were their center while leaving `anchor_point` at `[0,0]`. The fixtures now set their intended center anchors explicitly, after which all 32 filtered tests passed. Scene shadows are still only projected onto a flat z=0 receiver, so arbitrary receiver geometry and multiple caster interactions need more coverage.

## Follow-up: software-renderer fixtures assumed centered anchors

Several render tests placed full-frame layers at the composition center but left their anchor at the origin. Once the renderer applied the AE-style anchor offset, those fixtures covered only part of the frame and sampled pixels outside the layer; this looked like failures in Add blending, masks, adjustment effects, ACES output, shadows, and shape sizing. Updated the fixtures to set center anchors wherever their expected geometry is centered. This is test setup correction; the renderer behavior was not changed by this follow-up.

Validation: Linux `cargo test --no-default-features --features cli-gpu --lib shadow_tests -- --test-threads=1` passed all 32 tests. The GPU/software full-frame parity suite had already passed all 10 tests after the 3D caster-shadow fix. Local `cargo fmt --all -- --check` and `git diff --check` passed.

## Follow-up: spot-light shadows ignored the cone and receiver distance

The shadow builder applied one attenuation value to an entire polygon directly below the light and never called the existing spot-cone helper. As a result, spot lights cast shadows outside their cone, and distance falloff did not vary across the receiver plane. Density accumulation now evaluates distance attenuation and the spot cone at each shadow-map pixel before adding the light's contribution.

Added coverage that checks an in-cone shadow remains visible, a caster fully outside a narrow cone leaves no shadow, and overlapping casters deepen their shared shadow. Validation: Linux `cargo test --no-default-features --features cli-gpu --lib shadow_tests -- --test-threads=1` passed all 34 tests; the GPU/software full-frame parity suite passed all 10 tests after the shadow accumulation change; local `cargo fmt --all -- --check` and `git diff --check` passed. Parallel-light projection and physically accurate soft penumbra remained unimplemented at that point.

## Follow-up: parallel lights used point-position shadow projection

`LightType::Parallel` was projected through the light's position in the scene shadow map, even though the 3D renderer defines parallel-light rays as a fixed `[-1,-1,1]` surface-to-light direction. The shadow builder now traces the opposite `[1,1,-1]` direction to the z=0 receiver and ignores point/spot distance attenuation for parallel lights. Added a regression where moving the stored light position far away and setting a tiny point-light radius must not move or remove the parallel shadow.

Validation: Linux `cargo test --no-default-features --features cli-gpu --lib shadow_tests -- --test-threads=1` passed all 35 tests, including parallel projection, spot cone, overlapping casters, and the earlier 3D-transform case. `cargo test --no-default-features --features cli-gpu --test gpu_preview_parity` passed all 10 tests. Local `cargo fmt --all -- --check` and `git diff --check` passed. The UI still has no control for rotating the fixed parallel-light direction; physical penumbra behavior remains approximate.

## Follow-up: layer and mesh lighting disagreed on parallel and spot lights

The flat 3D-layer uniform path treated a parallel light as a point and applied distance falloff. The imported-mesh path also treated parallel lights as points, and omitted the spot cone test entirely. Both paths now use the same fixed parallel-light direction as the shadow pass; parallel lights ignore source position and point-distance attenuation. Mesh spot shading now uses the shared cone calculation, so surfaces outside the cone receive no direct contribution.

Added regressions for parallel-light position independence in both flat-layer and mesh shading, mesh spot-light falloff outside the cone, and full-frame GPU/software parity for a parallel-lit 3D layer. Validation: Linux `cargo test --no-default-features --features cli-gpu --lib lighting_tests` passed 2 new mesh-light tests; `cargo test --no-default-features --features cli-gpu --lib model3d_raster::tests` passed both OBJ raster tests; `cargo test --no-default-features --features cli-gpu --lib shadow_tests -- --test-threads=1` passed all 36 tests; `cargo test --no-default-features --features cli-gpu --test gpu_preview_parity` passed all 11 tests. Local formatting and diff checks passed. Ambient-light behavior was reviewed in the next follow-up; user-adjustable parallel direction remains open.

## Follow-up: ambient and mesh lighting ignored light placement, color, and material response

Flat 3D layers treated ambient lights as positional and directional. Imported 3D meshes also ignored light color and the material's specular response, and did not apply point-light falloff. Ambient contributions are now position independent and tinted by the light color. Mesh lighting accumulates per-channel ambient/diffuse/specular contributions, evaluates the material's specular exponent, and applies point/spot attenuation and cone coverage; parallel lights continue to use the shared fixed direction.

Added tests for colored, position-independent ambient light on flat layers and meshes, plus a full-frame GPU/software parity case for a red ambient light. Validation: Linux `cargo test --no-default-features --features cli-gpu --lib lighting_tests` passed all 3 mesh-light tests; `cargo test --no-default-features --features cli-gpu --lib model3d_raster::tests` passed both OBJ rendering tests; `cargo test --no-default-features --features cli-gpu --lib shadow_tests -- --test-threads=1` passed all 37 tests; `cargo test --no-default-features --features cli-gpu --test gpu_preview_parity` passed all 12 tests. Local formatting and diff checks passed. Mesh material metalness and shadow reception remained open at that point.

## Follow-up: the visible Metalness material control had no render effect

`MaterialOptions::metalness` is exposed in both layer inspectors but was unused by the render paths. Flat 3D layers now reduce ambient and diffuse response as metalness rises. Imported meshes use the same diffuse-energy reduction and blend dielectric specular reflectance toward the mesh base color for metals, producing colored metallic highlights.

Added tests proving that Metalness reduces flat-layer diffuse response and tints mesh specular reflection; the GPU parity fixture now exercises nonzero metalness on a 3D layer. Validation: Linux `cargo test --no-default-features --features cli-gpu --lib lighting_tests` passed all 4 mesh-light tests; `cargo test --no-default-features --features cli-gpu --lib model3d_raster::tests` passed both OBJ rendering tests; `cargo test --no-default-features --features cli-gpu --lib shadow_tests -- --test-threads=1` passed all 38 tests; `cargo test --no-default-features --features cli-gpu --test gpu_preview_parity` passed all 12 tests. Local formatting and diff checks passed. Metallic shadow reception and glTF material metalness import remain unverified.

## Follow-up: imported metallic/roughness values did not affect mesh shading

The glTF loader populated `PbrMaterial3D.metallic` and `.roughness`, but the per-triangle raster path passed only imported base color and emission into `shade`. Imported materials now supply metallic to the diffuse/specular energy split, use the glTF dielectric F0 of 0.04 before metallic tinting, and map roughness to the existing Blinn-Phong highlight exponent. OBJ materials use the same connected path. This is a roughness approximation in the software mesh renderer; it is not a microfacet BRDF, metallic-roughness textures remain unsupported, and GPU mesh shading remains unavailable.

Added a loader fixture that checks metallic/roughness factors survive glTF import, plus a shading regression proving those values change metallic highlight tint and off-axis highlight width. Validation: Linux `cargo test --offline --no-default-features --features cli-gpu --lib lighting_tests -- --test-threads=1` passed all 5 mesh-light tests; local `cargo fmt --all` and `git diff --check` passed. The `cli`-only test configuration cannot compile the existing library test suite because `text_raster` references the `wgpu`-gated renderer; the GPU-enabled CLI configuration compiled and ran the focused suite successfully. Import and shading are covered separately; a full glTF asset render fixture remains absent.

## Review finding: roto/paint support was counted from the optional legacy UI

The feature matrix described roto propagation and paint as available but did not identify that these tools are only connected in the optional legacy egui Studio. The shipped/default `kagari-studio` uses the AmeUI Studio; its toolbar actions and `kagari.viewport` pointer handler expose selection, hand, zoom, text, shape, and pen tools, but no roto/paint tool. The only bundled video fixture is `assets/library/av1_forest.webm` (73×67 px, 3 seconds), a landscape thumbnail that cannot validate production subject extraction. Updated the matrix to state the default-product gap and the actual acceptance workflow.

The AmeUI Canvas event contract currently distinguishes only Mouse and Touch and carries no pen pressure, tilt, or coalesced stylus samples. Registered REQ-353 in the AmeUI request tracker, with the API behavior, reproducible evidence, acceptance tests, and ownership split: AmeUI owns host input delivery; Kagari owns the roto/paint workflow and rendering. No AmeUI code or Kagari feature code was changed in this review slice. Validation: inspected the default `kagari-studio` entry point, toolbar/action definitions, Canvas event contract, and available bundled video; `git diff --check` passed. A production-footage roto journey remains unverified and is now correctly marked as not available in the primary product.

## Follow-up: Roto Brush source isolation was trapped in the legacy UI module

The default Studio will need the same isolated source-frame input as the legacy Roto Brush: transparent composition background, selected-layer masks removed during analysis, and track-matte inputs disabled. That preparation lived in `legacy_egui`, preventing the primary UI from using the shared behavior. Moved it to `core::roto_assist::prepare_roto_brush_source`; the optional legacy workflow now calls the core API, leaving room for primary Studio integration without duplicating matte-source semantics.

Validation: Linux `cargo test --offline --no-default-features --features cli-gpu --lib roto_source_composition_clears_only_input_mattes_and_background -- --test-threads=1` passed; Linux `cargo test --offline --no-default-features --features gui --lib roto_source_frame_isolated_from_other_layers_background_and_existing_masks -- --test-threads=1` passed. Local `cargo fmt --all -- --check` and `git diff --check` passed. Primary AmeUI Studio still has no Roto Brush interface or processing workflow.

## Follow-up: the default Studio had no Roto Brush path

Added the first primary-Studio Roto Brush workflow: toolbar tool and controls for foreground/background, brush radius, feather, and clearing; mouse stroke capture with bounded interpolation; persistent per-layer/per-frame stroke storage; visible stroke overlays; and background current-frame matte analysis for image, video, and precomp layers. Analysis runs off the UI thread, checks cancellation, project/composition identity, and a serialized source-layer snapshot before applying its result, then stores the generated contour as a feathered `Roto Brush Matte` mask keyframe. Undo/redo restores saved stroke and matte state. Shared source isolation and filtered rendering are exposed through the core API for this integration.

The feature matrix now records this as a partial primary workflow. At this point in the work, temporal propagation, correction review, pressure/tilt/coalesced stylus input (REQ-353), production-footage acceptance, caching, and preview/export parity remain gaps. Validation: local `cargo check --bin kagari-studio --features ame,wgpu` and `cargo fmt --all` passed; no tests or on-screen workflow were run, and disk space was reclaimed from the temporary Cargo incremental build cache after the initial check hit a full volume.

## Follow-up: temporal Roto Brush propagation was not available in the standard Studio

Connected the existing core bidirectional optical-flow Roto Brush bake to the default AmeUI Studio. The toolbar can propagate from the current anchor frame in both directions across the selected layer's visible range and cancel the background operation. The worker uses the isolated source-layer renderer, checks cancellation between frames, and applies baked mask animation only when the project, composition, and full source-layer snapshot still match. The generated animation is committed as one project-history change, so it can be undone/redone.

Updated the feature matrix: propagation is now implemented but still needs multi-frame correction acceptance on real footage, plus preview/export comparison, cache behavior, and stylus input. Validation: `cargo check --bin kagari-studio --features ame,wgpu`, `cargo fmt --all -- --check`, and `git diff --check` passed. Launching the Studio parsed the updated `.ame` UI and remained running; visual inspection and propagation on a real clip were not performed, and no tests were run.

## Review follow-up: roto results could outlive composition settings

The asynchronous Roto workers guarded the project generation, composition ID, and source-layer snapshot, but composition settings can change without replacing the project or touching the layer. A current-frame result could therefore return a contour traced at the old resolution, and a propagation bake could return keys from an obsolete frame rate or duration. Both completion payloads now capture composition width, height, FPS, and duration, and discard results if any of those settings changed before application.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib optical_flow_propagation_ -- --test-threads=1` passed both motion and occlusion-correction tests. After syncing and refreshing the scratch build inputs, remote `cargo check --offline --bin kagari-studio --features ame,wgpu` passed. Local `cargo fmt --all -- --check` and `git diff --check` passed. No real-footage propagation or visible UI inspection was performed.

## Review follow-up: long Roto jobs could lose their cancel control

The cancel button was inside the Roto-only toolbar panel. Switching to another tool hid that panel while the optical-flow worker kept running. Moved cancellation into the shared toolbar so it stays reachable during processing, and invalidate/cancel workers plus any live stroke immediately when entering new-project setup or installing another project. Invalidation drops the receiver so a new project's Roto work is not blocked by a stale job; the UI status explains the discard.

Validation: local `cargo check --bin kagari-studio --features ame,wgpu`, `.ame` Studio startup parsing, `cargo fmt --all -- --check`, and `git diff --check` passed. The earlier two focused optical-flow tests passed on Linux. Visual interaction with the running window remains unverified because the Mac session is locked.

## Review follow-up: live roto strokes could cross composition resize

A stroke kept composition identity and project generation, but not its coordinate-space dimensions. If the user resized the composition while holding the pointer, subsequent samples could be clipped against the new dimensions and the completed stroke could be saved against a different coordinate space. Live strokes now retain their starting width/height, stop accepting samples after a settings change, and are discarded at pointer-up if the coordinate space changed.

Validation: local `cargo check --bin kagari-studio --features ame,wgpu`, `cargo fmt --all -- --check`, and `git diff --check` passed. Pointer-level resize behavior was not exercised in the locked desktop session.

## Validation follow-up: primary Roto pointer and history path

Added AmeUI Studio regression coverage that sends mouse down/move/up events through `kagari.viewport`, confirms the per-frame stroke points persist, and verifies Undo/Redo remove and restore them. A second test changes composition width during an active stroke and verifies the stroke is discarded. The first test build also exposed an existing stale `draw_timeline` call in the Studio test harness; updated it to pass the current vertical-scroll argument.

Validation: remote Linux `cargo test --offline --no-default-features --features ame,wgpu --bin kagari-studio roto_brush_ -- --test-threads=1` passed both tests. Local formatting and `git diff --check` passed. This covers pointer-to-history behavior, not matte quality on footage.

## Review finding: Roto Brush selected user masks by display name

Both Studio paths located, overwrote, or deleted the generated roto mask using the display name `Roto Brush Matte`. A regular user mask with the same name could be treated as generated output, causing segmentation to replace it or Clear to delete it. Added the stable `ROTO_BRUSH_MATTE_ID` owner key and changed the default and legacy paths to select/remove generated output by ID.

Added a default Studio regression test with a user mask and generated mask sharing the display name; Clear must remove only the generated ID. Validation: remote Linux `cargo test --offline --no-default-features --features ame,wgpu --bin kagari-studio roto_ -- --test-threads=1` passed all 3 Roto UI tests, and `cargo check --offline --no-default-features --features gui --bin kagari-studio-egui` passed for legacy compatibility. Local formatting and `git diff --check` passed.

## Review finding: GPU effect-order slots disagreed with shader execution

The GPU capability checker used a fixed execution-order table to decide whether an effect stack could stay on the GPU. It assigned Lens Flare after Color Tint, but Lens Flare actually runs inside `sample_layer_color`, before Color Tint and Levels. A user-authored Tint→Flare stack therefore passed the monotonic-order check even though the shader rendered Flare→Tint, diverging from CPU/export output. Reordered the slots to match the fragment shader and added a capability regression proving Flare→Tint stays GPU-capable while Tint→Flare falls back with an ordering reason.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib gpu_preview_falls_back_when_lens_flare_stack_order_differs_from_shader -- --test-threads=1` passed. Extended the real GPU readback regression to check each prefix of Lens Flare → Color Tint → Levels → Invert with a fresh renderer; the full 12-test `gpu_preview_parity` integration suite then passed. The original single-render version had one observed 2.072/255 mismatch at pixel (28, 23), which has not recurred in the extended harness; its cause is not established, so broad GPU/software parity remains unproven. Local `cargo fmt --all -- --check` and `git diff --check` passed. These fixtures do not establish parity on production footage or across all effect parameter ranges.

## Review finding: Roto matte cancellation did not interrupt CPU analysis

The Studio set a cancellation flag, but current-frame matte generation only checked it before and after a full segmentation pass. Large frames could therefore keep the worker busy after the user cancelled. Added a cancellable segmentation API and checkpoints while collecting stroke samples, building both distance fields, classifying image rows, feathering, and applying hard stroke constraints; the Studio now passes the worker cancellation token through those stages and checks it around contour tracing. A regression cancels during the distance-field pass and verifies immediate `None` completion.

The imported-footage workflow fixture also had invalid coordinates: two distant background marks were encoded as one continuous stroke through the subject, and the source layer was offset from its marks. Corrected the fixture to import a 128×96 moving subject with a green occluder, keep disconnected marks as separate strokes, propagate a frame-two correction, serialize/reopen, and verify the retained subject and removed occluder in the software preview. This is a synthetic H.264 fixture, not real-footage quality acceptance. Validation: remote Linux passed 9 Roto engine tests, 2 optical-flow correction tests, 2 AmeUI pointer/history tests, and the imported WebP workflow test. `cargo check --offline --no-default-features --features ame,wgpu --bin kagari-studio` passed; local formatting and `git diff --check` passed.

## Real-footage review: optical-flow Roto Brush leaked across the scene

Ran the propagation path on a 2-second, 1280×720 crop of the public `People waiting to cross the street` clip from Wikimedia Commons (Amada44, CC BY-SA 3.0). The initial output was visibly unusable: matte coverage grew from 140,049 opaque pixels at frame 0 to 457,665 at frame 12 and 665,566 at frame 24, absorbing pedestrians and scenery. Added an area-growth guard for automatic re-segmentation against both the previous frame and original anchor; explicit user-correction frames bypass this guard so correction strokes remain authoritative. Tightened the guard from 2.0× to 1.35× after measuring cumulative frame-to-frame growth. Added a regression for rejecting area jumps, and preserved the occlusion correction contract.

Validation: remote Linux passed `roto_resegmentation_rejects_catastrophic_area_jumps` and both `optical_flow_propagation_` unit tests. Re-running the clip after the 1.35× guard produced 140,049, 181,609, and 242,337 opaque pixels at frames 0, 12, and 24 respectively. Visual review still showed major leakage into other people and the street by frame 24, so production subject-isolation acceptance **fails**; the guard is only a containment measure. The source is available at [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:People_waiting_to_cross_the_street.webm) under CC BY-SA 3.0. No source footage or rendered frames were added to the repository.

## Roto propagation follow-up: corrected foreground seed was ignored

Review found that connected-component selection always used the first foreground stroke, so a newer per-frame foreground correction could not move the component seed. It now uses the latest foreground stroke, including a user correction. Added a regression for seed selection, scanline-rasterized and dilated motion-prior bounds for automatic re-segmentation, and asymmetric area limits: explicit corrections may shrink the matte, but expansion remains bounded. Corrections can still produce a bad matte in the real clip; the constraints reduce runaway growth but cannot preserve identity when the block-matching flow and color model drift to nearby pedestrians.

Validation: remote Linux passed 3 focused Roto re-segmentation tests and both optical-flow motion/occlusion tests. On the same clip with a midpoint correction on the red-haired pedestrian, rendered opaque coverage was 140,049 pixels at frame 0, 181,609 at frame 12, and 175,989 at frame 24. Visual review still showed unrelated pedestrians and scenery selected, and the target was lost by frame 24. The real-footage acceptance remains **failed**; the next engineering task is improving identity-aware motion and appearance modeling, then evaluating against annotated reference mattes. No rendered QA output was committed.

## Roto preview performance follow-up: per-pixel polygon scans dominated CPU rendering

The real-footage preview stalled for minutes while CPU mask coverage called point-in-polygon and edge-distance scans for every output pixel, including pixels far outside the matte bounds. Added feather-padded mask bounds for early rejection, made straight mask edges emit only their necessary vertices, and capped traced Roto contours at 512 evenly spaced perimeter samples after simplification. The contour cap bounds mask coverage work on noisy mattes; very fine details beyond that vertex budget are necessarily approximated.

Remote Linux `cargo test --offline --no-default-features --features cli-gpu roto_ --lib` passed 30 tests, including soft-alpha tracing and the new contour-cap regression. The first 24-frame probe completed in 26.93 seconds end-to-end, but its source layer was incorrectly placed at the composition origin, unlike the centered Studio import path; its visual output and coverage counts are invalid as Studio acceptance evidence. The CPU mask optimization itself is covered by the mask and roto tests, but a valid performance comparison against the old multi-minute run is still missing. No claim of After Effects parity is justified by this performance fix.

## Roto matte follow-up: neutral pixels were biased into the foreground

Inspection of the classifier found that color and brush-distance terms could leave ambiguous pixels at exactly 50% alpha. The contour tracer treats 50% as foreground, so large neutral regions could connect unrelated background to the subject. Replaced the two-class blend with log-odds from color distance and foreground/background brush proximity, with a slight background bias for exact ties. Tightened the spatial prior from 35% to 12% of the larger frame dimension so stroke influence decays locally. Added a regression asserting an equidistant, color-ambiguous pixel becomes background while hard stroke constraints remain intact.

Remote Linux `cargo test --offline --no-default-features --features cli-gpu roto_ --lib` passed all 34 tests, and `cargo check --offline --no-default-features --features ame,wgpu --bin kagari-studio` passed after syncing the current `src/` tree to the builder. Per the available bundled-material choice, evaluated the still image `assets/studio/studio_city_reference.webp` (803×421), centered as in the Studio import path. Tightening the spatial prior and distributing background corrections across the sky, buildings, and ground reduced opaque coverage from 181,413 to 57,879 pixels. Visual review still shows building and reflection fragments and holes in the person silhouette. This is a static single-frame smoke check, not motion or commercial-quality acceptance. The matte model remains substantially below AE Roto Brush; next work needs a stronger boundary-aware segmentation model and a multi-frame asset suitable for temporal acceptance.

## Roto matte follow-up: stroke labels now propagate along image edges

The color/spatial classifier still spread labels across similar-looking regions. Added a multi-source geodesic distance field using an eight-connected pixel graph: each step costs for pixel distance plus RGB edge contrast, so crossing a strong boundary raises the path cost. Foreground and background path costs now drive the class probability, with a lower-weight color-model term. Cancellation is checked while the priority queue expands. Added tests for strong-edge penalties, cancellation, and a complete high-contrast subject/background matte.

The 803×421 bundled city-reference smoke test now retains the centered person after foreground strokes stay within the person and separate background corrections cover the adjacent buildings, sky, and foot reflection. Binary coverage is 6,544 pixels; visual inspection shows the person mostly isolated, with small edge artifacts. That result includes multiple manual background corrections and is only a still-image smoke check. Linux runtime for the full matte pass, including loading the bundled image, was 0.18 seconds. Remote Linux passed all 37 Roto tests, including geodesic cancellation, and the default Studio `cargo check`. Moving-subject propagation and quality acceptance remain open.

## Roto matte follow-up: geodesic frontier memory is bounded

The multi-source geodesic Dijkstra queue used duplicate heap entries for every improved path. On a large frame, stale entries could grow beyond the number of pixels and make memory use unpredictable. Replaced it with an indexed min-heap that keeps at most one pending entry per pixel, uses fallible allocation for the full-frame distance/index tables, and skips queue allocation when there are no seeds. This bounds frontier storage by image dimensions; it does not make the matte model more accurate or lower the base per-frame memory cost.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu roto_ --lib -- --test-threads=1` passed all 38 filtered tests, including a direct indexed-heap decrease-key/no-duplicate regression, edge penalties, and cancellation; `cargo check --offline --no-default-features --features ame,wgpu --bin kagari-studio` passed. The bundled `assets/library/av1_forest.webm` is only 73×67 px, so it cannot establish production boundary or propagation quality. Production-footage roto acceptance remains failed as documented above.

## Roto temporal regression: imported moving-footage journey still passes

Re-ran the full imported-footage workflow with the geodesic matte model: a generated three-frame clip with a moving red subject, a green occluder, and a frame-two background correction. The workflow still propagates one key per frame, retains the subject, removes the occluder, serializes and reopens the project, and renders the expected preview. Remote Linux `cargo test --offline --no-default-features --features cli-gpu --test roto_footage_workflow -- --nocapture` passed in 1.40 seconds. This protects the temporal/persistence journey but remains synthetic; real moving-footage subject isolation is still unverified.


## AmeUI contribution follow-up: Canvas stylus data now reaches the host callback

Following the REQ-353 ownership boundary, contributed a partial AmeUI API/dispatch path without taking ownership of the AmeUI project. `CanvasPointerEvent` now carries a Stylus kind, optional normalized pressure and tilt, and ordered coalesced samples; stylus IDs participate in the existing capture/cancel contract. Winit touch force is preserved only when present and valid. The AmeUI tracker now records the contribution and remaining native-backend work.

Validation: AmeUI Linux `cargo test --offline --lib` passed all 285 tests, including new logical-coordinate, sample-order, pressure normalization, capture, and cancellation regressions; `cargo fmt --all` and `git diff --check` passed. This is not native stylus acceptance: current desktop winit events still do not supply pen identity, tilt, or coalesced samples.

## Roto Brush follow-up: Canvas pressure now affects saved strokes and matte geometry

Kagari now consumes the partial AmeUI pointer data path. Coalesced samples are transformed from viewport to composition coordinates and retained in order; normalized pressure is interpolated for points inserted between events, saved parallel to stroke points with backward-compatible deserialization, used to scale hard stroke constraints and color-sampling radius, and shown in the Roto overlay as a variable-width stroke. Legacy and mouse strokes with no pressure data retain full brush radius. The AmeUI request remains open because standard desktop winit input still does not produce stylus identity, tilt, or OS-coalesced samples.

Validation: remote Linux passed 39 Roto core tests, all 3 Roto persistence tests, and all 3 default-Studio Roto tests, including pressure/sample capture, pressure-sensitive matte radius, serialization, legacy project loading, Undo/Redo, and live overlay generation. `cargo check --offline --all-features --all-targets` passed. This validates the event-to-matte software path with supplied stylus events; it does not validate hardware input or real-footage subject isolation.

## Particle preview follow-up: cache thrashed across emitter layers

The software renderer kept only one particle simulation entry per worker thread. A composition with several particle layers alternated cache identity on every frame, replaying each layer's simulation from frame zero and discarding its rewind checkpoints. Replaced the single slot with a 16-entry LRU. Each worker's cache is capped at 64 MiB, while each entry retains the existing 8 MiB checkpoint budget; oversized entries are returned for the current render but not retained. Added regressions for preserving independent layer entries and evicting the oldest entry at the count bound.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib particle_layer` passed all 4 matching cache and renderer tests. Formatting and `git diff --check` passed. This verifies cache retention/eviction and existing deterministic rendering; multi-emitter playback performance has not yet been benchmarked.

## Review follow-up: full software-renderer suite exposed stale fixtures and cancelled proxy startup

The first full Linux library run passed 1,294 of 1,307 tests and exposed seven renderer failures after AE-style anchor coordinates were introduced. Their fixtures still placed centered layers at the composition center with the default origin anchor; the 3D lighting fixture also put its solid at the camera origin, and its depth-of-field fixture put the layer behind the camera. Updated those fixtures to express centered anchors and visible 3D depths. The same run exposed a real proxy cancellation defect: a job cancelled before start attempted to open its first source frame and returned a missing-file error. `generate_video_proxy` now honors cancellation before any source or destination access.

Also corrected three stale assertions/fixtures: a fresh disk-cache temporary file is intentionally retained because it may belong to an active writer; non-image import errors now assert the decoder's current explicit error classes; and the invalid legacy project test now corrupts the `project_data` payload instead of asking the validated serializer to create an invalid project. Malformed frame-cache entries are now removed after failed validation, including zero dimensions, invalid lengths, and checksum mismatches.

Validation: all seven old-fixture renderer failures and the six remaining failures passed individually. The complete remote Linux library suite then passed all 1,307 tests with `cargo test --offline --no-default-features --features cli-gpu --lib -- --test-threads=1`.

## All-features review follow-up: six stale assumptions in legacy Studio coverage

The first all-features run passed 1,445 of 1,451 library tests. Corrected the selected-layer fixture for Content-Aware Fill to use a centered anchor; the legacy demo composition had 3840×2160 dimensions while every layer position and sizing comment described 1280×720, so it now uses the intended 1280×720 canvas and centers its background anchor. `PropertyGrid` now shrinks and truncates its label within narrow panels while reserving the control width, preventing numeric fields from extending beyond the panel. The mask-vertex shortcut test now supplies a viewport and dismisses the blocking welcome dialog, matching an interactive editing state. The GPU-effect fallback test now uses reversed effect-slot order, which is the case the software fallback must preserve. The Roto propagation fixture now uses the exclusive out-frame value 3 to cover frames 0, 1, and 2.

Validation: each of the six follow-up tests passed individually, then the complete Linux all-features library suite passed all 1,451 tests with `cargo test --offline --all-features --lib -- --test-threads=1`.

The all-target clippy gate initially reported 10 library warnings and further test/binary warnings. Fixed redundant unit bindings in the primary Studio Roto path, constant-sized chunk handling in GPU parity tests, plus the library issues. During this cleanup a camera-view matrix translation assignment briefly targeted the wrong column; the six projection regressions caught it, and the values now populate the translation column as intended.

Final validation: Linux `cargo test --offline --all-features --lib -- --test-threads=1` passed 1,451/1,451; Linux `cargo clippy --offline --all-features --all-targets -- -D warnings` passed with zero warnings. Physical GPU preview and native hardware input remain outside this headless run.
## Full-suite follow-up: Shape Repeater fixture used the wrong anchor

The all-features integration run exposed four Shape Repeater render failures. The test helper positioned a 12×12 shape at (16, 32) but left its anchor at the origin, while Shape layers use a square source space based on composition width; the visible shape center was therefore shifted off the expected pixel locations. Centered the 128×128 source-space anchor so the requested position is the visible center. All eight `shape_repeater_render` tests then passed, including animated copy count, far-copy clipping, rotation, and serialization parity.

Final Linux verification: `cargo test --offline --all-features -- --test-threads=1` passed; `cargo clippy --offline --all-features --all-targets -- -D warnings` passed; `cargo fmt --all -- --check` and `git diff --check` passed. The AppKit Stylus path remains partial: Apple exposes coalescing as a process-wide toggle and does not expose a coalesced mouse/tablet sample list, so no global input behavior was changed; the evidence and remaining platform work are recorded under AmeUI REQ-353.

## Maintainability follow-up: OBJ import and CLI render exceeded the 800-line rule

Added the repository rule that hand-written source files stay at or below 800 lines and oversized modules are split by responsibility. Extracted OBJ material-library loading/parsing from the geometry parser into `core/obj_loader/material_library.rs` while retaining the existing `obj_loader` public entry points. Split CLI render output backends and command families into focused child modules; the former 3,635-line `src/bin/cli/render.rs` is now 641 lines, and each extracted module is under 700 lines. Split `project_migration/bundle.rs` into project collection and model-companion parsing/rewriting modules; those files are now 373 and 458 lines. Existing public and caller-facing entry points remain unchanged.

Validation: Linux `cargo check --offline --all-features --all-targets` and `cargo clippy --offline --all-features --all-targets -- -D warnings` passed after the final split. The full `cargo test --offline --all-features -- --test-threads=1` suite passed after the CLI split; the 11 project-migration tests passed after the bundle split. `cargo fmt --all -- --check` and `git diff --check` passed. This is an incremental reduction pass; the repository still has oversized Studio/rendering modules, led by `ame_studio/app.rs` (3,151), `timeline_canvas.rs` (2,724), `gltf_loader.rs` (2,239), and `ame_studio/sync_bindings.rs` (2,238). The repository-wide 800-line target remains open.

## Roto motion follow-up: block matching discarded subpixel displacement

Dense optical flow returned only integer-pixel shifts even though the field stores floating-point vectors. At the Roto propagation resolution, a one-pixel error scales back to several source pixels and can roughen moving boundaries. Added a local parabolic fit around each block's minimum SAD cost to estimate a bounded fractional offset on each axis; flat or non-finite cost curves retain the integer result. The extra work evaluates at most four neighboring block costs, and the existing displacement search remains unchanged.

Added an end-to-end synthetic translation regression with known horizontal and vertical subpixel offsets. The targeted test passed, as did Linux `cargo clippy --offline --all-features --all-targets -- -D warnings` and the complete `cargo test --offline --all-features -- --test-threads=1` suite (1,452 library tests plus all integration/bin targets). `cargo fmt --all -- --check` and `git diff --check` passed. This improves motion precision only; it does not solve identity drift, occlusion reasoning, or the previously failed real-footage Roto Brush acceptance.

## Studio maintainability follow-up: application helpers extracted by responsibility

Split the former 3,151-line `ame_studio/app.rs` by moving effect/graph parameter helpers, scene/media helpers, and keyframe traversal/editing helpers into three focused modules. `app.rs` is now 1,737 lines; each extracted module is below 800 lines. Updated the sole cross-module consumer of `rgba_to_hex` to import from its new owning module. This is progress toward the 800-line maximum, not completion: the `App` trait implementation still dominates `app.rs`, and the other oversized Studio/canvas/rendering modules listed above remain open.

Validation: remote Linux `cargo check --offline --all-features --bin kagari-studio` passed. `cargo fmt --all -- --check` and scoped `git diff --check` passed. No tests were added or run for this structural-only change.

## Studio maintainability follow-up: canvas callbacks and preference storage split

Reduced `ame_studio/app.rs` from 1,737 to 312 lines. The `App` trait now forwards canvas callbacks to focused inherent methods: graph/viewport pointer handling, timeline/flowchart pointer handling, and canvas drawing live in separate modules, each below 600 lines. This keeps the trait adapter small while preserving the existing canvas behavior and event routing. Also split the 1,027-line preferences module into UI actions (434 lines) and persistence/path/recent-item utilities (594 lines), updating the one direct helper import to its new owner.

Validation: remote Linux `cargo check --offline --all-features --bin kagari-studio` passed; `cargo fmt --all -- --check` and `git diff --check` passed. No tests were added or run for these structural changes. The repo-wide 800-line target remains open; the largest remaining modules still need separate, responsibility-based splits.

## Timeline maintainability follow-up: 2,724-line canvas module split

Replaced the monolithic `ame_studio/timeline_canvas.rs` with focused modules for timeline clip drawing, graph channels, graph interpolation, graph keyframe editing, graph rendering, audio waveforms/time labels, asset previews, and flowchart drawing. The largest extracted module is 644 lines; all are below the 800-line project limit. Existing exported function names are preserved through crate-level re-exports, so current call sites do not need behavior changes.

Validation: remote Linux `cargo check --offline --all-features --bin kagari-studio` passed, then `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` passed. `cargo fmt --all -- --check` and `git diff --check` passed. This is a structural split only. Several other source files still exceed 800 lines, so the repository-wide rule remains in progress.

## Export maintainability follow-up: formats, stills, and video separated

Split the 1,125-line Studio export module into format exports (MOGRT, Lottie, MLT, subtitles, OTIO, GIF), still/sequence exports, and video export/event handling. Shared sequence/PNG file helpers remain in the format module with explicit crate visibility only where the still-export module needs them. The audio-mix cleanup guard now has a constructor so sibling modules do not depend on its storage representation. The resulting modules are 506, 376, and 261 lines.

Validation: remote Linux `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` passed (and includes compilation); `cargo fmt --all -- --check` and `git diff --check` passed. This is a structure-only change, so no tests were added or run. Other oversized files remain and the 800-line project limit is not yet met repository-wide.

## Media import maintainability follow-up: import, proxies, and asset placement separated

Split the 1,094-line Studio media module into media import/relinking (755 lines), proxy management (259 lines), and project-asset placement (86 lines). This keeps background import lifecycle separate from proxy lifecycle, while preserving the existing `AfterEffectsStudio` method entry points used by the action router.

Validation: remote Linux `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` passed; `cargo fmt --all -- --check` and `git diff --check` passed. This is a structural refactor, so no tests were added or run. Additional oversized modules remain; the repository-wide 800-line rule is still in progress.

## Viewport maintainability follow-up: preview frame and editor overlays separated

Reduced `ame_studio/viewport.rs` from 899 to 270 lines by extracting selection bounds/anchor gizmos, mask and Roto overlays, pen feedback, 3D light gizmos, and camera navigation HUD into `viewport_overlays.rs` (671 lines). The overlay boundary uses a small context struct for viewport and composition geometry, avoiding a long positional parameter list. Preview rendering and cache scheduling remain in the original module.

Validation: remote Linux `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` passed; `cargo fmt --all -- --check` and `git diff --check` passed. The change preserves the existing overlay code path and is structural. Other files still exceed the repository's 800-line rule.

## Shape and effect operation maintainability follow-up

Brought two near-limit editor modules under the 800-line cap by separating effect creation from parameter/order/keyframe editing (`effect_ops.rs`: 457 lines; `effect_edit_ops.rs`: 378) and shape editing from shape-layer creation (`shape_ops.rs`: 695; `shape_creation.rs`: 142). Existing `AfterEffectsStudio` method names and action call sites are unchanged.

Validation: remote Linux `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` passed; `cargo fmt --all -- --check` and `git diff --check` passed. This is a structural-only change. Larger event routing and binding modules still exceed 800 lines and remain the next refactor targets.

## Action-router maintainability follow-up: text, shape, and effect actions extracted

Moved text and shape property actions into `event_text_shape_handlers.rs` (372 lines), and effect catalog/order/parameter actions into `event_effect_handlers.rs` (167 lines). The main action router now delegates these domains only after its existing selected-layer lock check; its common current-frame synchronization and binding refresh remain centralized. The router fell from 2,116 to 1,859 lines.

Validation: remote Linux `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` passed; `cargo fmt --all -- --check` and `git diff --check` passed. No tests were added or run for this routing-only refactor. The router is still oversized and further domain extraction remains necessary.

## Action-router maintainability follow-up: graph, timeline, and project setup actions extracted

Extracted graph/time-remap actions (511 lines), timeline markers (289), project/export controls (115), playhead and transform controls (171), and navigation/project-setup inputs (188). The central router is now 1,289 lines. Graph editing has existing early-return paths that intentionally skip the router's final binding refresh; the graph dispatcher now reports normal handling separately from those early returns so that control flow is preserved. The shared selected-layer lock check still runs before action dispatch.

Validation: remote Linux `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` passed; `cargo fmt --all -- --check` and `git diff --check` passed. No tests were added or run for this routing refactor. The central router and several other modules remain above 800 lines, so splitting is still in progress.

## Action-router maintainability follow-up: workspace, masks, layer organization, keyframes, and assets separated

Extracted editor tool/workspace and Roto Brush controls (157 lines), mask operations (170), scene visibility/organization/alignment/parenting (232), layer transform-keyframe commands (57), and project-asset browser/import actions (268). Existing action names and the common lock gate remain in the same routing path. This brings `event_handlers.rs` from 1,289 to 778 lines, under the repository's 800-line cap. Each extracted module is also below 300 lines.

Validation: remote Linux `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` passed; `cargo fmt --all -- --check` and `git diff --check` passed. No tests were added or run for this structural refactor. Other source modules remain above 800 lines and are still being addressed.

## Binding maintainability follow-up: Studio state sync decomposed by panel

Reduced the 2,238-line `sync_bindings.rs` to a 30-line orchestration method. Project/composition and timeline bindings now live in `sync_project_timeline.rs` (412 lines) and pass the visible timeline layer list to `sync_scene_assets.rs` (542). Selected-layer bindings are split into particle/style (75), graph/mask/transform (491), media and 3D (404), and text/animator (353) modules. Each pass performs its own selected-layer lookup, avoiding a borrowed layer reference spanning mutable binding updates. Existing binding names and order of synchronization are preserved.

Validation: remote Linux `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` passed; `cargo fmt --all -- --check` and `git diff --check` passed. This is a structural change; no tests were added or run. Other oversized renderer/core files remain and are still part of the repository-wide 800-line cleanup.

## glTF loader maintainability follow-up: parsing split by responsibility

Decomposed the 2,239-line glTF loader into container parsing, geometry/accessor parsing, material and embedded-texture setup, scene transforms/instance expansion, texture sampler transforms, and generated-normal passes. The main loader is 759 lines and each extracted module is 777 lines or fewer. Existing public entry points (`parse_glb`, `parse_gltf_json_with_binary`, and `parse_gltf_json_with_buffers`) remain available through re-exports.

Validation: `cargo fmt --all` and `git diff --check` pass. Local `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` reaches unrelated compile/clippy failures in the pre-existing untracked `project_migration/bundle` work (`model_companions` visibility and unused imports), so a clean full-target validation is still pending. No tests were added or run for this structural refactor.

## 3D software rasterizer follow-up: cache ceiling and module split

Fixed an oversized-mesh cache path that evicted all entries and then retained a mesh larger than the cache's 512 MiB ceiling. Oversized meshes now remain usable by the current render through its `Arc`, but are not retained in the shared cache. Split mesh loading/cache management and material shading into dedicated modules, and moved the existing lighting and raster integration tests into their own files. The raster module is 644 lines; all sibling modules are below 330 lines.

Validation: `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` pass. No tests were run; this change preserves the existing test cases and updates the cache's oversized-entry behavior.

## Studio layer operations maintainability follow-up

Split the 1,579-line `layer_ops.rs` implementation into style, scene arrangement, keyframe, layer structure, transform, and layer creation modules. Each operation keeps its existing `AfterEffectsStudio` method name and call sites; the largest extracted module is 399 lines.

Validation: `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` pass. No tests were run for this behavior-preserving refactor.

## 3D mesh camera review: depth and near clipping now use camera space

The mesh rasterizer projected vertices through the rotated camera, but sorted transparent triangles, clipped against the near plane, and depth-tested fragments using world-space Z. This made the image projection and visibility model disagree whenever the camera rotated. Mesh vertices now carry their view-space positions; near clipping uses the same camera-facing depth threshold as projection, and transparent sorting plus the depth buffer use view-space Z. World-space positions remain the inputs for lighting.

Validation: `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy --offline --all-features --bin kagari-studio -- -D warnings` pass. A CLI software-render probe with a camera yaw of 120° and a mesh at negative world Z rendered the triangle in the expected view; the old world-Z near-plane test would have clipped it. No new tests were added or run.

## CLI render module review: extracted command exports restored

The new private CLI render submodules exposed commands as `pub(super)` and then re-exported them one level farther, making the `kagari` binary fail to compile. Made only the command entry points public within their private child modules so the parent CLI module can re-export them. Fixed strict-Clippy attribute spacing and retained a local argument-count allowance for the existing audio-spectrum command.

Validation: `cargo check --offline --no-default-features --features cli-gpu --bin kagari`, `cargo clippy --offline --no-default-features --features cli-gpu --bin kagari -- -D warnings`, and `cargo run --offline --no-default-features --features cli-gpu --bin kagari -- --help` pass. The manual rotated-camera render probe also completed through this CLI target. No tests were run.

## Combined source verification snapshot

After the renderer, Studio operation, and CLI fixes, `cargo clippy --offline --all-features --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check` all pass. This compiles every target, including existing test code, without executing tests.

## Roto propagation review: correction frames no longer disable the motion prior

The re-segmentation path skipped the motion-prior bound entirely whenever a frame had any manual correction. A background exclusion could therefore trigger unrestricted scene-wide expansion, despite the correction marking only a local area. Corrected frames now remain bounded to the dilated warped matte plus support around explicitly painted foreground correction strokes; background correction strokes can remove regions but do not open new expansion areas. The foreground stroke support reuses the engine's pressure-aware hard-stroke rasterizer. This narrows the route for accidental leakage while preserving a deliberate local path for extending the subject.

Validation: `cargo clippy --offline --all-features --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check` pass. No tests were run, and the real pedestrian probe has not yet been re-rendered, so this is an implementation improvement rather than evidence of subject-isolation acceptance. The feature-gap matrix retains the previous failed result as the baseline.



## Project I/O module split

Split the 1,019-line AmeUI Studio project I/O module by responsibility: preferences and settings (`project_io/settings.rs`, 447 lines), project open/save/autosave (`project_io/save.rs`, 400 lines), and asset relinking plus AmeUI reload (`project_io/assets.rs`, 178 lines). The entry module is now 8 lines. Existing `AfterEffectsStudio` method names and call sites are preserved.

Review finding follow-up: autosave backup copying/pruning, project serialization, directory creation, and atomic file writing now run on a joined background worker. The canvas path only takes an immutable project snapshot, launches one worker, and polls completion. Manual save, project replacement, and orderly shutdown join a pending save so an older snapshot cannot write after those operations. Completion compares both project generation/path and preview revision; newer edits keep the project dirty. Failed jobs retry after approximately five seconds instead of waiting the configured full interval. Project cloning still occurs on the UI thread to create a safe immutable snapshot and may remain noticeable for exceptionally large projects; a copy-on-write or immutable project model would be needed to move that cost safely.

Validation: `cargo check --offline --all-features --bin kagari-studio`, `cargo clippy --offline --all-features --all-targets -- -D warnings`, `cargo fmt --check`, and `git diff --check` pass. No tests were run. The `--no-default-features --features ame` check does not compile due to an existing `software_renderer/text_raster.rs` reference to the feature-gated `core::renderer`; the all-features build passes.


## Studio source-size follow-up

Moved Studio enums, settings drafts, worker job records, workspace defaults, and action classification into `src/bin/ame_studio/types.rs` (646 lines), reducing `src/bin/ame_studio.rs` from 1,222 to 582 lines. Split the 1,453-line AmeUI Studio interaction test module into six focused files under `src/bin/ame_studio/tests/`; the largest is 457 lines. Split the 1,092-line GPU preview parity integration test into 2D and 3D suites; the largest is 650 lines.

Validation: `cargo check --offline --all-features --bin kagari-studio`, `cargo clippy --offline --all-features --all-targets -- -D warnings`, `cargo fmt --check`, and `git diff --check` pass. Tests were not executed. These files were structurally split without changing their assertions.


The active CLI line-count follow-up split the 993-line argument definition into the 732-line base plus a 273-line render-command enum, extracted dispatch into a 650-line module, and moved mask editing to a focused 79-line module (leaving `edit.rs` at 752 lines). The top-level `cli.rs` is now 118 lines. Clap flattening keeps the existing command names in the same namespace; `kagari --help` and `kagari render --help` were run to verify the visible interface. Legacy egui preferences were also split into storage and reusable UI helpers; `preferences_dialog.rs` is now 766 lines. The legacy timeline and viewport were then split by responsibility: the timeline row renderer is now a 740-line child module, while its canvas coordinator is 199 lines; viewport tool/tab/context controls moved into a 111-line child module, leaving the viewport coordinator at 781 lines. A fresh `rg --files -g '*.rs'` line-count audit reports no Rust source file above 800 lines.

Validation for the CLI extraction: `cargo check --offline --no-default-features --features cli-gpu --bin kagari`, `cargo clippy --offline --no-default-features --features cli-gpu --bin kagari -- -D warnings`, `cargo fmt --check`, `git diff --check`, and CLI help invocations passed. Tests were not run. The legacy preferences split is included in the upcoming all-feature/all-target Clippy validation.


## Legacy UI and repository-wide source-size audit

Split legacy preferences persistence/default application from the dialog and moved reusable preference widgets into `preferences_dialog/`; the main dialog is 767 lines. Split the timeline's row rendering from its panel orchestration into `timeline/canvas/track_rows.rs` (740 lines) and reduced the coordinator to 199 lines. Moved composition tabs and floating tool/context controls from the viewport into `viewport/viewer_controls.rs` (111 lines), leaving the viewport at 781 lines. Together with the CLI and AmeUI Studio splits above, the current Rust source audit has no `.rs` file over 800 lines.

Validation: `cargo clippy --offline --all-features --all-targets -- -D warnings`, `cargo fmt --check`, `git diff --check`, and the repository-wide `.rs` line-count audit pass. The CLI help checks also pass. Tests were not executed.


## CPU-only AmeUI distribution build dependency fix

The repository's documented Studio packaging configuration (`--no-default-features --features ame`) failed because software text rasterization called `renderer::bake_text_stroke`, while `core::renderer` is compiled only with the optional `wgpu` feature. Moved the pure CPU stroke-baking routine to `core::text_stroke`, kept the GPU renderer's existing internal re-export, and switched the software renderer to the shared CPU module. The GPU-only `layer_3d_shading` convenience re-export is now gated to `wgpu` or unit-test builds, removing an unused-import warning in the CPU-only release configuration.

Validation: `cargo clippy --offline --no-default-features --features ame --bin kagari-studio -- -D warnings` passes; `cargo clippy --offline --all-features --all-targets -- -D warnings` also passes. The official optimized Release Studio build (`cargo build --release --offline --no-default-features --features ame --bin kagari-studio`) passes. Tests were not run.


## Enforced source-file size and shader/UI module splits

Split the AmeUI Studio root into declaration fragments for record types (151 lines), bound state (561), and actions (603); `kagari_studio.ame` is now 79 lines and keeps its view/import wiring. Split the GPU renderer shader by shared types, sampling, layer styles, and fragment processing (260/432/66/350 lines). Split the compute shader into shared declarations, spatial effects, stylize effects, lens effects, and the Gaussian entry point (94/403/275/379/46 lines). The Rust hosts concatenate these fragments in their original execution/declaration order.

Added `scripts/check_source_line_limit.py` and a CI step that rejects hand-written `.rs`, `.ame`, and `.wgsl` files above 800 lines. The current repository audit passes.

Validation: AmeUI `amec check` accepts the split Studio source; a temporary Naga checker parsed and type-validated both concatenated shaders; `cargo check --offline --all-features --all-targets`, `cargo fmt --check`, `git diff --check`, and the new line-limit check pass. Tests were not run.


## Roto matte spatial prior uses Euclidean expansion

Continued review found that separable boolean dilation made the propagation prior square: diagonal pixels could be admitted up to about 1.41 times farther than the configured margin. Replaced that with a separable squared Euclidean distance transform and reused line-transform scratch buffers. Foreground correction support remains an explicit local expansion path. Refactored the 1D distance transform to accept caller-owned buffers so each scanline does not allocate its own work arrays.

The current bundled media does not provide a usable isolated moving subject, and no new pedestrian result is claimed. The previous real-footage failure remains the acceptance baseline. `cargo clippy --offline --all-features --all-targets -- -D warnings`, formatting, diff whitespace, and the 800-line source check pass. Tests were not run.


## Roto flow downsampling removes single-pixel aliasing

The motion estimator previously downsampled each source frame by selecting one pixel at the center of every flow cell. Fine textures and narrow edges could disappear or shift with that sample grid. Replaced point sampling with deterministic area-weighted RGBA averaging and added checked dimension/input validation. This makes the optical-flow input represent each low-resolution cell rather than an arbitrary source pixel.

Validation: `cargo clippy --offline --all-features --all-targets -- -D warnings`, `cargo fmt --check`, `git diff --check`, and the 800-line source audit pass. No tests were run. The bundled footage still cannot establish moving-subject quality, so the pedestrian-clip failure remains the acceptance baseline.


## Roto footage workflow no longer silently skips in CI

The existing imported-footage workflow test returned success when FFmpeg was unavailable, while the main CI system dependency step did not install FFmpeg explicitly. CI now installs FFmpeg so the encode/import/propagate/persist/preview path is available to execute instead of relying on an image's preinstalled tools. Local tests were not run.

The 800-line gate now scans repository source files across Rust, Ame, WGSL, Python, and shell while excluding generated/build/vendor directories; the current whole-repository audit passes.


## GPU preview now falls back for unverified spatial distortions

The renderer allowlist claimed GPU support for Turbulent Displace, Wave Warp, Twirl, Bulge, Spherize, and Heat Distortion despite having no full-frame GPU/software parity cases for them. The mapping already drops Spherize's refractive-index parameter, and the heat/wave phase conversion differs from the CPU path's frame-rate-based time. These effects now route through the software preview until each complete animated parameter mapping matches export. Updated the existing capability assertions to reflect the safe fallback contract.

Validation: `cargo clippy --offline --all-features --all-targets -- -D warnings`, `cargo fmt --check`, `git diff --check`, and the repository-wide 800-line source check pass. Test code compiled under Clippy but tests were not executed; GPU runtime parity remains to be established before re-enabling any distortion effect.


## GPU fallback decision and user-facing reason now share one source

`software_preview_required` and `software_preview_reason` duplicated the same capability rules but had drifted: Freeform shapes, shape strokes/extrusion, and text animators forced software rendering while the reason API returned `None`. Made the required check delegate to the reason function and added the missing reason branches, so rendering and diagnostic decisions cannot disagree on feature coverage.

Validation: `cargo clippy --offline --all-features --all-targets -- -D warnings`, `cargo fmt --check`, `git diff --check`, and the whole-repository source line check pass. Existing test code compiled but tests were not run.

## Roto Brush project data is bounded and validated

Project validation bounded mask paths and particle counts but did not validate persisted Roto Brush strokes. A hand-authored or damaged project could provide excessive stroke points, non-finite coordinates/radii, malformed pressure arrays, or frame references outside the composition. The first three could make asynchronous matte analysis spend unbounded work or allocate large duplicate point arrays. Loading now limits a layer to 10,000 strokes and 1,000,000 total points, requires finite non-negative radii and finite coordinates, checks optional pressure samples against point count and normalized range, and rejects frame-specific strokes outside the composition duration. Legacy strokes with omitted pressure arrays remain valid.

Validation: `cargo fmt --check`, `cargo check --offline --no-default-features --features cli --lib`, `git diff --check`, and the 800-line source check passed. Tests were not run; rejection behavior and backward compatibility are not covered by a runtime fixture in this change.

## Roto propagation releases the full-resolution anchor after downsampling

The optical-flow worker keeps a full-resolution RGBA anchor frame for its entire propagation range even though later frames use only the downsampled anchor. It now releases that buffer immediately after producing the flow input, reducing peak memory by four bytes per composition pixel during multi-frame propagation. On a 4K frame this releases about 32 MiB before the first adjacent-frame render; on an 8K UHD frame, about 127 MiB. This is a peak-memory reduction only; the existing 8K-scale aggregate matte buffers remain a separate optimization target.

Validation: `cargo fmt --check`, `cargo check --offline --no-default-features --features cli --lib`, `git diff --check`, and the 800-line source check passed. Tests were not run; no runtime memory profile was collected.

## Compressed audio packets have a tighter temporary-memory bound

The decoder capped total normalized PCM at 512 MiB but allowed a single decoded packet to allocate the same amount in its reusable sample buffer, in addition to the packet and accumulated output. Added a separate 64 MiB decoded-packet cap before growing the reusable sample buffer; the full-stream 512 MiB limit is unchanged. This bounds large sample-buffer allocations after Symphonia returns a packet; it cannot constrain memory that a codec internally allocates while decoding that packet.

Validation: `cargo fmt --check`, the AmeUI Studio `cargo check`, `git diff --check`, and the repository-wide 800-line audit passed. No tests or compressed-audio fixture checks were run; unusual but valid packet sizes above 64 MiB will now be rejected.

## Video export failure paths reap FFmpeg and join stderr readers

The video-export loop returned early on cancellation, render callback failure, invalid frame bytes, or pipe write failure. `ChildGuard` eventually killed/reaped FFmpeg, but the stderr-draining thread was detached on those paths, and a broken pipe report discarded FFmpeg's diagnostic. The early exits now explicitly stop/reap the child and join the reader; pipe-write errors include bounded FFmpeg stderr when available. Successful completion keeps the existing wait and publication path.

Validation: `cargo fmt --check`, the AmeUI Studio `cargo check`, `git diff --check`, and the repository-wide 800-line audit passed. Tests and forced FFmpeg failure/cancellation scenarios were not run.

## GIF export cleanup matches video export failure handling

Follow-up review found that the two-pass GIF exporter still detached stderr readers and relied on guard destruction for child cleanup on cancellation, render failures, malformed frame data, and pipe errors. Both palette and encode passes now stop/reap FFmpeg and join stderr readers on each such exit; pipe errors include bounded FFmpeg diagnostics when available. RAII palette/output guards continue to own temporary-file cleanup.

Validation: `cargo fmt --check`, the AmeUI Studio `cargo check`, `git diff --check`, and the repository-wide 800-line audit passed. Tests and forced GIF failure/cancellation scenarios were not run.

## Roto color-model sampling honors its per-class budget

The matte color sampler reserved `samples_per_point` slots per selected stroke point, then always inserted the brush-center sample before filling the remaining grid. That allowed the foreground/background sample vectors to exceed the documented 512-sample class budget, in the one-sample case nearly doubling the model's input. Sampling now stops after the center has consumed the point's quota, while still preferring the painted center when it is in bounds.

Validation: the AmeUI Studio `cargo check`, strict all-feature/all-target Clippy, formatting, diff whitespace, and the repository-wide 800-line audit pass. Tests were not run; no segmentation-quality claim is made from this bounded-sampling correction.

## Roto resegmentation skips empty foreground strokes when selecting its seed

Project validation permits empty strokes, and older or edited project data can contain them. Seed selection previously chose the last foreground stroke before checking whether it had any points. An empty stroke therefore made resegmentation stop even when earlier valid foreground strokes existed. Seed selection now searches backward for the most recent foreground stroke containing a finite point, preserving existing behavior for valid strokes and ignoring unusable entries.

Validation: the AmeUI Studio `cargo check`, strict all-feature/all-target Clippy, formatting, diff whitespace, and the repository-wide 800-line audit pass. Tests were not run; no malformed-project fixture or propagation render was exercised.

## Export cancellation is not reported as a render failure

The AmeUI and legacy export panels distinguish cancellation from failure using the terminal error message. If cancellation arrived during frame rendering, an empty frame, callback abort, or FFmpeg broken pipe could win the race and be reported as a failed export instead. Video and both GIF passes now classify those frame/pipe errors as cancellation when the shared cancel flag is set, while preserving the diagnostic error when cancellation was not requested.

Validation: `cargo fmt --check`, the CLI library `cargo check`, `git diff --check`, and the repository-wide 800-line audit passed. UI cancellation timing and FFmpeg race behavior were not exercised; no tests were run.

## Atomic saves do not expose partial contents with destination permissions

`write_atomic` copied an existing destination's permissions to the staging file before writing. In a directory readable by other local users, a permissive destination mode could make partially written project/preferences content readable through the predictable staging filename. The staging file now keeps its restrictive creation mode during the write and adopts the destination mode only after all bytes are present, before the file sync and atomic replacement.

Validation: formatting, CLI library compilation, diff whitespace, and the repository-wide 800-line source check passed. No concurrent-reader test was run; ACLs and extended attributes are not preserved by the existing replacement strategy.

The same permission-order issue existed in the project backup staging path: it applied the original mode before copying bytes into the temporary backup. It now applies permissions after the copy completes and before syncing. Formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No concurrent-reader test was run; backup ACL/xattr preservation remains outside this fix.

## Atomic save does not silently drop destination permissions

The atomic writer previously ignored every metadata error while reading the existing destination's permissions. If the target existed but metadata lookup failed, it could still replace the file using the staging file's default mode. Permission lookup now continues only when the destination is genuinely absent; other metadata errors abort the save before replacement.

Validation: formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No injected filesystem-error test was run.

## Content-Aware Fill honors the render-buffer size limit before allocating its mask

The fill engine converted an oversized RGBA buffer request into a zero-byte output, but continued into a full `width × height` mask allocation and per-pixel scan. A maximum-axis composition can exceed the 512 MiB RGBA limit while still triggering hundreds of MiB of mask work. The function now returns an empty result immediately when the shared checked buffer-size function rejects the dimensions, before allocating output or mask storage.

Validation: `cargo fmt --check`, CLI library compilation, diff whitespace, and the repository-wide 800-line check pass. No large-dimension runtime probe was run.

## Media metadata probes cannot wait forever

FFprobe metadata calls were cancellation-aware but had no elapsed-time limit, so a hung or pathological media source could leave the import operation pending indefinitely unless the user cancelled manually. Metadata probes now stop, reap FFprobe, and join the bounded-output reader after 120 seconds. The timeout applies only to metadata probes, not long frame extraction.

Validation: formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No simulated hung-process probe was run.

## Video import FFmpeg does not inherit parent standard input

The video-import FFmpeg child had its output and diagnostics redirected but inherited the parent process's stdin. FFmpeg can use stdin for interactive commands, making its behavior depend on whether the host has an attached terminal or input pipe. The child now receives a null stdin explicitly. This keeps the import process non-interactive and consistent in GUI, CLI, and service contexts.

Validation: formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No subprocess stdin behavior probe was run.

## Proxy image decoding applies its declared allocation budget

Proxy generation checked image dimensions before decoding, but then used `image::open`, which bypassed the decoder allocation limit already used by the renderer's shared image cache. High-bit-depth or unusual formats could therefore use substantially more decode memory than the proxy's RGBA estimate. The proxy source decoder now rechecks dimensions before decoding, supplies image dimension/allocation limits, and verifies the decoded dimensions still match the inspected header. SVG decoding keeps its existing dedicated limits.

Validation: formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No high-bit-depth image fixture was decoded; the configured decoder limit is the same conservative 512 MiB RGBA source budget used by proxy validation.

The same decoder limit now also applies to each source frame while generating a video proxy. This closes the remaining direct `image::open` path after the first-frame dimension preflight; a frame whose header or decoded dimensions change fails and the partial proxy directory is removed. Formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No high-bit-depth video-frame fixture was decoded.

## Image-sequence decoding enforces its 128 MiB import budget

Numbered image-sequence import and legacy frame-cache validation checked declared dimensions but decoded with `image::open`, bypassing explicit decoder allocation limits. Added one import decoder that checks dimensions before decode, applies dimension and allocation limits to the reader, and verifies dimensions after decoding. Both import and legacy cache validation now use it, so an old cache cannot trigger an unbounded decode just to be accepted.

Validation: formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No high-bit-depth or changing-file fixture was decoded.

## Recent-project thumbnails decode media within a bounded allocation

The home-screen recent-project preview read the referenced media file entirely into a `Vec` and decoded it with `load_from_memory` without dimension or decoder-allocation limits. A project that referenced a very large or malformed image could cause a large allocation during home-screen setup. The preview now inspects dimensions through an image reader, rejects images outside the renderer's axis limit or a 256 MiB RGBA budget, decodes with explicit limits, verifies dimensions after decode, and only then creates the small thumbnail. It no longer reads the complete encoded source into memory first.

Validation: formatting, the AmeUI Studio binary compilation, diff whitespace, and the 800-line source check pass. No oversized-image startup fixture was run.

## CLI render resume validation bounds PNG decoding

When resuming a render, the CLI verified an existing PNG by checking its header and then calling `image::open` without decoder limits. A hostile or damaged output file could therefore allocate far more memory than the expected frame size. PNG resume checks now reject dimensions outside the renderer limit or a 512 MiB RGBA budget, decode with explicit image/allocation limits, and still require an exact dimension match. EXR resume checks remain on their existing bounded EXR reader.

Validation: formatting, offline CLI binary compilation, diff whitespace, and the 800-line source check pass. No malformed resume-output fixture was decoded.

## Roto matte settings cannot propagate non-finite values into the alpha calculation

The matte generator clamped contrast and edge sensitivity without first checking finiteness. In Rust, `NaN.clamp(...)` remains NaN; contrast shaping could then cast NaN alpha values to zero and produce an all-background matte. Feather NaN silently disabled refinement, while infinite feather could request excessive work. The generator now substitutes documented defaults for non-finite settings and clamps feather radius to 0–256 before edge refinement.

Validation: formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No runtime malformed-settings fixture was run.

## Roto strokes outside the frame do not collapse onto its border

Hard-stroke constraint interpolation clamps each line segment endpoint into a frame-expanded rectangle. A line segment fully outside one side of the image could therefore collapse onto the expanded border and stamp edge pixels despite never crossing the image. Added an expanded-bounds intersection check before endpoint clamping; segments too far away to reach the image at their maximum brush radius are skipped. Validation: formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No off-canvas stroke fixture was run.

## Roto propagation reports corrections it cannot apply

A user-authored correction frame could fail matte re-segmentation or the area-plausibility guard, while the bake silently kept only the motion-warped prior and still reported success. The propagation now returns a frame-specific error when a correction cannot be segmented or cannot be accepted safely. Frames without user corrections retain the prior fallback behavior. Validation: formatting, CLI library compilation, diff whitespace, and the 800-line source check pass. No deliberately ambiguous correction frame was exercised.

## Roto correction errors are localized in the AmeUI workflow

The new correction-failure messages from the shared propagation engine were displayed verbatim in the Japanese AmeUI status field. Added a small message adapter that translates the two correction-specific outcomes with their frame number while leaving unrelated engine errors intact. The adapter is in a separate module so the existing Roto UI source stays below the 800-line limit (797 lines after the split).

Validation: AmeUI Studio binary compilation, formatting, diff whitespace, and repository-wide 800-line check pass. No interactive propagation failure was triggered.

Follow-up audit of the Roto propagation completion handlers confirmed both front ends commit only on success: AmeUI updates the matte path and project history after validating the project/layer snapshot, while errors only update status; legacy egui emits an error toast and returns before committing. Strict all-feature/all-target Clippy also passes with `-D warnings`. Cargo reports only the existing upstream future-incompatibility notice for `block v0.1.6`.

## glTF embedded texture decoding enforces per-image allocation limits

Embedded glTF images were dimension-checked against the aggregate 256 MiB RGBA budget, then decoded through `load_from_memory` without passing decoder limits. A high-bit-depth image could allocate more decoded memory than its RGBA8 estimate before conversion. Texture dimensions now also honor the renderer's maximum axis, the decoder receives explicit 256 MiB allocation and dimension limits, and conversion consumes the decoded image with `into_rgba8` to avoid cloning RGBA8 images. The aggregate decoded-texture budget remains enforced.

Validation: formatting, CLI library compilation, diff whitespace, and the repository-wide 800-line source check pass. No high-bit-depth embedded glTF fixture was loaded.

Follow-up on the 3D asset audit confirmed the surrounding glTF loader also caps the model file at 128 MiB, external binary buffers combined at 128 MiB, and expanded accessor data at 256 MiB. Strict all-feature/all-target Clippy, diff whitespace, and the repository-wide 800-line check pass after the texture decoder change; the only toolchain notice remains the upstream `block v0.1.6` future-incompatibility warning. No tests were run.

## Flat future-version projects cannot bypass schema migration checks

`load_project_migrated` tried deserializing a flat project directly before inspecting `schema_version`. Serde ignores unknown fields by default, so a project with a future `schema_version` and otherwise compatible fields could load as the current schema; saving it could discard newer data. Added a streaming top-level metadata pass that reads only `schema_version` and skips other values with `IgnoredAny`, rejecting future or malformed versions before project deserialization without building a duplicate `serde_json::Value` tree.

Validation: formatting, CLI library compilation, strict all-feature/all-target Clippy, diff whitespace, and the 800-line source check pass. No future-version JSON fixture was executed. Clippy retains the upstream `block v0.1.6` future-incompatibility notice.

## Backup recovery resolves symlinked project paths consistently with saves

Atomic saves canonicalize an existing destination before writing its `.json.bak` file. Backup recovery previously derived the backup beside the user-supplied path, so opening/saving through a symlink wrote the backup beside the real project while recovery looked beside the symlink. Recovery now resolves the existing primary path before deriving the backup path, matching the save behavior; missing/unresolvable primaries keep the supplied path for fallback lookup.

Validation: formatting, CLI library compilation, diff whitespace, and the repository-wide 800-line source check pass. No symlinked corruption/recovery scenario was executed.

## Backup recovery can follow a dangling project symlink

The first symlink recovery fix worked only while the symlink target still existed, because canonicalization fails for a dangling link. If the project file was deleted or moved but its adjacent `.json.bak` remained, recovery still searched beside the alias instead of the link target. Backup-path resolution now follows the stored symlink target and canonicalizes its existing parent when the target itself is absent, while ordinary missing paths retain their original lookup behavior.

Validation: formatting, CLI library compilation, strict all-feature/all-target Clippy, diff whitespace, and the 800-line source check pass. No dangling-symlink recovery scenario was executed; the upstream `block v0.1.6` future-incompatibility notice remains.

The schema-version preflight also rejects duplicate top-level `schema_version` keys. Duplicate JSON keys otherwise permit different consumers to select different values, making version checks ambiguous before deserialization. Formatting, CLI library compilation, diff whitespace, and the 800-line check pass; no duplicate-key fixture was run.

## Graph key dragging no longer relies on a panic after a state guard

The graph pointer handler matched `Moved if graph_key_drag.is_some()` and then unwrapped the same field. The current guard makes the unwrap logically safe, but the handler's safety depended on that guard and access remaining synchronized. It now handles the absent state explicitly and exits without changing the project, keeping malformed or future-reordered pointer events from panicking the Studio.

Validation: `cargo fmt --all --check`, `git diff --check`, and the repository-wide 800-line source check pass. No pointer-event race or interactive drag was exercised; tests were not run.

## Video export preserves destinations changed after overwrite confirmation

Video export previously treated a user-confirmed overwrite as permission to replace whatever occupied the destination at the end of encoding. If another process replaced the file during a long export, the newly arrived file was silently overwritten. The export now snapshots the destination's file identity before starting the worker and compares it immediately before publication. A changed or newly appeared destination is preserved and finalization reports an error; outputs that were absent at start still publish with no-replace semantics.

Validation: `cargo fmt --all --check`, strict all-feature/all-target Clippy, `git diff --check`, and the repository-wide 800-line source check pass. Concurrent destination replacement was not runtime-exercised; tests were not run.

## Still PNG export preserves a destination changed during rendering

The still-PNG overwrite confirmation was checked before a potentially expensive frame render, but the final atomic replacement did not verify that the destination was still the file the user had approved. The worker now captures that file identity before rendering and verifies it again immediately before replacement. If the destination was absent at confirmation, publication uses no-replace semantics so a file created during rendering is preserved. PNG sequence export already performs equivalent identity checks with rollback.

Validation: formatting, strict all-feature/all-target Clippy, diff whitespace, and the repository-wide 800-line source check pass. Concurrent replacement was not runtime-exercised; tests were not run.

## CLI report and render outputs cannot overwrite their inputs

The CLI `frame` and `render` commands wrote outputs atomically but did not reject paths resolving to the input project. QC report output could replace the project after analysis, and motion-capture output could replace one of the PNG frames it analyzed. Added canonical-path checks before processing: frame/render/QC protect the project input, while mocap checks every source frame against its output.

Validation: formatting, strict all-feature/all-target Clippy, diff whitespace, and the repository-wide 800-line source check pass. The destructive paths were not invoked; tests were not run.

## Batch render selection and paths use stable, safe identifiers

Batch queue output paths were built by concatenating each project-controlled composition name directly into a filename. Names containing path separators or traversal components could direct output outside the chosen folder, and sanitized-name collisions could overwrite a prior composition's render. Queue names are now reduced to filename-safe Unicode alphanumeric, dash, and underscore characters, with a one-based queue index prefixed to guarantee uniqueness across compositions.

The queue also passed each composition name back into name-based lookup. Duplicate composition names therefore rendered the first matching composition repeatedly, despite distinct output paths. Queue dispatch now passes the exact composition index, so duplicate names still render their own composition.

Validation: formatting, strict all-feature/all-target Clippy, diff whitespace, and the repository-wide 800-line source check pass. Adversarial composition names were not rendered; tests were not run.

## Bundled media filenames remain portable across Windows, macOS, and Linux

The CLI now uses the shared production-document collector for bundles. It canonicalizes and deduplicates media, rewrites nested project references, includes sequence caches, and recursively collects OBJ/MTL companion textures. Its generated media names use portable characters and bounded filename lengths, with unique prefixes.

Validation: formatting, strict all-feature/all-target Clippy, diff whitespace, and the repository-wide 800-line source check pass. No cross-platform bundle fixture was created; tests were not run.

## CLI project bundles use the shared full dependency collector

The CLI bundle command had a hand-written collector separate from the GUI's production-document collector. It missed nested precomposition media, image-sequence caches, embedded video audio, project-level asset references, and OBJ/MTL texture companions. It now parses production documents without downgrading metadata, upgrades legacy projects in memory when needed, and invokes the shared collector with the source project's directory so relative paths resolve consistently.

Validation: formatting, strict all-feature/all-target Clippy, diff whitespace, and the repository-wide 800-line source check pass. A nested-media bundle was not created and reopened; tests were not run.

## GIF export applies the same confirmed-overwrite identity check

The two-pass GIF encoder used a separate finalization branch and retained the same race: it could replace a destination that changed while the palette and GIF passes were running. Moved output identity snapshotting and guarded publication into shared FFmpeg export helpers, then applied them to GIF. Both video and GIF now preserve destinations that changed or appeared after the export began.

Validation: `cargo fmt --all --check`, strict all-feature/all-target Clippy, `git diff --check`, and the repository-wide 800-line source check pass. Concurrent destination replacement was not runtime-exercised; tests were not run.

## Roto optical flow retains more subject detail

The propagation estimator reduced every HD frame to a maximum dimension of 160 pixels before block matching. A pedestrian or narrow silhouette could occupy only a few flow cells, making its motion indistinguishable from nearby people. Raised the bounded analysis dimension to 320 pixels; for HD this increases the flow grid from 160×90 to 320×180, while 4K inputs remain bounded. This increases optical-flow work by about 4× at the cap, so runtime latency needs measurement. The bundled promo footage does not provide an isolated, annotated subject, and the known pedestrian probe is not available in the workspace; no subject-isolation improvement is claimed.

Validation: source review confirms the downsample dimensions are derived from this shared cap for both forward and reverse flow. `cargo fmt --all -- --check`, `git diff --check`, `python3 scripts/check_source_line_limit.py`, and remote Linux `cargo clippy --offline --all-features --all-targets -- -D warnings` pass. The first isolated Clippy attempt lacked the local AmeUI path dependency; syncing the current AmeUI checkout into the temporary validation directory resolved it. No tests were run.

## Dense flow resolves flat-region ties toward zero motion

Block matching visited negative shifts first and replaced the best candidate only for a strictly lower SAD score. In a textureless block, every candidate has identical error, so the estimator incorrectly returned the maximum negative displacement. This produced false motion in static and low-texture areas, affecting Roto propagation, markerless tracking, and timewarp. Exact SAD ties now prefer the smaller displacement, which resolves fully ambiguous flat blocks to zero while retaining lower-error matches. Added a flat-frame regression.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib optical_flow -- --test-threads=1` passed all 30 matching optical-flow and Roto-propagation tests, including integer/fractional translation and the new flat-frame case. Remote strict all-feature/all-target Clippy, local formatting, diff whitespace, and the 800-line source audit pass. No real-footage Roto acceptance is claimed.

## Timewarp sampled the second frame in the wrong direction

`compute_dense_optical_flow(frame_b, frame_a, ...)` returns B-to-A vectors. Intermediate-frame synthesis used those vectors with a positive offset when sampling frame B, which shifted the sample away from the corresponding moving subject. The B-side source coordinate now subtracts the reverse vector, matching the inverse-warp mapping. Added a translated-patch midpoint regression that fails with the old sign.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib timewarp_uses_the_reverse_flow_sign -- --test-threads=1` passed the new regression. The complete optical-flow group passed all 31 tests, including Roto propagation; remote strict all-feature/all-target Clippy and local formatting, diff whitespace, and the 800-line source audit pass. This validates the reverse-warp sign for a translated patch, not broader timewarp quality or UI integration.

## Timewarp inverse mapping accounts for spatially varying motion

The first timewarp correction fixed the B-to-A vector sign, but sampling each flow only at the output pixel still assumed motion is constant around that pixel. Scaling and deformation violate that assumption. Source coordinates now use four fixed-point inverse-warp iterations, bilinearly sampling each field at the latest source estimate. Added a regression for a spatially varying expansion field.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib inverse_warp -- --test-threads=1` passed the spatial-expansion regression; the optical-flow group passed all 32 tests, including translation and Roto propagation. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass. These synthetic checks do not establish full motion-compensated playback quality.

## Frame Mix let hidden RGB from transparent pixels tint the result

The production software-render path linearly interpolated straight RGBA channels. A fully transparent red source frame mixed with an opaque blue frame therefore produced purple RGB at half alpha, which becomes a colored fringe when composited. The shared video raster path now mixes samples in premultiplied-alpha space and converts back to straight alpha. Added both a sample-level regression and a rendered WebP footage regression. Pixel Motion remains a separate helper not connected to the persisted layer option or production render path; this fix only corrects Frame Mix.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib frame_mix -- --test-threads=1` passed all 3 matching cases, including the rendered alpha WebP regression. The broader `frame_blending` group passed 12 tests, covering Frame Mix, Pixel Motion helper behavior, nested precomp, and luma matte paths. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Luma track mattes treated transparent RGB as opaque luminance

The composite path computed luma from RGB but ignored the matte pixel's alpha. A transparent white image therefore acted as a fully white luma matte and revealed the content layer. Luma coverage now multiplies luminance by matte alpha. The video-matte source path also uses the shared premultiplied-alpha frame-mix helper, so transparent RGB cannot create a false gray matte during time interpolation. Added regressions for transparent-image luma coverage and a video matte mixing transparent white with opaque black.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib luma_matte -- --test-threads=1` passed all 3 luma-matte cases, including transparent white sources and the existing interpolated video-matte workflow. `video_frame_mix` passed its rendered alpha regression; the broader frame-blending group passed all 12 tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Pixel Motion no longer invents motion for ambiguous blocks

The standalone block matcher scanned negative offsets first and accepted the first equal-SAD candidate. In flat or repetitive image regions, that could select arbitrary motion and warp otherwise static pixels. Candidate offsets are now searched from zero displacement outward, so exact ties keep the least motion. Instead of full-resolution SAD for every offset, the estimator ranks candidates on a 4×4 stratified sample and refines the four best at full resolution; default settings reduce pixel comparisons by roughly 11× before early exits. Full-resolution SAD accumulation also stops once a candidate cannot beat the current best, and exact zero-SAD matches end that block's search immediately. Candidate blocks at image borders can use partial overlap when at least half the target block remains visible. The helper remains disconnected from layer settings and production rendering, so this is an engine-quality improvement only, not a completed Pixel Motion feature.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib frame_blending -- --test-threads=1` passed all 14 matching engine and software-renderer tests, including identical-frame stationarity, textured translation, and near-edge interpolation. A temporary warmed benchmark on 1920×1080 independent random textures measured 48.2 ms median across three runs on the Linux builder; benchmark code was removed. A prior single-run measurement was 42.8 ms, so it is not treated as a stable improvement comparison. The warmed result remains above a 30fps frame budget and is one synthetic sample; Pixel Motion still needs caching/background computation and real-footage/hardware timing before integration. `cargo fmt --all -- --check`, `git diff --check`, and `python3 scripts/check_source_line_limit.py` pass. Local test linking failed because the Mac volume had limited free space; no local build output was cleaned or deleted.

## Legacy frame-blending selector was disconnected from the project

The Time Remap panel stored its Off / Frame Mix / Pixel Motion selection only in temporary egui state. It did not update the selected layer, while Pixel Motion was not wired to the renderer. The selector now reads the saved `frame_blending` value and commits Off or Frame Mix changes to the project; it is shown only for video layers. The unsupported Pixel Motion choice was removed so the panel no longer advertises a mode that has no production effect.

Validation: remote Linux `cargo check --offline --no-default-features --features gui --bin kagari-studio-egui` passes. Formatting, diff whitespace, and the repository-wide 800-line source check pass. Project save/reopen and interactive dropdown behavior were not exercised in a desktop session.

## AmeUI frame-blending switch did not show its saved state

The default Studio exposed a single “フレーム補間” toggle with no indication whether Frame Mix was enabled. It now labels the selected video layer's current state as ON or OFF from the existing synchronized binding. Added an AmeUI Studio action regression that toggles a video layer, checks the model and binding agree, then saves/reloads the versioned project and checks the option remains enabled.

Validation: remote Linux `cargo test --offline --no-default-features --features ame --bin kagari-studio video_frame_blending_action_updates_binding_and_survives_project_roundtrip -- --nocapture` passed the new test. It exercises both ON and OFF states, verifies both labels in the generated AmeUI ViewTree, checks model-to-binding synchronization, and versioned save/reload. Formatting, diff whitespace, and the repository-wide 800-line source check pass. The Mac was locked, so desktop visual layout was not inspected.

## Roto matte generation could allocate from unchecked dimensions

The engine created a zero-filled matte for malformed dimensions before applying its checks, and it had no working-set limit for valid but extremely large frames. An overflow case could request an enormous zero vector. The engine now rejects invalid dimensions and caps matte generation at 8,388,608 pixels (4K UHD) before allocating, while preserving the existing zero-matte result for mismatched pixel buffers within the supported limit. AmeUI and legacy Roto Brush reject oversized frames before rendering and validate result length before contour tracing; temporal resegmentation aborts on an invalid matte. Added regressions for oversized/unsupported dimensions and the legacy mismatch contract.

The same frame bound now covers the auxiliary Euclidean distance-field helper, which otherwise could allocate three frame-sized buffers independently of the matte entry point.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib roto_ -- --test-threads=1` passed all 41 Roto-matched engine/propagation tests; `cargo test --offline --no-default-features --features cli-gpu --test adversarial_features roto_ -- --test-threads=1` passed all 7 existing Roto contract tests. The distance-field allocation-limit assertion and direct propagation pre-render guard regression also pass as focused remote tests. `cargo check --offline --no-default-features --features gui --bin kagari-studio-egui` passed with the UI pre-render guard. Formatting, diff whitespace, and the repository-wide 800-line source audit pass. Real-footage segmentation quality remains unverified because the bundled video assets contain no suitable moving subject.

## Roto propagation API could render beyond the matte budget

The Studio and legacy UI reject oversized Roto frames, but callers can invoke the public optical-flow propagation API directly. That API requested the anchor frame before reaching matte generation, bypassing the engine's pixel cap and potentially causing an oversized render allocation. The propagation entry point now enforces the same 8,388,608-pixel limit before calling the frame provider. Added a regression that verifies oversized dimensions fail without invoking the provider.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib roto_propagation_rejects_oversized_frames_before_requesting_pixels -- --test-threads=1` passed. Formatting, diff whitespace, and the repository-wide 800-line source audit pass.

## GPU effect parity now covers transparent image pixels

The existing GPU effect parity fixture used only fully opaque solids, so it could not exercise transparency or hidden RGB under zero alpha. Added a patterned PNG fixture with alpha values from 0 through 255 and full-frame comparisons for every GPU-allowlisted effect. The test converts the software renderer's straight RGBA reference to premultiplied RGB before comparing with the GPU render target, whose source-over blend stores premultiplied color. An initial raw-RGB comparison showed a large mismatch because it compared different alpha representations; the corrected contract passes without a renderer change.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --test gpu_preview_parity -- --test-threads=1` passed all 13 tests on the available GPU adapter. The new alpha test covers Color Tint, Levels, unlinked Lens Flare, RGB-only Invert, and Posterize across the entire frame with a 2/255 tolerance. Formatting, diff whitespace, and the repository-wide 800-line source audit pass.

## Translucent GPU backgrounds now use consistent premultiplied color

An expanded alpha test for Normal and Screen compositing found that the GPU render target blended layers in premultiplied form but cleared the composition background with straight RGB. Screen over a 40%-alpha background diverged by 34/255 before the cache issue was isolated. GPU clear RGB is now multiplied by the clamped background alpha so the target stays premultiplied from its first pixel.

The same same-frame blend-mode comparison also exposed a cache key omission: `composition_cache_key` did not include the layer blend mode, so switching Normal to Screen could leave the previous GPU image onscreen. The blend mode is now included in the key and has a direct regression test.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib composition_cache_key_changes_when_layer_blend_mode_changes -- --test-threads=1` passed. The transparent-image GPU test passed for both Normal and Screen over a 40%-alpha background with 63.7% layer opacity. The full `cargo test --offline --no-default-features --features cli-gpu --test gpu_preview_parity -- --test-threads=1` suite passed all 13 tests on the available GPU adapter. Formatting, diff whitespace, and the repository-wide 800-line source audit pass.

## CPU linked flares now map projected lights into cropped layer buffers

The software renderer projected linked lights using the cropped layer-buffer dimensions as if they were the full composition. This changed camera focal projection for cropped layers and ignored the crop origin. Linked Lens Flare positions now project through composition dimensions, convert the result to composition pixels, subtract the layer-buffer crop origin, and normalize into the actual buffer. GPU preview remains on the software fallback for linked flares because its shader still needs the equivalent transformed local UV.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib linked_flare_maps_comp_light_into_the_cropped_layer_buffer -- --test-threads=1` passed; the complete GPU preview parity suite passed all 13 tests on the available adapter. `cargo clippy --offline --all-features --all-targets -- -D warnings`, formatting, diff whitespace, and the repository-wide 800-line source audit pass. The linked-flare GPU path remains disabled pending local-UV parity validation.

The expanded transparent test then found a real case that the existing opaque chain missed: Posterize after Lens Flare, Color Tint, Levels, and Invert diverged from software by 43/255 at a quantization boundary. CPU applies and rounds each effect in 8-bit buffers; the GPU executes the chain in one shader pass and carries higher precision between slots. Posterize is now routed to software when stacked after another GPU effect, preserving the reference output until stage-rounding parity is implemented. Added a capability regression and changed the transparent chain test to verify one-to-four effect GPU prefixes and assert fallback for the five-effect stack.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib gpu_preview_falls_back_when_posterize_follows_another_effect -- --test-threads=1` passed; the GPU alpha/chain regression passed; the complete `cargo test --offline --no-default-features --features cli-gpu --test gpu_preview_parity -- --test-threads=1` suite passed all 13 tests on the available adapter. The 1-to-4 effect transparent prefixes stay within 2/255 at 63.7% layer opacity, and the five-effect chain reports a CPU fallback. Formatting, diff whitespace, and the repository-wide 800-line source audit pass.

## GPU parity now covers a transformed transparent image

The existing preview comparisons covered negative scale and opacity independently, but did not combine ordinary 2D transforms with varying source alpha. Added a full-frame CPU/GPU pixel comparison for an 8×8 patterned RGBA image with alpha variation, subpixel position, nonuniform scale, rotation, and layer opacity. The case passes within a 3/255 error bound, validating bilinear sampling, transformed layer coverage, and source-over composition together without widening the GPU allowlist.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --test gpu_preview_parity transformed_transparent_image_matches_gpu_and_software -- --nocapture` passed on the available adapter; the complete parity suite passed all 14 tests, and `cargo clippy --offline --all-features --all-targets -- -D warnings` passed. Formatting, diff whitespace, and the repository-wide 800-line source audit pass.

## Interactive AmeUI edits now invalidate nested PreComp render caches

The standard Studio increments its viewport preview revision for every edit, but the software PreComp cache uses the shared frame-cache generation. During an active drag, history commits are deferred until pointer-up, so that shared generation previously stayed unchanged and the viewer could reuse stale nested-composition pixels while the user was editing. Loading a project also replaces the history stack without committing an edit, leaving the shared generation unchanged. The central `mark_project_changed` path now advances the shared generation immediately only while a history transaction is open; ordinary edits rely on the generation bump from their history commit, avoiding a duplicate cache invalidation. Project installation advances the generation after replacing the project. Studio regressions now verify that drag edits, committed edits, and project loads each advance it exactly once.

Validation: remote Linux `cargo test --offline --no-default-features --features ame --bin kagari-studio preview_cache -- --test-threads=1` passed all three cache-generation regressions. `cargo clippy --offline --no-default-features --features ame --bin kagari-studio -- -D warnings`, formatting, diff whitespace, and the repository-wide 800-line source audit pass.

## PNG and EXR sequence exports now pass cooperative cancellation into rendering

The PNG and EXR sequence workers checked cancellation only before and after rendering each frame, but did not install the cancellation flag used by the software renderer. A cancel request therefore could not stop layer processing and a partially rendered frame could be written before cleanup. Both workers now render with a scoped cancellation flag, stop before publishing a frame if cancellation arrived, and convert render panics into an export error while removing already-created frames. The scoped helper restores any previous thread-local flag during normal return or panic unwinding.

Validation: remote Linux `cargo test --offline --no-default-features --features ame --bin kagari-studio -- --test-threads=1` passed all 27 Studio tests, including both cancellation-scope regressions. Studio strict Clippy, formatting, diff whitespace, and the repository-wide 800-line source audit pass. Full sequence-export UI cancellation was not exercised end to end.

## Studio action coverage follows the split handler modules

The full Studio test run exposed a stale action-coverage test: it scanned only four original Rust handler files, while the action handlers had since been split into `event_*` modules. It therefore reported hundreds of valid actions as unhandled. The test now recursively scans the current split handler modules and still compares their literal action arms with declarations in the `.ame` UI files. It passes, and the full Studio test set now passes 27/27.

Validation: remote Linux `cargo test --offline --no-default-features --features ame --bin kagari-studio core_ame_studio_actions_coverage -- --nocapture` passed; the full Studio unit suite passed all 27 tests. Strict Studio Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Software rendering stops at completed layer stages after cancellation

The software renderer previously checked its cancellation flag only before starting each layer. If a request arrived during rasterization, paint, effects, or matte generation, it could still run every remaining stage of that layer. Added cancellation checks after those stages so the renderer stops before doing later work or compositing a cancelled layer. The check is cooperative at stage boundaries; a currently executing raster/effect operation must still return first.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib cancel_flag -- --test-threads=1` passed the layer-cancellation regression. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Raster cancellation now responds between scanlines

Stage-boundary checks still left large image and flat-layer raster loops running to completion after a cancel request. Image and flat rasterization now poll the render cancellation flag at each scanline. Post-effect mask and shadow passes do the same, so large cropped buffers stop before processing further rows. Individual effect kernels (such as blur and glow) remain cooperative only at their call boundary.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib cancel_flag -- --test-threads=1` passed; strict all-feature/all-target Clippy passed. Formatting, diff whitespace, and the 800-line source audit pass.

## Shadow-map preparation now cooperates with render cancellation

The render deadline previously could cancel layer rasterization but not an in-progress shadow-map build, which runs before the layer loop. Shadow caster traversal and polygon filling now poll cancellation, as do the horizontal and vertical penumbra passes. A cancelled build returns its allocated density buffer promptly; the frame renderer observes the flag before drawing or compositing layers. Added a regression proving an already-cancelled shadow request returns a valid zero-density buffer.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib shadow_tests -- --test-threads=1` passed all 39 selected tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Render deadlines no longer depend on watchdog polling timing

The deadline worker polls every 10 ms. A render that returned after its deadline but before the worker's next poll could therefore be reported as successful. Deadline measurement now starts before the worker is launched, and the result checks elapsed time as well as the cancellation flag. Added a zero-duration regression that deterministically verifies expired deadlines report `timed_out` even when rendering finishes before the first worker poll.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib deadline -- --test-threads=1` passed both deadline tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Frame cancellation now skips or interrupts expensive preparation

Cancellation was observed during layer rendering, but a request already present at frame start still ran audio-reactive injection and full parallel layer-data preparation. Preparation now receives the shared atomic flag and checks it per layer and mask, including inside Rayon workers where thread-local cancellation state is unavailable. The frame entry point also exits after background initialization when cancellation is already set, and checks again after audio and layer preparation before building shadows or rasterizing layers.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib -- --test-threads=1` passed all 1325 library tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass. Audio mixing itself remains non-interruptible; cancellation is observed immediately after it returns.

## Output transforms stop at bounded pixel chunks after cancellation

A frame cancellation could still spend time running exposure, color-space conversion, and LUT/ACES output work over every pixel after composition rendering had stopped. Output processing now works in 4096-pixel parallel chunks and checks the shared cancellation flag before each chunk. The chunking preserves the original global pixel index used by output dithering; a focused test verifies pre-cancelled output leaves pixels untouched.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib -- --test-threads=1` passed all 1326 library tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Track-matte rasterization responds to render cancellation

Track-matte sources can require a full-frame solid, text, image, fallback, or nested PreComp buffer. Those branches previously completed their entire pixel pass after a cancellation request. Matte rendering now checks per scanline, and bulk pixel copies check every 4096 pixels; it returns no matte when cancelled so the parent frame exits before compositing. Added a regression for an already-cancelled matte request.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib -- --test-threads=1` passed all 1327 library tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Reject invalid renders and skip PreComp flattening when already cancelled

`render_frame_to_pixels_filtered` expanded collapsed PreComps before validating output dimensions or observing an already-set cancellation flag. Invalid requests could therefore pay the cost of a deep composition clone/flatten before rejection, and cancelled frames did the same unnecessary work. Dimension validation and background-buffer initialization now happen first; an already-cancelled render returns its initialized background before flattening. Added a regression checking the valid RGBA background result for a pre-cancelled render.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib -- --test-threads=1` passed all 1328 library tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Render entry validates dimensions before composition expansion

Collapsed-PreComp flattening was performed before the requested output dimensions were checked. Invalid-size requests could therefore deep-clone and expand a composition only to be rejected afterward. Validation, output allocation, and background initialization now precede flattening; a pre-cancelled frame returns at that point as well. The existing cancellation test now also asserts the exact initialized RGBA background.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib -- --test-threads=1` passed all 1328 library tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Snapped timeline moves stay within the composition's clip-start range

The clip body drag first clamped its proposed start to `total_frames - clip_span`, then applied snapping without rechecking the range. Snapping to a nearby layer edge near the composition end could move the clip back past its allowed start and push the clip outside the composition. Added a range-aware snap helper and used it for body slides; valid in-range snaps remain unchanged while out-of-range snaps clamp to the nearest legal start.

Validation: remote Linux GUI `cargo test --offline --no-default-features --features gui --lib ui::timeline -- --test-threads=1` passed all 25 timeline tests, including the new boundary regression. GUI all-target strict Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Timeline viewport end calculations saturate instead of overflowing

Work-area visibility and layer-marker visibility used plain `start_frame + zoom_span`, unlike the viewport's existing saturating end calculation. Near `u32::MAX`, those checks could overflow in debug builds or wrap in optimized builds, producing incorrect visibility. Both paths and the timecode-label test now use saturating addition, with a viewport regression covering the upper frame limit.

Validation: remote Linux GUI `cargo test --offline --no-default-features --features gui --lib ui::timeline -- --test-threads=1` passed all 26 timeline tests. GUI all-target strict Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Clip trimming normalizes ranges safely at frame limits

The legacy timeline trim path used unchecked `layer.in_frame + 1` and assumed the current layer range was valid and inside the composition. A malformed or extreme range could overflow or violate the non-empty clip invariant during a drag. Trim updates now normalize the current range to the composition first, preserve at least one frame when duration permits, handle zero-duration compositions safely, and use saturating subtraction for slide span. Added regressions for the `u32` limit, inverted input ranges, and zero duration.

Validation: remote Linux GUI `cargo test --offline --no-default-features --features gui --lib ui::timeline -- --test-threads=1` passed all 29 timeline tests. GUI all-target strict Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Shape SDF rasterization cooperates with cancellation

Large shape layers use a CPU SDF pixel loop that previously ran to completion after render cancellation. SDF rasterization now checks the shared cancellation flag before setup, after frame-invariant preparation, and once per scanline. Shape-layer entry, Repeater instances, and Merge Paths alpha combination also stop when cancelled, avoiding extra buffers or post-processing work. Added a regression verifying a pre-cancelled SDF call leaves its destination pixels untouched.

Validation: remote Linux `cargo test --offline --no-default-features --features cli-gpu --lib -- --test-threads=1` passed all 1329 library tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Gaussian blur now runs in linear time with one fewer full-frame buffer

The three-box Gaussian approximation recomputed every radius sample for every pixel, making CPU work O(pixels × radius), and cloned the input before allocating two more full-size buffers. A 1000 px shadow softness could therefore create pathological work and four live RGBA buffers. Replaced per-pixel window sums with rolling sums (O(pixels)), reused the caller's input as one side of the ping-pong pair, and added row-bounded cancellation checks. The blur stage now needs three full-size RGBA buffers total instead of four. The public entry also rejects malformed buffer dimensions and caps out-of-range radii at the effect UI's 1000 px maximum. A reference test confirms rolling box output matches the prior naive implementation exactly on representative edge cases.

Validation: remote Linux Gaussian blur tests passed (11 selected), and the full `cargo test --offline --no-default-features --features cli-gpu --lib -- --test-threads=1` suite passed all 1332 tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Particle rendering stops during particle and trail rasterization

Particle simulation already polled cancellation, but rendering then traversed every live particle and every trail dot, with each soft-circle raster running a full pixel rectangle. Large emitters could therefore remain busy after simulation cancellation. Rendering now checks the shared flag every 64 particles, while the cancellation-aware dot raster checks each scanline. The particle layer exits after a cancelled draw and skips premultiplication, paint/effects, mask/matte work, and compositing. Added a regression verifying a cancelled dot draw leaves pixels untouched.

Validation: remote Linux particle-system tests passed (27 selected), and the full `cargo test --offline --no-default-features --features cli-gpu --lib -- --test-threads=1` suite passed all 1333 tests. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit pass.

## Text and PreComp raster loops now stop on cancellation

Glyph blits, transformed text sampling, image/pixel layer sampling, flat PreComp fills, PreComp layer traversal, and final PreComp compositing could continue through full scanline ranges after a render deadline expired. These paths now poll the renderer's cancellation flag per row (and before each path-text glyph), then return without further rasterization or compositing.

Validation: local `cargo check --lib`, `cargo fmt --all -- --check`, `git diff --check`, and the 800-line source audit passed. The existing worktree contains many unrelated edits; only the four renderer files and this review log were changed in this step.

## 3D triangles and nested PreComp sampling respond to cancellation

The software 3D rasterizer could finish a large triangle's full pixel bounds and continue all remaining triangles after a deadline. It now polls before each pass and triangle and at each triangle scanline. Nested PreComp transformed sampling also polls each row, and adjustment-layer mixing checks cancellation after effects and between bounded pixel chunks.

Validation: local `cargo check --lib`, `cargo fmt --all -- --check`, `git diff --check`, and the 800-line source audit passed.

## Blur and shadow CPU kernels validate inputs and stop cooperatively

Directional/radial blur now reject dimension/buffer mismatches and poll the shared flag inside Rayon work. Unsharp mask, glow, and drop shadow stop between their expensive stages and during full-buffer scans; the shadow raster polls each row. A strict library Clippy pass also caught and drove correction of an unnecessary cast in the style scan checks.

Validation: `cargo check --lib`, `cargo clippy --lib -- -D warnings`, formatting, diff whitespace, and the 800-line source audit passed.

## Directional Blur work is bounded consistently on CPU and GPU

The CPU kernel converted arbitrary animated blur lengths into a power-of-two sample count without a maximum, while the GPU path capped taps at 256. Very large values could allocate huge weight tables and multiply per-pixel work; non-finite values could also saturate to an invalid sample count. The CPU and GPU dispatch now share the finite check and 2–256 bound, with near-zero length remaining a no-op.

Validation: `cargo check --lib`, strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Expensive effect kernels skip allocations when already cancelled

Compound Blur, Minimax, motion blur, mesh warp, chromatic aberration, displacement mapping, film grain, and fast box blur previously allocated or cloned full-frame buffers before their first cancellation poll. They now check the shared flag before that work, avoiding unnecessary high-resolution memory traffic when a deadline has already expired.

Validation: `cargo check --lib`, strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Core Displacement Map raster observes render cancellation

Both CPU-effect dispatch paths share a displacement kernel whose full-frame nested pixel loop previously ignored render deadlines. It now checks the shared flag before output allocation and at each row/every 4096 pixels, returning the original target buffer if cancelled so no partial warp is committed.

Validation: `cargo check --lib`, strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Extended CPU effects reject zero or mismatched image dimensions

Several public lens and warp kernels accepted a zero-sized image paired with a non-empty pixel slice, then divided or took a remainder by the zero width inside a Rayon worker. Dimension checks now require positive width/height and an exact checked RGBA byte count across the dimension-based extended effects. Film-grain noise-map allocation also checks its block-count multiplication.

Validation: `cargo check --lib`, strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## CPU and GPU Minimax now use the same channel and neighborhood semantics

The first CPU review changed Minimax to compare luminance, but inspection of the actual GPU kernel showed that the product already defines this effect as independent RGB-channel extrema over a square neighborhood. CPU grayscale output diverged from GPU output, and the CPU radius 50 cap diverged from the GPU's 16-pixel cap. CPU now matches GPU per-channel square morphology, the effect dispatcher clamps to 1–16 px, and the UI exposes that same effective range.

Validation: `cargo check --lib`, strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Roto re-segmentation prior is bounded to a smaller local margin

Automatic Roto re-segmentation was allowed to extend up to 2% of the full frame dimension beyond the warped subject contour. That permits roughly 26 px at 1280-wide HD and 77 px at 4K, giving nearby people and scenery a large region to enter the matte. Reduced the margin to 0.5% (about 7 px at HD and 20 px at UHD), with a 2 px floor for small frames. Added a focused regression for HD, Full HD, UHD, and small-frame bounds.

Validation: all 22 `core::roto_assist` unit tests passed on the Linux builder, including `roto_motion_prior_margin_stays_local_at_hd_and_uhd`, existing synthetic flow propagation, occlusion correction, and motion-prior containment. `cargo fmt --all -- --check`, `git diff --check`, and the 800-line source audit passed. Compilation emitted existing warnings in `exr_reader.rs` and two mask constants. This constrains the re-segmentation search area; it does not establish subject-identity quality on the previously failing pedestrian footage, which remains the acceptance baseline.

## Single-stroke Roto extraction restores its documented spatial falloff

The single-class Roto path's spatial falloff multiplier had drifted from 0.12 to 1.2, making same-colored pixels several pixels away from a foreground-only stroke remain selected on large frames. Restored 0.12 and added a uniform-color regression that keeps the painted seed opaque while rejecting a matching pixel five pixels away. The previous two-class foreground/background path is unchanged.

Validation: all 44 Linux library tests matching `roto_` passed, including `single_foreground_stroke_does_not_select_distant_matching_color`, propagation, stroke constraints, and the previous HD/UHD prior-bound regression. `cargo fmt --all -- --check`, `git diff --check`, and the 800-line source audit passed. Compilation emitted existing warnings in `exr_reader.rs` and two mask constants. The synthetic regression isolates one-class spatial falloff; real-footage subject-isolation remains unverified.

The bundled video probe is `assets/library/av1_forest.webm`: 73×67 px and 3 seconds, with no isolated moving subject. It is unsuitable for subject-isolation acceptance, so no real-footage quality claim is made.

## Preview parity acceptance now matches the primary Studio render path

The workflow table described the default preview as a GPU allowlist, but the source shows the default AmeUI Studio generates its viewport through `software_renderer::render_frame_to_pixels_preview` on a worker thread and labels it “CPU プレビュー.” `software_preview_required` gates the optional legacy egui GPU viewport and CLI path. Updated the acceptance row to separate AmeUI preview-to-export parity from legacy GPU fallback checks, avoiding evidence from the wrong product path.

Validation: source call paths and AmeUI bindings were inspected; `git diff --check` and the 800-line source audit passed. No image parity renders were run; pixel-tolerance acceptance remains open.

## AmeUI viewport preview cancels obsolete paused renders

The single viewport worker kept rendering an old frame after the user scrubbed or edited the composition, delaying the requested preview. Preview jobs now carry a shared cancellation flag installed in the software renderer. Paused frame changes, project revisions, and preview-size changes cancel obsolete work; normal playback frame advances do not cancel every render. Cancelled or partial frames are discarded before entering the preview cache.

Validation: remote Linux `cargo check --offline --no-default-features --features ame --bin kagari-studio` passed. Preview revision changes on edits and project loads were verified. Nine focused Studio tests passed, including paused scrubbing, playback continuity, project/size invalidation, partial-frame rejection, and cache eviction policy. Strict Studio Clippy, formatting, diff whitespace, and the 800-line source audit passed.

The viewport preview cache also had only a 120-entry limit, allowing up to roughly 422 MiB for 960×960 RGBA frames. It now evicts to both the 120-entry cap and a 128 MiB byte budget, preferring old project revisions and then frames far from the playhead. Added bounds and eviction-order regressions; all nine preview-related Studio tests and strict Studio Clippy passed after this follow-up.

## Extended CPU color, lens, and warp workers now stop cooperatively

The `cpu_effects_new` color conversions, LUT grading, lens effects, motion blur, and mesh warp previously parallelized work without propagating deadline cancellation to Rayon workers. They now capture and poll the shared flag in bounded chunks; temporary-buffer warps skip copying incomplete worker output back after cancellation. Procedural row and channel-shift loops also poll cancellation. LUT volume sizing now uses checked multiplication so hostile dimensions cannot wrap before validation.

Validation: `cargo check --lib`, strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Procedural blur and minimax work is bounded and cancellable

Compound Blur and Minimax used Rayon workers that could not see the render thread's thread-local cancellation state. Both now capture the shared flag and check at row, pixel-batch, and neighborhood-scan intervals. Compound Blur rejects malformed dimensions and non-finite radii and caps the direct API at its UI maximum of 100 px; this prevents overflow and pathological radii from hostile or imported project values. Fractal Noise now exits between rows when cancelled.

Validation: `cargo check --lib`, strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Color and matte kernels now stop on cancellation

Choker, tint, tritone, posterize, invert, and threshold each scan the full pixel buffer; they now poll the shared cancel flag in bounded chunks. Matte Choker also exits between its two passes. Glow now rejects buffers whose dimensions do not match its storage before allocating or running the blur.

Validation: `cargo check --lib`, strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Public CPU effect kernels validate RGBA dimensions before indexing

Several exported distortion and style kernels trusted callers to provide exactly `width × height × 4` bytes, then indexed or allocated with unchecked products. They now share checked dimension validation and return without mutation on mismatched or overflowing dimensions. This protects direct API callers in addition to the validated layer-effect dispatcher.

Validation: local `cargo check --lib`, `cargo fmt --all -- --check`, `git diff --check`, and the 800-line source audit passed.

## Layer-style kernels stop during deadline cancellation

Inner shadow, inner glow, bevel/emboss, satin, gradient overlay, color overlay, and lens flare all scan full layer buffers in CPU loops. Their row and long-row passes now poll cancellation at row boundaries and every 4096 pixels; full-buffer blend passes poll every 16384 bytes. Bevel's intermediate buffers now stop filling when cancelled and skip the second buffer and final composite. This also prevents a cancelled style from continuing its mask construction or composite pass.

Validation: local `cargo check --lib`, `cargo fmt --all -- --check`, `git diff --check`, and the 800-line source audit passed.

## Common CPU distortion kernels stop during deadline cancellation

Twirl and bulge process pixels on Rayon workers, which do not inherit the render thread's thread-local flag; they now capture the shared atomic and poll every 4096 pixels. Offset, Venetian blinds, and linear wipe poll per row (or bounded chunks for the full-wipe fast path). The fast box blur also checks between each horizontal row and vertical column, so cancellation does not proceed into the rest of its two-pass scan. The effect stack dispatcher now stops before starting the next effect after cancellation.

Validation: local `cargo check --lib`, `cargo fmt --all -- --check`, `git diff --check`, and the 800-line source audit passed.

## Parallel compositing now observes the render's cancellation flag

The thread-local cancellation flag is not automatically present on Rayon worker threads. The main layer-composite and adjustment-layer paths parallelize by row or pixel, so their worker-side cancellation checks would otherwise always report “not cancelled.” These paths now capture the shared atomic flag before dispatch and poll it at row boundaries and every 256 pixels; the linear adjustment copy also checks while running serially.

Validation: local `cargo check --lib`, `cargo fmt --all -- --check`, `git diff --check`, and the 800-line source audit passed.

## PNG sequence recovery no longer overwrites a concurrent replacement

When an overwrite export moved an existing frame into its recovery directory, an identity mismatch triggered `rename(backup, output)`. On platforms where rename replaces destinations, a file created at `output` during that window could be silently destroyed. The mismatch path now preserves the recovery directory and reports its location without renaming over the destination.

Validation: `cargo test --offline --no-default-features --features cli ffmpeg_export --lib` passed all five matching export tests. Formatting, diff whitespace, and the 800-line source audit passed. The narrow identity-race branch is guarded by inspection; a deterministic concurrent filesystem race test is not present.

## Invalid optional autosave metadata no longer hides a recoverable project

The recovery history listed snapshots when their project payload was valid, even if optional ProductionDocument metadata was invalid. The background loader then rejected the entire snapshot, so the user could see a generation in the picker but fail to recover its valid project. It now logs and drops invalid optional metadata while preserving the project, matching the synchronous recovery path.

Validation: all 18 `core::autosave::tests` passed, including the new regression for synchronous and stateless/background loading. The test confirms invalid ProductionDocument metadata is not returned while the valid project remains recoverable. Formatting, diff whitespace, and the 800-line source audit passed.

## Dense particle collisions now have a deterministic per-frame work bound

The spatial grid avoided quadratic work for sparse particles, but a dense cluster could still compare every pair. With up to 20,000 particles exposed in the inspector (and a 100,000 direct-API cap), one simulation step could take billions of candidate checks and stall preview or export. Pair resolution now stops after 250,000 candidate tests per step in stable particle order. Normal sparse collision behavior is unchanged; very dense scenes resolve only the tested prefix and may look less physically complete, but remain responsive.

The first capped step also emits a once-per-process warning so the approximation is visible in diagnostic logs without flooding them during playback.

Validation: all 15 `core::particle_forces::tests` passed. The new dense-cluster regression confirms the configured bound and identical results for repeated inputs; the existing collision-impulse test still passes. Strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Relinking an image sequence now invalidates its old decoded cache

Cache validation treated every complete image-sequence cache as valid without comparing its source. After a relink, a different numbered sequence could therefore continue showing the old decoded frames. Image-sequence manifests now fingerprint the canonical sequence location and every numbered member's frame number, size, and modification time; refresh compares that fingerprint before reusing cached frames. A missing source still leaves a complete offline cache usable. The regression covers relinking to a different sequence and changing a non-selected member of the relinked sequence.

Validation: all 11 `core::video_import::tests` passed, including the new relink and member-change cases. Strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed. Fingerprinting uses source metadata rather than reading and hashing all original media bytes.

## Image-sequence relink support now matches the recognized formats

The cache refresh classifier recognized numbered TIFF and BMP sequences, but `import_image_sequence` rejected them. A stale or missing cache for those formats therefore could not be rebuilt. Image-sequence import now accepts TIFF/BMP extensions, and the `image` dependency enables its BMP codec so both decode and test fixture encoding are available.

Validation: all 12 `core::video_import::tests` passed, including TIFF, TIFF-long-extension, and BMP sequence imports. Strict library Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Distant foreground-colored scenery no longer overrides RotoBrush strokes

When both foreground and background strokes existed, the matte combined color likelihood with geodesic path costs but did not account for how far an unmarked pixel was from all strokes. Large regions resembling the foreground could therefore survive far from any user guidance. The foreground evidence now fades beyond a local support radius measured with the Euclidean stroke-distance field. Negative/background evidence and nearby geodesic boundary behavior remain intact, while painted stroke pixels remain hard constraints.

Validation: all 19 `core::roto_brush_engine::tests` passed, including a regression with a distant foreground-colored region and the high-contrast subject-boundary case. This is synthetic coverage; the bundled media still does not provide suitable real-footage evidence for pedestrian isolation quality.

## GPU masks now follow composition pixels and match CPU mask application order

The GPU mask raster is generated in composition/output space, but the fragment shader sampled it with transformed source-layer UVs and linear filtering. A scaled/rotated layer therefore shifted and softened its mask differently from CPU export. The shader now reads the mask at the fragment's output-pixel coordinate. CPU rendering isolates the source with the mask before effects and reapplies it after effects to contain effect spill; the GPU now mirrors both passes so partially opaque masks match the CPU reference. The first feathered comparison also exposed a 13.6/255 edge mismatch: GPU used pixel-grid distance to binary coverage while CPU used continuous distance to the polygon. GPU feathering now keeps its linear-time EDT to identify the narrow edge band, then applies the same continuous polygon distance formula to those pixels. When a path is too thin to cover any pixel center, its feather band is bounded directly from the path bounds so it remains visible without treating the entire canvas as the edge band. Broad feather radii retain the linear-time EDT path so large soft mattes do not turn into a per-pixel scan of every edge.

Validation: all 15 `core::renderer::mask_raster` tests passed, including a no-covered-pixel regression for a subpixel-width path. The full-frame Metal regression compares a rotated, scaled image and off-center partial-opacity mask at feather radii 2.5, 128, and 5,000 px, with each case within 3/255 on the available Apple GPU adapter. The GPU's dirty-check generation is advanced between property mutations, matching Studio edit commits. The complete 15-test `gpu_preview_parity` suite and strict `cargo clippy --offline --all-features --all-targets -- -D warnings` passed; formatting, diff whitespace, and the 800-line source audit passed. The parity matrix covers one rectangular mask geometry and selected feather widths, not all curves and multiple-mask combinations. Cargo still reports the upstream `block v0.1.6` future-incompatibility notice.

## Preview worker cancellation regression remains at the end of its module

Strict all-target Clippy caught a test module inserted before later worker functions and methods in `preview_workers.rs`. Moved the test module after production items without changing behavior.

Validation: strict `cargo clippy --offline --all-features --all-targets -- -D warnings`, formatting, diff whitespace, and the 800-line source audit passed.

## GPU/software mask parity now covers curved geometry and all blend modes

The transformed-layer GPU parity regression previously exercised only one rectangular Add mask. It now combines that mask with a partially opaque, feathered Bezier ellipse and checks every pair of Add, Subtract, Intersect, Lighten, Darken, and Difference across both masks. Each pair is compared full-frame at 2.5, 128, and 5,000 px feather radii on the rotated/scaled layer. A separate 1920×1080 readback case checks a hard-edged integer-coordinate curve as well as feathered curved and subtract masks.

The full-HD hard-edge case exposed a one-pixel CPU/GPU mismatch: scanline fill excluded pixel centers on polygon boundaries while the CPU point-in-polygon rule includes points within 0.01 px of an edge. GPU fill now checks pixel centers near clipped polygon segments using the same point-to-segment distance threshold; a direct regression covers near-right and bottom rectangle boundaries.

The new mask parity cases live in `tests/gpu_preview_parity/masks.rs` so the existing 2D parity source stays under the 800-line limit.

Validation: all 16 `core::renderer::mask_raster` tests and the complete 16-test `gpu_preview_parity` suite passed. The GPU suite includes 108 first/second-mask mode pairs and feather-radius combinations plus a full-frame 1920×1080 matte comparison on the available Apple GPU adapter. Strict all-target Clippy, formatting, diff whitespace, and the 800-line source limit check passed. Cargo emitted the existing upstream `block v0.1.6` future-incompatibility notice. Performance on production footage and hardware beyond this test adapter remains unverified.

## Roto propagation cancellation reaches matte re-segmentation

The default AmeUI and legacy Studio exposed a Cancel action, but the core propagation worker only observed it when requesting the next frame. A cancel during full-frame matte generation could therefore leave a costly 4K segmentation running until it finished. Added a cancellable propagation API while preserving the existing non-cancellable wrapper for callers that do not need cancellation. Both Studios now pass their job flag through optical-flow steps and into `generate_rotobrush_matte_cancellable`; cancellation is reported distinctly when a frame provider returns no pixels because the job was cancelled.

Validation: all 46 Linux Roto tests passed, including a new regression that cancels during matte re-segmentation/distance-field construction. The primary AmeUI Studio and legacy egui Studio both pass `cargo check`. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit passed. The AmeUI-only check retains an existing feature-gated unused-import warning in `software_renderer/mod.rs`; it does not fail the all-feature Clippy gate.

## Tracked-mask baking rejects non-finite project geometry

The public tracker-to-mask bake path accepted NaN/infinite mask vertices and tracker samples, and finite extreme samples could overflow while calculating displacement. Those values could be serialized into animated mask keyframes and later poison rendering or export geometry. Baking now validates the source polygon, anchor and sampled tracker positions, displacement, and final vertices before creating each keyframe. Added regression coverage for invalid mask vertices, invalid anchor/later samples, and displacement overflow.

Validation: the focused regression passed after all cases were added; all 47 Linux Roto-related library tests passed before the final cases were expanded, and the updated focused test passed again. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line limit passed.

## Layer proxies no longer shrink the preview's layer geometry

Preview rasterization sampled the proxy image dimensions to determine the layer's composition-space bounds. A quarter-resolution proxy therefore made the layer occupy only a quarter of its intended width and height in the primary Studio preview, while final renders used the full source dimensions. Geometry now comes from the original still image or first video frame; proxy pixels replace the image samples only. When the source is offline, preview infers the original geometry from the proxy dimensions and the resolution recorded for that proxy file. This is stored separately from the next-generation resolution, so changing the selected resolution does not corrupt geometry for an already attached proxy. Legacy project files default the new field to unknown and continue using their existing resolution setting. The regressions use a 12×12 source and 4×4 proxy and verify offline geometry remains stable after changing the next-generation resolution.

Validation: all 80 `core::software_renderer::tests` and all 15 proxy-related library tests passed on the Linux builder, including old-project deserialization and online/offline proxy geometry regressions. Strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source limit passed. These tests verify geometry and proxy sampling on synthetic media; broader preview/export parity across real media and effect stacks remains open.

## Proxy controls distinguish the active file from the next generation setting

The proxy resolution control changes the resolution for the next generation while the existing proxy may remain active. The properties panel labeled that value only as “Resolution” and reported any existing file as “in use,” even when the proxy was disabled. It now labels the selected value as the next generation resolution, displays the active file's recorded resolution separately, and identifies registered but disabled proxies as unapplied. Legacy files without recorded path resolution display “unknown” instead of implying a value.

Validation: all 33 AmeUI Studio tests passed in the normal parallel run, including the runtime Ame source reload test. The preview-cache tests now assert app-owned preview, history, and project generations rather than requiring a process-global cache counter to change by exactly one while other tests run. Strict all-feature/all-target Clippy, AmeUI/legacy Studio checks, formatting, diff whitespace, and the 800-line source audit passed.

## AmeUI-only builds no longer report unused GPU mask imports

The AmeUI feature set does not compile the optional WGPU renderer, but the software renderer unconditionally re-exported two mask helpers used only by that GPU path. The AmeUI-only build therefore emitted an unused-import warning even though the all-feature lint passed. Those re-exports are now gated on `wgpu`; CPU mask helpers remain available in all builds.

Validation: Linux checks for both the AmeUI Studio and legacy egui Studio passed without warnings. The 15 proxy tests, all 80 software-renderer tests, strict all-feature/all-target Clippy, formatting, diff whitespace, and the 800-line source audit passed.

## Filesystem export fallback publishes only complete outputs

The no-overwrite export helper first tried to hard-link the completed temporary output. On filesystems without hard-link support, its fallback created the final destination and copied into it directly, exposing a partial video or image-sequence frame during a potentially long copy and leaving it behind after a crash. The fallback now stages the copy in the destination directory, syncs it, then publishes it using an atomic no-replace rename (Linux `renameat2`, macOS `renamex_np`, or Windows `MoveFileExW` without replacement). The hard-link path and overwrite path now sync completed output contents before publishing and best-effort sync the parent directory after publication, including relative output paths. If the destination already exists, the hard-link fast path returns immediately instead of copying the entire source before discovering the conflict.

Validation: On macOS and Linux builder, the focused `cargo test --offline --no-default-features --features ame --lib core::ffmpeg_export::types::publish_tests` command compiled the library and passed all five cases: destination remains absent during fallback copying, replacement conflict preserves the old file and removes staging data, the existing-destination fast path skips fallback, confirmed overwrites publish, and a destination replaced during rendering is preserved. `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Filesystem behavior on deployment volumes beyond APFS and the Linux builder's filesystem remains unverified. Overwrite identity validation is a check followed by a replace, so an external writer racing in that very short interval is not atomically excluded; the tests cover replacements made before the commit check.

## Particle simulation now has time-remap freeze and reverse coverage

The renderer feeds each particle layer's effective source frame into its deterministic simulation, but the software-renderer regressions only covered ordinary frame rendering. Added a frame-parity case that compares remapped particle output against direct source-frame output for a constant freeze and a linear reverse pass. This protects cached simulation rewinds as well as the time-remap integration point.

Validation: `test_particle_time_remap_freezes_and_reverses_simulation` passed on macOS and the Linux builder. `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. The regression uses deterministic synthetic particles; it does not assess visual quality of an authored particle effect.

## Zero-speed Bezier evaluation now honors handle influence

The generic animation evaluator used CSS `ease` whenever a Bezier key had no explicit custom curve. This ignored the stored influence handles used by Easy Ease and other no-custom-curve tracks. The evaluator now maps zero-speed handles to cubic controls `[outgoing.influence, 0, 1 - incoming.influence, 1]`, clamping both influences to the normalized time range. Explicit custom curves and the prior fallback for non-zero speed handles are preserved. Non-zero speed values still need a frame rate and value delta, which this generic evaluator does not receive.

Validation: inspected the `easy_ease` constructor and no-custom-curve Bezier constructors; `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run in this pass.

## Stale animation interval hints cannot overflow during lookup

The sequential-playback interval hint is an optimization and may become stale if a track shrinks. A caller-provided `usize::MAX` hint could overflow in the one-step-ahead check in debug builds instead of falling back to binary search. That check now uses saturating addition, leaving invalid hints on the normal fallback path.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run in this pass.

## Degenerate 3D LUT dimensions no longer panic during sampling

`Lut3D::apply` underflowed the cube edge for a one-point LUT and passed a negative upper bound to `f32::clamp`, which panics. It now handles a one-point LUT as a constant color, checks the declared cube size for overflow and sufficient backing data before interpolation, and treats malformed tables as pass-through. `Lut3D::identity` now produces the minimum meaningful 2×2×2 identity table for requested sizes below two. Identity generation also has a fallible API with checked allocation and a 256-edge limit; the compatibility helper returns a pass-through LUT when generation is unsupported.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run in this pass.

## Project validation now rejects invalid Bezier handles

Project validation checked keyframe order and animated values but skipped interpolation metadata. Non-finite speeds or custom curve points, and time controls outside `[0, 1]`, could therefore pass validation and later serialize or evaluate inconsistently. The shared keyframe validation now checks both handle influences, speeds, custom point finiteness, and custom time-control bounds; it applies to the existing scalar and vector animation validators.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run in this pass.

## Bezier solver bounds time controls and contains non-finite inputs

The shared Bezier solver assumed finite time coordinates inside `[0, 1]`, which its binary-search fallback requires for a monotonic X curve. Non-finite X or Y controls could also propagate NaNs into evaluated animation values. X controls are now clamped to `[0, 1]`, non-finite X/Y controls use linear endpoint defaults, and a non-finite query returns the segment start.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run in this pass.


## Text rasterization bounds allocations and preserves alignment boxes

Formatted text and animated text allocated RGBA buffers directly from dimensions derived from font metrics and user parameters, including unchecked integer multiplication for per-glyph buffers. A hostile or extreme project could overflow sizes or trigger unbounded allocation. Raster output now uses checked dimensions, the renderer-wide per-axis limit, a 256 MiB bitmap ceiling, and fallible allocation. Animated glyph intermediates are limited by the existing glyph bitmap budget and use fallible allocation. Faux small-caps, bold, and italic expansion use checked, fallible allocations. The formatted rasterizer also previously sized its bitmap to visible content only even when center/right alignment positioned content within a wider layout box; output now includes that box width so aligned text is not clipped. The same review found that the formatted and animated rasterizers ignored the text color alpha channel; glyph coverage now incorporates that alpha in all three rendering paths.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run in this pass.


## glTF file loading now reserves the bounded input buffer fallibly

The file-size limit prevented unbounded glTF reads, but the initial `Vec` growth path could still panic on allocation failure instead of returning the import error used by the rest of the loader. The reader now reserves the already-validated file size with `try_reserve_exact` and reports address-space or allocation failure. The bounded read remains in place to handle a file that grows after metadata inspection.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run in this pass.


## Image decoding no longer serializes all renderer cache misses

The global image-cache mutex remained held across disk reads and image decoding. A single large image could therefore block unrelated software renders, GPU texture uploads, matte passes, and tracking loads that needed the same cache. Renderer hot paths now use a shared-image API that checks the cache under a short lock, decodes outside it, and rechecks before inserting. A weakly-held per-path lock lets concurrent requests for the same uncached image share one decode without retaining an unbounded lock table; unrelated image paths remain concurrent. Cache removal on missing files and byte accounting remain intact.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed without compiler warnings. Automated tests were not run in this pass.

## Project validation rejects unsafe text animator magnitudes

Text-layer validation checked the base text appearance but did not inspect either animator model. Non-finite or extreme scales, offsets, blur values, selector ranges, and advanced animator fields could enter a production document and later reach per-character rasterization math. Validation now checks the standard and stacked animator fields against the editor's supported ranges, verifies animated selector offsets and expression lengths, and rejects malformed character ranges before rendering. Animated glyph rasterization also clips its destination loops to the output bitmap, so large but finite scale combinations cannot turn off-screen pixels into billions of CPU iterations or overflow the loop bounds.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run in this pass.


## Glyph pixel documentation now matches the stored alpha representation

`RasterizedGlyph` documented its pixel data as premultiplied, but glyph rasterization stores RGB independently from coverage alpha and the text compositor later supplies the requested color. That is straight-alpha data. The type documentation now states the actual contract. The same review found that 2D CPU and GPU paths linearly filtered straight RGB, allowing hidden colors in transparent texels to create colored fringes. CPU bilinear sampling now interpolates premultiplied values and returns straight-alpha samples; GPU diffuse textures are uploaded premultiplied, filtered by hardware, then safely unpremultiplied in the shader. Temporal frame mixing remains alpha-correct in both paths, and alpha-only mask sampling is unchanged.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. The runtime WGSL validation suite and automated tests were not run in this pass.


## GPU upload conversion returns allocation failures instead of panicking

Converting a large RGBA8 image to RGBA16Float requires a second buffer twice the input size. The shared converter used infallible capacity allocation, so memory pressure during upload could panic the renderer even after image decoding had succeeded. It now checks output-size arithmetic and uses `try_reserve_exact`; image, video, precomp, text, mask, and track-matte upload paths abort the current render/texture request cleanly if conversion cannot be allocated.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run in this pass.


## macOS CI publishes both supported CPU architectures

The macOS package job ran only on `macos-latest`, which produces an Apple Silicon binary and leaves Intel Mac users without a matching download. CI now builds on the current GitHub-hosted Apple Silicon and Intel macOS 26 runners and publishes distinct `arm64` and `x86_64` DMG artifacts. The README clarifies that locally built DMGs are host-architecture specific and the CI artifacts are not universal binaries.

Validation: workflow YAML parsing, `bash -n` for the packaging scripts, `git diff --check`, and the 800-line source audit passed. Hosted macOS matrix jobs were not run locally.

## Image-sequence cache fingerprinting now has the import frame and allocation limits

Project cache validation fingerprints image sequences before it decides whether a decoded cache can be reused. That directory scan accumulated every matching file in an unbounded `Vec`, bypassing the import path's two-million-frame limit and fallible allocation. Fingerprinting now rejects sequences above the shared frame cap and reserves each entry fallibly; cache refresh and manifest generation fail cleanly instead of growing without bound.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. The check emitted the existing `block 0.1.6` future-incompatibility notice. Automated tests were not run in this pass.

## GPU render dirty-check now observes source-media revisions

The GPU renderer's early-return key included project version and composition structure but omitted external media revisions. Replacing or editing a still image or the current video frame on disk therefore did not trigger a new draw until another project edit occurred, even though texture loading already knew how to reload changed files. The render key now includes the active frame's media revision, covering direct media and nested precomp media through the existing revision walker.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. The check emitted the existing `block 0.1.6` future-incompatibility notice. Automated tests were not run in this pass.

Follow-up: the media-revision walker now skips hidden or out-of-range layers. This avoids metadata I/O and unnecessary render-key changes for media that cannot contribute to the requested frame.

GPU-path verification follow-up: `cargo check --offline --no-default-features --features gui --lib` passed, compiling the WGPU renderer and shared media-revision integration. `rustfmt --check`, `git diff --check`, and the 800-line source audit also passed. Automated tests were not run.

## AmeUI viewport preview cache also observes external media changes

The primary AmeUI viewport uses its own CPU preview cache and did not use the GPU renderer's dirty-check. Its key changed only for app-side project edits, so an external still/video frame update could leave the currently cached preview stale. Media revision calculation now lives in the shared core module and participates in the exact preview-cache key.

Validation: `cargo check --offline --no-default-features --features ame --bin kagari-studio`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. The check emitted the existing `block 0.1.6` future-incompatibility notice. Automated tests were not run in this pass.

Follow-up: the shared media-revision walker now enforces the same 128-level nesting limit as project validation, so direct callers cannot recurse indefinitely on an unvalidated in-memory composition tree. Validation: the AmeUI Studio offline check, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed; only the existing `block 0.1.6` future-incompatibility notice was emitted. Automated tests were not run.

Playback follow-up: project-edit revision and media revision are stored separately. During playback, frame-local media changes no longer look like project edits that cancel each in-flight frame; paused media changes still invalidate the exact preview cache key and request a fresh render. The preview-cache key retains media revision so stale frames are not mistaken for exact hits.

Asset-panel follow-up: composition thumbnails now include frame-0 media revision in their cache key, so changing footage on disk also invalidates the selected composition's thumbnail without an unrelated project edit. Validation: the AmeUI Studio offline check, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed; the existing `block 0.1.6` future-incompatibility notice remains. Automated tests were not run.

Proxy/model follow-up: the shared revision walker now tracks enabled image/video proxy files and Model3D source files, and ignores non-solo layers when solo mode is active. This matches the content that preview rendering can actually use more closely and invalidates stale thumbnails when a proxy or model changes. Validation: AmeUI Studio and WGPU library offline checks, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed; the existing `block 0.1.6` future-incompatibility notice remains. Automated tests were not run.

## Deep composition trees are rejected before recursive project validation can exhaust the stack

Production-document validation accepted up to 1,024 nested compositions, then recursively traversed that tree again to validate references and build the precomp index. Public callers can construct a document in memory without passing through the JSON parser's nesting guard. The validator now rejects nesting beyond 128 levels, bounding all subsequent recursive traversals too.

Validation: `cargo check --offline --no-default-features --features ame --lib`, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. The check emitted the existing `block 0.1.6` future-incompatibility notice. Automated tests were not run in this pass.

## Large asset and layer panels now build windowed view trees

The project asset browser and scene layer panel created a full widget tree for every row. The project format allows many assets, making the asset panel particularly costly to open and update. Both lists now use AmeUI's existing windowed list support from Kagari's `.ame` views. The asset list enables variable row heights because file paths and usage details add optional captions. Asset usage counts are also indexed in one pass over compositions instead of rescanning all layers once per asset; folder child checks now use a set.

Validation: `cargo check --offline --no-default-features --features ame --bin kagari-studio`, `rustfmt --check src/bin/ame_studio/sync_assets.rs`, `git diff --check`, and the 800-line source audit passed. The existing `block 0.1.6` future-incompatibility notice remains. Automated tests were not run.

## All-folder asset search no longer rescans the entire asset table for every path segment

The all-folder search built each item's display path by repeatedly scanning the full asset vector to resolve parent folders. At the 100,000 asset document limit, this made path matching approach quadratic work. Asset IDs are now indexed once, and each resolved folder path is cached for reuse across descendants. Folderless items do not allocate path-cache entries. Sorting also uses cached lowercase sort keys instead of allocating lowercase names on every comparator call. The obsolete linear-scan path helper was removed.

Validation: `cargo check --offline --no-default-features --features ame --bin kagari-studio`, `rustfmt --check src/bin/ame_studio/sync_assets.rs`, `git diff --check`, and the 800-line source audit passed. The existing `block 0.1.6` future-incompatibility notice remains. Automated tests were not run.

Scene-workspace follow-up: the alternate Scene Asset Browser and Scene Hierarchy panel had separate unwindowed copies of the same potentially large lists. They now use the existing virtual list/windowed `ForEach` widgets as well; scene asset rows opt into natural-height measurement for missing-media and source-path captions. The AmeUI Studio offline check, `git diff --check`, and the 800-line source audit passed. Automated tests were not run.

Timeline follow-up: the editor's layer row list and the layer selector in the marker workspace also rendered one widget per layer. Both now use AmeUI windowed `ForEach` with the existing 28 px timeline row size and a compact 26 px marker-selector row. This keeps large timelines responsive as projects approach the document's 100,000-layer limit. Validation: the AmeUI Studio offline check, `git diff --check`, and the 800-line source audit passed. Automated tests were not run.

## Large horizontal marker collections are tracked as an AmeUI virtualization request

The project format accepts up to 100,000 composition markers; layer markers have no separate count limit. Both composition markers and selected-layer markers are shown as `ForEach` rows inside `HorizontalScrollView`. Current AmeUI windowed lists virtualize vertical rows only, so these horizontal collections still expand every button and event node. Changing them to a vertical list would damage their timeline-oriented browsing behavior. REQ-354 is recorded in AmeUI's canonical `リクエスト.md`, with reproduction steps, expected behavior, acceptance criteria, and the AmeUI/Kagari ownership boundary. The outstanding request table and count summary were updated. No AmeUI implementation files or Hirari files were changed.

Evidence: `ProductionDocument::MAX_MARKERS_PER_COMPOSITION` is 100,000; `timeline_markers.ame` contains both unwindowed horizontal `ForEach` collections; AmeUI's DSL parser exposes vertical `height`/`row_height` virtualization options without an axis option. Kagari's AmeUI Studio offline check, `git diff --check`, and the 800-line source audit passed. Automated tests were not run.

## H.264/H.265 reject odd dimensions before expensive audio preparation

The Studio allows arbitrary positive dimensions, while its H.264/H.265 encoders force `yuv420p`, which requires even width and height. Previously, an odd-sized export could fail only after rendering the entire audio mix. The Studio now reports the dimension constraint before starting a worker, and the shared FFmpeg export validator also rejects invalid H.264/H.265 dimensions for direct API callers. Image sequence and ProRes paths remain available for odd-sized compositions.

Validation: AmeUI Studio and GUI renderer offline `cargo check` passed; `git diff --check` and the 800-line source audit passed. The existing `block 0.1.6` future-incompatibility notice remains. Automated tests were not run.

## Video import now preserves odd source dimensions and edge pixels

The video import path forced every source dimension down to the nearest even value before extracting WebP/PNG frames. Those image formats do not require even dimensions, so odd-sized footage silently lost the rightmost and bottommost source pixels. Extraction now changes only frame rate and retains the probed source width and height in both cache metadata and the imported asset.

Validation: offline `cargo check` passed for both the core Ame build and the GUI renderer build; `git diff --check` and the 800-line source audit passed. Automated tests were not run.

## In-memory animated properties now satisfy the evaluator's sorted-keyframe contract

`Animatable::value_at_f32_with_hint` uses temporal interval hints and binary search, both of which assume keyframes are ordered by frame. Project-file validation rejects out-of-order keys, and edit APIs sort after insertion, but the public `Animatable::new_animated` constructor previously retained arbitrary input order. Newly constructed in-memory animations could therefore evaluate the wrong interval or value. The constructor now sorts by frame, matching the mutation APIs and the evaluator's contract.

Validation: offline Ame core `cargo check`, `rustfmt --check src/core/property.rs`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run.

## Audio waveform caches now invalidate when a source file changes

Waveforms and decode failures were cached by path alone. Replacing an audio file at the same path therefore kept stale peaks indefinitely, and a corrected file remained marked as failed. Waveform requests/results now carry the source media revision; stale async results are discarded, and cache/failure entries are invalidated when the active file revision changes. Failed-path tracking is capped at 128 entries so browsing many unreadable files cannot grow that set without bound.

Validation: AmeUI Studio offline `cargo check`, `rustfmt --check` for the touched Rust modules, `git diff --check`, and the 800-line source audit passed. The existing `block 0.1.6` future-incompatibility notice remains. Automated tests were not run.

## Media revision keys distinguish same-size file replacements more reliably

The shared media revision originally hashed only the path, file length, and modification timestamp. Replacing a file while preserving its size and mtime could leave image/video previews and audio waveforms stale. Revisions now also include file creation time where the platform exposes it, and Unix device/inode/change-time metadata. An initial Windows implementation attempt used unstable standard-library file-ID methods and failed the Windows cross-check; those calls were removed. Stable metadata only is used. This detects same-metadata path replacements on Windows through creation time and also detects in-place writes or replacements on Unix through ctime and inode.

Validation: offline Ame core checks passed on macOS and `x86_64-pc-windows-msvc`; offline GUI renderer check passed; `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. The existing `block 0.1.6` future-incompatibility notice remains. Automated tests were not run.

## Video export progress notifications cannot accumulate without bound

Kagari permits up to 10,000,000 frames per composition, and the FFmpeg exporter emits a progress event for every encoded frame. The Studio forwarded those events into an unbounded channel that the UI drains on its frame callback; if UI dispatch paused or lagged, queued strings/events could accumulate across the entire export. The UI-facing export channel is now bounded to four events. Replaceable progress notifications use nonblocking sends and may be dropped while the queue is full; terminal success/error events are preserved separately.

Validation: AmeUI Studio offline `cargo check`, `rustfmt --check src/bin/ame_studio/export_video.rs`, `git diff --check`, and the 800-line source audit passed. The existing `block 0.1.6` future-incompatibility notice remains. Automated tests were not run.

Video-export shutdown follow-up: blocking on a full progress channel while delivering the terminal event could deadlock `AfterEffectsStudio::drop`, which cancels and joins the worker after the UI stops polling. Progress remains bounded and nonblocking; terminal success/error now uses a separate single-slot shared handoff polled by the UI. This preserves terminal status during normal operation without allowing shutdown to block on a full notification queue. Validation: AmeUI Studio offline check, `rustfmt --check`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run.

## PNG and EXR sequence progress events stay bounded

PNG and EXR sequence exports also emitted one progress event for every frame, but used an unbounded channel. A long render could accumulate millions of event strings if the Studio UI stopped draining the queue. Sequence progress now uses the same four-event bounded, nonblocking pattern as video export; success and error are stored in a separate terminal slot so queue saturation cannot hide completion or block shutdown.

Validation: offline AmeUI Studio `cargo check`, `rustfmt --check src/bin/ame_studio/export_stills.rs`, `git diff --check`, and the 800-line source audit passed. The existing `block 0.1.6` future-incompatibility notice remains. Automated tests were not run.

Sequence rollback metadata no longer retains a heap-allocated output path or an in-memory identity for every frame. PNG/EXR cleanup reconstructs sequential output names from the directory, prefix, extension, and index; it keeps a 4,096-identity window in memory and appends older fixed-size identities to a temporary manifest. Cleanup reads the manifest in bounded reverse chunks and checks each path identity immediately before removal, preserving files replaced by another process without an extra identity lookup. The manifest handle is closed before its temporary file is removed when the worker exits.

Validation: offline AmeUI Studio and `x86_64-pc-windows-msvc` Studio `cargo check`, `rustfmt --check src/core/ffmpeg_export/types.rs src/bin/ame_studio/export.rs src/bin/ame_studio/export_stills.rs`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run.

## Autosave cleanup no longer deletes a colliding temporary file

Autosave snapshots use `create_new` for a process-and-sequence temporary name. If a stale file already occupied that name, opening it failed, but the common error cleanup then removed the path anyway, even though the current write never created it. Cleanup now runs only after this call successfully created the temporary file, and ownership is cleared immediately after publishing it to the recovery slot.

Validation: AmeUI Studio offline `cargo check`, `rustfmt --check src/core/autosave/snapshot_writer.rs`, `git diff --check`, and the 800-line source audit passed. Automated tests were not run.

## Decoded audio cache keys use the shared media revision

Decoded audio was cached by path, byte length, and modification time. A same-size replacement with a preserved timestamp could therefore replay stale samples even though the shared media revision detects Unix inode/change-time and supported creation-time changes. The audio cache key now uses that shared revision, while retaining the metadata lookup for a clear missing/unreadable-file error. Existing cache-key fixtures were updated to the revised key shape.

Validation: offline AmeUI Studio and `x86_64-pc-windows-msvc` Studio `cargo check`, `rustfmt --check src/core/audio_engine/cache.rs src/core/audio_engine/tests.rs`, `git diff --check`, and the 800-line source audit passed. Tests were not run.

## Decoded image cache uses the same media revision as preview invalidation

The composition preview key included the shared media revision, but the lower-level decoded image cache still compared only file size and modification time. A replaced image with preserved metadata could therefore trigger a new preview render that still sampled stale cached pixels. Image entries now use the shared media revision too; test builds use its uncached calculation so file mutation checks remain immediate and deterministic.

The shared revision cache retains a separate file-exists bit so the image cache can evict deleted sources without an extra metadata lookup on each request. Test builds use the uncached path for deterministic file mutation behavior.

Validation: offline AmeUI Studio and `x86_64-pc-windows-msvc` Studio `cargo check`, `rustfmt --check src/core/media_revision.rs src/core/image_cache.rs`, `git diff --check`, and the 800-line source audit passed. Tests were not run.

## Precomp, model, and asset-preview caches share the stronger media revision

The decoded image and top-level composition fixes did not cover precomp output caching, 3D mesh dependency fingerprints, or the asset browser's image/model preview keys. Those paths had separate size/mtime-only revisions, so an unchanged-metadata replacement could still leave stale nested renders, geometry, or thumbnails. They now use the shared media revision, including its platform-supported file replacement metadata and one-second stat cache.

Validation: offline AmeUI Studio and `x86_64-pc-windows-msvc` Studio `cargo check`, `rustfmt --check src/core/media_revision.rs src/core/software_renderer/precomp/revision.rs src/core/software_renderer/model3d_raster/cache.rs src/bin/ame_studio/timeline_waveform.rs`, `git diff --check`, and the 800-line source audit passed. Tests were not run.

## Relinked video frame caches detect same-metadata source replacements

The relinked-frame-cache manifest compared a source movie by canonical path, size, and modification time, so replacing the movie while preserving those attributes could incorrectly reuse old extracted frames. It now also records the shared media revision; old manifests deserialize with a zero revision and trigger one safe refresh. Image-sequence imports aggregate the same per-file revision metadata while reusing their already-read filesystem metadata.

Validation: offline AmeUI Studio and `x86_64-pc-windows-msvc` Studio `cargo check`, `rustfmt --check src/core/media_revision.rs src/core/video_import/frame_cache.rs`, `git diff --check`, and the 800-line source audit passed. Tests were not run.

## Video frame-sequence timebase (resolved)

Original finding: imported video sequences were sampled one frame per composition frame, so changing composition FPS could change footage playback speed and desynchronize extracted audio. This is resolved by storing the extraction rate in the frame-cache manifest and mapping composition time to sequence-frame time through the shared video-time conversion API. The cross-path implementation and follow-up validation are recorded below; real-media regression coverage remains a separate acceptance item.

Follow-up: the extraction rate is now stored in the existing frame-cache manifest, avoiding a project-schema change. A shared conversion maps composition-frame time into that sequence's frame domain. GPU and CPU rendering, precompositions, track mattes, render capability checks, preview cache identity, legacy viewport/media thumbnails, asset waveforms, and embedded-video audio mixing now use that conversion. Audio waveform time is measured against the same extracted sequence rate. Older manifests have no extraction rate and retain their historic frame-for-frame interpretation. The per-directory lookup cache is capped at 256 entries; imported frame directories are immutable and uniquely created. Validation: offline `cargo check --lib`, GUI Studio `cargo check --features gui --bin kagari-studio`, rustfmt, `git diff --check`, and touched-file 800-line audit passed. Automated tests were not run.

Legacy-cache follow-up: when a manifest is readable but has no extraction rate, first media access stamps the current composition rate back into that manifest using atomic replacement and a refreshed frame-content fingerprint. When no manifest exists, first access writes a small, versioned FPS sidecar atomically only if frame zero exists in the expected decoded-cache naming format; this avoids writing metadata into an arbitrary or user-owned folder. Neither migration scans the sequence. Both preserve the current interpretation and keep later composition-FPS changes stable across project restarts. If the cache folder is read-only, runtime retains the legacy frame-for-frame fallback.

The second review pass found tracker and markerless-pose analysis loading frame `N` directly from the image sequence, bypassing the new conversion. Those operations now map both current and next composition frames through the shared timebase and honor layer time remapping/playback speed. GUI Studio offline build passed after this follow-up; rustfmt and `git diff --check` passed. Automated tests were not run.

The in-process FPS lookup is also refreshed whenever a manifest or legacy sidecar is written and discarded when a manifest has no usable rate. This prevents a deleted cache directory recreated at the same path from inheriting the previous media's cached FPS. The lookup remains capped at 256 directories.

Timeline waveform sampling previously reacquired the shared FPS-cache mutex for every horizontal sample. The extraction rate is now resolved once per waveform draw, then the lock-free rate-parameterized conversion is used in the pixel loop.

Project collection includes dotfiles in copied image-sequence cache directories, so both manifest and sidecar timebases travel with those sequences; movie caches are intentionally regenerated from the collected source. Cache FPS values are now restricted to the supported 1–240 FPS import range, and invalid metadata falls back to the composition rate instead of pushing sampling beyond the sequence.

Final cross-target validation for this video-timebase review: offline GUI Studio checks passed on macOS and `x86_64-pc-windows-msvc`. `rustfmt`, `git diff --check`, and the touched-file 800-line audit passed. Automated tests were not run.

The shared video-time conversion helpers were split from `timeline/layer.rs` into `timeline/video_time.rs` after the former reached 787 lines. The public `core::timeline` exports remain unchanged, and this gives the layer model room for continued edits under the 800-line source-file limit.

Post-split verification: offline GUI Studio checks passed for both the native macOS build and `x86_64-pc-windows-msvc`. A call-site audit confirmed render, audio, tracker, and thumbnail paths all resolve extraction FPS through the shared conversion API or its rate-parameterized variant.

## Ripple edits now move later layers through the timeline operation

The legacy timeline ripple path directly changed `in_frame` and `out_frame` on every later layer. That skipped `Layer::move_timeline_range`, which shifts transform/keyframe tracks and preserves video/precomp source time while moving a clip. Ripple edits now calculate bounded one-frame-minimum ranges and use the shared operation, avoiding source jumps, stale keyframe timing, and `u32` wraparound on extreme shifts.

Validation: offline GUI Studio check, rustfmt, `git diff --check`, and the touched-file 800-line audit passed. Automated tests were not run.

## Video overwrite confirmation is bound to the confirmed file

The Studio previously captured an existing output's identity only after the background worker finished audio preparation. If another process replaced the destination after the confirmation dialog appeared, the export could snapshot and overwrite that unconfirmed replacement. The Studio now captures the file identity when it presents the overwrite confirmation, clears that snapshot when the user cancels or changes the destination/codec, and passes it through the export worker. Final publication still checks that the destination has the same identity; changed or removed files are left untouched. New destinations continue to publish without replacement.

Validation: offline AmeUI Studio `cargo check`, `rustfmt`, the repository 800-line source-limit checker, and `git diff --check` passed. Automated tests were not run.

## Nested precomposition audio preserves fractional retiming

Recursive precomposition audio rounded each parent-mapped source frame to an integer before sampling its child. Fractional offsets therefore shifted nested audio by as much as half a frame, especially visible with retimed or chained precompositions. The layer time-remap evaluator now accepts fractional timeline positions, and recursive audio mixing carries the mapped position without rounding. Timing helpers were split into `mixing/source_timing.rs` to keep the implementation file under the 800-line limit.

Validation: offline library `cargo check`, `rustfmt`, the repository 800-line source-limit checker, `git diff --check`, and touched-file line counts passed. Automated tests were not run.

## Recovery retention changes no longer remove another process's snapshots

Autosave files use per-process namespaces so multiple Studio instances can retain distinct crash-recovery generations. Changing the retention limit previously enumerated all namespaces and deleted every snapshot beyond the limit, which could remove another running instance's recovery files. Retention pruning now filters to the current manager's namespace; recovery discovery remains global so snapshots from earlier sessions can still be restored.

Validation: offline library and AmeUI Studio `cargo check`, `rustfmt`, the repository 800-line source-limit checker, and `git diff --check` passed. Automated tests were not run.

## Preview-cache eviction data follows the media-aware key

The preview cache key gained a media revision so replacing footage invalidates stale pixels, but the eviction regression fixtures still used the former four-field key. Normal Studio builds passed while `cargo check --all-features --all-targets` could not compile the test target. The fixtures now use the five-field key shape. This restores all-target compilation; the regression itself was not executed.

Validation: offline `cargo check --all-features --all-targets`, `rustfmt`, the repository 800-line source-limit checker, and `git diff --check` passed. Automated tests were not run.

## Backup snapshots publish on filesystems without hard-link support

Backup creation previously depended on `hard_link`; filesystems that reject hard links caused snapshot creation to fail even though the destination directory was writable. Snapshot creation now uses the shared no-replace publisher, which falls back to a synced staged copy, creates its own temporary snapshot exclusively with owner-only permissions on Unix, restores the source file permissions after publication, and verifies/removes only the file identity it published if finalization fails.

Validation: offline `cargo check --all-features --all-targets`, `rustfmt`, the repository 800-line source-limit checker, and `git diff --check` passed. Automated tests were not run.

Follow-up: temporary snapshot names now retry up to 100 exclusive-create attempts when a stale file from a crashed process collides with a reused process ID and sequence. Other creation errors still fail immediately, and cleanup remains limited to the path created by this invocation. Permission restoration now uses the already identity-checked open file handle, so a concurrent path replacement cannot redirect the permission change onto a different file. The same compile, formatting, line-limit, and diff checks passed; automated tests were not run.

## Full-target Clippy warnings are clean

`cargo clippy --offline --all-features --all-targets -- -D warnings` exposed 17 findings: verbose early returns during GPU texture upload, redundant sample-index casts, duplicated odd-dimension checks, an opaque optional asset-sort key, and a complex waveform-worker return type. These were simplified or given named aliases. The markerless pose APIs retain their existing call signatures and now explicitly allow `clippy::too_many_arguments`, matching the existing policy on the related multi-tracker API. The same strict Clippy command now passes, and `rustfmt --check --edition 2021` passes on every touched Rust source file. Workspace-wide `cargo fmt --all -- --check` still reports formatting differences in other pre-existing dirty files; those unrelated files were not reformatted. Automated tests were not run.

## Roto Brush module descriptions no longer claim parity

The Roto Brush engine and related shape/mask propagation module headers claimed “AE Parity,” while `docs/ae_feature_gap_matrix.md` records failed subject isolation on real pedestrian footage and incomplete review/cache, render parity, paint/clone, and puppet workflows. Their module descriptions now name the implemented matte, path, and propagation work without claiming feature parity. The adjacent Studio flow was reviewed: worker errors are surfaced without applying a partial matte, and results are discarded if the project or source layer changed during propagation. The existing real-footage acceptance failure remains open. Targeted formatting, `git diff --check`, and the repository 800-line audit passed; automated tests were not run.

The bundled-media inventory contains only a forest video, not footage suitable for subject isolation. The `roto_footage_workflow` integration scenario synthesizes three frames with a moving rectangle and a rectangular occluder. The feature-gap matrix now explicitly distinguishes that synthetic workflow from the failed pedestrian-footage acceptance evidence, avoiding a misleading coverage claim. No representative clip was added or downloaded. Markdown diff checks passed; automated tests were not run.

## PNG overwrite confirmation retains the confirmed destination identity

Video export snapshots the destination identity when it displays the overwrite prompt, but still-image export previously looked up the identity only after the user confirmed. Replacing the file while that prompt was open could therefore cause Kagari to overwrite an unconfirmed file. PNG export now captures the identity before showing the prompt, consumes that snapshot once on confirmation, and clears it on cancel or path changes. A stale or invalid confirmation stops before rendering. Validation: strict offline all-feature/all-target Clippy, targeted Rust formatting, the 800-line audit, and `git diff --check` passed. Automated tests were not run.

## In-progress video and GIF exports stay private on Unix

Video, GIF, and GIF palette temporary files are created beside the destination, where the normal umask can make incomplete media readable to other local users during encoding. The temporary-output guard now records the newly reserved file's identity and permissions, restricts it to owner-only access while FFmpeg writes, then restores its original mode through the verified file handle immediately before publication. A replaced temporary path is rejected. Other platforms retain their inherited ACL behavior. Validation: offline `cargo check --all-features --all-targets` passed on macOS and Windows, as did `rustfmt`, the repository 800-line source-limit checker, and `git diff --check`. Automated tests were not run.

Follow-up: PNG temporary files now retry exclusive creation when stale names collide, are owner-only while image bytes are written on Unix, and restore the captured creation mode through an identity-checked handle before publication. This aligns still-image privacy and crash-recovery behavior with video/GIF output. Offline strict Clippy passed for all features/targets on macOS, and Windows all-target checking passed. Targeted formatting, the 800-line audit, and `git diff --check` also passed. Automated tests were not run.

## Sequence export rolls back created frames during worker unwinding

PNG/EXR sequence workers explicitly removed identity-verified frames for normal render errors and cancellation, but an unexpected panic bypassed those branches and could leave a partial sequence. `SequenceFileOwnership` now owns the destination/prefix/extension and rolls back on drop unless the sequence reaches its completed state. Explicit cleanup remains one-shot, and a committed sequence is preserved. Offline strict Clippy passed on macOS; Windows all-target `cargo check`, targeted formatting, the 800-line audit, and `git diff --check` passed. Automated tests were not run.

## Image-sequence cancellation is observed during directory scanning

Image-sequence import checked its cancellation flag while converting frames but not while enumerating the source directory, and it did not reject a job already canceled before entry. On folders with many unrelated entries, canceling could leave the import busy scanning; a pre-canceled job could also do unnecessary file and disk checks. The importer now checks at entry, during each directory entry, and immediately after enumeration, before validation or cache creation. Targeted formatting, the 800-line audit, strict all-feature/all-target Clippy, and `git diff --check` passed; automated tests were not run.

## Preview resolution currently resizes after full-resolution rendering

The AmeUI viewport selects a smaller output size for playback and preview-quality settings, but `render_frame_to_pixels_preview` calls the full composition renderer at `comp.width × comp.height` and only then resizes the completed buffer. Reducing the preview setting therefore does not reduce the core raster work or full composition-buffer allocation. The helper also returns an empty buffer when the source composition exceeds the renderer's configured dimension/byte limits, even if the requested preview buffer is smaller. The feature-gap matrix now records this as a separate performance gap from pixel parity and sets acceptance evidence for measuring lower raster work and peak memory at reduced resolutions while preserving output tolerances. Static source inspection and `git diff --check` passed; no renderer code or tests were changed in this review slice.

## Concurrent media import attempts report why they cannot start

The Studio intentionally allows one video/image-sequence import worker at a time, but file-dialog actions and dropped-media starts silently returned while a worker was active. They now share one busy check that reports either an active import or cancellation cleanup through the asset status binding. This keeps the single-worker policy visible and avoids dropping an import attempt without feedback. Targeted formatting, the 800-line audit, strict all-feature/all-target Clippy, and `git diff --check` passed; automated tests were not run.

## Image-sequence picker matches the decoder's supported formats

The core sequence importer accepts BMP and TIFF frames, but the AmeUI file picker filtered them out, so users could not select those otherwise supported sequences through the normal workflow. The dialog now includes BMP, TIFF, and TIF alongside PNG/JPEG/WebP. Targeted formatting, the 800-line audit, strict all-feature/all-target Clippy, and `git diff --check` passed; automated tests were not run.

## Imported media caches are private while being written

Video and image-sequence caches are created under the system temporary directory. Their import root previously used the process umask, which can leave frames and extracted audio readable to other local accounts on shared Unix machines. Cache roots are now created atomically with owner-only `0700` permissions on Unix; child frame directories/files remain unreachable to other accounts through that protected root. Other platforms keep their inherited ACL behavior. Targeted formatting, the 800-line audit, strict all-feature/all-target Clippy, and `git diff --check` passed; filesystem permission behavior was reviewed statically and automated tests were not run.


## Video frame counting remains cancellable after FFmpeg completes

The video importer checked cancellation while FFmpeg ran and before audio extraction, but not while it enumerated the extracted frame directory to calculate the cache length. For a very large sequence, cancellation at that stage could leave the worker scanning millions of entries and delay cleanup. Frame counting now checks the cancel flag for every entry and once after the scan, before validating or writing the cache manifest. The first Clippy attempt hit a full work disk; cleaning only Kagari's generated Cargo artifacts recovered 6.5 GiB. Targeted formatting, the 800-line audit, strict all-feature/all-target Clippy, Windows all-feature/all-target `cargo check`, and `git diff --check` then passed. Automated tests were not run.

## Video cache fingerprinting also stops on cancellation

After frame counting, the importer fingerprints every extracted file to verify and publish its cache manifest. This second full-cache pass previously ignored cancellation while opening frames and reading their contents. The fingerprint helper now accepts an optional cancel flag and checks it before each frame and each 64 KiB read; video import passes its flag, while ordinary cache validation and image-sequence callers retain their existing non-cancellable behavior. Cancellation aborts before a manifest is published, and the import-directory guard removes the incomplete cache. Targeted formatting, the 800-line audit, strict all-feature/all-target Clippy, Windows all-feature/all-target `cargo check`, and `git diff --check` passed. Automated tests were not run.

## Image-sequence source fingerprinting stops on cancellation

Image-sequence cache manifest creation also re-enumerates every source frame and reads its metadata after conversion, so canceling could wait through a second uninterruptible directory walk and metadata pass. The source-fingerprint path now polls the import cancellation flag during enumeration, after enumeration, and for each frame while collecting revision metadata. Cache-content fingerprinting also receives the same cancel flag. Fingerprinting helpers were split into `frame_cache/fingerprint.rs` to keep each Rust source file below the 800-line limit. Targeted formatting, the 800-line audit, strict all-feature/all-target Clippy, Windows all-feature/all-target `cargo check`, and `git diff --check` passed. Automated tests were not run.

## Failed import-cache cleanup is visible in logs

`ImportDirectoryGuard` intentionally removes partial video/sequence caches on errors, cancellation, or unwinding, but silently ignored removal failures. The guard now ignores an already-missing directory and logs other cleanup errors with the cache path, so leftover storage is diagnosable without masking the original import result. Targeted formatting, the 800-line audit, strict all-feature/all-target Clippy, and `git diff --check` passed. Automated tests were not run.

## Imported audio assets show their decoded duration

Newly imported audio project items store `duration_sec: 0.0`, while the asynchronous waveform worker already decodes the audio and calculates its duration. The project asset details now prefer that decoded duration when available, retaining the stored duration as a fallback. This fixes the persistent `0.0 秒` label without mutating project state or creating an undo/dirty event; the waveform result already triggers a binding refresh. Validation: targeted formatting, the 800-line source audit, strict all-feature/all-target Clippy, and `git diff --check` passed. Automated tests were not run.

## Waveform cache continues serving projects with many audio clips

The waveform cache stopped accepting requests at 128 entries, leaving later timeline clips and selected audio assets permanently without a waveform and with a loading status. The cache now evicts its least recently used waveform when full, updates recency for visible timeline audio and the selected audio asset, and removes recency metadata when a file revision invalidates its cached waveform. The cache remains capped at 128 entries. Validation: targeted formatting, the 800-line source audit, strict all-feature/all-target Clippy, and `git diff --check` passed. Automated tests were not run.

## Roto strokes reject invalid and out-of-range sample points

The segmentation sampler converted every stroke coordinate to `i32` before validating it. Non-finite coordinates could accidentally seed a stroke at the image origin, and very large finite values could overflow when the brush radius was added. Samples that are non-finite or farther outside the image than the brush radius are now skipped before conversion; strokes that partially overlap the image edge still contribute normally. Validation: targeted formatting, the 800-line source audit, strict all-feature/all-target Clippy, and `git diff --check` passed. Automated tests were not run.

## Video export reports disconnection after progress updates

The Studio treated any queued event as evidence that the video export worker had completed normally. If the event channel disconnected after one or more progress updates but before a terminal Finished/Error event, it cleared the worker state while leaving the UI stuck on “書き出し中…”. The receiver now tracks whether a terminal event actually arrived and reports the connection loss otherwise. Validation: targeted formatting, the 800-line source audit, strict all-feature/all-target Clippy, and `git diff --check` passed. Automated tests were not run.

## Preview-resolution performance gap is assigned to the correct repository

The feature-gap matrix attributed full-resolution CPU preview followed by resize to an AmeUI API. Source inspection showed AmeUI supplies the viewport's requested output dimensions, while Kagari's own `core/software_renderer::render_frame_to_pixels_preview` renders the full composition before resizing. The matrix now assigns the performance gap and its implementation to Kagari; no AmeUI request was filed for behavior owned here. The renderer performance and preview/export parity gaps remain open. Validation: static source cross-check and `git diff --check`; no code behavior or tests changed.

## Temporary audio mixes are private while written

Video export creates its intermediate stereo WAV in the system temporary directory with `create_new`, but did not set restrictive Unix permissions. On systems with a permissive umask, other local users could read audio content before the export guard removed it. The WAV is now created atomically with owner-only `0600` permissions on Unix; non-Unix platforms retain their inherited ACL behavior. Validation: targeted formatting, the 800-line source audit, strict all-feature/all-target Clippy, and `git diff --check` passed. Automated tests were not run.

## Atomic project and backup staging files start private

The shared `atomic_file::write_atomic` helper and ProductionDocument backup replacement created sibling staging files with default Unix permissions, despite keeping their contents temporary until the atomic rename. On shared directories, project JSON and its backup could be readable while being written. Both paths now create staging files with owner-only `0600` permissions; existing destination permissions are restored after the full write and before publication. Newly created atomic outputs remain private by default. Validation: targeted formatting, the 800-line source audit, strict all-feature/all-target Clippy, and `git diff --check` passed. Automated tests were not run.

## Render staging files and temporary proxy workspaces stay private

The EXR writer and CLI still-PNG writer staged rendered pixels in the output directory with default Unix permissions, and PNG sequence exports used a default-permission staging directory. EXR and CLI PNG temporary files now start at `0600` and restore an existing destination's permissions only after the complete write; newly created files remain owner-only. PNG sequence staging directories now start at `0700`, so their in-progress frames and recovery copies are not traversable by other local users. Layer preview proxies were also generated under a predictable shared `/tmp` directory; their per-job workspace now starts at `0700` and is removed on generation failure, cancellation, or worker-start failure. Failed cleanup is logged where the staging guard owns it. Validation: targeted formatting, the 800-line source audit, strict all-feature/all-target Clippy, Windows all-feature/all-target `cargo check`, and `git diff --check` passed. Automated tests were not run.

## Frame cache and autosave are private per-user

The default frame cache and recovery snapshots were stored in predictable shared `/tmp` directories. Frame-cache RGBA data can contain rendered project imagery, while autosave snapshots contain the project itself. Unix defaults now use platform-standard per-user cache/state paths (XDG cache/state paths on Linux and Library Caches/Application Support on macOS), with directories restricted to `0700` and new frame, lock, probe, and autosave staging files restricted to `0600`. On startup, valid legacy recovery snapshots owned by the current Unix user are copied to the private recovery directory after checking regular-file identity and the project-size limit; other users' files, symlinks, corrupt snapshots, and oversized files are skipped. Migration scans at most 4,096 candidate names and copies at most the newest 50 snapshots so a large legacy directory cannot stall startup or overfill recovery history. Explicit frame-cache directories keep their existing directory permissions. Frame-cache corruption fixtures now resolve the configured cache path instead of assuming the old shared `/tmp` path. Windows retains the previous temporary-directory defaults and ACL behavior. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, the 800-line source audit, and `git diff --check`; automated tests were not run.

## Backup pruning uses file age and tolerates concurrent cleanup

AME Studio backup pruning ordered generations by timestamp text in the filename. Local clock changes and same-millisecond saves from separate processes could make that ordering inaccurate. Pruning now orders matching regular files by filesystem modification time with the filename as a deterministic tie-breaker, ignores symlinks, and treats a file already removed by another process as successful cleanup. Other deletion or metadata errors still surface. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests were not run.

## Layer proxy paths survive system temporary-file cleanup

Generated image/video layer proxies were stored under the OS temporary directory, then their absolute paths were saved into the project. Temporary-directory cleanup or a reboot could remove those files while leaving the project configured to use them. Proxy generation now writes under the configured preview-cache directory (falling back to its platform default), in a per-job private workspace. If a user clears the preview cache, the existing renderer already falls back to source media when a proxy path is missing. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests were not run.

The cache-clear action only removed the frame cache, leaving generated layer proxies in the configured preview-cache directory with no way to reclaim their disk space. It now also removes only the Kagari-owned `LayerProxies` subdirectory, refuses while proxy generation is active, and reports filesystem errors in the settings status. The default preview-cache directory now follows macOS, Linux/XDG, and Windows conventions instead of using a macOS path on every platform. Missing proxy paths already fall back to source media. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, Windows all-target/all-feature `cargo check`, and `git diff --check`; automated tests were not run.

After cache clearing, a layer could still have `proxy.enabled = true` while its proxy file no longer existed. The renderer correctly fell back to source media, but the properties panel still claimed the proxy was active and allowed a no-op toggle. The active indicator now requires an existing proxy file, and the toggle is disabled while the file is missing; generation and reference removal remain available. Validation: all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, AME source inclusion through the build, and `git diff --check`; automated tests were not run.

Proxy workspace cleanup previously ran only when generation returned an error. A panic or a dropped result receiver could leave a partial or unused workspace under the now-persistent preview cache. A drop guard now removes the workspace on errors, cancellation, panics, and undelivered results, retaining it only after successful generation and delivery to the UI. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests were not run.

The video-import availability probe used `Command::output()` for `ffmpeg -version`, capturing the tool's full stdout in memory even though only its exit status matters. It now discards both output streams and reads only the process status, matching the export-side probe. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests were not run.

Subtitle import used `read_to_string` on the selected external file with no size bound, then converted every parsed cue into a full text layer. It now reads at most 4 MiB (including a growth check after metadata inspection) and rejects more than 20,000 cues before allocating layer objects, with a visible status message. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests were not run.

Lottie import had unbounded file/string JSON parsing and no cap on imported layer or precomposition counts. File and string entry points now reject JSON over 32 MiB; file reads remain bounded if the source changes while being read. Individual compositions are capped at 20,000 layers, and full projects at 50,000 layers and 1,000 compositions, returning the existing invalid-import result for over-limit input. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests were not run.

The legacy effects panel enumerated and parsed every preset JSON on each immediate-mode UI frame, causing repeated synchronous filesystem and serde work. Preset listing is now cached for one second (and invalidated after Kagari saves a preset), discovery scans at most 4,096 entries, and individual preset JSON is bounded to 1 MiB. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests were not run.

## CPU LUT effects no longer re-read and re-parse the file per render

The CPU ColorGradeLUT path used unbounded `read_to_string` and parsed the `.cube` file on every effect invocation, bypassing the bounded loader. Its thread-local color-result cache keyed only on LUT size and input color, so swapping to another same-size LUT could reuse incorrect colors. CPU grading now uses the 64 MiB-bounded loader and a shared 64 MiB LRU of parsed LUT data, invalidated by file size/modification stamp. Same-path cache misses are serialized with per-path locks to avoid multiple MFR workers reading and parsing one large LUT simultaneously, while unrelated LUT files can load in parallel. Pixel cache keys now include a deterministic LUT identity; in-memory batch application clears its bounded result cache between calls so it cannot reuse entries from another LUT. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, 800-line source audit, and `git diff --check`; automated tests were not run.

## Video frame renders restore an enclosing cancellation scope

FFmpeg export and its shared panic-safe frame helper installed a thread-local render cancellation flag, then always cleared it to `None`. When frame rendering ran inside an existing cancellation scope, completion or panic silently discarded the caller's flag, so later work in that outer scope could ignore cancellation. Both paths now use the renderer's RAII scope helper, which restores the previous flag on normal return and unwinding. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, 800-line source audit, and `git diff --check`; automated tests were not run.

## Repository source files meet the 800-line cap

Re-audited hand-written Rust, Ame, WGSL, Python, and shell sources under `src/`, `tests/`, `examples/`, and `scripts/`. The largest files are at or below 800 lines; `src/bin/ame_studio/roto_brush.rs` is exactly 800. No further split is needed to satisfy the current per-file cap, though modules should continue to be split when additions would exceed it. This is a source-size audit, not a product-behavior validation.

## Software rendering reports large buffer allocation failure without panicking

The software renderer validated composition buffers against its 512 MiB limit, but then used infallible `vec!` allocation for the full composition and every cropped layer raster. Adjustment layers also cloned the full accumulated frame with `to_vec`, which could panic despite valid dimensions. These principal buffers, plus full-frame particle and track-matte buffers, the pre-comp layer copy/cropped raster and composition/adjustment buffers, Merge Paths, Clone Stamp, Corner Pin, Echo/Set Matte, and audio-effect luma work arrays now use checked sizes and `try_reserve_exact`. Nested render scopes carry allocation failure back to the top-level frame so partial pre-comps are not accepted as complete output. The renderer returns an empty frame rather than partial output or a panic. Buffer helpers and top-level frame rendering were split into responsibility-specific modules to keep every source file within the 800-line cap. Validation: targeted formatting, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, 800-line source audit, and `git diff --check`; automated tests were not run.

Follow-up review found additional fallible-allocation gaps in external 3D model rasterization and 3D shadow caster outlines. Model world/view vertex arrays and transformed normals now reserve fallibly. Shadow shape outlines, transformed caster points, and projected points do the same; star point-count multiplication is checked before allocation. These failures mark the current render allocation scope so it cannot publish a partial frame. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. Workspace-wide `cargo fmt --check` still reports a pre-existing import-order difference in `src/bin/ame_studio/media_proxy.rs`, which this change did not modify. No tests were run.

The Roto Brush Euclidean distance-field pass allocated and collected a temporary row vector for every image row and a temporary column vector for every image column. It now reuses one bounded input/output line pair plus transform scratch arrays for the full pass, eliminating O(width + height) heap allocations per matte while retaining the existing 1D transform and output layout. The full-frame distance, seed, and intermediate buffers now also reserve fallibly. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. No tests or runtime benchmark were run; performance improvement is established as fewer allocations, not a measured wall-time claim.

Follow-up review found the optical-flow Roto motion-prior mask had a second Euclidean distance transform with infallible full-frame allocations for the interior mask, distance image, and foreground-correction support. Those buffers and its line scratch arrays now reserve fallibly, and the downsampled flow image reports allocation failure before optical-flow processing. Binary alpha conversion now thresholds the generated matte in place instead of allocating a second full-frame mask. The motion-prior rasterization and dilation were moved into `roto_assist/motion_prior.rs` to keep the propagation module under the 800-line source limit. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, the source line-count check, and `git diff --check` passed. No tests or footage run were performed.

The Roto Brush matte generator still used infallible full-frame allocations for the output matte and stroke distance inputs, and could duplicate every foreground/background stroke point into a third vector. It now reserves bounded color samples and exact finite stroke-point counts fallibly, reserves the output matte fallibly, and calculates the union stroke distance from two borrowed point slices without copying them together. Edge-refinement frame buffers also reserve fallibly. The feature matrix was synchronized with AmeUI REQ-353's current working-tree progress: macOS AppKit stylus delivery is present but hardware acceptance, Windows/Linux backends, and native coalesced samples remain open. Validation: targeted `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. Tests and real-footage review were not run.

The optional GPU viewport's capability gate correctly ignored the effect stack when `Layer::effects_enabled` was false, but the GPU uniform builder still evaluated all enabled effect records and sent their flags to the shader. The same project could therefore show effects in the GPU preview after the layer-wide Effects switch was disabled, while the software renderer correctly skipped them. GPU effect parameters now use the default disabled set whenever the layer switch is off, keeping rendering aligned with the capability decision and CPU behavior. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. Tests were not run.

Cross-target validation found two Windows-only Clippy errors: constant-size UTF-16 chunks used `chunks_exact(2)`, and the Windows process probe used a needless `return`. The code now uses `as_chunks::<2>()` after its existing even-length guard and returns the predicate expression directly. Validation: Windows MSVC all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, touched-file `rustfmt`, and `git diff --check` passed. No automated tests were run.

Linux target validation: the all-feature cross-check on macOS could not use the Linux pkg-config sysroot. The default AmeUI Studio target was then checked and linted with Linux cfg using the host Fontconfig headers via `PKG_CONFIG_ALLOW_CROSS=1`; `cargo check --no-default-features --features ame,wgpu --bin kagari-studio --target x86_64-unknown-linux-gnu` and strict Clippy both passed. This confirms Linux Rust source/cfg compilation only; it does not establish Linux system-library linking or runtime behavior. The configured Linux builder currently has a different hash for the edited files, so its stale worktree was not used as evidence.

The active Roto contour path had several frame-sized infallible allocations after matte generation: binary thresholding, connected-component visitation, contour state tracking, and guided edge filtering. Roto contour, segmentation, and guided-filter entry points now enforce the same 8.4-megapixel limit and return an empty result on allocation failure. Connected-component traversal marks visited foreground pixels in its private binary mask and grows a fallible queue, avoiding a second full-frame visited map. Moore contour state and contour growth reserve fallibly. Douglas-Peucker simplification now uses an explicit range stack instead of recursive calls and intermediate vectors, preventing deep recursion on noisy contours; negative or NaN simplify tolerances normalize to zero so the range split always makes progress. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, source-size check, and `git diff --check` passed. Tests and footage review were not run.

The legacy public Roto segmentation API accumulated every pixel under every brush point as a color sample, then compared each frame pixel against the full sample list. Long or dense stroke histories could therefore drive memory and runtime in proportion to brush area × point count × frame pixels. It now samples at most 256 in-frame brush centers per class, reduces them to at most eight color prototypes per class, and reserves the bounded sample buffers fallibly. Its frame-size limit and fallible output allocation remain in force. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, source-size check, and `git diff --check` passed. Tests and footage review were not run.

`ParticleSystem::update` is a public simulation entry point, but it accepted negative or non-finite time steps and emitter coordinates. Those values could propagate NaNs through particle state and poison later cached frames. It now ignores updates unless `dt` is finite and positive and both emitter coordinates are finite; renderer-driven time remapping continues to advance deterministic positive frame steps. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. Tests and runtime footage review were not run.

Project validation accepted particle lifetime, speed, and rotation-speed variance values above their documented 0–1 range. Lifetime variance above 1 could produce particles with negative maximum lifetimes, while the other fields no longer matched their stated normalized controls. Production-document validation now rejects all three outside 0–1. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. Tests and sample-project round trips were not run.

The legacy particle inspector still allowed speed variation up to 2.0, contradicting the engine's documented 0–1 range and the new project validation. Its control now uses the same 0–1 range so the UI cannot create a project that validation rejects. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. UI interaction was not run.

The legacy single-composition `.aeproj` load/save APIs enforced a 100 MiB JSON size limit but bypassed ProductionDocument content validation. Malformed dimensions, non-finite animation values, invalid masks, particle settings, or broken precomposition references could therefore reach renderer and CLI paths. Those APIs now share the existing standalone composition validation rules on both load and save, returning `InvalidData` for invalid input and `InvalidInput` for invalid save data. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. Automated tests were not run.

User export presets were loaded with unbounded `read_to_string` and had no count ceiling, so a damaged or oversized per-user JSON file could consume excessive memory while opening export controls. Reading now checks metadata and uses a bounded read (4 MiB maximum), rejects more than 4,096 presets, and saving enforces the same count and byte limits before atomic publication. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. Tests and UI interaction were not run.

The export dialog discarded errors from the “Save Current as Preset” action, so hitting preset count or disk/write limits appeared to do nothing. The dialog now retains and shows a success or error message after the action. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. UI interaction was not run.

Export preset selection previously ignored the stored codec, treated every unknown format as H.264/MP4, guessed a resolution scale from width alone, and did not restore the audio toggle. The dialog now records the selected codec and effective resolution, restores only representable H.264, ProRes 422/4444, PNG-sequence, and WebP presets (including the WebP format identifier), restores FPS/audio, and updates the output extension where applicable. Unsupported formats, codec combinations, and dimensions that do not exactly match the dialog's available scales are visibly disabled instead of silently becoming MP4. Preset quality now reaches H.264/H.265 CRF and WebP quality settings; AAC bitrate reaches the muxer. Export configuration validates quality and bitrate ranges, and non-dialog call sites retain the established H.264/H.265 and AAC defaults. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. UI interaction and rendered-output verification were not run.

Opening the export dialog re-read and reparsed the user preset file on every UI frame. The bounded preset list is now cached for one second, with cache invalidation after a successful save. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. UI responsiveness was not benchmarked.

Preset loading errors were converted to an empty list and only written to the log, making a corrupt or over-limit user file look like there were simply no presets. The cached loader now preserves and returns its error, and the export dialog shows the reason beside the preset control. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. UI interaction was not run.

Global font-rasterizer initialization read and retained every discovered system font without per-file, total-memory, or font-count limits. Font loading now uses bounded reads (64 MiB per file), retains at most 1,024 fonts and 256 MiB of font data, and discovery stores at most 4,096 candidate paths. Custom font-byte loading applies the same per-file and retained-memory ceilings. These limits bound startup and per-user font memory while keeping already-loaded fonts usable after the ceiling is reached. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check` passed. Real system-font startup and text rendering were not exercised.

Font family identifiers were unbounded even after font bytes, count, and discovery paths were capped. Public font-loading calls could retain an arbitrarily large family-name key or build oversized candidate paths. Byte-backed and system-font loading now reject empty or over-256-byte family names before parsing or path lookup. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, 800-line source audit, and `git diff --check`; automated tests and runtime font loading were not run.

Project validation checked text size and style values but not either persisted font-family field. Oversized names could be cloned into the render path even though the font loader rejects them. Standalone and production project validation now cap both the base text-layer family and rich-text formatting family at 256 UTF-8 bytes, aligned with the font loader's accepted identifier limit. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests and project round trips were not run.

Rich-text formatting was not validated at the project boundary. Non-finite tracking, leading, box dimensions, or stroke values and out-of-range alignment could reach text layout; oversized family names were also copied before rasterizer fallback. Added shared font-family byte limit and validation for rich-text family, tracking, leading, stroke color/width, paragraph alignment, and box dimensions. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests and project round trips were not run.

The font-family limits on project validation and font insertion did not cover direct renderer API calls: `resolve_family` normalized an arbitrarily long request into a new string, and `rasterize_glyph` copied the requested family into a cache key before checking whether it was loaded. Family normalization now runs only for names within the shared limit, and glyph lookup rejects unloaded families before constructing a cache key. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; automated tests and runtime rendering were not run.

The primary Studio video-export worker always ran the full-range audio mixer, even when the composition contained no audio-bearing layer. The mixer later removed its silent WAV and returned `None`, so this did not add a silent AAC stream, but it still performed unnecessary per-frame work and reported that audio was being prepared. Export now uses the shared recursive audio-source check to skip the mixer for compositions without audio and reports the correct status. Validation: touched-file `rustfmt`, all-target/all-feature `cargo check`, strict all-target/all-feature Clippy, and `git diff --check`; export timing and container output were not measured.

Primary Studio video export exposed only a codec cycle and hard-coded quality/audio policy. Added a 0–100 quality slider and an audio-inclusion checkbox; the quality value is rounded/clamped at the action boundary and passed into `ExportConfig`, while audio inclusion gates the recursive audio-source check and WAV mixer. Validated the full embedded Studio source with the current AmeUI compiler (`amec check src/ui/ame/kagari_studio.ame`); this also exposed one surplus closing brace left by the existing VirtualList conversions in each of `asset_panel.ame` and `scene_panel.ame`, which were removed. Rust all-feature/all-target `cargo check`, strict Clippy, and `git diff --check` passed. UI runtime, encoder output quality, and muxed audio were not exercised.

During video export, the primary Studio left the output path, codec, quality, and audio controls editable even though the worker had already captured those values. A user could change the displayed destination while the active job continued writing to the original path, then see a completion message naming the wrong path. The UI now presents captured export settings as read-only while rendering, and host action handlers reject path, codec, quality, and audio changes during any export. Validation: all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, current AmeUI `amec check src/ui/ame/kagari_studio.ame`, touched-file `rustfmt`, source-size audit, and `git diff --check`; no export UI interaction or output file was exercised.

Paragraph formatting exposed Justify, but the software text renderer mapped alignment value 3 back to Left; the layout helper also did not distribute inter-word spacing. Alignment now reaches all plain, character-animated, and stacked-animator raster paths. Soft-wrapped non-final paragraph lines distribute remaining width over spaces; final paragraph lines retain natural spacing. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, source-size audit, and `git diff --check` passed. Rendered text samples and UI interaction were not inspected.

The text wrapper collected every word into a temporary vector and cloned each flushed line before emitting it. It now streams words from the paragraph iterator and moves flushed line buffers into the result, reducing temporary allocations for long text blocks. Validation: all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, and `git diff --check` passed; text rendering and runtime performance were not measured.

Center and right alignment in plain, character-animated, and stacked-animator text rendering used the layout engine's approximate line width, while glyph placement uses the selected font's actual advances. This could visibly shift text for fonts whose metrics differ from the approximation. All three paths now align using the same measured advances and tracking used to position their glyphs. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, and `git diff --check` passed. Rendered samples were not visually compared.

OCIO config loading checked file metadata before calling unbounded `read_to_string`; a config or referenced LUT that grew during loading could exceed the 64 MiB limit. A config with many LUT references could also retain an excessive candidate-path list. Both text reads now enforce the byte cap while reading, and config scanning keeps at most 4,096 LUT candidates. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, and `git diff --check` passed. File-growth races and candidate-limit behavior were not exercised with fixtures.

The public OCIO `Lut3D` sampler indexed backing data without validating that the declared dimensions fit or that the data contained a complete cube, so a manually constructed malformed LUT could panic during render. It now passes input through for malformed tables and handles one-point tables as constants. The previous `s - 0.001` clamp also kept normalized input 1.0 short of the final grid point; interpolation now clamps to the inclusive endpoint and limits the base index to the last valid cell. NaN input channels normalize to zero. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, and `git diff --check` passed. Malformed and endpoint samples were not runtime-tested.

The separate `color_science::Lut3D` used by the main color pipeline had the same `s - 0.001` endpoint truncation. Its interpolation now also includes the maximum lattice coordinate and clamps the base index to the final valid cell, matching the OCIO LUT sampler. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, and `git diff --check` passed. LUT endpoint output was not runtime-tested.

GPU mask scan conversion allocated a fresh crossing list for every output row. It now retains one list for the polygon and clears/reuses it per row, avoiding repeated heap allocation during high-resolution mask previews. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, and `git diff --check` passed. Preview performance was not benchmarked.

GPU mask feathering allocated full-frame coverage, distance-transform, and output buffers infallibly; output-size multiplication was also unchecked. Those working arrays now use fallible reservations, mask rasters reject dimensions above 9,437,184 pixels (enough for 4,096×2,304), and the render path handles allocation/budget refusal without panicking or uploading an incomplete matte. On failure it clears the selected target to the composition background using the same clamped premultiplied RGBA values as a normal render, then records the failed render key so the UI does not keep showing an older frame or retry the same deterministic failure on every repaint. Existing mask unit helpers were adapted to the fallible return values. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check` (including test-target compilation), strict all-feature/all-target Clippy, source-size audit, and `git diff --check` passed. No tests or GPU preview were run.

OBJ polygon faces were triangulated as a fan, which can place triangles outside a concave face and corrupt imported models. Convex faces retain the linear fan path; concave faces are projected onto their dominant plane and triangulated with ear clipping while preserving source winding. Concave faces above 4,096 vertices and degenerate/self-intersecting faces return an import error rather than silently producing invalid geometry. The triangulator lives in its own module to preserve the 800-line source limit. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check` (including test-target compilation), strict all-feature/all-target Clippy, source-size audit, and `git diff --check` passed. Concave model fixtures and rendered imports were not exercised.

On an OBJ mesh-cache miss, the renderer read the whole OBJ to discover material dependencies, then `parse_obj_file` read it again and called the dependency scanner, causing a third full-file read. OBJ references could also list the same MTL repeatedly or an excessive number of distinct MTL paths. The loader now creates one bounded `ObjFileSource` containing the parsed OBJ text and up to 1,024 unique MTL paths (each path capped at 4,096 bytes); cache fingerprinting and mesh parsing share that source. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check` (including test-target compilation), strict all-feature/all-target Clippy, source-size audit, and `git diff --check` passed. Large OBJ load I/O and duplicate-reference behavior were not runtime-profiled.

Roto Brush optical-flow propagation accepted a three-or-more-point anchor path without checking its coordinates, unlike the tracked-mask bake API. A malformed path could therefore place non-finite vertices into the initial keyframe and contaminate propagated geometry. The Roto Brush entry point now rejects non-finite boundary coordinates before rendering or allocating flow buffers. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check` (including test-target compilation), strict all-feature/all-target Clippy, 800-line source audit, and `git diff --check`; no tests or runtime propagation were run.

`DenseFlowField::new` allowed up to 16,777,216 pixels but allocated its vector buffer infallibly, so memory pressure at an otherwise valid public input size could abort the process. It now reserves fallibly and leaves the vector empty on allocation failure; `compute_dense_optical_flow` already rejects a field whose storage does not match the requested dimensions. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check` (including test-target compilation), strict all-feature/all-target Clippy, source-size audit, and `git diff --check`; allocation-failure injection and runtime optical-flow output were not exercised.

After making dense-flow allocation fallible, two callers still interpreted an incomplete field as valid zero motion: Roto Brush could bake a stationary matte, and markerless tracking could report stationary motion with high confidence. Added a shared completeness check; Roto Brush now fails the bake with the affected frame identified, while markerless tracking preserves the prior position and records zero confidence for that frame. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, source-size audit, and `git diff --check`; allocation failure and runtime tracking behavior were not injected or exercised.

The matte generator continued into color classification when a frame had background strokes but no usable foreground points. Its white fallback foreground color could classify unmarked white areas as foreground, producing a phantom matte despite the absence of a foreground seed. The generator now returns a bounded zero matte in that case, matching the explicit foreground-seed requirement in the contour and propagation paths. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, 800-line source audit, and `git diff --check`; no tests or UI matte preview were run.

The default AmeUI Roto Brush worker mapped every `None` from cancellable matte generation to “analysis cancelled,” although `None` also represents fallible working-buffer allocation failures. The worker now checks the cancellation flag and reports a matte-generation failure for non-cancellation errors, preserving the actionable distinction for users. Its frame-analysis worker method moved into `roto_brush/analysis_worker.rs`, bringing the host module below the 800-line limit while keeping asynchronous analysis together. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, source-size audit, and `git diff --check`; neither allocation failure nor the Studio notification was runtime-exercised.

Roto Brush analysis and propagation workers could send a successful result, then receive a cancellation request before the UI handled the ready event. Completion handlers did not re-check the job's cancellation flag, so the canceled result could still modify the project. Both handlers now discard a completed result when cancellation was requested before application. Validation: touched-file `rustfmt`, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, source-size audit, and `git diff --check`; the completion/cancel interleaving was not runtime-reproduced.

The AmeUI export panel exposed a 0–100 quality slider for ProRes even though the FFmpeg ProRes branches use fixed codec profiles and never consume that value. The panel now shows the slider only for H.264/H.265, explains that it does not apply to ProRes, and the action handler ignores stale or direct quality-change events while ProRes is selected. Validation: AmeUI compiler check, all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, 800-line source audit, and `git diff --check`; encoded output and interactive codec switching were not exercised.

Video and GIF exports honored cancellation while FFmpeg was running but did not check the flag during output sync and final publication. A cancel request after FFmpeg exited successfully could therefore still replace or create the destination while the UI showed cancellation in progress. Added a cancel-aware publication path that checks cancellation after syncing and immediately before atomic publication; the cross-filesystem copy fallback also checks between chunks and before rename. The publication and file-identity code lives in `types/publication.rs`, keeping the shared export type module under the 800-line limit. The temporary output guard removes unpublished files and the UI now reports this path as cancellation. Validation: all-feature/all-target `cargo check`, strict all-feature/all-target Clippy, source-size audit, and `git diff --check`; cancellation during final sync/copy and actual video/GIF publishing were not runtime-exercised.

Text-to-shapes conversion assigned every glyph contour an Add mask, so the inner counters of procedural glyphs such as A and O were filled in the resulting shape. Contours now use their nesting depth to alternate Add and Subtract, preserving holes and nested islands while leaving overlapping, non-contained outlines as additions. Validation: macOS `cargo check --no-default-features --features gui --bin kagari-studio-egui`, strict library Clippy for the GUI feature, the 800-line source audit, and `git diff --check` passed. No automated tests were run and the converted glyphs were not visually inspected; the current procedural glyph source still limits outline fidelity to its generated contours.

The same conversion path claimed font outline extraction but always substituted a small procedural glyph set, so most text converted to boxes or inaccurate approximations. It now reads outlines, advances, kerning, and ascent from the loaded/resolved font via `ab_glyph`, flattens quadratic/cubic curves to bounded polygons, and applies counter subtraction per glyph. This avoids a global all-contour comparison that would become quadratic for long text. Conversion refuses runs above 25,000 characters, a glyph above 256 contours, or generated outlines above the curve/point budgets; missing fonts retain the procedural fallback. Both legacy entry points now report a clear size/complexity error instead of claiming no text layer was selected; the original Option-returning API remains as a compatibility wrapper. Validation: macOS GUI Studio `cargo check`, strict GUI-bin Clippy, the 800-line source audit, and `git diff --check` passed. No tests were run and converted font samples were not visually inspected; stroke fidelity and exact curve retention remain unverified because the mask API stores polygons rather than Bézier curves.

Text layer strokes were converted as rectangle strokes around the shape layer, while the CPU stroke effect ignored width and position; the GPU shader also did not implement the selected inside/center/outside position. Replaced the CPU implementation with an alpha distance-transform stroke using requested width, color, opacity, and position; expanded the software layer raster bounds for outside strokes; and route enabled layer-style strokes to the CPU renderer so preview/export use that implementation. Text-to-shape conversion now transfers the effective formatted text stroke to the layer-style stroke path instead of the rectangle primitive. Validation: macOS legacy GUI Studio `cargo check`, strict GUI-bin Clippy, the default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, the repository 800-line source audit, and `git diff --check` passed. No automated tests or rendered stroke samples were run; exact edge coverage and performance remain visually unverified.

Text-to-shape conversion used the raw text string even though live rendering applies `all_caps`, `small_caps`, and faux italic from `TextFormatting`. Conversion now uses the same uppercased text and applies the matching 0.84 small-caps scaling and faux italic shear to sampled outlines. REQ-353 was reviewed in the AmeUI request ledger and already contains the OS-input ownership boundary, acceptance criteria, and remaining work, so no duplicate request was filed. Validation at that stage: default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, repository 800-line source audit, and `git diff --check` passed. No visual conversion sample was inspected; faux bold, text-box wrapping/vertical overflow, per-character formatting, and exact Bézier retention still differ from live text rendering.

The font-outline and procedural text-to-shape paths split only explicit newline characters, so formatted text with a paragraph box converted as a single unwrapped line even though live rendering applied word wrapping and box alignment. Both paths now use the shared paragraph layout to wrap, align, and distribute justified spaces; outline placement still uses the selected font's advances and kerning. Validation: default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, repository 800-line source audit, and `git diff --check` passed. No converted text samples were visually compared; vertical box clipping, faux bold, per-character formatting, and exact Bézier retention remain open.

Font outlines included pair kerning, but plain, character-animated, and stacked-animator raster text advanced only by each glyph's horizontal advance. The stacked-animator path also measured base tracking without applying it to glyph positions. Live text and text-to-shapes therefore differed on kerning pairs and tracking. All three raster paths now use scaled font kerning when measuring and advancing glyph placement; the stacked path also applies the base tracking value. Validation: default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, repository 800-line source audit, and `git diff --check` passed. No rendered typography samples were visually compared and no automated tests were run.

The particle time-remap transform mapped particle centers and trail points between source-time and current-time layer transforms, but left particle radii unchanged. Scaling or resizing a particle layer therefore moved particle positions without scaling their visible size. Particle sizes now follow the current layer scale using the geometric mean of its axis scales, which preserves approximate particle area when the layer scale is non-uniform. A follow-up review corrected the initial relative-ratio approach so constant layer scaling is applied too. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, 800-line limit check for the touched source file (679 lines), and `git diff --check` passed. No automated tests or rendered scale samples were run, so the non-uniform-scale approximation still needs visual confirmation.

Particle simulation review found two repeated per-step costs inside `ParticleSystem::update`: cloning the gravity curve's `Vec<f32>` every simulation frame and recomputing the same gust wind vector once for every live particle. The curve is now borrowed for evaluation, and the frame's gust vector is computed once before iterating particles. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, source line-count check (under 800 lines), and `git diff --check` passed. No benchmark or runtime comparison was run; the change removes one curve allocation/copy per simulation step and reduces gust evaluation from once per particle to once per step by inspection.

Death-spawn processing also collected every dead particle's position into a temporary vector on each simulation step. It now snapshots the original particle count and reads each dead parent by index while appending children, preserving parent order and random-number consumption while eliminating the temporary allocation. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, the 800-line limit check, and `git diff --check` passed. No tests or particle-death render were run.

Roto Brush propagation checked cancellation between flow and matte stages but not inside RGBA area-downsampling. On a large frame, canceling while reducing an anchor or neighboring frame could remain unresponsive until that full reduction finished. Downsampling now checks the cancellation callback once per output row and propagates cancellation as a canceled bake rather than an allocation error. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, touched-file source-size check (under 800 lines), and `git diff --check` passed. No test or large-frame cancellation timing measurement was run.

Cancellation coverage for Roto propagation now includes dense block-matching optical flow. The previous and reverse flow searches both check cancellation between block rows, and the propagation path reports cancellation separately from an incomplete allocation. The existing non-cancellable flow API remains available to other callers. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, all-Rust source-size audit (998 files, none over 800 lines), and `git diff --check` passed. No automated tests or large-frame cancellation timing measurement were run.

The public particle `draw_dot` raster helper computed pixel offsets with `u32` multiplication and did not check whether the supplied RGBA slice was large enough for its dimensions. Mismatched external calls could wrap indices on large dimensions or partially modify an undersized buffer. It now validates checked RGBA size before drawing and uses `usize` for pixel offsets. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, source line-count check, and `git diff --check` passed. No tests or external malformed-buffer calls were run.

Follow-up review corrected the particle-size transform implementation: the emitted size is authored in layer-space units and is not pre-scaled at simulation time, so applying only the current/source scale ratio failed to scale layers with an unchanged transform. The CPU pass now applies the current absolute layer scale to particle radii even when transforms match; non-uniform axis scaling uses a geometric-mean area approximation because raster particles are circles. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, 800-line source check (670 lines), and `git diff --check` passed. No rendered samples were inspected; non-uniform scaling remains an approximation.

The software preview downsized the completed straight-alpha RGBA frame by filtering RGB and alpha independently. Colors stored in fully transparent pixels could therefore bleed into partially transparent edges and create colored/dark halos. Preview resizing now premultiplies RGB by alpha before the Triangle filter and unpremultiplies afterward, clearing RGB where output alpha is zero. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, touched-file source-size check (under 800 lines), and `git diff --check` passed. No transparent-edge preview was rendered for visual inspection; high-contrast fringe behavior remains unverified in the application.

Particle trails were sampled immediately after integration, before axis-boundary and particle-particle collision resolution. On collision frames the rendered particle was moved but its newest trail point stayed at the unresolved position, creating a visible gap or a trail extending through the collision boundary. Trail history now records each live particle's final resolved position after both collision passes. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, source line-count check (under 800 lines), and `git diff --check` passed. No collision/trail sample was rendered and automated tests were not run.

Viewport fallback selection matched cached frames by project revision and preview dimensions but omitted the external-media revision used by exact preview keys. After a source file changed, playback could therefore temporarily show a cached frame rendered from the old media until the current frame finished rendering. Fallback candidates now also require the current media revision, so stale-media images are not presented as current. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, touched-file source-size check, and `git diff --check`; external media replacement and viewport playback were not runtime-exercised.

Roto Brush propagation still had a non-cancellable full-resolution stage after optical-flow warping: the dilated motion-prior constraint rasterized the prior boundary, applied correction strokes, and ran horizontal/vertical distance transforms across the whole frame without consulting the job cancellation flag. Added a cancellable variant and wired propagation into it, checking each image row/column and passing the callback through correction-stroke rasterization; the existing non-cancellable helper remains available to unit-test callers. Validation: touched-file `rustfmt`, default Studio `cargo check`, strict Studio Clippy, source-size and `git diff --check`; automated tests and large-frame cancellation latency were not run.

Follow-up to the Roto cancellation audit: thresholding the generated full-resolution alpha matte also traversed the entire pixel buffer without a cancellation check. The pass now processes one row at a time, checks cancellation between rows, and rejects zero-sized frames before forming row chunks. Validation: touched-file `rustfmt`, default Studio `cargo check`, strict Studio Clippy, and `git diff --check`; cancellation timing and runtime matte output were not exercised.

Roto propagation cancellation reached the frame-processing callback, but both Studio frame providers called `render_frame_to_pixels_filtered` without installing the renderer's cooperative cancel flag. Canceling during an expensive source-frame render therefore waited for that render to finish. The AmeUI Studio and legacy egui frame providers now wrap each source render with `with_render_cancel_flag` using the propagation job's shared atomic flag, allowing renderer cancellation checks to stop the in-flight frame. Validation: touched-file `rustfmt`, AmeUI Studio and legacy egui `cargo check`, strict Clippy for both Studio binaries, and `git diff --check`; cancellation during a real large-frame render was not runtime-measured.

The software renderer's first full-frame pass initialized every RGBA pixel to the composition background before checking its cooperative cancellation flag. At the 512 MiB render-buffer ceiling, canceling before layer processing could still wait for that entire write. Background initialization now checks cancellation every 4,096 pixels and returns an empty frame when canceled, matching downstream cancellation callers' incomplete-render handling. Validation: touched-file `rustfmt`, default AmeUI Studio and legacy egui `cargo check`, strict Clippy for both Studio binaries, and `git diff --check`; maximum-size cancellation latency was not benchmarked.

Documentation audit found the video-timebase fix had already landed across source paths, with follow-up validation recorded above, but its original section still labeled the issue open and the feature matrix omitted the fix. Updated the section status and media-import summary after auditing the shared conversion call sites in CPU/GPU render, matte/precomp, audio, tracking, capability, preview, waveform, thumbnail, and media-revision paths. Validation: targeted call-site search and `git diff --check`; no source code or automated tests changed in this documentation-only follow-up.

Expression evaluation during `prepare_layer_render_data` ran inside Rayon workers, while `audio_reactive::inject_audio_reactive_data` installed the current frame's amplitude/bands only in the renderer thread's thread-local state. Audio-driven transform expressions could therefore evaluate with worker-local defaults and vary depending on which thread executed the layer. The current audio expression snapshot is now copied into each layer-preparation worker through a panic-safe scoped context that restores the worker's prior values afterward. Validation: touched-file `rustfmt`, default AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, source-size audit, and `git diff --check`; parallel audio-expression rendering was not runtime-tested and no tests were run.

The GPU preview's `WgpuRenderer::render_internal` evaluates expression-driven transforms and effects but never installed the frame's audio-reactive thread-local values; the software renderer did. Legacy GPU previews could therefore disagree with software/export for audio-driven expressions and could inherit default or stale thread-local values. GPU rendering now runs the same audio-reactive frame preparation before evaluating layer state. Validation: touched-file `rustfmt`, legacy egui GPU Studio `cargo check`, strict GUI-bin Clippy, source-size audit, and `git diff --check`; GPU/software audio-reactive pixels were not runtime-compared and automated tests were not run.

GPU capability review found that `Composition::blend_linear` was part of the render cache identity but absent from the GPU fallback decision. The WGPU pipeline uses standard source-over blending and does not implement the software renderer's linear-light compositing, so enabling the project option silently displayed the wrong blend result. Capability detection now routes linear-light compositions to the CPU renderer before the GPU draw. Validation: touched-file `rustfmt`, legacy egui GPU Studio `cargo check`, strict GUI-bin Clippy, source-size audit, and `git diff --check`; no linear-light comparison frame was rendered and automated tests were not run.

A broader GPU capability audit found two more composition output settings that are applied by the software renderer but absent from the WGPU render path: conversion from a non-Rec.709 working space and triangular-PDF output dithering. GPU preview selection now requires software rendering when either setting is enabled, preserving the composition's configured color transform and banding treatment. Validation: touched-file `rustfmt`, legacy egui GPU Studio `cargo check`, strict GUI-bin Clippy, source-size audit, and `git diff --check`; color-space/dither preview pixels were not runtime-compared and automated tests were not run.

Material settings review found that the exposed `accepts_lights` option was ignored by flat image/text/shape layer shading, and `accepts_shadows` was ignored by both software post-processing and the WGPU fragment shader. Flat-layer lighting now skips light contributions when `accepts_lights` is disabled while preserving material ambient/emission, and both renderers honor `accepts_shadows`. The existing uniform padding slot carries the shadow flag, preserving the Rust/WGSL uniform layout. Validation: touched Rust formatting, legacy egui GPU Studio `cargo check`, strict GUI-bin Clippy, file-size audit (all touched source files under 800 lines), and `git diff --check`; CPU/GPU material output was not visually compared and automated tests were not run.

The 3D material inspector exposed `light_transmission`, but the software shadow-map caster ignored the value, so changing it had no visible effect. Each caster now scales its shadow density by `1 - clamp(transmission / 100, 0, 1)` and skips fully transmitting casters. Validation: legacy egui GPU Studio `cargo check`, strict GUI-bin Clippy, touched-file size (426 lines), and `git diff --check` passed; translucent shadow appearance was not runtime-rendered and automated tests were not run.

Follow-up review of the separate 3D surface shading API found it used `MaterialOptions.cast_shadows` to decide whether a surface receives a shadow. That flag controls casting; the receiver flag is `accepts_shadows`. The shading path now checks the receiver flag. Validation: AmeUI Studio `cargo check` passed with an unrelated existing unused-import warning in `software_renderer/mod.rs`; strict AmeUI Clippy is blocked by that same warning. Strict legacy egui GUI-bin Clippy and `git diff --check` passed; no 3D surface render was visually inspected and automated tests were not run.

The AmeUI-only build exposed a renderer audio-context re-export that was unconditional even though its only consumer is the feature-gated WGPU renderer. This emitted an unused-import warning without `wgpu`, causing the strict AmeUI Studio Clippy gate to fail. The re-export is now guarded by the same feature. Validation: strict AmeUI Studio Clippy and `git diff --check` passed; file size is 342 lines. Packaging/CI review found the workflow's AmeUI build flags and binary paths align with the packaging script; no artifact-path mismatch was found.

AmeUI integration review found the primary Audio Mixer emitted one full component subtree per audio/video-audio layer. ProductionDocument accepts up to 100,000 composition layers, and unlike the timeline/project hierarchy the mixer did not use the available windowed `ForEach`, so large sessions could inflate the Ame ViewTree. The mixer now declares a 300 px viewport, 54 px estimated rows, and four-row overscan, matching its two compact control rows. Validation: AmeUI `amec check` of the complete Kagari root app (including imported modules) passed, `git diff --check` passed, and the component is 29 lines. No large-session UI was opened; the Rust binding still constructs the full audio-track value list and may warrant a future data-bridge optimization if profiling shows it dominates.

Follow-up on the Audio Mixer virtualization: the public AmeUI API reports visible ranges to the runtime but does not expose a range-backed binding provider to app hosts. Full `List<Record>` construction and selection/key scans therefore remain for large data sources even when the ViewTree is windowed. Added AmeUI **REQ-355** with the 100,000-layer reproduction, API contract, owner split, and measurable acceptance criteria to `/Users/dev/Desktop/AI-SLOP-Family/AmeUI/リクエスト.md`; Kagari tracks only the host migration after that API lands. AmeUI ledger `git diff --check` passed.

AmeUI host API audit found Kagari's `AfterEffectsStudio` implemented only `shared_view_tree()`, which calls `AmeRuntime::view_tree()` and clones the cached `ViewTree` on each host request. AmeUI exposes `try_shared_view_tree_arc()` specifically so the native event loop can retain the runtime's cached `Arc<ViewTree>`. Kagari now forwards that method to its `AmeRuntime`, avoiding the full tree clone on the primary host path while preserving the owned-tree fallback. Validation: AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, source size (318 lines), and `git diff --check` passed; no large-tree allocation benchmark was run.

Large-list UI audit found more Ame components expanding candidate/target rows without a window: track-matte source layers and motion-tracker source/target layers can each approach the 100,000-layer composition limit; masks and per-layer effects have 10,000-item validation ceilings. These vertical lists now use bounded windowed `ForEach` viewports with stable keys and row estimates. Validation: `amec check` of the complete Kagari app passed, AmeUI Studio `cargo check` passed, all edited components are under 120 lines, and `git diff --check` passed. No long-list UI was opened, so nested-scroll usability and row-height estimates still need application-level inspection.

The remaining horizontal full-expansion audit also found Kagari's parent-layer candidate chips: both Scene Panel and Project Panel build candidates from the full layer stack and place every variable-width button in `HorizontalScrollView`. Extended AmeUI **REQ-354** to include this third consumer and its selection acceptance condition; `REQ-355` separately tracks the full-list host data cost. The AmeUI request ledger passes `git diff --check`.

Parent-candidate generation in the Studio host walked each candidate's ancestor chain and searched the entire layer vector for every parent step. With a large/deep hierarchy this could degrade toward O(layer_count × hierarchy_depth) on each scene-binding refresh. It now builds a reverse parent→children index once and marks the selected layer's descendants with a visited set, then filters candidates in one pass. This also terminates safely on malformed cycles. Validation: touched-file `rustfmt --check`, AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, source size (549 lines), and `git diff --check` passed; no 100k-layer timing benchmark was run.

The Audio Mixer is only present in the Audio workspace, but `sync_scene_asset_bindings` rebuilt its audio-track records for every Studio binding refresh in every workspace. With large layer counts this added an avoidable full-layer scan and record allocation during unrelated editor work. Kagari now populates that binding only while `Workspace::Audio` is active and clears it elsewhere; workspace changes trigger the normal binding synchronization before the mixer is shown. Validation: touched-file rustfmt check, AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, source size (554 lines), and `git diff --check` passed. Workspace-switch UI behavior and allocation savings were not runtime-measured.

`scene_assets` was also rebuilt on every Studio binding refresh, including workspace layouts that do not show `SceneAssetPanel`; each media layer triggered filesystem metadata checks for its source and possibly frame directory. The host now clears/skips this list in Composition, Camera/Light, Color, Layer Style, and Text workspaces, where the source is not rendered. It continues to populate in asset/scene workspaces and the layouts that include `AssetPanel`. Validation: touched-file rustfmt, AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, source size (567 lines), and `git diff --check` passed. Workspace coverage and skipped filesystem-call counts were not runtime-measured.

Follow-up in the same scene-binding pass found three more linear searches over `composition_layers` for the selected layer, current parent, and parent name even though a layer-by-ID index had already been built at the start of the pass. The existing index is now reused for those lookups, and the same shared layer slice is used for scene presence and alignment checks. Validation: touched-file rustfmt check, AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, repository source-size gate, and `git diff --check` passed.

The large-scene hierarchy sync repeated ancestry work in two places: every matching search row separately walked its parents, and each uncollapsed row walked ancestors to test hidden state. Search visibility now propagates upward from all matched rows with a shared visited set, so each visible ancestor is processed once. Collapse filtering builds a parent→children index and collects hidden descendants once per sync, and skips the index entirely when there are no collapsed layers or a search is active. Existing per-row helper remains for single-action callers. Validation: touched-file rustfmt check, AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, repository 800-line source limit, and `git diff --check` passed. Search/collapse interaction was not runtime-exercised and no tests were run.

Property-binding audit found two hidden candidate lists rebuilt even when their panels had nothing to show: track-matte source rows were generated whenever a layer was selected, even with track matte disabled; tracker target rows were generated from every composition layer even when no tracker existed. Matte sources now build only when the selected layer has an active matte mode, and tracker target layers only when the selected layer has tracker rows. Validation: touched-file rustfmt check, AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, touched-file size audit, and `git diff --check` passed. Switching matte/tracker controls in the running Studio was not exercised.

Scene hierarchy synchronization built the same parent→children index independently for collapse visibility and parent-candidate filtering. It now shares one index per binding refresh and skips construction when neither an active collapsed-group filter nor a selected layer requires descendant traversal; collapsed state does not trigger indexing during search, when collapse filtering is inactive. Validation: touched-file `rustfmt` check, AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, repository 800-line source limit, and `git diff --check` passed. No large-scene allocation benchmark or live hierarchy interaction was run.

The software viewport rendered the full composition-sized buffer before resizing, even for basic 2D compositions at quarter resolution. Added a direct target-resolution path for 2D layers without effects, track mattes, precomps, particles, paint, 3D, or layer styles; it scales layer transforms and mask geometry before rasterization. Other compositions retain the existing full-resolution path until their effect and nested-render coordinates are scaled. Validation: AmeUI Studio `cargo check`, strict AmeUI-bin Clippy, touched-file formatting, repository 800-line source audit, and `git diff --check` passed. No automated tests or runtime image/memory benchmark were run, so visual parity and actual frame-time savings remain unmeasured.
