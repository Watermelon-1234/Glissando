use std::{sync::Arc};

pub use winit::{
    window::Window,
    dpi::PhysicalSize,
};


pub struct WgpuApp {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    size_changed: bool,
    color : wgpu::Color,
    /// 避免窗口被释放
    #[allow(unused)]
    pub(crate) window: Arc<Window>,
}


impl WgpuApp {
    pub async fn new(window: Arc<Window>) -> Self {

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance // temporary
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                // WebGL 后端并不支持 wgpu 的所有功能，
                // 所以如果要以 web 为构建目标，就必须禁用一些功能。
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                experimental_features: wgpu::ExperimentalFeatures::disabled(), // temporary
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
                // 追踪 API 调用路径
                trace: wgpu::Trace::Off,
            }
        ).await.unwrap();

        let caps = surface.get_capabilities(&adapter);
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width, // temporary
            height: size.height, // temporary
            present_mode: wgpu::PresentMode::Fifo, //temporary
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            size_changed: false,
            window,
            color: wgpu::Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }
        }
    }
    pub fn set_window_resized(&mut self, new_size: PhysicalSize<u32>) {
        if new_size == self.size {
            return;
        }
        self.size = new_size;
        self.size_changed = true;
        self.window.request_redraw();
    }
    /// 必要的时候调整 surface 大小
    pub fn resize_surface_if_needed(&mut self) {
        if self.size_changed {
            self.config.width = self.size.width;
            self.config.height = self.size.height;
            self.surface.configure(&self.device, &self.config);
            self.size_changed = false;
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.color),
                        store: wgpu::StoreOp::Store
                    },
                })],
                ..Default::default()
            });
        }

        // submit 命令能接受任何实现了 IntoIter trait 的参数
        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn update(&mut self) {
        if self.color.r == 1.0 {
            self.color.g = 1.0;
            self.color.r = 0.0;
        }else if self.color.g == 1.0 {
            self.color.b = 1.0;
            self.color.g = 0.0;
        }else if self.color.b == 1.0 {
            self.color.r = 1.0;
            self.color.b = 0.0;
        }
    }
}

