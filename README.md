# pre-proxy

A fast and simple Minecraft proxy which validates the initial protocol handshake and forwards connections using the Proxy Protocol V2.

## Configuration

This proxy can be configured using the `config.yml` file in YAML format. The following options are available:

- `bind_address`: A single IPv4 or IPv6 socket address to bind on
- `connection_address`: A single IPv4 or IPv6 socket address to forward connections to
- `hosts`: A list of regular expressions which will be matched against the hostname used for connecting, connections which fail to match will be denied

Please note that unix domain sockets are not supported at the moment.

## How to compile

To compile this software you need to have rust installed. Installation via [rustup](https://rustup.rs/) is recommended.

1. Clone this repository
2. Run `cargo build --release`
3. Use the binary at `target/release/cc-pre-proxy`

## How to use (via binary)

Run the binary once, a sample `config.yml` file will be generated. See above for how to configure the `config.yml` file and then run the binary again.

## How to use (via docker)

Build the docker image using the [Dockerfile](./Dockerfile) or directly use the docker compose setup in this repository (just use `docker compose up -d` after configuring `docker-compose.override.yml`).

## Licensing

This software is licensed under the terms of the [GNU General Public License v3.0](./LICENSE).

This software uses code from the following projects:
- [https://github.com/Eoghanmc22/rust-mc-bot](https://github.com/Eoghanmc22/rust-mc-bot/tree/b77382357cb0995446a6973d786085aa3abea029) (GPL-3.0)
- [https://github.com/DoubleCheck0001/rust-minecraft-proxy](https://github.com/DoubleCheck0001/rust-minecraft-proxy/tree/47923992632b4990e9149b663817cbef4f01e388) (MIT)
- [https://github.com/mqudsi/tcpproxy](https://github.com/mqudsi/tcpproxy/tree/99f64a3b3d7509ca77fbfa5e9cade48d72202166) (MIT)
