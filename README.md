# mc-proxy
This repo contains a WIP reverse proxy for Minecraft servers. It listens on a
configurable address (default: `127.0.0.1:25565`) and will proxy incoming 
connections to a  hardcoded set of destinations.

# Next Steps
- Add configurability
  - mappings
- Refactor into a more sane project structure
- Implement support for encryption even when destination game server doesn't
have it enabled

# Ideas
- Use the [notify](https://crates.io/crates/notify) crate to watch a mappings 
file (toml?) in a config directory to allow live updates

# CLI
```
Reverse proxy for Minecraft servers

Usage: mc-proxy [OPTIONS]

Options:
  -l, --listen <LISTEN>  [default: 127.0.0.1:25565]
  -h, --help             Print help
  -V, --version          Print version
```