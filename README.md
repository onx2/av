# Aurora's Veil

The Veil drifts across Aelynmar, a sentient mist bound to a forgotten artifact. Its presence has warped the land, twisting creatures and unsettling the minds of those who wander too close. The world remains fractured in its shadow, where scattered settlements struggle to endure — caught between fearing the Veil’s influence and seeking to harness its power.

Even in decay, Aelynmar carries a strange beauty. Glowing flora bloom in darkened wilds, ruins hang suspended against broken skies, and the very shape of the land bends in unnatural ways. The Veil shows neither malice nor mercy — its nature is as uncertain as the world it has remade.

The story of its coming is told through Aurora, the one who unearthed the artifact and set the Veil loose upon the world. Now bound to it, her voice is never wholly her own; at times it is a guide, at times a warning, at times the whisper of the Aetherheart itself. Truth lingers in her words, always in fragments — hidden in ruins, scattered journals, and the broken shards of what was once whole.

## Development
- Install Rust and Cargo
- Install SpacetimeDB
- Clone/Fork the repository
- Start the spacetimeDB server: `spacetime start`
- Publish spacetime locally: `spacetime publish --server local --project-path ./server aurorasveil`
- Generate client/server binding  `spacetime generate --lang rust -p ./server -o ./client/src/module_bindings`
- Start the client: `cargo run -p client`

### Local token testing (multiple clients on localhost)
- Obtain a dev identity token from your local SpacetimeDB:
  - `token=$(curl -s -X POST http://localhost:3000/v1/identity)`
- Pass different tokens per client process to test replication:
  - Use the `--token` flag:  
    `cargo run -p client -- --token "$token"`
  - Or via environment variable:  
    `STDB_TOKEN="$token" cargo run -p client`
- Internally the client uses `.with_token(token)` to connect as that identity. Reusing the same token connects as the same user; different tokens simulate different users.

## Todo
  - pathfinding on client
  - client-side prediction for local player movement
  - avian3d on client with settings that match server's raw parry3d implementation
  
### World
  - create a tilemap world (many maps in Tiled editor)
  - render tilemap on client (chunked)
  - use object layer for labeling 3d prop positions and add metadata for collision
  - read in object layer and parse into rows for spacetimeDB

### UI
- title screen
- gameplay screen
- settings menu
- main menu (setting, log out, exit game)

### Ability system
-
