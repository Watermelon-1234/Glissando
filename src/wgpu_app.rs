use core::panic;
use std::sync::Arc;

use winit::dpi::LogicalSize;
// use scap::capturer;
pub use winit::{
    window::Window,
};

use crate::screen;

use wgpu::{Adapter, util::DeviceExt};
use bytemuck;

// configuration
use crate::config::AppConfig;

// for vr
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VRParams {
    pub offset: f32,     // 瞳距偏移
    pub z_distance: f32, // 畫面距離
    pub k1: f32,         // 畸變係數1
    pub k2: f32,         // 畸變係數2
}

impl Default for VRParams {
    fn default() -> Self {
        Self {
            offset: 0.032,
            z_distance: 0.8,
            k1: 0.21,
            k2: 0.12,
        }
    }
}

pub struct WgpuApp {
    pub app_config: AppConfig,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::LogicalSize<u32>, // PhysicalSize<u32>,

    screen_texture: wgpu::Texture,
    screen_texture_view: wgpu::TextureView,
    render_pipeline: wgpu::RenderPipeline,
    vr_params: VRParams,
    vr_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,

    capturer: scap::capturer::Capturer,
    is_capturing: bool,
    current_frame: scap::frame::Frame,
    bind_group: wgpu::BindGroup,

    /// 避免窗口被释放
    #[allow(unused)]
    pub(crate) window: Arc<Window>,
}


impl WgpuApp {
    pub async fn new(window: Arc<Window>, app_config: AppConfig) -> Self {

        // capturer
        let mut capturer = screen::init_capture(Some(app_config.clone())).unwrap_or_else(|e| panic!("error: {}", e));
        // // print!("test_only main() init_capture\n");
        // let frame = scap::frame::Frame::BGRA(
        //     scap::frame::BGRAFrame {
        //         data: vec![0; (size.width * size.height * 4) as usize],
        //         width: size.width as i32,
        //         height: size.height as i32,
        //         display_time: 0,
        //     }
        // );  
        capturer.start_capture();
        // // print!("test_only main() start_capture\n");
        let is_capturing = true;
        let size: LogicalSize<u32> = window.inner_size().to_logical(window.scale_factor());
        let frame  = capturer.get_next_frame().unwrap_or(scap::frame::Frame::BGRA(
            scap::frame::BGRAFrame {
                data: vec![0; (size.width * size.height * 4) as usize],
                width: size.width as i32,
                height: size.height as i32,
                display_time: 0,
            }
        ));

        let (mut size, frame) = match frame
        {
            scap::frame::Frame::BGRA(bgra_frame) => {
                let width = bgra_frame.width as u32;
                let height = bgra_frame.height as u32;

                let size = LogicalSize::new(width, height);

                (size, scap::frame::Frame::BGRA(bgra_frame))
            }
            _ => panic!("unexpected frame type"),
        };

        size.width = size.width.max(1);
        size.height = size.height.max(1);
        // let frame = scap::frame::Frame::BGRA(
        //     scap::frame::BGRAFrame {
        //         data: vec![0; (size.width * size.height * 4) as usize],
        //         width: size.width as i32,
        //         height: size.height as i32,
        //         display_time: 0,
        //     }
        // );
        // // print!("test_only main() make a frame\n");
        if let scap::frame::Frame::BGRA(ref bgra_frame) = frame {
            print!("frame: {:?}\n", bgra_frame.data.len());
            print!("frame: {:?},{:?}\n", bgra_frame.width,bgra_frame.height);
        }
        // capturer.stop_capture();
        // // // print!("test_only main() stop_capture\n");        


        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter_with_option: Option<Adapter>  = match app_config.system.adapter_name != "" && app_config.system.adapter_name != "None" {
            true =>  instance
                .enumerate_adapters(wgpu::Backends::all())
                .await
                // .iter().for_each(|adapter| println!("{},{},{},{}",adapter.get_info().name,adapter.get_info().backend,adapter.get_info().device,adapter.get_info().vendor));
                .into_iter()
                .filter(|adapter| adapter.is_surface_supported(&surface) && adapter.get_info().name == app_config.system.adapter_name)
                .collect::<Vec<Adapter>>()
                .first()
                .cloned(),
            false => Some(instance // temporary
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }
            )
            .await
            .unwrap()),
        };

        let adapter = match adapter_with_option {
            Some(adapter) => adapter,
            None => panic!("no adapter found"),
        };
        

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
        print!("test_only window.inner_size: {:?}\n", size);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: app_config.system.present_mode,// wgpu::PresentMode::Fifo, //temporary
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // screen texture
        let texture_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let screen_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screen texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            format: texture_format,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });


        let screen_texture_view = screen_texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor{
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let vr_params = VRParams{
            offset: app_config.vr_render.offset,
            z_distance: app_config.vr_render.z_distance,
            k1: app_config.vr_render.k1,
            k2: app_config.vr_render.k2,
        };

        // 2. 建立 GPU Buffer
        let vr_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VR Params Buffer"),
            contents: bytemuck::cast_slice(&[vr_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, // 兩邊都要用
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            }
        );

        // // print!("test_only main() texture_bind_group_layout\n");

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&screen_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: vr_buffer.as_entire_binding(),
                    },
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        // // print!("test_only main() bind_group\n");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout], // different from tutorial
                immediate_size: 0,
            }
        );

        // // print!("test_only main() render_pipeline_layout\n");

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("vs_main"), // 1.
                buffers: &[], // 2.
            },
            fragment: Some(wgpu::FragmentState { // 3.
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // 将此设置为 Fill 以外的任何值都要需要开启 Feature::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // 需要开启 Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // 需要开启 Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1, // 2.
                mask: !0, // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview_mask: None, // 5.
            cache: None,
        });

        // print!("test_only main() render_pipeline\n");

        Self {
            app_config,
            surface,
            device,
            queue,
            config,
            size,
            window,

            screen_texture,
            screen_texture_view,
            vr_params,
            vr_buffer,
            render_pipeline,
            sampler,
            bind_group,

            capturer,
            is_capturing,
            current_frame: frame,
        }
        
    }
    // pub fn set_window_resized(&mut self, new_size: PhysicalSize<u32>) {
    //     // print!("test_only set_window_resized() start\n");
    //     if new_size == self.size.to_physical(self.window.scale_factor()) {
    //         return;
    //     }
    //     self.size = new_size.to_logical(self.window.scale_factor());
    //     self.size_changed = true;
    //     self.window.request_redraw();
    // }
    /// 必要的时候调整 surface 大小
    // pub fn resize_surface_if_needed(&mut self) {
    //     // print!("test_only resize_surface_if_needed() start\n");
    //     if self.size_changed {
    //         self.config.width = self.size.width;
    //         self.config.height = self.size.height;
    //         self.surface.configure(&self.device, &self.config);
    //         self.size_changed = false;
    //     }
    // }




    /* give me the structure of if let pls
    if let type(item) = var {
        item
    }

    */
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // self.resize_surface_if_needed();
        // print!("test_only render() start\n");
        if self.is_capturing {
            // self.current_frame = self.capturer.get_next_frame().unwrap(); // must be bgra frame
            if let Ok(frame) = self.capturer.get_next_frame() {
                self.current_frame = frame;
                // print!("test_only render() frame\n");
            }
        }

        // print!("test_only render() frame done");
        if let scap::frame::Frame::BGRA(ref frame) = self.current_frame {
                    if (frame.width as u32 != self.config.width || frame.height as u32 != self.config.height) && (frame.width != 0 && frame.height != 0)
        {
            print!("reshape texture and surface to {}x{}\n", frame.width, frame.height);
            self.config.width = frame.width as u32;
            self.config.height = frame.height as u32;
            self.surface.configure(&self.device, &self.config);

            self.screen_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("screen texture"),
                size: wgpu::Extent3d {
                    width: frame.width as u32,
                    height: frame.height as u32,
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Bgra8Unorm,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                usage: wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            self.screen_texture_view =
                self.screen_texture.create_view(&wgpu::TextureViewDescriptor::default());
        }
            let size = self.size;
            
            let expected_size = (frame.width as usize)
            * (frame.height as usize)
            * 4;
            if expected_size == 0 {
                return Ok(());
            }

            // println!(
            //     "frame: {}x{}, data_len: {}",
            //     frame.width,
            //     frame.height,
            //     frame.data.len()
            // );

            // println!("expected_size: {}, frame.data.len(): {}", expected_size, frame.data.len());

            if expected_size != frame.data.len(){
                println!("expected_size: {}, frame.data.len(): {}", expected_size, frame.data.len());
                return Ok(());
            }

            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.screen_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &frame.data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * size.width),
                    rows_per_image: Some(size.height),
                },
                wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
            );



            
        }

        // print!("test_only render() start render pass\n");
        // begin render pass
        
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store
                    },
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..12, 0..1); // 0..6 means it need 2 triangles as a rectangle = 2 * 3 vertices / 0..1 indicates it only render these verticles once
            
        }


        // submit 命令能接受任何实现了 IntoIter trait 的参数
        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn update(&mut self) {
        
    }
    // pub fn update_params(&mut self, offset_delta: f32, z_delta: f32) {
    //     todo!();    
    // }
}

