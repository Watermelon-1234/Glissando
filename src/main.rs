#[allow(dead_code)]
use winit::{
    event_loop::EventLoop,
    error::EventLoopError
};

use std::default::Default;

mod wgpu_app_handler;
use wgpu_app_handler::WgpuAppHandler;

mod wgpu_app;

use env_logger;

fn main()-> Result<(), EventLoopError> {
    env_logger::init();

    let events_loop = EventLoop::new().unwrap();
    // events_loop.set_control_flow(ControlFlow::Poll);
    let mut app = WgpuAppHandler::default();
    events_loop.run_app(&mut app)
}