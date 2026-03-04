use scap::{
    capturer::{Capturer, Options},
};

use anyhow::{Ok};

use crate::config::{AppConfig, CaptureArgs};


pub fn init_capture(app_config: Option<AppConfig>) -> anyhow::Result<Capturer> {
    let app_config = app_config.unwrap_or(AppConfig { capture: CaptureArgs { fps: 60, ..Default::default()}, ..Default::default() });
    println!("app_config: {:#?}\n", app_config);
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
    
    let mut target:Option<scap::Target> = None; // None captures the primary display>

    let targets = scap::get_all_targets();

    if app_config.capture.display_name.len() != 0 // display is set
    || app_config.capture.display_name != "None" 
    || app_config.capture.display_name != "" {
        target = targets.clone()
            .into_iter()
            .find(|target| {
                if let scap::Target::Display(display) = target {
                    display.title == app_config.capture.display_name
                } else {
                    false
                }
            }
        );
    
    }
    
    
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
        fps: app_config.capture.fps, // temporary
        target: target, // None captures the primary display // temporary
        show_cursor: true,
        show_highlight: true,
        excluded_targets: excluded_target.map(|t| vec![t]),
        output_type: scap::frame::FrameType::BGRAFrame,
        output_resolution: app_config.capture.resolution, // _1080p,// temporary
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
    let mut capturer = init_capture(None).unwrap();
    let frame_size = capturer.get_output_frame_size(); // [u32,2]
    Ok(winit::dpi::LogicalSize::new(frame_size[0], frame_size[1]))
}