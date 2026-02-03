# mc-proxy
This repo contains a WIP reverse proxy for Minecraft servers. In its current state it binds to `127.0.0.1:25565` and will proxy incoming connections to a hardcoded set of destinations.

# Next Steps
- Add configurability
  - address and port the proxy listens on
  - mappings
- Refactor into a more sane project structure
- Implement support for encryption even when destination game server doesn't have it enabled
