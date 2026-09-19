# Kagari VFX

Open-source CLI-first VFX & compositing engine.
Built in Rust with GPU acceleration, headless rendering, and automation-friendly tooling for AI agents.

Kagari is designed to handle the VFX work that code-first video tools eventually outgrow — compositing, effects, tracking, particles, masks, 3D, and more — without requiring a proprietary editor.

---

## Quick start

```bash
git clone https://github.com/AI-SLOP-BOX/kagari.git
cd kagari
cargo run --release --features gui --bin kagari-studio
```

## CLI

```bash
kagari render --project project.json --comp main --from 120 --to 120 \
  --format png --output ./output
kagari frame --project project.json --frame 120 --output frame.png
kagari effects
kagari lottie --project project.json --output lottie_export.json
```

## What it is

Kagari VFX is a node-based compositor and motion graphics engine written in Rust.
The CLI can render PNG sequences, MP4, and GIF output, export Lottie JSON,
and run headlessly for scripting or automation. ProRes and MLT are available
through the library export modules; they are not separate CLI subcommands yet.

## Stack

- **Rust + wgpu** — GPU compositing with Metal backend on macOS
- **egui** — dark professional UI
- **FFmpeg** — decode / encode pipeline
- **Rhai** — embedded expression engine for animatable properties
- **rayon** — parallel CPU effects

## License

MIT OR Apache-2.0. Do whatever you want.
