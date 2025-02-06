use std::{collections::HashMap, fs::File, io::Write, net::{IpAddr, Ipv4Addr, SocketAddr}};
use novastar_core;
use sacn_unofficial::{self, packet::ACN_SDT_MULTICAST_PORT, receive::SacnReceiver};
use config::Config;

#[tokio::main]
async fn main() {

    let settings: Config;
    match Config::builder()
    .add_source(config::File::with_name("sacn-novastar-brightness.toml"))
    .build() {
        Ok(s) => settings = s,
        Err(e) => {
            println!("Config Err {e}");
            let mut new_file = File::create("sacn-novastar-brightness.toml").unwrap();
            let _ = new_file.write("universe = 15\ndmx_start = 1".as_bytes());
            let _ = new_file.flush();
            drop(new_file);
            settings = Config::builder()
            .add_source(config::File::with_name("sacn-novastar-brightness.toml"))
            .build()
            .unwrap();
        },
    };
    

    novastar_core::discover();
    let mut dmx_rx = SacnReceiver::with_ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), ACN_SDT_MULTICAST_PORT), None).unwrap();

    match settings.get_int("universe") {
        Ok(s) => dmx_rx.listen_universes(&[s as u16]).unwrap(),
        Err(e) => {
            println!("Config Err {e}");
            dmx_rx.listen_universes(&[15]).unwrap();
        },
    };
    
    let mut dmx_start: usize = 1;
    match settings.get_int("dmx_start"){
        Ok(s) => dmx_start = s as usize,
        Err(e) => println!("Config Err {e}"),
    };

    let mut last_map: HashMap<String, u8> = HashMap::new();

    loop {
        let controllers = novastar_core::get_controllers();
        let _ = match dmx_rx.recv(None) {
            Ok(dmx_packet) => {
                //println!("dmx packet {:?}", dmx_packet[0].values);
                if controllers.len() > 0 {
                    for i in 0..controllers.len() {
                        let current_port = controllers[i].port_name.to_owned();

                        let max = settings.get_int(format!("{}/max", current_port).as_str()).unwrap_or(255) as u8;
                        let min = settings.get_int(format!("{}/min", current_port).as_str()).unwrap_or(0) as u8;
                        let transpose = settings.get_bool(format!("{}/transpose", current_port).as_str()).unwrap_or(false);

                        let last = last_map.entry( current_port.to_string()).or_insert(0).to_owned();

                        let new_bright = match transpose {
                            true => dmx_packet[0].values[i+dmx_start]/255*(max-min)+min,
                            false => dmx_packet[0].values[i+dmx_start],
                        };
                        
                        if last != new_bright {
                            controllers[i].set_brightness(new_bright);
                        }

                        *last_map.get_mut(&current_port).unwrap() = new_bright;
                    }
                } 
            },
            Err(e) => println!("DMX Error {e}"),
        };
    };

}
