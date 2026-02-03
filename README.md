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


## TODO

### Self-hosting
The pricing of spacetimeDB's Maincloud is pretty substantial at scale... For just the movement tick at 20hz (30days) the price is over $200. However a self-hosted EC2 instance
would probably be more like $50 to process the same amount of data/traffic.

- https://spacetimedb.com/docs/how-to/deploy/self-hosting/


### [ ] Pathfinding
- The main source of pathfinding should be client/server agnostic in the `shared` folder.
- Use [`rerecast`](https://github.com/janhohenheim/rerecast) for navmesh generation from Rapier3D colliders and [`polyanya`](https://docs.rs/polyanya) for any-angle pathfinding (engine/client agnostic).
- Precompute the full navmesh in the `init` reducer from DB-based colliders, convert to Polyanya format, serialize vertices/polygons, store in a singleton table row.
- Client/server load the precomputed navmesh from DB for queries.
- Client sends `request_move()` reducer with `MoveIntent::Path` (points). Server runs pathfinding on precomputed navmesh, verifies path validity, updates position in DB.
- Client may compute predictive paths locally using same navmesh; rare differences reconciled on server update (usually unnoticeable).

### [ ] Attribute Set/System
- Attributes are integer and float values that represent some statistics/meaning about an actor such as health, strength, critical hit chance etc...
- Attributes are split and grouped by function and "hot" vs "cold" (Vital, Primary, Seconday)
  - Vital (health, mana, stamina)
  - Primary (strength, dexterity, fortitude, intelligence, piety)
  - Secondary (movement_speed, max_health, max_mana, max_stamina)
- Secondary stats are computed based on Vital and Primary attributes
- Vital attributes are considered "Hot", as they need to potentially update each frame, so they should be subscribed to separately to reduce network overhead
- All attributes should be restricted to thei owner, meaning one actor cannot view the attributes of another actor (security and network perf).
- Computed stats do not _need_ to be stored in a table but could as a cache, however, they might make more sense as regular functions in the `shared` (for prediction) or just as functions in server.

### [ ] Ability System
- The ability system is a set of reducers, tables, and functions that are used to represent the abilities the player can activate and the response and action of them.
- Abilities are spells or actions the player activates via keybinds, passively, or triggered by something in the world (Fireball, health regen, teleport, etc...)
- Abilities can be immediate or targeted cast
  - Immediate the spell is immediately triggered and based on the details of the abilities can target the caster or others - this is defined by the spell itself.
  - Targetted abilities "queue" up the spell and wait for the player to provide input to cast it.
- Abilities should have cooldowns and costs associated with them (available to cast every 1.2s and costs 50mp)

  **Example** (Targetted Fireball)
  - The player goes into a spell menu and binds Fireball to the `A` key
  - The player presses the `A` key to queue up the Fireball spell, the cursor changes to a wand to indicate
  - The player pressed the `LMB` on top of a monster, the Fireball is triggered with the monster as the target
  - The ability is de-queued and goes into cooldown
