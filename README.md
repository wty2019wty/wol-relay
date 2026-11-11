# Wake-on-LAN Relay

Relay Wake-on-LAN (WOL) packets between multiple networks or interfaces. This is useful
for environments where devices are on separate subnets and need to be woken up remotely.

## Features

- Listens for WOL packets on specified interfaces
- Relays valid WOL packets to all other monitored networks
- Prevents packet loops and duplicate broadcasts
- Lightweight and easy to deploy (single binary or Docker image)
- Runs on Linux and Windows (x86, x86_64 and ARM64)
- Multi-arch container images (linux/amd64, linux/arm64 and linux/386)
- Accepts both interface names and CIDR notation for target networks

## Usage

### Docker Compose

Add the following service to your `compose.yaml` to relay WOL packets between the `eno1`
and `eno2` network interfaces:

```yaml
services:
  wol-relay:
    image: ghcr.io/danroc/wol-relay:latest
    container_name: wol-relay
    network_mode: host
    command: eno1 eno2
    security_opt:
      - no-new-privileges:true
    restart: unless-stopped
```

> **Note:** `network_mode: host` is required for the container to access host network
> interfaces and broadcast packets.

### Standalone

Prebuilt binaries for Linux and Windows (x86, x86_64 and ARM64) are attached to each
[GitHub release](https://github.com/danroc/wol-relay/releases). You can also build
and run the relay directly on your host:

```sh
# Build the binary
cargo build --release

# Run the relay
sudo ./target/release/wol-relay eno1 eno2
```

Replace `eno1` and `eno2` with the names of your network interfaces. On Windows, use
the interface aliases shown by `netsh interface` (e.g. `"Ethernet 2"`), and note that
sending broadcast packets may require elevated privileges.

### Options

The relay can also be configured with flags or environment variables (flags win):

| Flag                 | Environment variable   | Default | Description                                                             |
| -------------------- | ---------------------- | ------- | ----------------------------------------------------------------------- |
| `-l`, `--log-level`  | `WOL_RELAY_LOG_LEVEL`  | `info`  | Log level: `debug`, `info`, `warn`, `error`                             |
| `-F`, `--log-format` | `WOL_RELAY_LOG_FORMAT` | `auto`  | Log format: `auto` (text on a terminal, JSON otherwise), `json`, `text` |
| `--version`          |                        |         | Print the version and exit                                              |

Targets can also be set via `WOL_RELAY_TARGETS` (comma-separated), e.g. in Docker
Compose:

```yaml
services:
  wol-relay:
    image: ghcr.io/danroc/wol-relay:latest
    network_mode: host
    environment:
      WOL_RELAY_TARGETS: eno1,eno2
```

Logs are structured and written to stdout: one JSON object per line with `time`,
`level`, `message` and event fields such as `source_ip`, `target_network`, `target_mac`
and `packet_size`. `RUST_LOG` overrides the configured level.

### CIDR Targets

You can also specify networks directly using CIDR notation, including /32 for unicast
relay to individual hosts:

```sh
# Relay between interface eno1 and a remote subnet
sudo ./wol-relay eno1 10.0.0.0/24

# Unicast relay to a specific host
sudo ./wol-relay eno1 192.168.2.5/32

# Mixed interfaces and CIDRs
sudo ./wol-relay eno1 eno2 10.0.0.0/24 172.16.0.10/32
```

## Requirements

- Linux or Windows (x86, x86_64 and ARM64)
- Rust 1.88+ (for building from source)
- Sufficient privileges to access network interfaces and send broadcast packets

## Development

- Run tests:

  ```sh
  cargo test
  ```

- Build Docker image:

  ```sh
  docker build -t wol-relay .
  ```
