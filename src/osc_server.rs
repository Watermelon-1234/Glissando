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
                //    if let [x, y, z] = msg.args.as_slice() {
                //         // test
                //         println!("grivity: ({}, {}, {})", x, y, z); 
                //         // todo!("calculate offset");
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
                        p.q_current = [*x, *y, *z, *w];
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