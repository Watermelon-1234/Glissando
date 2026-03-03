use env_logger::Target;
use scap::{
    capturer::{self, Area, Capturer, Options, Point, Size},
    frame::{self, Frame},
};

use anyhow::{Ok, Result};

pub fn init_capture() -> anyhow::Result<Capturer> {
    // print!("test_only init_capture() start\n");
    if !scap::is_supported() 
    {
        return Err(anyhow::anyhow!("scap is not supported"));
    }
    // print!("test_only init_capture() scap is supported\n");
    if !scap::has_permission()
    {
        log::warn!("scap has no permission, requesting permission");
        if !scap::request_permission()
        {
            log::error!("scap request permission failed");
            return Err(anyhow::anyhow!("scap request permission failed"));
        }
    }
    // print!("test_only init_capture() scap has permission\n");
    // Get recording targets
    let targets = scap::get_all_targets();
    // println!("Targets: {:?}", targets);
    // print!("test_only init_capture() targets: {:?}\n", targets);
    let excluded_target = targets
        .into_iter()
        .find(|target| {
            if let scap::Target::Window(window) = target {
                window.title == "Glissando"
            } else {
                false
            }
        }
    );
    // print!("test_only init_capture() excluded_target: {:?}\n", excluded_target);
    // Create Options
    let options = Options {
        fps: 60, // temporary
        target: None, // None captures the primary display // temporary
        show_cursor: true,
        show_highlight: true,
        excluded_targets: excluded_target.map(|t| vec![t]),
        output_type: scap::frame::FrameType::BGRAFrame,
        output_resolution: scap::capturer::Resolution:: _1080p,// _720p, // temporary
        ..Default::default()
    };
    // print!("test_only init_capture() options: {:?}\n", options);
    // Create Capturer
    let capturer = Capturer::build(options).unwrap();
    // capturer.start_capture();
    // print!("test_only init_capture()\n");
    Ok(capturer)
}

// pub fn get_frame(capturer: Capturer) -> anyhow::Result<Frame>{
//     let frame = capturer.get_next_frame().unwrap();
//     print!("frame: {:?}\n", frame);
//     Ok(frame)
// }

pub fn get_frame_size() -> anyhow::Result<winit::dpi::LogicalSize<u32>> {
    let mut capturer = init_capture().unwrap();
    let frame_size = capturer.get_output_frame_size(); // [u32,2]
    Ok(winit::dpi::LogicalSize::new(frame_size[0], frame_size[1]))
}