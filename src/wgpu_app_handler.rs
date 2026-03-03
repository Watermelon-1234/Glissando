use std::{sync::Arc, sync::Mutex};

use winit::{
    application::ApplicationHandler,
    event::*,
    window::{Window, WindowId},
    dpi::{PhysicalSize,LogicalSize},
    event_loop::{ActiveEventLoop},
    monitor::MonitorHandle,
};

use crate::{screen, wgpu_app::WgpuApp};

#[derive(Default)]
pub struct WgpuAppHandler {
    app: Arc<Mutex<Option<WgpuApp>>>,

    missed_resize: Arc<Mutex<Option<PhysicalSize<u32>>>>,
}

impl ApplicationHandler for WgpuAppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // 恢复事件
        if self.app.lock().unwrap().is_some() {
            return;
        }

        // let monitor = ActiveEventLoop::primary_monitor(event_loop).unwrap();
        // let scale_factor = monitor.scale_factor();
        // let size: LogicalSize<f64> = monitor.size().to_logical(scale_factor);
        let size  = screen::get_frame_size().unwrap();

        print!("size: {:?}\n", size);

        let window_attributes = 
            Window::default_attributes()
            .with_title("Glissando")
            .with_min_inner_size(size).with_max_inner_size(size) // genius idea!
            .with_inner_size(size).with_resizable(false);
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let wgpu_app = pollster::block_on(WgpuApp::new(window.clone()));
        let mut app = self.app.lock().unwrap();
        *app = Some(wgpu_app);
        window.request_redraw();


        // screen::screen_shot().unwrap();
        // self.app.lock().unwrap().replace(wgpu_app);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // 暂停事件
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let mut binding = self.app.lock().unwrap();
        let app = binding.as_mut().unwrap();
        // 窗口事件
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            // WindowEvent::Resized(physical_size) => {
            //     // 窗口大小改变
            //     if physical_size.width == 0 || physical_size.height == 0 {
            //         // 处理最小化窗口的事件
            //         print!("minimized\n");
            //         todo!();
            //     } else {
            //         app.set_window_resized(physical_size);
                    
            //     }
            // }
            WindowEvent::KeyboardInput { .. } => {
                // 键盘事件
            }
            WindowEvent::RedrawRequested => {
                // surface重绘事件
                // print!("RedrawRequested\n");
                app.window.pre_present_notify();

                match app.render() {
                    Ok(_) => {}
                    // 当展示平面的上下文丢失，就需重新配置
                    Err(wgpu::SurfaceError::Lost) => eprintln!("Surface is lost"),
                    // 所有其他错误（过期、超时等）应在下一帧解决
                    Err(e) => eprintln!("{e:?}"),
                }
                // 除非我们手动请求，RedrawRequested 将只会触发一次。
                app.update();
                app.window.request_redraw();
            }
            _ => (),
        }
    }
}