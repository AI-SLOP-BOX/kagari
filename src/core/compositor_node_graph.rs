//! DaVinci Fusion / Nuke-style Compositing Node Graph (DAG) evaluator.
//!
//! Connects `MediaIn`, `ColorWheelGrade` (3-Way Lift/Gamma/Gain/Offset),
//! `DeepMerge` (Z-depth / multi-sample deep compositing), `OverMerge`,
//! `OpenFx` (via [`OpenFxPluginBridge`]), and `MediaOut` into an acyclic
//! execution graph with cycle detection and optional GPU acceleration.

use std::collections::{HashMap, VecDeque};

use crate::core::color_wheel_gpu::{apply_color_wheel_grade_cpu, ColorWheelGradeParams};
use crate::core::deep_compositor::DeepImage;
use crate::core::openfx_bridge::{OfxStatus, OpenFxPluginBridge};
use crate::core::pro_keyer::{apply_pro_keyer_cpu, ProKeyerParams};
#[cfg(feature = "wgpu")]
use crate::core::saber_fx_gpu::{render_saber_gpu, SaberParams};
#[cfg(feature = "wgpu")]
use crate::core::sapphire_fx_gpu::{apply_sapphire_optical_gpu, SapphireOpticalParams};

pub type NodeId = u64;

/// Linear HDR `RGBA32F` + optional scalar `Z` depth buffer flowing between nodes.
#[derive(Debug, Clone)]
pub struct FrameBufferF32 {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<f32>,
    pub depth: Option<Vec<f32>>,
}

impl FrameBufferF32 {
    pub fn new(width: u32, height: u32, rgba: Vec<f32>, depth: Option<Vec<f32>>) -> Result<Self, String> {
        let count = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| "FrameBufferF32 dimensions overflow".to_string())?;
        if rgba.len() != count * 4 {
            return Err(format!(
                "rgba len {} != {}x{}x4",
                rgba.len(),
                width,
                height
            ));
        }
        if let Some(d) = &depth {
            if d.len() != count {
                return Err(format!("depth len {} != {}x{}", d.len(), width, height));
            }
        }
        Ok(Self {
            width,
            height,
            rgba,
            depth,
        })
    }
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    MediaIn,
    ColorWheelGrade(ColorWheelGradeParams),
    Transform2D {
        translate: [f32; 2],
        scale: [f32; 2],
        rotate_deg: f32,
    },
    GaussianBlur {
        radius: f32,
    },
    DeepMerge { z_epsilon: f32 },
    DeepHoldout,
    OverMerge,
    ProKeyer(ProKeyerParams),
    OpenFx { plugin_name: String, frame: f64 },
    #[cfg(feature = "wgpu")]
    SapphireOptical(SapphireOpticalParams),
    #[cfg(feature = "wgpu")]
    SaberBeam {
        width: u32,
        height: u32,
        points: Vec<[f32; 2]>,
        params: SaberParams,
    },
    MediaOut,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: NodeId,
    pub label: String,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    pub from: NodeId,
    pub to: NodeId,
    /// Input slot on `to` (`0` = primary/fg, `1` = secondary/bg).
    pub to_slot: u8,
}

/// Optional GPU device/queue for hardware-accelerated node evaluation.
#[cfg(feature = "wgpu")]
pub struct GpuNodeContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

/// Directed acyclic compositing graph.
#[derive(Debug, Default, Clone)]
pub struct CompositorNodeGraph {
    pub nodes: HashMap<NodeId, GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl CompositorNodeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, id: NodeId, label: impl Into<String>, kind: NodeKind) {
        self.nodes.insert(
            id,
            GraphNode {
                id,
                label: label.into(),
                kind,
            },
        );
    }

    pub fn connect(&mut self, from: NodeId, to: NodeId, to_slot: u8) -> Result<(), String> {
        if !self.nodes.contains_key(&from) {
            return Err(format!("unknown source node {from}"));
        }
        if !self.nodes.contains_key(&to) {
            return Err(format!("unknown target node {to}"));
        }
        if from == to {
            return Err("self-loop edge is not allowed".to_string());
        }
        self.edges.push(GraphEdge { from, to, to_slot });
        Ok(())
    }

    /// Computes a deterministic topological execution order, returning an error if a cycle exists.
    pub fn topological_order(&self) -> Result<Vec<NodeId>, String> {
        let mut indegree: HashMap<NodeId, usize> =
            self.nodes.keys().map(|&id| (id, 0)).collect();
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for e in &self.edges {
            if let Some(deg) = indegree.get_mut(&e.to) {
                *deg += 1;
            }
            adj.entry(e.from).or_default().push(e.to);
        }

        let mut ready: Vec<NodeId> = indegree
            .iter()
            .filter_map(|(&id, &d)| if d == 0 { Some(id) } else { None })
            .collect();
        ready.sort_unstable();
        let mut queue: VecDeque<NodeId> = ready.into();
        let mut order = Vec::with_capacity(self.nodes.len());

        while let Some(id) = queue.pop_front() {
            order.push(id);
            if let Some(nexts) = adj.get(&id) {
                let mut newly_ready = Vec::new();
                for &nxt in nexts {
                    if let Some(deg) = indegree.get_mut(&nxt) {
                        *deg -= 1;
                        if *deg == 0 {
                            newly_ready.push(nxt);
                        }
                    }
                }
                newly_ready.sort_unstable();
                queue.extend(newly_ready);
            }
        }

        if order.len() != self.nodes.len() {
            return Err("cycle detected in compositor node graph".to_string());
        }
        Ok(order)
    }

    /// Evaluates the graph up to `output_node` on CPU (or GPU when `gpu` is provided).
    pub fn evaluate(
        &self,
        media_inputs: &HashMap<NodeId, FrameBufferF32>,
        output_node: NodeId,
        #[cfg(feature = "wgpu")] gpu: Option<&GpuNodeContext<'_>>,
    ) -> Result<FrameBufferF32, String> {
        let order = self.topological_order()?;
        let mut results: HashMap<NodeId, FrameBufferF32> = HashMap::new();

        for id in order {
            let node = self
                .nodes
                .get(&id)
                .ok_or_else(|| format!("missing node {id}"))?;
            let mut slot0: Option<&FrameBufferF32> = None;
            let mut slot1: Option<&FrameBufferF32> = None;
            for e in self.edges.iter().filter(|e| e.to == id) {
                let buf = results
                    .get(&e.from)
                    .ok_or_else(|| format!("missing upstream result from node {}", e.from))?;
                if e.to_slot == 0 {
                    slot0 = Some(buf);
                } else if e.to_slot == 1 {
                    slot1 = Some(buf);
                }
            }

            let out = match &node.kind {
                NodeKind::MediaIn => media_inputs
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| format!("missing MediaIn buffer for node {id}"))?,
                NodeKind::ColorWheelGrade(params) => {
                    let src = slot0.ok_or_else(|| format!("ColorWheelGrade {id} missing input"))?;
                    #[cfg(feature = "wgpu")]
                    let graded = if let Some(ctx) = gpu {
                        crate::core::color_wheel_gpu::gpu::apply_color_wheel_grade_gpu(
                            ctx.device,
                            ctx.queue,
                            &src.rgba,
                            src.width,
                            src.height,
                            params,
                        )
                    } else {
                        None
                    };
                    #[cfg(not(feature = "wgpu"))]
                    let graded: Option<Vec<f32>> = None;

                    let rgba = match graded {
                        Some(v) => v,
                        None => {
                            let mut v = src.rgba.clone();
                            apply_color_wheel_grade_cpu(&mut v, params);
                            v
                        }
                    };
                    FrameBufferF32::new(src.width, src.height, rgba, src.depth.clone())?
                }
                NodeKind::DeepMerge { z_epsilon } => {
                    #[cfg(not(feature = "wgpu"))]
                    let _ = z_epsilon;
                    let a = slot0.ok_or_else(|| format!("DeepMerge {id} missing input 0"))?;
                    let b = slot1.ok_or_else(|| format!("DeepMerge {id} missing input 1"))?;
                    if a.width != b.width || a.height != b.height {
                        return Err("DeepMerge input size mismatch".to_string());
                    }
                    let count = (a.width as usize) * (a.height as usize);
                    let da = a.depth.clone().unwrap_or_else(|| vec![0.0; count]);
                    let db = b.depth.clone().unwrap_or_else(|| vec![1.0; count]);

                    #[cfg(feature = "wgpu")]
                    let gpu_merged = if let Some(ctx) = gpu {
                        crate::core::deep_compositor::gpu::deep_merge_gpu(
                            ctx.device,
                            ctx.queue,
                            &a.rgba,
                            &da,
                            &b.rgba,
                            &db,
                            a.width,
                            a.height,
                            *z_epsilon,
                        )
                    } else {
                        None
                    };
                    #[cfg(not(feature = "wgpu"))]
                    let gpu_merged: Option<(Vec<f32>, Vec<f32>)> = None;

                    let (rgba, depth) = match gpu_merged {
                        Some((r, d)) => (r, d),
                        None => {
                            let deep_a =
                                DeepImage::from_rgba_depth(&a.rgba, &da, a.width, a.height)?;
                            let deep_b =
                                DeepImage::from_rgba_depth(&b.rgba, &db, b.width, b.height)?;
                            let merged = deep_a.deep_merge(&deep_b)?;
                            (merged.flatten_rgba(), merged.flatten_depth())
                        }
                    };
                    FrameBufferF32::new(a.width, a.height, rgba, Some(depth))?
                }
                NodeKind::OverMerge => {
                    let fg = slot0.ok_or_else(|| format!("OverMerge {id} missing input 0"))?;
                    let bg = slot1.ok_or_else(|| format!("OverMerge {id} missing input 1"))?;
                    if fg.width != bg.width || fg.height != bg.height {
                        return Err("OverMerge input size mismatch".to_string());
                    }
                    let mut out_rgba = vec![0.0_f32; fg.rgba.len()];
                    for ((dst, f), b) in out_rgba
                        .chunks_exact_mut(4)
                        .zip(fg.rgba.chunks_exact(4))
                        .zip(bg.rgba.chunks_exact(4))
                    {
                        let inv_a = (1.0 - f[3].clamp(0.0, 1.0)).max(0.0);
                        dst[0] = f[0] + inv_a * b[0];
                        dst[1] = f[1] + inv_a * b[1];
                        dst[2] = f[2] + inv_a * b[2];
                        dst[3] = (f[3] + inv_a * b[3]).clamp(0.0, 1.0);
                    }
                    FrameBufferF32::new(fg.width, fg.height, out_rgba, fg.depth.clone())?
                }
                NodeKind::ProKeyer(params) => {
                    let src = slot0.ok_or_else(|| format!("ProKeyer {id} missing input"))?;
                    #[cfg(feature = "wgpu")]
                    let keyed = if let Some(ctx) = gpu {
                        crate::core::pro_keyer::gpu::apply_pro_keyer_gpu(
                            ctx.device,
                            ctx.queue,
                            &src.rgba,
                            src.width,
                            src.height,
                            params,
                        )
                    } else {
                        None
                    };
                    #[cfg(not(feature = "wgpu"))]
                    let keyed: Option<Vec<f32>> = None;

                    let rgba = match keyed {
                        Some(v) => v,
                        None => {
                            let mut v = src.rgba.clone();
                            apply_pro_keyer_cpu(&mut v, params);
                            v
                        }
                    };
                    FrameBufferF32::new(src.width, src.height, rgba, src.depth.clone())?
                }
                NodeKind::OpenFx { plugin_name, frame } => {
                    let src = slot0.ok_or_else(|| format!("OpenFx {id} missing input"))?;
                    let in_u8: Vec<u8> = src
                        .rgba
                        .iter()
                        .map(|&v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
                        .collect();
                    let mut out_u8 = vec![0u8; in_u8.len()];
                    let bridge = OpenFxPluginBridge::new(plugin_name);
                    match bridge.render_frame(&in_u8, &mut out_u8, src.width, src.height, *frame) {
                        OfxStatus::OK => {
                            let rgba: Vec<f32> =
                                out_u8.into_iter().map(|b| b as f32 / 255.0).collect();
                            FrameBufferF32::new(src.width, src.height, rgba, src.depth.clone())?
                        }
                        status => {
                            return Err(format!(
                                "OpenFX plugin '{plugin_name}' failed with status {status:?}"
                            ))
                        }
                    }
                }
                #[cfg(feature = "wgpu")]
                NodeKind::SapphireOptical(params) => {
                    let src = slot0.ok_or_else(|| format!("SapphireOptical {id} missing input"))?;
                    let ctx = gpu.ok_or_else(|| {
                        format!("SapphireOptical {id} requires GPU context")
                    })?;
                    let rgba = apply_sapphire_optical_gpu(
                        ctx.device,
                        ctx.queue,
                        &src.rgba,
                        src.width,
                        src.height,
                        params,
                    )
                    .ok_or_else(|| format!("SapphireOptical {id} GPU execution failed"))?;
                    FrameBufferF32::new(src.width, src.height, rgba, src.depth.clone())?
                }
                #[cfg(feature = "wgpu")]
                NodeKind::SaberBeam {
                    width,
                    height,
                    points,
                    params,
                } => {
                    let ctx = gpu.ok_or_else(|| format!("SaberBeam {id} requires GPU context"))?;
                    let rgba =
                        render_saber_gpu(ctx.device, ctx.queue, points, *width, *height, params)
                            .ok_or_else(|| format!("SaberBeam {id} GPU execution failed"))?;
                    FrameBufferF32::new(*width, *height, rgba, None)?
                }
                NodeKind::Transform2D {
                    translate,
                    scale,
                    rotate_deg,
                } => {
                    let src = slot0.ok_or_else(|| format!("Transform2D {id} missing input"))?;
                    let rgba = transform_2d_f32(
                        &src.rgba,
                        src.width,
                        src.height,
                        *translate,
                        *scale,
                        *rotate_deg,
                    );
                    FrameBufferF32::new(src.width, src.height, rgba, src.depth.clone())?
                }
                NodeKind::GaussianBlur { radius } => {
                    let src = slot0.ok_or_else(|| format!("GaussianBlur {id} missing input"))?;
                    let rgba = gaussian_blur_f32(&src.rgba, src.width, src.height, *radius);
                    FrameBufferF32::new(src.width, src.height, rgba, src.depth.clone())?
                }
                NodeKind::DeepHoldout => {
                    let fg = slot0.ok_or_else(|| format!("DeepHoldout {id} missing input 0"))?;
                    let hold = slot1.ok_or_else(|| format!("DeepHoldout {id} missing input 1"))?;
                    if fg.width != hold.width || fg.height != hold.height {
                        return Err("DeepHoldout input size mismatch".to_string());
                    }
                    let count = (fg.width as usize) * (fg.height as usize);
                    let dfg = fg.depth.clone().unwrap_or_else(|| vec![1.0; count]);
                    let dhold = hold.depth.clone().unwrap_or_else(|| vec![0.0; count]);
                    let deep_fg = DeepImage::from_rgba_depth(&fg.rgba, &dfg, fg.width, fg.height)?;
                    let deep_hold =
                        DeepImage::from_rgba_depth(&hold.rgba, &dhold, hold.width, hold.height)?;
                    let cut = deep_fg.deep_holdout(&deep_hold)?;
                    FrameBufferF32::new(
                        fg.width,
                        fg.height,
                        cut.flatten_rgba(),
                        Some(cut.flatten_depth()),
                    )?
                }
                NodeKind::MediaOut => slot0
                    .cloned()
                    .ok_or_else(|| format!("MediaOut {id} missing input"))?,
            };
            results.insert(id, out);
        }

        results
            .remove(&output_node)
            .ok_or_else(|| format!("output node {output_node} was not evaluated"))
    }
}

fn transform_2d_f32(
    src: &[f32],
    width: u32,
    height: u32,
    translate: [f32; 2],
    scale: [f32; 2],
    rotate_deg: f32,
) -> Vec<f32> {
    let w = width as usize;
    let h = height as usize;
    let mut out = vec![0.0_f32; src.len()];
    if w == 0 || h == 0 {
        return out;
    }
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;
    let rad = rotate_deg.to_radians();
    let (sin_r, cos_r) = rad.sin_cos();
    let sx = if scale[0].abs() < 1e-4 { 1e-4 } else { scale[0] };
    let sy = if scale[1].abs() < 1e-4 { 1e-4 } else { scale[1] };

    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx - translate[0];
            let dy = y as f32 - cy - translate[1];
            let rx = (dx * cos_r + dy * sin_r) / sx + cx;
            let ry = (-dx * sin_r + dy * cos_r) / sy + cy;
            if rx < 0.0 || ry < 0.0 || rx > (w - 1) as f32 || ry > (h - 1) as f32 {
                continue;
            }
            let x0 = (rx.floor() as usize).min(w - 1);
            let y0 = (ry.floor() as usize).min(h - 1);
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);
            let fx = rx - x0 as f32;
            let fy = ry - y0 as f32;
            let i00 = (y0 * w + x0) * 4;
            let i10 = (y0 * w + x1) * 4;
            let i01 = (y1 * w + x0) * 4;
            let i11 = (y1 * w + x1) * 4;
            let dst = (y * w + x) * 4;
            for c in 0..4 {
                let top = src[i00 + c] * (1.0 - fx) + src[i10 + c] * fx;
                let bot = src[i01 + c] * (1.0 - fx) + src[i11 + c] * fx;
                out[dst + c] = top * (1.0 - fy) + bot * fy;
            }
        }
    }
    out
}

fn gaussian_blur_f32(src: &[f32], width: u32, height: u32, radius: f32) -> Vec<f32> {
    let r = radius.round().clamp(0.0, 32.0) as i32;
    if r == 0 || width == 0 || height == 0 {
        return src.to_vec();
    }
    let sigma = (radius * 0.5).max(0.5);
    let mut weights = Vec::with_capacity((r * 2 + 1) as usize);
    let mut wsum = 0.0_f32;
    for k in -r..=r {
        let w = (-(k as f32 * k as f32) / (2.0 * sigma * sigma)).exp();
        weights.push(w);
        wsum += w;
    }
    for w in &mut weights {
        *w /= wsum;
    }

    let w = width as usize;
    let h = height as usize;
    let mut tmp = vec![0.0_f32; src.len()];
    let mut out = vec![0.0_f32; src.len()];

    // Horizontal pass
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0_f32; 4];
            for (ki, k) in (-r..=r).enumerate() {
                let sx = (x as i32 + k).clamp(0, w as i32 - 1) as usize;
                let idx = (y * w + sx) * 4;
                let wt = weights[ki];
                for c in 0..4 {
                    acc[c] += src[idx + c] * wt;
                }
            }
            let dst = (y * w + x) * 4;
            tmp[dst..dst + 4].copy_from_slice(&acc);
        }
    }
    // Vertical pass
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0_f32; 4];
            for (ki, k) in (-r..=r).enumerate() {
                let sy = (y as i32 + k).clamp(0, h as i32 - 1) as usize;
                let idx = (sy * w + x) * 4;
                let wt = weights[ki];
                for c in 0..4 {
                    acc[c] += tmp[idx + c] * wt;
                }
            }
            let dst = (y * w + x) * 4;
            out[dst..dst + 4].copy_from_slice(&acc);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_detection_rejects_cyclic_graph() {
        let mut g = CompositorNodeGraph::new();
        g.add_node(1, "A", NodeKind::MediaIn);
        g.add_node(2, "B", NodeKind::MediaOut);
        g.connect(1, 2, 0).unwrap();
        g.connect(2, 1, 0).unwrap();
        assert!(g.topological_order().is_err());
    }

    #[cfg(feature = "wgpu")]
    #[test]
    fn node_graph_executes_color_wheel_and_deep_merge_on_gpu() {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("gpu adapter");
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("gpu device");

        let mut g = CompositorNodeGraph::new();
        g.add_node(1, "FG", NodeKind::MediaIn);
        g.add_node(2, "BG", NodeKind::MediaIn);
        g.add_node(
            3,
            "GradeFG",
            NodeKind::ColorWheelGrade(ColorWheelGradeParams {
                gain: [1.5, 1.0, 1.0, 1.0],
                ..Default::default()
            }),
        );
        g.add_node(4, "DeepMerge", NodeKind::DeepMerge { z_epsilon: 0.01 });
        g.add_node(5, "Out", NodeKind::MediaOut);

        g.connect(1, 3, 0).unwrap();
        g.connect(3, 4, 0).unwrap();
        g.connect(2, 4, 1).unwrap();
        g.connect(4, 5, 0).unwrap();

        let mut inputs = HashMap::new();
        // Pixel 0: FG closer (z=1 < z=5); Pixel 1: BG closer (z=8 > z=2)
        inputs.insert(
            1,
            FrameBufferF32::new(
                2,
                1,
                vec![0.5, 0.0, 0.0, 1.0, 0.5, 0.0, 0.0, 1.0],
                Some(vec![1.0, 8.0]),
            )
            .unwrap(),
        );
        inputs.insert(
            2,
            FrameBufferF32::new(
                2,
                1,
                vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0],
                Some(vec![5.0, 2.0]),
            )
            .unwrap(),
        );

        let ctx = GpuNodeContext {
            device: &device,
            queue: &queue,
        };
        let out = g.evaluate(&inputs, 5, Some(&ctx)).expect("graph eval");
        // Pixel 0: graded red (0.5 * 1.5 = 0.75) in front
        assert!((out.rgba[0] - 0.75).abs() < 1e-3);
        // Pixel 1: blue in front
        assert!(out.rgba[6] > 0.99 && out.rgba[4] < 0.01);
    }

    #[test]
    fn node_graph_transform_blur_and_deep_holdout() {
        let mut g = CompositorNodeGraph::new();
        g.add_node(1, "FG", NodeKind::MediaIn);
        g.add_node(2, "Matte", NodeKind::MediaIn);
        g.add_node(
            3,
            "Xform",
            NodeKind::Transform2D {
                translate: [0.0, 0.0],
                scale: [1.0, 1.0],
                rotate_deg: 0.0,
            },
        );
        g.add_node(4, "Blur", NodeKind::GaussianBlur { radius: 1.0 });
        g.add_node(5, "Holdout", NodeKind::DeepHoldout);
        g.add_node(6, "Out", NodeKind::MediaOut);

        g.connect(1, 3, 0).unwrap();
        g.connect(3, 4, 0).unwrap();
        g.connect(4, 5, 0).unwrap();
        g.connect(2, 5, 1).unwrap();
        g.connect(5, 6, 0).unwrap();

        let mut inputs = HashMap::new();
        inputs.insert(
            1,
            FrameBufferF32::new(
                2,
                2,
                vec![
                    1.0, 0.5, 0.0, 1.0, 1.0, 0.5, 0.0, 1.0,
                    1.0, 0.5, 0.0, 1.0, 1.0, 0.5, 0.0, 1.0,
                ],
                Some(vec![10.0; 4]),
            )
            .unwrap(),
        );
        inputs.insert(
            2,
            FrameBufferF32::new(
                2,
                2,
                [0.0_f32, 0.0, 0.0, 0.5].repeat(4),
                Some(vec![2.0; 4]),
            )
            .unwrap(),
        );

        #[cfg(feature = "wgpu")]
        let out = g.evaluate(&inputs, 6, None).expect("eval");
        #[cfg(not(feature = "wgpu"))]
        let out = g.evaluate(&inputs, 6).expect("eval");

        assert!((out.rgba[0] - 0.5).abs() < 1e-4);
        assert!((out.rgba[3] - 0.5).abs() < 1e-4);
    }

    #[cfg(feature = "wgpu")]
    #[test]
    fn node_graph_executes_saber_and_sapphire_optical_on_gpu() {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("gpu adapter");
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("gpu device");

        let mut g = CompositorNodeGraph::new();
        g.add_node(
            1,
            "Saber",
            NodeKind::SaberBeam {
                width: 32,
                height: 32,
                points: vec![[4.0, 16.0], [28.0, 16.0]],
                params: SaberParams::default(),
            },
        );
        g.add_node(
            2,
            "SapphireGlow",
            NodeKind::SapphireOptical(SapphireOpticalParams::default()),
        );
        g.add_node(3, "Out", NodeKind::MediaOut);

        g.connect(1, 2, 0).unwrap();
        g.connect(2, 3, 0).unwrap();

        let ctx = GpuNodeContext {
            device: &device,
            queue: &queue,
        };
        let out = g.evaluate(&HashMap::new(), 3, Some(&ctx)).expect("eval");
        let center = (16 * 32 + 16) * 4;
        assert!(out.rgba[center] > 1.0 && out.rgba[center + 2] > 1.5);
    }

    #[cfg(feature = "wgpu")]
    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        use std::sync::Arc;
        use std::task::{Context, Poll, Wake, Waker};
        struct W(std::thread::Thread);
        impl Wake for W {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
            fn wake_by_ref(self: &Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(W(std::thread::current())));
        let mut cx = Context::from_waker(&waker);
        let mut fut = std::pin::pin!(fut);
        loop {
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => std::thread::park(),
            }
        }
    }
}
