use notify::{Watcher, RecursiveMode, event};
use std::sync::{Arc, Mutex};
use crate::wgpu_app_handler::WgpuAppHandler;
use crate::config;

fn watch_settings(app_handler: Arc<Mutex<WgpuAppHandler>>) {
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        match res {
            Ok(event) => {
                // 如果文件被寫入（存檔）
                if event.kind.is_modify() {
                    println!("Settings.toml changed! Reloading...");
                    let new_config = config::load();
                    // 這裡要把 new_config 更新到你的 app 實例中
                    // 例如：app.lock().unwrap().update_config(new_config);
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }).unwrap();

    watcher.watch(std::path::Path::new("settings.toml"), RecursiveMode::NonRecursive).unwrap();
    
    // 為了讓 watcher 持續存在，我們需要 loop 或將其存起來
    loop { std::thread::sleep(std::time::Duration::from_secs(1)); }
}