use std::{sync::Arc, sync::Mutex};

use winit::{
    application::ApplicationHandler,
    event::*,
    window::{Window, WindowId},
    event_loop::{ActiveEventLoop},
    keyboard::Key,
};


use crate::{wgpu_app::WgpuApp, config, osc_server};

#[derive(Default)]
pub struct WgpuAppHandler {
    app: Arc<Mutex<Option<WgpuApp>>>,
}


#[cfg(target_os = "macos")]
fn prevent_app_nap() {
    use objc::{msg_send, sel, sel_impl, runtime::Object, class};
    use std::os::raw::c_char;

    unsafe {
        let process_info: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
        
        // 這些是 macOS 的 Activity Options 位元遮罩
        // NSActivityUserInitiated (0x000000FF) | NSActivityLatencyCritical (0xFF00000000)
        let options: u64 = 0x000000FF | 0xFF00000000;

        // 建立 NSString 的 C 方式：呼叫 NSString 的 stringWithUTF8String:
        let reason_str = "VR Streaming Performance\0".as_ptr() as *const c_char;
        let reason: *mut Object = msg_send![class!(NSString), stringWithUTF8String: reason_str];

        // 呼叫 beginActivityWithOptions:reason:
        let _: () = msg_send![process_info, beginActivityWithOptions:options reason:reason];
        println!("macOS App Nap disabled.");
    }
}

#[cfg(target_os = "windows")] // not tested
fn prevent_windows_throttling() {
    // 這裡需要 windows crate 的 Win32_System_Power feature
    use windows::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED
    };
    unsafe {
        // ES_CONTINUOUS 表示持續狀態，後面兩個確保系統和螢幕不進入低功耗
        SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED);
        println!("Windows power throttling disabled.");
    }
}

#[cfg(target_os = "linux")] // not tested
fn boost_linux_performance() {
    unsafe {
        // 設定 nice 值，範圍 -20 到 19，越低優先權越高
        // 注意：設定負值通常需要 root 權限或特定的 CAP_SYS_NICE
        libc::setpriority(libc::PRIO_PROCESS, 0, -10);
    }
    println!("Linux process priority boosted.");
    use std::process::Command;
    // 抑制省電模式
    let _ = Command::new("xdg-screensaver")
        .arg("suspend")
        .spawn();
    println!("Linux idle inhibited via xdg-screensaver.");
}

impl ApplicationHandler for WgpuAppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "macos")]
        prevent_app_nap();

        #[cfg(target_os = "windows")]
        prevent_windows_throttling();

        #[cfg(target_os = "linux")]
        boost_linux_performance();

        // 恢复事件
        if self.app.lock().unwrap().is_some() {
            return;
        }

        let appconfig = config::load();
        println!("appconfig: {:#?}\n", appconfig);

        // let monitor = ActiveEventLoop::primary_monitor(event_loop).unwrap();
        // let scale_factor = monitor.scale_factor();
        // let size: LogicalSize<f64> = monitor.size().to_logical(scale_factor);

        let target_monitor = event_loop.available_monitors().find(|m| {
            println!("monitor name: {}\n monitor scale: {} = {}x{}", m.name().unwrap(), m.scale_factor(), m.size().width,m.size().height);
            // m.name().map(|n| n.contains(&appconfig.system.display_monitor)).unwrap_or(false)
            // m.size().width == 1180 && m.size().height == 820
            m.size().width == 3840 && m.size().height == 2160
            // false
        }).or_else(|| {
            println!("cannot find monitor");
            event_loop.primary_monitor()
        }); // 找不到就用主螢幕

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        if let Some(monitor) = target_monitor {
            // let size  = screen::get_frame_size().unwrap();

            // let video_mode = monitor.video_modes().next().unwrap();
            // let size = video_mode.size();

            let size = monitor.size();

            println!("monitor name: {}", monitor.name().unwrap());

            print!("size: {:?}\n", size);

            // println!("appconfig: {:#?}\n", appconfig);

            let window_attributes = 
                Window::default_attributes()
                .with_title("Glissando")
                .with_min_inner_size(size).with_max_inner_size(size) // genius idea!
                .with_inner_size(size).with_resizable(false).with_decorations(false);
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(monitor))));
            window.set_visible(true);
            let wgpu_app = pollster::block_on(WgpuApp::new(window.clone(), appconfig.clone()));
            let mut app = self.app.lock().unwrap();
            *app = Some(wgpu_app);

            // todo!("correspond vr params");
            // osc_server::start_osc_server(appconfig.network.osc_server_port, appconfig, self.app.clone().lock().unwrap().as_ref().unwrap().vr_params.clone()); //params, appconfig.network.osc_server_port);
            if let Some(ref mut app) = *app {//*self.app.lock().unwrap() { // 會造成二次 lock :卡死
                let port = appconfig.network.osc_server_port;
                let params = Arc::clone(&app.vr_params); 
                println!("port: {}",port);
                osc_server::start_osc_server(port, appconfig, params);
            }
            else {
                // panic!("app is none");
                println!("app is none");
            }

            println!("window request redraw");
            window.request_redraw();

            // screen::screen_shot().unwrap();
            // self.app.lock().unwrap().replace(wgpu_app);
        }

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
            WindowEvent::KeyboardInput {event, .. } => {
                // 键盘事件
                if event.state == ElementState::Pressed && event.logical_key == Key::Character("r".into()) {
                    println!("Key r pressed");
                    // if let Some(ref mut app) = *self.app.lock().unwrap() {
                    //     osc_server::adjust_center(app.vr_params.clone());
                    //     println!("Orientation Calibrated!");
                    // }// already locked at binding
                    println!("{:?}",osc_server::adjust_center(app.vr_params.clone()));
                }
                else if event.state ==  ElementState::Pressed && event.logical_key == Key::Character("q".into()) {
                    println!("Key q pressed");
                    
                    println!("quit");
                    event_loop.exit();
                }
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

                // if let Some(yuv_data) = pollster::block_on(app.capture_yuv_frame()) {
                //     // 如果 streamer 已經初始化，就推出去
                //     if let Some(ref streamer) = app.streamer {
                //         // println!("Pushing frame, size: {}", yuv_data.len());
                //         let _ = streamer.push_frame(yuv_data);
                //     }
                // }

                // 除非我们手动请求，RedrawRequested 将只会触发一次。
                app.update();
                app.window.request_redraw();
            }
            _ => (),
        }
    }
}