// inspired by https://github.com/DoubleCheck0001/rust-minecraft-proxy/blob/47923992632b4990e9149b663817cbef4f01e388/src/config.rs

use std::collections::HashSet;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use log::info;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct ConfigStruct {
    bind_address: String,
    connection_address: String,
    hosts: Vec<String>,
}

#[derive(Clone)]
pub struct Config {
    bind: SocketAddr,
    connect: SocketAddr,
    hosts: HashSet<String>,
}

impl ConfigStruct {
    fn parse(&self) -> Config {
        Config {
            bind: self.bind_address.parse().unwrap(),
            connect: self.connection_address.parse().unwrap(),
            hosts: (&self.hosts)
                .into_iter()
                .map(|host| String::from(host))
                .collect(),
        }
    }
}

impl Config {
    pub fn load_or_init(path: &Path) -> Config {
        let cfg_struct: ConfigStruct;
        if path.exists() {
            info!("reading config file at {}", path.display());
            cfg_struct = serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        } else {
            info!(
                "config file at {} doesn't exist, using defaults",
                path.display()
            );
            cfg_struct = ConfigStruct {
                bind_address: String::from("127.0.0.1:50580"),
                connection_address: String::from("192.168.42.42:50542"),
                hosts: Vec::new(),
            };
        }

        let serialized = serde_yaml::to_string(&cfg_struct).unwrap();
        fs::write(path, &serialized).unwrap();

        return cfg_struct.parse();
    }

    pub fn check_host(&self, mut host: &str) -> bool {
        if host.contains("\0") {
            host = host.split_once("\0").unwrap().0;
        }

        if host.ends_with(".") {
            host = host.split_at(host.len() - 1).0;
        }

        let host = host.to_lowercase();
        return self.hosts.contains(&host);
    }

    pub fn get_bind_addr(&self) -> SocketAddr {
        return self.bind;
    }

    pub fn get_connect_addr(&self) -> SocketAddr {
        return self.connect;
    }
}
