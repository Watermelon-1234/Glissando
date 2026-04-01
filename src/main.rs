#[allow(dead_code)]
use winit::{
    event_loop::EventLoop,
    error::EventLoopError
};

use std::{default::Default};

mod wgpu_app_handler;
use wgpu_app_handler::WgpuAppHandler;

use env_logger;


mod wgpu_app;
mod screen;
mod config;
mod osc_server; 
mod debug;



fn main()-> Result<(), EventLoopError> {
    
    //todo!("implement window resume");
    env_logger::init();

    let events_loop = EventLoop::new().unwrap();
    
    // events_loop.set_control_flow(ControlFlow::Poll); // temporary
    let mut app = WgpuAppHandler::default();
    events_loop.run_app(&mut app)
}