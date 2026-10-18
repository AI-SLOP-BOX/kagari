use kagari_vfx::core::{
    production_document::ProductionDocument,
    project_migration,
    renderer::WgpuRenderer,
    timeline::{Composition, Project},
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PreviewUniform {
    composition_aspect: f32,
    surface_aspect: f32,
    _padding: [f32; 2],
}

struct GpuPreview {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    renderer: WgpuRenderer,
    pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    texture: Option<wgpu::BindGroup>,
}

impl GpuPreview {
    fn new(window: Arc<Window>) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| format!("surface creation failed: {error}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "no compatible WebGPU adapter is available".to_owned())?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Kagari preview device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .map_err(|error| format!("device creation failed: {error}"))?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: capabilities
                .present_modes
                .iter()
                .copied()
                .find(|mode| *mode == wgpu::PresentMode::Fifo)
                .unwrap_or(capabilities.present_modes[0]),
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("preview texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("preview linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("preview aspect uniform"),
            contents: bytemuck::bytes_of(&PreviewUniform {
                composition_aspect: 16.0 / 9.0,
                surface_aspect: config.width as f32 / config.height as f32,
                _padding: [0.0; 2],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("preview pipeline layout"),
            bind_group_layouts: &[&texture_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("preview blit shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct PreviewUniform {
    composition_aspect: f32,
    surface_aspect: f32,
    padding: vec2<f32>,
};
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> preview: PreviewUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0)
    );
    let x_scale = min(1.0, preview.surface_aspect / preview.composition_aspect);
    let y_scale = min(1.0, preview.composition_aspect / preview.surface_aspect);
    var output: VertexOutput;
    output.position = vec4<f32>(positions[index] * vec2<f32>(x_scale, y_scale), 0.0, 1.0);
    output.uv = uvs[index];
    return output;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(source_texture, source_sampler, input.uv);
}
"#
                .into(),
            ),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("preview surface pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let renderer = WgpuRenderer::new(device.clone(), queue.clone());

        Ok(Self {
            _instance: instance,
            surface,
            config,
            device,
            queue,
            renderer,
            pipeline,
            texture_layout,
            sampler,
            uniform,
            texture: None,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&mut self, comp: Option<&Composition>, frame: u32) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("preview frame encoder"),
            });
        if let Some(comp) = comp {
            self.queue.write_buffer(
                &self.uniform,
                0,
                bytemuck::bytes_of(&PreviewUniform {
                    composition_aspect: comp.width.max(1) as f32 / comp.height.max(1) as f32,
                    surface_aspect: self.config.width as f32 / self.config.height.max(1) as f32,
                    _padding: [0.0; 2],
                }),
            );
            let (view, recreated) = self.renderer.render(comp, frame, 0.0, 0);
            if self.texture.is_none() || recreated {
                self.texture = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("preview composition texture"),
                    layout: &self.texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.uniform.as_entire_binding(),
                        },
                    ],
                }));
            }
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("preview blit pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            if let Some(texture) = &self.texture {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, texture, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        output.present();
        Ok(())
    }
}

struct PreviewApp {
    project_path: PathBuf,
    document: ProductionDocument,
    modified: Option<std::time::SystemTime>,
    frame: u32,
    playing: bool,
    window: Option<Arc<Window>>,
    gpu: Option<GpuPreview>,
    status: String,
    playback_started_at: Option<std::time::Instant>,
    playback_start_frame: u32,
}

impl PreviewApp {
    fn load_project(path: &std::path::Path) -> Result<ProductionDocument, String> {
        let json = project_migration::read_project_json_file(path)?;
        ProductionDocument::from_json(&json)
            .or_else(|_| ProductionDocument::from_legacy_project_json(&json))
    }

    fn active_composition(&self) -> Option<&Composition> {
        self.document.project.compositions.get(
            self.document
                .project
                .active_composition_idx
                .min(self.document.project.compositions.len().saturating_sub(1)),
        )
    }

    fn check_project_file(&mut self) -> bool {
        let modified = std::fs::metadata(&self.project_path)
            .and_then(|metadata| metadata.modified())
            .ok();
        if modified.is_some() && modified != self.modified {
            match Self::load_project(&self.project_path) {
                Ok(document) => {
                    self.frame = document
                        .project
                        .compositions
                        .get(
                            document
                                .project
                                .active_composition_idx
                                .min(document.project.compositions.len().saturating_sub(1)),
                        )
                        .map(|comp| self.frame.min(comp.duration_frames.saturating_sub(1)))
                        .unwrap_or(0);
                    self.document = document;
                    self.modified = modified;
                    self.playback_start_frame = self.frame;
                    self.playback_started_at = self.playing.then(std::time::Instant::now);
                    self.status = "プロジェクトファイルの変更を反映".into();
                    return true;
                }
                Err(error) => self.status = format!("再読み込みエラー: {error}"),
            }
        }
        false
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            let name = self
                .active_composition()
                .map(|comp| comp.name.as_str())
                .unwrap_or("No Composition");
            let total = self
                .active_composition()
                .map(|comp| comp.duration_frames)
                .unwrap_or(0);
            window.set_title(&format!(
                "Kagari WebGPU Preview · {name} · Frame {}/{} · {}",
                self.frame,
                total,
                self.project_path.display()
            ));
        }
    }
}

impl ApplicationHandler for PreviewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("Kagari WebGPU Preview")
                .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0)),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.status = format!("ウィンドウ作成エラー: {error}");
                event_loop.exit();
                return;
            }
        };
        match GpuPreview::new(window.clone()) {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(error) => {
                self.status = format!("WebGPU初期化エラー: {error}");
                eprintln!("{}", self.status);
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window);
        self.update_title();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Space) => {
                        self.playing = !self.playing;
                        if self.playing {
                            self.playback_start_frame = self.frame;
                            self.playback_started_at = Some(std::time::Instant::now());
                        } else {
                            self.playback_started_at = None;
                        }
                    }
                    PhysicalKey::Code(KeyCode::ArrowLeft) => {
                        self.frame = self.frame.saturating_sub(1);
                        self.playing = false;
                        self.playback_started_at = None;
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) => {
                        self.frame = self.frame.saturating_add(1);
                        self.playing = false;
                        self.playback_started_at = None;
                    }
                    _ => return,
                }
                self.update_title();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let composition_index = self.document.project.active_composition_idx;
                let comp = self
                    .document
                    .composition_for_frame(composition_index, self.frame);
                if let Some(comp) = comp.as_ref() {
                    self.frame = self.frame.min(comp.duration_frames.saturating_sub(1));
                }
                if let Some(gpu) = &mut self.gpu {
                    match gpu.render(comp.as_ref(), self.frame) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            gpu.resize(gpu.config.width, gpu.config.height);
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                        Err(wgpu::SurfaceError::Timeout) => {}
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            self.status = "WebGPU surface ran out of memory".into();
                            event_loop.exit();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let changed = self.check_project_file();
        let now = std::time::Instant::now();
        let mut redraw = changed;
        if self.playing {
            let fps = self
                .active_composition()
                .map(|comp| comp.fps.max(1))
                .unwrap_or(24);
            let duration = self
                .active_composition()
                .map(|comp| comp.duration_frames.max(1))
                .unwrap_or(1);
            let elapsed_frames = self
                .playback_started_at
                .map(|started| (now.duration_since(started).as_secs_f64() * f64::from(fps)) as u64)
                .unwrap_or(0);
            let frame =
                (u64::from(self.playback_start_frame) + elapsed_frames) % u64::from(duration);
            if self.frame != frame as u32 {
                self.frame = frame as u32;
                self.update_title();
                redraw = true;
            }
            let frame_period = Duration::from_secs_f64(1.0 / f64::from(fps));
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                now + frame_period.min(Duration::from_millis(16)),
            ));
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(now + Duration::from_millis(350)));
        }
        if redraw {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let project_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("project.json"));
    let loaded = PreviewApp::load_project(&project_path);
    let (document, status) = match loaded {
        Ok(document) => (document, format!("監視中: {}", project_path.display())),
        Err(error) => (
            ProductionDocument::new(Project::default()),
            format!("読み込みエラー: {error}"),
        ),
    };
    let modified = std::fs::metadata(&project_path)
        .and_then(|metadata| metadata.modified())
        .ok();
    let mut app = PreviewApp {
        project_path,
        document,
        modified,
        frame: 0,
        playing: false,
        window: None,
        gpu: None,
        status,
        playback_started_at: None,
        playback_start_frame: 0,
    };
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
