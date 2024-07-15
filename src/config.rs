// inspired by https://github.com/DoubleCheck0001/rust-minecraft-proxy/blob/47923992632b4990e9149b663817cbef4f01e388/src/config.rs

use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use log::info;
use regex::Regex;
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
    host_regex: Regex,
}

impl ConfigStruct {
    fn parse(&self) -> Config {
        let joined_hosts = (&self.hosts)
            .into_iter()
            .map(|host| String::from(host))
            .fold(
                String::new(),
                |a, b| {
                    if a.is_empty() {
                        b
                    } else {
                        a + "|" + &b
                    }
                },
            );
        let host_regex_str = String::from("^(?:") + &joined_hosts + ")$";
        let host_regex = Regex::new(host_regex_str.as_str());

        let config = Config {
            bind: self.bind_address.parse().unwrap(),
            connect: self.connection_address.parse().unwrap(),
            host_regex: host_regex.unwrap(),
        };

        info!("  bind: {}", config.bind.to_string());
        info!("  connection target: {}", config.connect.to_string());
        info!("  host regex: {}", config.host_regex.as_str());

        config
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
        return self.host_regex.is_match(host.as_str());
    }

    pub fn get_bind_addr(&self) -> SocketAddr {
        return self.bind;
    }

    pub fn get_connect_addr(&self) -> SocketAddr {
        return self.connect;
    }
}
