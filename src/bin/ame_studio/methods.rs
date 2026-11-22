// src/bin/ame_studio/methods.rs
//
// This file previously contained ~6,700 lines of monolithic methods.
// All operations have been refactored and modularized into dedicated domain modules:
// - layer_ops.rs: Layer lifecycle, reordering, duplicate, split, parenting, transforms, easing
// - shape_ops.rs: Parametric shapes, strokes, fills, trim paths, repeaters
// - camera_light_ops.rs: 3D Camera & Light management, depth of field, shadows
// - particle_ops.rs: Particle systems & emitter properties
// - audio_video_ops.rs: Time remapping, posterize time, audio volume keyframing
// - text_ops.rs: Text layers, text animators, typography
// - mask_ops.rs: Mask manipulation & keyframing
// - composition_ops.rs: Composition settings & tree cloning
// - media_import.rs: Asset placing, proxies, media import
// - project_io.rs: Project loading/saving, templates, presets
// - playback_ops.rs: Playhead scrubbing, playback loop, work area
//
