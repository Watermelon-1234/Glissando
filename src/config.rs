use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use wgpu::PresentMode;
use scap::capturer::Resolution;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
    pub system: SystemArgs,
    pub capture: CaptureArgs,
    pub vr_render: VrRenderArgs,
    pub debug: DebugArgs,
    pub network: NetworkArgs,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemArgs {
    pub adapter_name: String,
    pub present_mode: PresentMode,
}

impl Default for SystemArgs {
    fn default() -> Self {
        SystemArgs { 
            adapter_name: String::from("None"), 
            present_mode: PresentMode::Fifo,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CaptureArgs {
    pub display_name: String,
    pub fps: u32,
    #[serde(
        deserialize_with="AppConfig::deserialize_resolution",
        serialize_with="AppConfig::serialize_resolution"
    )]
    pub resolution: Resolution,
}

impl Default for CaptureArgs {
    fn default() -> Self {
        CaptureArgs { 
            display_name: String::from("None"), 
            fps: 60,
            resolution: Resolution::_720p,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VrRenderArgs {
    pub offset: f32,
    pub z_distance: f32,
    pub k1: f32,
    pub k2: f32,
}

impl Default for VrRenderArgs {
    fn default() -> Self {
        VrRenderArgs { 
            offset: 0.032,
            z_distance: 1.0,
            k1: 0.21,
            k2: 0.12,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DebugArgs {
    pub enable_debug: bool,
    pub debug_level: String,
    pub log_file_path: String,
}

impl Default for DebugArgs {
    fn default() -> Self {
        DebugArgs { 
            enable_debug: false,
            debug_level: String::from("info"),
            log_file_path: String::from("log.txt"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkArgs {
    pub video_server_port: u16,
    pub device_ip: String,
}

impl Default for NetworkArgs {
    fn default() -> Self {
        NetworkArgs { 
            video_server_port: 5000,
            device_ip: String::from("127.0.0.1"),
        }
    }
}

impl AppConfig {
    pub fn deserialize_resolution<'de, D> (deserializer: D) -> Result<Resolution, D::Error>
        where D: serde::de::Deserializer<'de>
    {
        
        struct ResolutionVisitor;

        impl<'de> serde::de::Visitor<'de> for ResolutionVisitor {
            type Value = Resolution;
        
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("_480p,_720p,_1080p,_1440p,_2160p,_4320p,default")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where E: serde::de::Error, {
                match v { //# _480p,_720p,_1080p,_1440p,_2160p,_4320p,default
                    "_480p" => Ok(Resolution::_480p),
                    "_720p" => Ok(Resolution::_720p),
                    "_1080p" => Ok(Resolution::_1080p),
                    "_1440p" => Ok(Resolution::_1440p),
                    "_2160p" => Ok(Resolution::_2160p),
                    "_4320p" => Ok(Resolution::_4320p),
                    "captured" => Ok(Resolution::Captured),
                    _ => Ok(Resolution::_1080p), // bug
                }
            }
        }
        deserializer.deserialize_str(ResolutionVisitor)
        
    }

    pub fn serialize_resolution<S>(resolution: &Resolution, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer {
        match resolution {
            Resolution::_480p => serializer.serialize_str("_480p"),
            Resolution::_720p => serializer.serialize_str("_720p"),
            Resolution::_1080p => serializer.serialize_str("_1080p"),
            Resolution::_1440p => serializer.serialize_str("_1440p"),
            Resolution::_2160p => serializer.serialize_str("_2160p"),
            Resolution::_4320p => serializer.serialize_str("_4320p"),
            Resolution::Captured => serializer.serialize_str("captured"),
        }
    }

}

pub fn load() -> AppConfig {
    let path = Path::new("settings.toml");
    let toml = fs::read_to_string(path).unwrap();
    println!("toml: {}", toml);
    toml::from_str(&toml).unwrap_or_default()
}