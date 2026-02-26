use std::{sync::Arc, sync::Mutex};

use winit::{
    application::ApplicationHandler,
    event::*,
    window::{Window, WindowId},
    dpi::PhysicalSize,
    event_loop::{ActiveEventLoop},
};

use crate::wgpu_app::WgpuApp;


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

        let window_attributes = Window::default_attributes().with_title("tutorial1-window");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let wgpu_app = pollster::block_on(WgpuApp::new(window));
        let mut app = self.app.lock().unwrap();
        *app = Some(wgpu_app);
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
            WindowEvent::Resized(physical_size) => {
                // 窗口大小改变
                if physical_size.width == 0 || physical_size.height == 0 {
                    // 处理最小化窗口的事件
                    print!("minimized\n");
                    todo!();
                } else {
                    app.set_window_resized(physical_size);
                    
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                // 键盘事件
            }
            WindowEvent::RedrawRequested => {
                // surface重绘事件
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