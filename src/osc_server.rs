use rosc::{OscPacket, OscType};
use std::net::{UdpSocket, SocketAddrV4};
use std::sync::{Arc, Mutex};
use crate::wgpu_app::VRParams;
use crate::config::{AppConfig};

pub fn start_osc_server(port: u16, app_config: AppConfig, params: Arc<Mutex<VRParams>>) {//params: Arc<Mutex<VRParams>>, port: u16) {
    println!("osc server started");
    std::thread::spawn(move || {
        let addr = SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), port);
        let socket = UdpSocket::bind(addr).expect("unable to bind OSC server");
        println!("OSC Server listening to {}", port);

        let mut buf: [u8; 1536] = [0u8; rosc::decoder::MTU];

        loop {
            match socket.recv_from(&mut buf) {
                Ok((size, _)) => { // I hope no boring guy use multi-device to send OSC
                    // println!("-------------------------");
                    // println!("OSC packet received of size {}", size);
                    // println!("OSC packet: {}", std::str::from_utf8(&buf[..size]).unwrap()); // this will crash "called `Result::unwrap()` on an `Err` value: Utf8Error { valid_up_to: 65, error_len: Some(1) }"
                    match rosc::decoder::decode_udp(&buf[..size]) {
                        Ok((_, packet)) => {
                            // println!("OSC packet decoded");
                            handle_packet(packet, app_config.clone(), params.clone());
                        }
                        Err(e) => eprintln!("OSC decode error: {}", e), // OSC decode error: error reading from buffer: Tag means osc sent in json format
                    }
                }
                Err(e) => eprintln!("OSC receive error: {}", e),
            }
        }
    });
}

fn handle_packet(packet: OscPacket, app_config: AppConfig, params: Arc<Mutex<VRParams>>) {// , params: &Arc<Mutex<VRParams>>) {
    // println!("-------------------------");
    // println!("OSC packet received");
    match packet {
        OscPacket::Message(msg) => {
            // println!("OSC address: {}", msg.addr);
            // println!("OSC arguments: {:?}", msg.args);
            // todo!("⬇️");
            // let mut p = params.lock().unwrap();

            // accel grivity gyro quaternion
            // let accel_addr = format!("/ZIGSIM/{}/accel", app_config.network.device_uuid);
            // let gravity_addr = format!("/ZIGSIM/{}/gravity", app_config.network.device_uuid);
            // let gyro_addr = format!("/ZIGSIM/{}/gyro", app_config.network.device_uuid);
            let quaternion_addr = format!("/ZIGSIM/{}/quaternion", app_config.network.device_uuid);

            // println!("quaternion_addr: {}", quaternion_addr);

            match msg.addr.as_str() {
                // addr if addr == accel_addr => {//accel_addr => { // match 是 match pattern not value 
                //     if let [x, y, z] = msg.args.as_slice() {
                //         // test
                //         println!("accel: ({}, {}, {})", x, y, z); 
                //         // todo!("calculate offset");
                //     }
                // },
                // addr if addr == gravity_addr => {// gravity_addr=> {
                //    if let [OscType::Float(x), OscType::Float(y), OscType::Float(z)] = msg.args.as_slice() {
                //         // test
                //         // println!("grivity: ({}, {}, {})", x, y, z); 
                //         let mut p = params.lock().unwrap();
                //         p.gravity = [*x, *y, *z];                        // todo!("calculate offset");
                //     } 
                // },
                // addr if addr == gyro_addr => { //gyro_addr => {
                //     if let [x, y, z] = msg.args.as_slice() {
                //         // test
                //         println!("gyro: ({}, {}, {})", x, y, z); 
                //         // todo!("calculate offset");
                //     }
                // },
                addr if addr == quaternion_addr => {
                    if let [OscType::Float(x), OscType::Float(y), OscType::Float(z), OscType::Float(w)] = msg.args.as_slice() {
                        // test
                        // println!("quaternion: ({}, {}, {}, {})", x, y, z, w);                         
                        let mut p = params.lock().unwrap();
                        // p.q_current = [*x, *y, *z, *w];
                        // pre-modify by phone_orientation
                        let phone_orientation = app_config.system.phone_orientation;
                        // println!("phone_orientation: {}, to radians: {}", phone_orientation, phone_orientation.to_radians());
                        // p.q_current = rotate_around_y(p.q_current, phone_orientation.to_radians());
                        if phone_orientation == -90.0 {
                            p.q_current = [-*y, *x, *z, *w];
                        }
                        else if phone_orientation == 90.0 {
                            p.q_current = [*y, -*x, *z, *w];
                        }
                        else 
                        {
                            p.q_current = [*x, *y, *z, *w];
                        }
                    }
                },
                _ => {}
            }
        }
        OscPacket::Bundle(bundle) => {
            // println!("received OSC bundle packet");
            for p in bundle.content {
                handle_packet(p, app_config.clone(), params.clone()); // , params);
            }
        }
    }
}

pub fn adjust_center(params: Arc<Mutex<VRParams>>) -> [f32; 4] {
    let mut p = params.lock().unwrap();
    p.q_base = p.q_current;
    p.q_base
}

// fn rotate_around_y(q: [f32; 4], angle_rad: f32) -> [f32; 4] {
//     // R(angel_rad)*V(x,y)
//     // let mut x = q[0];
//     // let mut y = q[1];

//     // x = x * (angle_rad.cos()) - y * (angle_rad.sin());
//     // y = x * (angle_rad.sin()) + y * (angle_rad.cos());
//     // [x, y, q[2], q[3]] 

//     let half_angle = angle_rad * 0.5;
//     let s = half_angle.sin();
//     let c = half_angle.cos();

//     // 建立繞 Y 軸旋轉的四元數 [x, y, z, w]
//     // 繞 Y 軸旋轉的標準形式是 [0, sin(theta/2), 0, cos(theta/2)]
//     let ry = [0.0, s, 0.0, c];

//     // 執行四元數乘法: ry * q
//     // 公式: 
//     // new_w = w1w2 - x1x2 - y1y2 - z1z2
//     // new_x = w1x2 + x1w2 + y1z2 - z1y2
//     // new_y = w1y2 - x1z2 + y1w2 + z1x2
//     // new_z = w1z2 + x1y2 - y1x2 + z1w2
    
//     let [qx, qy, qz, qw] = q;
//     let [rx, ry_y, rz, rw] = ry; // rx, rz 是 0，可以簡化但寫完整比較安全

//     [
//         rw * qx + rx * qw + ry_y * qz - rz * qy, // x
//         rw * qy - rx * qz + ry_y * qw + rz * qx, // y
//         rw * qz + rx * qy - ry_y * qx + rz * qw, // z
//         rw * qw - rx * qx - ry_y * qy - rz * qz  // w
//     ]
// }