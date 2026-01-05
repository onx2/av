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

800-255-7828

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

### [ ] Species and Class system
- Races and classes determine the look, feel, inherit perks, equipment and ability restrictions, starting location, etc...
- Species: Human, Tormŏg, Lümycus, Vrask
- Each race has its own set of classes that it applies to, so there are distinct combinations of race + class that can exist
- Each class in Aelynmar offers a unique approach to combat and a deep connection to the world's lore. Click a class below to explore its philosophy and specializations.

<details>
<summary><b>🔥 Arcanist</b> - Ranged DPS / Area Control</summary>
<br>

> *An elite scholar of the Veil who commands the fundamental forces of Aelynmar to reshape the battlefield through raw elemental mastery.*

| Specialization | Playstyle | Description |
| :--- | :--- | :--- |
| **Pyromancer** | High Single-Target | Bypasses enemy defenses to incinerate high-priority threats with focused, soul-searing heat. |
| **Cryomancer** | AoE / Control | Dictates the flow of combat by entombing entire groups of enemies in inescapable, freezing blizzards. |
| **Arcane Weaver** | Generalist / Utility | A versatile mage who manipulates the veil to provide a balance of magical force and utility. |

</details>

<details>
<summary><b>⚔️ Myrmidon</b> - Melee DPS / Tank / Control</summary>
<br>

> *A disciplined veteran of an ancient martial order who balances tactical precision with unwavering resilience to dominate the frontlines.*

| Specialization | Playstyle | Description |
| :--- | :--- | :--- |
| **Legionnaire** | Tank / Defensive | The immovable cornerstone of any formation, wielding sword and shield to neutralize threats while shielding allies. |
| **Centurion** | Heavy AoE DPS | A juggernaut who utilizes massive two-handed weapons to shatter enemy lines and crush groups with overwhelming force. |
| **Vanguard** | Reach / Mobility | A mobile tactician who exploits the reach of a polearm to dictate spacing and disrupt the flow of combat. |

</details>

<details>
<summary><b>💀 Occultist</b> - Ranged DPS / Debuff / Summoning</summary>
<br>

> *A pariah of the forbidden arts who taps into the Veil to dismantle enemies through lingering curses, dark summons, and insidious corruption.*

| Specialization | Playstyle | Description |
| :--- | :--- | :--- |
| **Emberborn** | Resource Attrition | Wields soul-gnawing netherflame whose lingering heat leaves enemies gasping and hollowed of their will. |
| **Corruptor** | Life Leech / AoE | A grim manipulator of decay who leeches vitality to fuel their own power while rotting foes from the inside out. |
| **Plaguespreader** | DoT / Debuff | A patient executioner who blankets the field in toxic mists, eroding the strength and speed of all who oppose them. |

</details>

<details>
<summary><b>🌿 Shaman</b> - Healer / Support / Buffs</summary>
<br>

> *A spiritual bridge between the physical and the unseen, channeling the living breath of Aelynmar to mend the broken and command the wild.*

| Specialization | Playstyle | Description |
| :--- | :--- | :--- |
| **Spiritmender** | Pure Healing | A devoted vessel of life who invokes ancestral spirits to restore the soul and cleanse the body. |
| **Earthshaper** | Defensive Support | A stalwart guardian who weaves the resilience of stone into allies while turning the ground against defilers. |
| **Stormcaller** | Offensive Support | A conduit of fury who dictates battle chaos with the violence of lightning and the disorienting rush of the gale. |

</details>

<details>
<summary><b>👣 Stalker</b> - Stealth / Assassin / Utility</summary>
<br>

> *An elusive predator touched by the mists of the Veil, specializing in silent infiltration and the art of the sudden, lethal ambush.*

| Specialization | Playstyle | Description |
| :--- | :--- | :--- |
| **Shadow Walker** | Stealth / Burst | A phantom of the mists who slips through the veil to strike from blind spots before vanishing into thin air. |
| **Sharpshooter** | Long-Range Precision | A patient marksman who dismantles high-value targets with surgical, armor-piercing precision. |
| **Blade Dancer** | Melee Agility | A blurring whirlwind of steel who relies on unmatched reflexes to flow through enemy lines with untouchable momentum. |

</details>

<details>
<summary><b>✨ Templar</b> - Melee DPS / Tank / Support</summary>
<br>

> *A consecrated champion of the Aurora’s Veil, wielding divine radiance as both a crushing hammer and an impenetrable shield.*

| Specialization | Playstyle | Description |
| :--- | :--- | :--- |
| **Guardian** | Heavy Mitigation | An unwavering bastion of faith who anchors the frontline, transmuting divine will into radiant barriers. |
| **Judge** | Holy Burst DPS | A relentless executioner who punishes the corrupt with searing heat and strikes of absolute retribution. |
| **Preacher** | Buffing / Inspiration | A luminous beacon whose sacred hymns and blessings elevate allies to transcend their mortal limits. |

</details>
