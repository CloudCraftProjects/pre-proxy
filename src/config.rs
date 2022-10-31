// inspired by https://github.com/DoubleCheck0001/rust-minecraft-proxy/blob/47923992632b4990e9149b663817cbef4f01e388/src/config.rs

use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;

use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    bind_address: String,
    redirect_address: String,
}

impl Config {
    pub fn load_or_init(path: &Path) -> Config {
        if path.exists() {
            return serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        }

        info!("config file doesn't exist, using defaults");
        let def_config = Config {
            bind_address: String::from("127.0.0.1:50580"),
            redirect_address: String::from("192.168.42.42:50542"),
        };

        let serialized = serde_yaml::to_string(&def_config).unwrap();
        fs::write(path, &serialized).unwrap();
        return def_config;
    }

    fn parse_address(address: String) -> SocketAddr {
        let mut addr_parts = address.split(":");
        let ip = addr_parts.next().expect("no ip provided");
        let port: u16 = addr_parts.next().map(|port_string| port_string.parse().expect("invalid port")).unwrap();
        return (ip, port).to_socket_addrs().expect("not valid socket address").next().expect("no valid socket address");
    }

    pub fn get_bind_addr(self) -> SocketAddr {
        return Config::parse_address(self.bind_address);
    }

    pub fn get_redir_addr(self) -> SocketAddr {
        return Config::parse_address(self.redirect_address);
    }
}
