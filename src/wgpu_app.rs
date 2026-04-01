use core::panic;
use std::sync::{Arc,Mutex};


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

use serde::{Deserialize, Serialize};


// for vr
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Deserialize, Serialize)]
pub struct VRParams {
    pub offset: f32,     // 瞳距偏移
    pub z_distance: f32, // 畫面距離
    pub k1: f32,         // 畸變係數1
    pub k2: f32,         // 畸變係數2

    pub sensitivity: f32,
    pub _padding1: [f32; 3],        // 保持 16 字節對齊 (WGPU Uniform 最佳實踐)

    // pub base_yaw: f32,
    // pub base_pitch: f32,
    // pub base_roll: f32,

    // pub yaw: f32,      // 頭部追蹤水平位移
    // pub pitch: f32,      // 頭部追蹤垂直位移
    // pub roll: f32,      // 頭部追蹤旋轉


    pub q_base: [f32; 4],    // 校正時的基準姿態
    pub q_current: [f32; 4], // 當前手機姿態
    #[serde(skip)]
    pub q_smooth: [f32; 4],  // 這是實際傳給 Shader 的「平滑後」數據

}

impl Default for VRParams {
    fn default() -> Self {
        Self {
            offset: 0.032,
            z_distance: 0.8,
            k1: 0.21,
            k2: 0.12,

            sensitivity: 0.5,
            _padding1: [0.0; 3],

            // base_yaw: 0.0,
            // base_pitch: 0.0,
            // base_roll: 0.0,

            // yaw: 0.0,
            // pitch: 0.0,
            // roll: 0.0,

            q_base: [0.0, 0.0, 0.0, 1.0],
            q_current: [0.0, 0.0, 0.0, 1.0],
            q_smooth: [0.0, 0.0, 0.0, 1.0],

        }
    }
}

pub struct WgpuApp {
    pub last_time: std::time::Instant,
    
    pub app_config: AppConfig,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::LogicalSize<u32>, // PhysicalSize<u32>,

    screen_texture: wgpu::Texture,
    screen_texture_view: wgpu::TextureView,
    render_pipeline: wgpu::RenderPipeline,
    pub vr_params: Arc<Mutex<VRParams>>, // since osc server neads to access it
    vr_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,

    capturer: scap::capturer::Capturer,
    is_capturing: bool,
    current_frame: scap::frame::Frame,
    bind_group: wgpu::BindGroup,

    pub distorted_texture: wgpu::Texture,
    pub distorted_texture_view: wgpu::TextureView,

    // pub yuv_data_size: u64,
    // pub yuv_shader: wgpu::ShaderModule,
    // pub yuv_compute_pipeline: wgpu::ComputePipeline,
    // pub yuv_bind_group: wgpu::BindGroup,
    // pub yuv_output_staging_buffer: wgpu::Buffer,     
    // pub yuv_storage_buffer: wgpu::Buffer,

    // pub streamer: Option<GStreamer>,

    /// 避免窗口被释放
    #[allow(unused)]
    pub(crate) window: Arc<Window>,
}


impl WgpuApp {
    pub async fn new(window: Arc<Window>, app_config: AppConfig) -> Self {
        println!("test_only main() new");
        
        // for calculate fps
        let last_time = std::time::Instant::now();
        
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

        println!("fuck");


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
        

        println!("fuck");

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

        println!("fuck");


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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT| wgpu::TextureUsages::COPY_SRC| wgpu::TextureUsages::COPY_DST,
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

        let vr_params: Arc<Mutex<VRParams>> = Arc::new(Mutex::new(VRParams{
            offset: app_config.vr_render.offset,
            z_distance: app_config.vr_render.z_distance,
            k1: app_config.vr_render.k1,
            k2: app_config.vr_render.k2,
            ..Default::default()
        }));

        // 2. 建立 GPU Buffer
        let vr_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VR Params Buffer"),
            contents: bytemuck::cast_slice(&[vr_params.lock().unwrap().clone()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        println!("fuck");

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


        let distorted_texture = device.create_texture(&wgpu::TextureDescriptor{
            label: Some("distort texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            format: config.format,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // 必須具備 RENDER_ATTACHMENT(可被渲染)、TEXTURE_BINDING(可被Shader讀取)、COPY_SRC(可被複製)
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let distorted_texture_view = distorted_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // let u32_size = std::mem::size_of::<u32>() as u32;
        // let bytes_per_row = (size.width * u32_size + 255) & !255; // 對齊 256 bytes
        // let yuv_data_size = (size.width * size.height * 3 / 2) as u64;

        // let yuv_storage_buffer = device.create_buffer(
        //     &wgpu::BufferDescriptor{
        //         label: Some("YUV Storage Buffer"),
        //         size: yuv_data_size,
        //         usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, // 注意 COPY_SRC
        //         mapped_at_creation: false,
        //     }
        // );

        // let yuv_output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //     label: Some("YUV Output Staging Buffer"),
        //     size: yuv_data_size,
        //     usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        //     mapped_at_creation: false,
        // });

        // print!("test_only main() render_pipeline\n");

        // let yuv_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        //     label: Some("YUV Compute Shader"),
        //     source: wgpu::ShaderSource::Wgsl(include_str!("yuv_convert.wgsl").into()), // 這是你寫好的 shader
        // });

        // let yuv_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //     label: Some("YUV Bind Group Layout"),
        //     entries: &[
        //         // Binding 0: 輸入的原始貼圖 (剛畫好的 SBS 畫面)
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: wgpu::ShaderStages::COMPUTE,
        //             ty: wgpu::BindingType::Texture {
        //                 sample_type: wgpu::TextureSampleType::Float { filterable: true },
        //                 view_dimension: wgpu::TextureViewDimension::D2,
        //                 multisampled: false,
        //             },
        //             count: None,
        //         },
        //         // Binding 1: 輸出的 Storage Buffer (放 YUV 數據)
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 1,
        //             visibility: wgpu::ShaderStages::COMPUTE,
        //             ty: wgpu::BindingType::Buffer {
        //                 ty: wgpu::BufferBindingType::Storage { read_only: false },
        //                 has_dynamic_offset: false,
        //                 min_binding_size: None,
        //             },
        //             count: None,
        //         },
        //     ],
        // });

        // let yuv_bind_group = device.create_bind_group(
        //     &wgpu::BindGroupDescriptor {
        //         layout: &yuv_bind_group_layout,
        //         label: Some("YUV Bind Group"),
        //         entries: &[
        //             wgpu::BindGroupEntry {
        //                 binding: 0,
        //                 resource: wgpu::BindingResource::TextureView(&distorted_texture_view),
        //             },
        //             wgpu::BindGroupEntry {
        //                 binding: 1,
        //                 resource: yuv_storage_buffer.as_entire_binding(),
        //             },
        //         ]
        //     }
        // );

        // let yuv_compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        //     label: Some("YUV Compute Pipeline"),
        //     layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //         label: Some("YUV Layout"),
        //         bind_group_layouts: &[&yuv_bind_group_layout],
        //         immediate_size: 0,
        //     })),
        //     module: &yuv_shader,
        //     entry_point: Some("main"), // 對應你 wgsl 裡的 @compute fn main
        //     compilation_options: Default::default(),
        //     cache: None,
        // });

        // let streamer:Option<GStreamer> = match GStreamer::new(&app_config.network.device_ip, app_config.network.video_server_port, size.width, size.height)
        // {
        //     Ok(streamer) => Some(streamer),
        //     Err(e) => {
        //         println!("Failed to create GStreamer: {}", e);
        //         None
        //     }
        // };


        Self {
            last_time,
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

            distorted_texture,
            distorted_texture_view,

            capturer,
            is_capturing,
            current_frame: frame,

            // yuv_data_size,
            // yuv_storage_buffer,
            // yuv_output_staging_buffer,
            // yuv_shader,
            // yuv_bind_group,
            // yuv_compute_pipeline,

            // streamer,
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
        let now = std::time::Instant::now();
        let fps = 1.0 / now.duration_since(self.last_time).as_secs_f32();
        print!("\r"); // 
        print!("\x1b[H\x1b[2KRendering FPS: {:.2}",fps); 
        self.last_time = now;

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
        if let scap::frame::Frame::BGRA(ref frame) = self.current_frame 
        {
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

                // --- 新增：視窗大小改變時，重建中介紋理 (Distorted Texture) ---
                self.distorted_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("distorted texture"),
                    size: wgpu::Extent3d {
                        width: frame.width as u32,
                        height: frame.height as u32,
                        depth_or_array_layers: 1,
                    },
                    format: self.config.format, 
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                self.distorted_texture_view = self.distorted_texture.create_view(&wgpu::TextureViewDescriptor::default());

                // --- 新增：重建 YUV Bind Group，綁定新的中介紋理 ---
                // self.yuv_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                //     layout: &self.yuv_compute_pipeline.get_bind_group_layout(0),
                //     label: Some("YUV Bind Group"),
                //     entries: &[
                //         wgpu::BindGroupEntry {
                //             binding: 0,
                //             resource: wgpu::BindingResource::TextureView(&self.distorted_texture_view),
                //         },
                //         wgpu::BindGroupEntry {
                //             binding: 1,
                //             resource: self.yuv_storage_buffer.as_entire_binding(),
                //         },
                //     ]
                // });
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


        // render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.distorted_texture_view, //changed
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

        // convert pass (BGRA -> YUV) with shader:yuv_convert.wgsl
        // {
        //     let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        //         label: Some("YUV Conversion Pass"),
        //         ..Default::default()
        //     });
        //     compute_pass.set_pipeline(&self.yuv_compute_pipeline); // 你新建立的 pipeline
        //     compute_pass.set_bind_group(0, &self.yuv_bind_group, &[]);
            
        //     // 根據畫面大小決定工作群組數量 (假設 8x8)
        //     // let workgroup_x = (self.config.width + 7) / 8;
        //     // let workgroup_y = (self.config.height + 7) / 8;
        //     // compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        //     // id.y 處理高度的 1.5 倍
        //     compute_pass.dispatch_workgroups((self.size.width + 63) / 64, (self.size.height * 3 / 2 + 15) / 16, 1);
        // }

        // encoder.copy_buffer_to_buffer(
        //     &self.yuv_storage_buffer, 0,
        //     &self.yuv_output_staging_buffer, 0,
        //     self.yuv_data_size
        // );


        // Copy/Blit Pass: 將中介紋理的內容直接複製到視窗 Surface 上，讓肉眼可以看到
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.distorted_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &output.texture, // 目標是視窗
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            }
        );

        // copy pass
        // let u32_size = std::mem::size_of::<u32>() as u32;
        // let bytes_per_row = (self.config.width * u32_size + 255) & !255; 

        // let output_buffer_layout = wgpu::TexelCopyBufferLayout {
        //     offset: 0,
        //     bytes_per_row: Some(bytes_per_row),
        //     rows_per_image: Some(self.config.height),
        // };

        // encoder.copy_texture_to_buffer(
        //     wgpu::TexelCopyTextureInfo {
        //         texture: &output.texture, // 直接抓取剛剛渲染完的畫面
        //         mip_level: 0,
        //         origin: wgpu::Origin3d::ZERO,
        //         aspect: wgpu::TextureAspect::All,
        //     },
        //     wgpu::TexelCopyBufferInfo{
        //         buffer: &self.yuv_output_staging_buffer,
        //         layout: output_buffer_layout,
        //     },
        //     wgpu::Extent3d {
        //         width: self.config.width,
        //         height: self.config.height,
        //         depth_or_array_layers: 1,
        //     },
        // );

        self.queue.submit(Some(encoder.finish()));
        output.present();

        

        Ok(())
    }

    pub fn update(&mut self) {
        // if let Some(yuv_data) = pollster::block_on(self.capture_yuv_frame()) {
        //     if let Some(ref streamer) = self.streamer {
        //         let _ = streamer.push_frame(yuv_data);
        //     }
        // } 
        let mut p = {
            let p = self.vr_params.lock().unwrap();
            *p // 解引用複製一份出來
        };

        // 平滑因子 (Alpha)：0.0 ~ 1.0
        // 越小越平滑但延遲感越高，0.1~0.3 是不錯的平衡點
        let alpha = 0.15; 

        // 對四元數的四個分量分別做插值 (簡易 Lerp + Normalize)
        let mut sum_sq = 0.0;
        for i in 0..4 {
            // q_smooth = q_smooth * (1-a) + q_current * a
            p.q_smooth[i] = p.q_smooth[i] * (1.0 - alpha) + p.q_current[i] * alpha;
            sum_sq += p.q_smooth[i] * p.q_smooth[i];
        }

        // 重新歸一化，防止四元數失效
        let magnitude = sum_sq.sqrt();
        if magnitude > 0.0 {
            for i in 0..4 {
                p.q_smooth[i] /= magnitude;
            }
        }
        self.queue.write_buffer(&self.vr_buffer, 0, bytemuck::cast_slice(&[p]));
    }
    // pub fn update_params(&mut self, offset_delta: f32, z_delta: f32) {
    //     todo!();    
    // }

    // 這是最詳盡的讀取流程
    // pub async fn frame_from_buffer(&self) -> Option<Vec<u8>> {
    //     let u32_size = std::mem::size_of::<u32>() as u32;
    //     let bytes_per_row = (selfconfig.width * u32_size + 255) & !255; 

    //     // 1. 建立一個 slice 指向 buffer
    //     let buffer_slice = self.yuv_output_staging_buffer.slice(..);

    //     // 2. 請求映射（Map）該緩衝區以便讀取
    //     // 使用 Oneshot Channel 來等待非同步結果
    //     let (tx, rx) = futures::channel::oneshot::channel();
    //     buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
    //         tx.send(result).unwrap();
    //     });

    //     // 3. 輪詢設備直到映射完成
    //     // 這是最重要的一步，沒有 poll，GPU 就不會執行地圖映射的指令
    //     match self.device.poll(wgpu::PollType::Wait { // wgpu::Maintain is renamed to wgpu::PollType in wpug v25.0.0 https://github.com/gfx-rs/wgpu/releases?q=maintain&expanded=true
    //         submission_index: None,
    //         timeout: None,
    //     }) {
    //         Ok(_) => {
    //             // theorically, this should never fail
    //         }
    //         Err(e) => {
    //             // 處理 GPU 錯誤，例如 Device Lost 或 Timeout
    //             eprintln!("GPU Poll Error: {:?}", e);
    //             return None;
    //         }
    //     }

    //     if let Ok(Ok(_)) = rx.await {
    //         // 4. 取得映射後的數據範圍
    //         let data = buffer_slice.get_mapped_range();
            
    //         // 5. 處理對齊（Padding）問題
    //         // GPU 的 bytes_per_row 往往大於畫面的寬度 * 4，必須剔除多餘的空白
    //         let mut result = Vec::with_capacity((self.config.width * self.config.height * 4) as usize);
    //         for chunk in data.chunks(bytes_per_row as usize) {
    //             result.extend_from_slice(&chunk[.. (self.config.width * 4) as usize]);
    //         }

    //         // 6. 必須手動解除映射，否則下一影格 GPU 無法寫入
    //         drop(data);
    //         self.yuv_output_staging_buffer.unmap();
            
    //         Some(result) // 這回傳的是 BGRA 格式的原始數據
    //     } else {
    //         None
    //     }
    // }

    // pub async fn capture_yuv_frame(&self) -> Option<Vec<u8>> {
    //     // 1. 定義 buffer slice
    //     let buffer_slice = self.yuv_output_staging_buffer.slice(..);
        
    //     // 2. 請求映射（Map）該緩衝區以便讀取
    //     // 使用 Oneshot Channel 來等待非同步結果
    //     let (tx, rx) = futures::channel::oneshot::channel();
    //     buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
    //         tx.send(result).unwrap();
    //     });

    //     // 3. 輪詢 GPU 執行指令
    //     match self.device.poll(wgpu::PollType::Wait { // wgpu::Maintain is renamed to wgpu::PollType in wpug v25.0.0 https://github.com/gfx-rs/wgpu/releases?q=maintain&expanded=true
    //         submission_index: None,
    //         timeout: None,
    //     }) {
    //         Ok(_) => {
    //             // theorically, this should never fail
    //         }
    //         Err(e) => {
    //             // 處理 GPU 錯誤，例如 Device Lost 或 Timeout
    //             eprintln!("GPU Poll Error: {:?}", e);
    //             return None;
    //         }
    //     }

    //     // 4. 等待映射結果
    //     if let Ok(Ok(_)) = rx.await {
    //         let data = buffer_slice.get_mapped_range();
            
    //         // 這裡的大小應該剛好是 width * height * 1.5
    //         // 因為我們在 WGSL 裡是用 u32 打包的，所以長度會剛好對齊
    //         let result = data.to_vec();
            
    //         // 記得釋放映射，否則下一幀 GPU 無法寫入
    //         drop(data);
    //         self.yuv_output_staging_buffer.unmap();
            
    //         Some(result)
    //     } else {
    //         None
    //     }
    // }
}

