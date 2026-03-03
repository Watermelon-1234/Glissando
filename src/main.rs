use scap::capturer;

#[allow(dead_code)]
use winit::{
    event_loop::EventLoop,
    error::EventLoopError
};

use std::{default::Default, env};

mod wgpu_app_handler;
use wgpu_app_handler::WgpuAppHandler;


mod wgpu_app;
mod screen;

use env_logger;


fn main()-> Result<(), EventLoopError> {
    
    //todo!("implement window resume");
    env_logger::init();

    let events_loop = EventLoop::new().unwrap();
    
    // events_loop.set_control_flow(ControlFlow::Poll); // temporary
    let mut app = WgpuAppHandler::default();
    events_loop.run_app(&mut app)
}

// fn main() {
//     print!("test_only main() start\n");
//     env_logger::init();
//     print!("test_only main() init_logger\n");
//     let mut capturer = screen::init_capture().unwrap();
//     capturer.start_capture();
//     let mut current_frame = capturer.get_next_frame().unwrap();
//     if let scap::frame::Frame::BGRA(ref frame) = current_frame {
//         print!("frame: {:?}\n", frame.data.len());
//     }
//     print!("test_only main()\n");
//     capturer.stop_capture();
//     print!("test_only main() end\n");
// }