# Symtropy Design Doc: Physicalized Cargo & Material Ledgers

## Working Title

**Logistics Is Not a Menu**

## Core Thesis

Cargo in *Symtropy* is not abstract inventory space.

Cargo is mass, volume, risk, thermodynamics, ownership, evidence, and infrastructure dependency.

A player should never feel like 40 metric tons of steel, toxic filters, valve parts, or archival cartridges have been magically compressed into a harmless spreadsheet.

Core rule:

```text
Logistics is physical until contained.
Contained logistics becomes a registered ledger.
A broken ledger becomes a civic problem.
```

Physical cargo teaches the same lesson as crafting, death, and infrastructure adjudication:

```text
Matter has weight.
Data has authority.
Movement has consequence.
```

---

# 1. Three-Scale Cargo Model

Cargo exists at three scales.

```text
1. Hand scale
2. Container scale
3. Settlement ledger scale
```

Each scale has different simulation rules.

---

## Scale 1: Hand Cargo

Hand cargo is physical and embodied.

Examples:

```text
Archive Witness Cartridge
Blueprint Cartridge
Firmware Slug
Null-Tainted Memory Core
Copper Conduit Pipe Segment
Valve Casing
Ceramic Seal Crate
Portable Battery Cell
Biofilter Canister
```

Hand cargo may occupy:

```text
gear slots
Field Deck cartridge slots
tool belt clips
back rig mounts
two-handed carry state
drag state
sled/cart state
```

Design rule:

```text
High-value small cargo should feel precious.
Heavy cargo should change the body.
```

---

## Scale 2: Container Cargo

Once cargo enters a container, it stops being individually simulated as physics bodies.

It becomes a manifest.

Containers include:

```text
storage locker
field crate
rover bed
vault hopper
settlement depot
conveyor node
cargo pod
fabricator intake
sealed quarantine bin
```

The container is physical.

The contents are represented as data until spilled, opened, dumped, or breached.

Design rule:

```text
A closed container is one physical object plus one manifest.
```

---

## Scale 3: Settlement Ledger Cargo

At settlement scale, cargo becomes civic infrastructure.

The question is no longer only:

```text
Where is the copper?
```

It becomes:

```text
Who may read the manifest?
Who may move the copper?
Who may spend it?
Who owns it?
Who needs it to survive?
Who is hiding it?
Who falsified the ledger?
```

Design rule:

```text
A material ledger is a political map of survival.
```

---

# 2. Hand Cargo Mechanics

## Physical Rig Slots

Small high-value objects should be physically handled.

Examples:

```text
slide cartridge into Field Deck
lock firmware slug into tool rig
clip witness token to chest strap
store sample vial in sealed pouch
```

These objects should be visible on the avatar when possible.

## Two-Handed Tax

Heavy items require two hands.

While carrying a heavy object, the player may be restricted:

```text
cannot sprint
cannot raise rifle
cannot climb freely
cannot use Field Deck fully
cannot weld
cannot patch cable quickly
reduced turn speed
louder movement
higher fall risk
```

Example item:

```text
Copper Conduit Pipe Segment
Mass: 38 kg
Carry Type: Two-Handed
Sprint: Disabled
Weapon Ready: Disabled
Panic Drop: Enabled
```

Design rule:

```text
Heavy cargo should create route planning, not just inventory pressure.
```

---

# 3. Panic Drop Cargo Behavior

When threatened while carrying heavy cargo, the player may trigger Panic Drop.

Panic Drop:

```text
drops cargo physically
frees hands
restores weapon/tool readiness
may damage fragile cargo
may block path
may create noise
may alter cover geometry
may interrupt repair timing
```

Example:

```text
PANIC DROP:
Copper Conduit Pipe Segment released.
Impact noise detected.
Cargo condition: dented / usable.
```

Cargo after Panic Drop remains in-world as a physical object.

It can:

```text
block a doorway
roll down stairs
fall into water
act as partial cover
damage fragile floor panels
be stolen or moved by enemies
```

Design rule:

```text
Dropping cargo should save the body but create a new material problem.
```

---

# 4. Container Manifests

When cargo is contained, the Device Bus exposes it as a manifest.

Example path:

```text
/dev/sym/logistics/vault_3
```

Recommended command pattern:

```sh
$ ls /dev/sym/logistics/
vault_0
vault_1
flooded_crate_2
rover_bed_alpha

$ read /dev/sym/logistics/vault_3
```

Recommended output:

```text
NODE: /dev/sym/logistics/vault_3
STATUS: CONTAINED
SEAL: CLOSED
TOTAL_MASS: 1,420 kg
VOLUME_USED: 72%
MANIFEST_INTEGRITY: SECURE
CIVIC_VISIBILITY: PUBLIC_READ / RESTRICTED_WRITE
AUTHORITY: WATERSHED_COMMONS_DEPOT
ALERTS: 1 UNVERIFIED SIGNATURE

CONTENTS:
- valve_heavy_copper            qty: 1    mass: 45 kg    condition: clean
- steel_plate_structural        qty: 30   mass: 900 kg   condition: worn
- ceramic_seal_crate            qty: 2    mass: 32 kg    condition: fragile
- null_tainted_core             qty: 1    mass: 4 kg     ALERT: SIGNATURE_UNVERIFIED
```

Design rule:

```text
Use shell commands for navigation.
Use instrumentation blocks for comprehension.
```

---

# 5. Manifest Integrity

Cargo manifests can become unreliable.

Manifest states:

```text
SECURE
UNVERIFIED
DIVERGENT
PARTIAL
FORGED
NULL_SUSPECT
QUARANTINED
SEALED
```

## Manifest Divergence

A divergence occurs when the digital manifest does not match physical reality.

Causes:

```text
container breach
manual theft
Null logistics parasite
corrupted sorter
unwitnessed transfer
counterfeit item tag
damaged RFID / identity marker
emergency dump
```

Field Deck reading:

```text
DIAG:
Manifest mass mismatch.
Expected: 1,420 kg.
Measured: 1,376 kg.

NULL:
One transfer event lacks witness signature.
```

Design rule:

```text
A false manifest is archive warfare at material scale.
```

---

# 6. Performance Firewall

Physicalized cargo must not destroy multiplayer performance.

## Internal State Rule

While cargo is inside a sealed container, closed vehicle bed, enclosed pipeline, or automated conveyor loop:

```text
do not replicate individual item bodies
replicate container transform
replicate manifest summary
replicate critical alerts
```

The network does not need every steel plate as an entity.

It needs:

```text
container id
total mass
volume used
contents hash
authority state
hazard flags
manifest integrity
```

## Spillage Trigger

Individual physics entities instantiate only when containment breaks.

Spillage triggers:

```text
container opened manually
vault hopper dumped
rover crash
explosion breach
pipe rupture
conveyor cut open
enemy sabotage
flood washout
manual cargo drop
```

When triggered:

```text
manifest decomposes into item entities
nearby clients replicate physical bodies
items inherit mass, condition, velocity, hazard flags
container manifest updates to spilled state
```

Design rule:

```text
Cargo becomes pixels only when the world can touch it.
```

---

# 7. Cargo as Faction Sovereignty

Different factions expose, hide, or weaponize cargo ledgers differently.

## Mutualist Assembly

Cargo doctrine:

```text
public manifests
transparent depots
commons auditing
shared repair priority
```

Risk:

```text
openness may expose supply weakness
hostile factions can see scarcity
```

## Utility Sovereigns

Cargo doctrine:

```text
encrypted manifests
licensed hoppers
subscription access
proprietary part locks
```

Risk:

```text
settlement can be locked out of its own tools
material debt becomes survival leverage
```

## Security Protectorates

Cargo doctrine:

```text
contraband scanners
sealed drop-chutes
movement permits
automated interdiction
```

Risk:

```text
security can become capture
false contraband tags can starve a district
```

## Watershed Commons

Cargo doctrine:

```text
water repair parts prioritized
biofilter transparency
public access ledgers
ecological routing constraints
```

Risk:

```text
may delay industrial repair to protect commons flow
```

Design rule:

```text
Every faction’s inventory system reveals its politics.
```

---

# 8. Cargo Hazards

Cargo can be dangerous.

Hazard tags:

```text
HEAVY
FRAGILE
TOXIC
RADIOACTIVE
BIOACTIVE
NULL_TAINTED
FIRMWARE_LOCKED
COLD_CHAIN_REQUIRED
PRESSURIZED
EVIDENCE_GRADE
```

Examples:

```text
Archive Witness Cartridge:
EVIDENCE_GRADE / FRAGILE / HIGH_VALUE

Null-Tainted Core:
NULL_TAINTED / QUARANTINE_REQUIRED / SIGNATURE_UNVERIFIED

Biofilter Canister:
BIOACTIVE / COLD_CHAIN_REQUIRED / ECOLOGICAL_RELEASE_RISK

Valve Casing:
HEAVY / TWO_HANDED / STRUCTURAL
```

Design rule:

```text
Cargo tags should affect movement, law, storage, and risk.
```

---

# 9. Seedworks v0.1 Scope

Seedworks should implement only two physicalized cargo items and one simple container manifest.

## Physical Item 1: Archive Witness Cartridge

Role:

```text
small high-value evidence object
must be physically carried
slides into terminal or Field Deck slot
authorizes or verifies a waterworks override
```

Mechanics:

```text
one-hand carry
gear slot compatible
can be dropped
can be inserted
can be read by Field Deck
```

Field Deck:

```text
SCAN:
Archive Witness Cartridge detected.

ARCHIVE:
Witness fragment intact.

CIVIC:
Can support public override review.
```

## Physical Item 2: Copper Conduit Pipe Segment

Role:

```text
heavy repair component
needed to patch broken waterworks junction
```

Mechanics:

```text
two-handed carry
blocks sprint
blocks weapon ready
Panic Drop enabled
can be welded into place
affects repair quality
```

Field Deck:

```text
SCAN:
Copper conduit segment compatible.

DIAG:
Mass requires two-handed carry.
Surface oxidation: moderate.
```

## Simple Container: Flooded Storage Crate

Path:

```text
/dev/sym/logistics/flooded_crate_0
```

Contents:

```text
1 copper_conduit_pipe_segment
1 ceramic_seal
1 damaged_firmware_tab
```

Field Deck:

```text
NODE: /dev/sym/logistics/flooded_crate_0
STATUS: PARTIAL / WATER-DAMAGED
TOTAL_MASS: 54 kg
MANIFEST_INTEGRITY: PARTIAL

CONTENTS:
- copper_conduit_pipe_segment    qty: 1    mass: 38 kg    condition: oxidized
- ceramic_seal                   qty: 1    mass: 4 kg     condition: fragile
- firmware_tab                   qty: 1    mass: 0.2 kg   condition: water-damaged
```

Design rule:

```text
Seedworks cargo should teach weight, evidence, and manifest trust in one room.
```

---

# 10. Cargo Loop in First 30 Minutes

Recommended sequence:

```text
1. Player scans broken pipe.
2. DIAG says Copper Conduit Pipe Segment required.
3. Player locates flooded storage crate.
4. Player reads crate manifest.
5. Manifest is partial due to water damage.
6. Player physically retrieves conduit segment.
7. Two-handed carry slows return route.
8. Null drone or hazard triggers optional Panic Drop.
9. Player installs conduit segment.
10. Player inserts Archive Witness Cartridge.
11. Device Bus registers repair.
12. Chronicle records cargo-dependent restoration.
```

Chronicle line:

```text
The player learned that restoring water began with carrying the weight of the part.
```

---

# 11. Cargo and Chronicle

Chronicle-worthy cargo events:

```text
evidence cartridge recovered
heavy repair component delivered under threat
manifest divergence exposed
Null-tainted cargo quarantined
private vault opened for public survival
cargo lost during Panic Drop
settlement saved by salvaged material
```

Example:

```json
{
  "event_type": "CriticalCargoRecovered",
  "site": "Old Waterworks",
  "item": "Archive Witness Cartridge",
  "condition": "intact",
  "used_for": "public_water_override",
  "chronicle_line": "The override did not begin with a speech. It began with a cartridge carried through floodwater."
}
```

---

# 12. Design Boundaries

## Allowed in v0.1

```text
physical cartridge
two-handed heavy component
single storage crate manifest
Panic Drop cargo behavior
basic cargo condition
simple manifest readout
cargo as repair requirement
```

## Deferred

```text
full conveyor networks
automated sorters
vehicle cargo physics
bulk ore simulation
settlement-scale depots
multi-faction logistics economy
cargo theft systems
complex container permissions
```

Design rule:

```text
Do not build the warehouse before the player respects one heavy pipe.
```

---

# 13. Final Principles

```text
Cargo is mass.

Cargo is memory.

Cargo is authority.

Cargo is risk.

A container is a promise that its manifest matches the world.

A manifest is only trusted when the world can verify it.

A heavy part should change how the player moves.

A critical cartridge should change what the settlement can prove.
```

Final line:

```text
The inventory was not a list.
It was the weight of civilization waiting to be carried.
```

# Addendum: Cargo Response Grammar, Cold Chain, and v0.1 Interaction Details

## Purpose

The Physicalized Cargo system already defines cargo as:

```text
mass
memory
authority
risk
```

This addendum makes the system more implementable by defining what the player does when cargo records are wrong, fragile, hazardous, temperature-sensitive, or politically contested.

Core rule:

```text
Detecting a cargo problem is not the mechanic.
Responding to it is the mechanic.
```

---

# 1. Manifest Response Grammar

When a player discovers a manifest problem, they should have clear response verbs.

Manifest states include:

```text
SECURE
UNVERIFIED
DIVERGENT
PARTIAL
FORGED
NULL_SUSPECT
QUARANTINED
SEALED
```

Each state should support a small number of actions.

## Response Verbs

```text
inspect
recount
flag
quarantine
seal
open
override
report
ignore
transfer
destroy
witness
adjudicate
```

## Example: DIVERGENT Manifest

Situation:

```text
The crate claims it contains one copper conduit segment.
Measured mass suggests something is missing or misreported.
```

Field Deck:

```text
DIAG:
Manifest divergence detected.
Expected mass: 54 kg.
Measured mass: 48 kg.

NULL:
No active spoofing detected.
Possible missing component or waterlogged mass error.
```

Player options:

```text
RECOUNT:
Physically open crate and inspect contents.

FLAG:
Mark container for later Archive Witness review.

IGNORE:
Proceed with current manifest, accepting repair risk.

ADJUDICATE:
Open civic claim if container belonged to a faction depot.

QUARANTINE:
Seal crate if divergence involves hazardous cargo.
```

Design rule:

```text
A manifest mismatch should create a choice, not just an alert.
```

---

# 2. Manifest Actions and Consequences

## Inspect / Recount

The player opens the container or manually verifies contents.

Result:

```text
highest certainty
takes time
may expose hazards
may spawn physical item entities
may require safe location
```

Chronicle-worthy if critical cargo is involved.

## Flag

The player marks the manifest as suspect.

Result:

```text
no immediate repair delay
future civic review possible
container trust reduced
faction responsible may react
```

## Ignore

The player proceeds.

Result:

```text
fast
risky
may reduce repair quality
may create later liability
```

Chronicle example:

```text
The player trusted the crate because the water could not wait.
```

## Quarantine

The player seals the item or container.

Result:

```text
hazard contained
repair delayed
Quarantine Authority trust rises
settlement pressure may worsen
```

## Witness

The player requests a second source of verification.

Sources:

```text
Archive Witness
teammate
settlement clerk
analog scale
Device Bus checksum
physical inspection
```

Design rule:

```text
Cargo truth should be recoverable through multiple verification paths.
```

---

# 3. Cold-Chain Cargo

Some cargo requires temperature control.

Tag:

```text
COLD_CHAIN_REQUIRED
```

Examples:

```text
biofilter canister
living microbial vial
seed vault packet
medical culture
enzyme membrane
alien tissue sample
water-memory sample
```

Cold-chain cargo has:

```text
temperature_range
time_out_of_range
viability
hazard_state
civic_sensitivity
```

Example:

```json
{
  "item_id": "biofilter_canister_mk0",
  "tags": ["BIOACTIVE", "COLD_CHAIN_REQUIRED"],
  "temperature_range": "2C–8C",
  "time_out_of_range": "00:07:14",
  "viability": 0.82,
  "hazard_state": "stable",
  "civic_sensitivity": "ecological_release_risk"
}
```

## Degradation Stages

```text
STABLE
WARMING
VIABILITY_LOSS
UNRELIABLE
BIOACTIVE_RISK
FAILED
```

## Gameplay Effects

If cold-chain cargo warms:

```text
repair effectiveness may drop
biofilter may fail
microbial strain may mutate or die
Quarantine Authority may intervene
Quiet Green may object to careless handling
Chronicle may record negligence if stakes are high
```

Field Deck:

```text
DIAG:
Biofilter canister temperature outside safe range.
Viability loss beginning.

CIVIC:
Deployment after cold-chain breach may require ecological review.
```

Design rule:

```text
Cold-chain failure should not be a timer for punishment.
It should turn logistics into care.
```

---

# 4. Archive Witness Cartridge Interaction

The Archive Witness Cartridge should be one of the first sacred physical objects the player handles.

It is not a generic keycard.

It is portable evidence.

## Physical Interaction

The cartridge can be:

```text
picked up
inspected
slotted into Field Deck
slotted into terminal witness bay
patched via cable if slot is damaged
removed
dropped
damaged
```

## Preferred v0.1 Interaction

The Old Waterworks terminal has a physical witness bay.

Interaction sequence:

```text
1. Player finds Archive Witness Cartridge.
2. Player inspects it with SCAN.
3. Player carries it to the waterworks terminal.
4. Player opens the terminal witness cover.
5. Player slides cartridge into bay.
6. Terminal reads source chain.
7. Field Deck switches to ARCHIVE or CIVIC.
8. Cartridge supports public override authorization.
```

Field Deck:

```text
SCAN:
Archive Witness Cartridge detected.
Physical seal intact.

ARCHIVE:
Witness fragment readable.
Source chain partial but valid.

CIVIC:
Cartridge may support temporary public water override.
```

If the terminal slot is damaged:

```text
DIAG:
Witness bay contact corroded.
Patch cable bridge possible.
```

Then the player can physically cable the cartridge reader to the Field Deck.

Design rule:

```text
A cartridge is not a key.
It is a witness you carry.
```

---

# 5. v0.1 Panic Drop Cargo Encounter

The first cargo hazard should be small and scripted enough to teach the mechanic.

Do not start with a full Null combat encounter.

## Recommended Encounter

The player is carrying the Copper Conduit Pipe Segment through a flooded corridor.

Hazard options:

```text
minor Null drone sweep
sudden pipe pressure burst
falling catwalk section
electrical water surge
Continuance patrol light
```

Best v0.1 option:

```text
a minor Null maintenance drone enters the corridor
```

The drone should be threatening enough to make the player consider Panic Drop, but not lethal enough to feel unfair.

## Player Choices

### Keep Carrying

Result:

```text
slower
quiet
risk being cornered
cargo remains undamaged
```

### Panic Drop

Result:

```text
hands freed
weapon/tool available
pipe hits floor loudly
cargo may dent
pipe may block drone path
repair quality slightly affected if damaged
```

### Set Down Carefully

Result:

```text
slower than Panic Drop
cargo undamaged
requires safe timing
```

Design rule:

```text
The first cargo encounter should teach vulnerability, not punish curiosity.
```

---

# 6. Cargo Condition to Repair Quality

Cargo condition should feed directly into assembly outcomes.

Example chain:

```text
oxidized copper conduit
→ harder surface prep
→ lower weld integrity if not cleaned
→ rough emergency seal
→ medium leak risk
→ inspection deadline
→ Chronicle line
```

Example data flow:

```json
{
  "item": "copper_conduit_pipe_segment",
  "condition": "oxidized_dented",
  "assembly_effects": {
    "surface_cleaning_time": "+20%",
    "alignment_penalty": 0.06,
    "weld_integrity_cap": 0.82
  },
  "device_effects": {
    "leak_risk_floor": "low",
    "maintenance_due": "soon"
  },
  "civic_effects": {
    "inspection_required": true
  }
}
```

Design rule:

```text
The condition of what you carried should be visible in what you built.
```

---

# 7. Cargo Disputes and Adjudication

Cargo can trigger Registered Infrastructure Adjudication when it affects survival systems.

Examples:

```text
manifest mismatch in public depot
private vault contains public repair part
Null-tainted core found in settlement storage
cold-chain biofilter spoiled by negligence
Archive Witness Cartridge damaged before hearing
heavy conduit stolen from Watershed Commons cache
```

Possible claimants:

```text
Watershed Commons
Utility Sovereign
Quarantine Authority
Archive Witness
Settlement Council
Continuance Office
```

Example dispute:

```text
Utility Sovereign claims the copper conduit belongs to its licensed repair kit.
Watershed Commons claims it is needed for public water restoration.
Quarantine Authority flags its flooded storage history.
```

Design rule:

```text
A contested item becomes infrastructure before it is installed.
```

---

# 8. Improved Seedworks Cargo Loop

Updated first 30-minute cargo sequence:

```text
1. Player scans broken pipe.
2. DIAG identifies missing Copper Conduit Pipe Segment.
3. Player finds flooded storage crate.
4. Player reads crate manifest.
5. Manifest is PARTIAL due to water damage.
6. Player opens crate and physically confirms conduit.
7. Player carries conduit two-handed.
8. Minor hazard creates Set Down vs Panic Drop choice.
9. Cargo condition updates if dropped or damaged.
10. Player installs conduit.
11. Repair quality reflects cargo condition.
12. Player inserts Archive Witness Cartridge into terminal witness bay.
13. Cartridge supports temporary public override.
14. Device Bus registers repaired node.
15. CIVIC mode opens adjudication.
16. Chronicle records cargo-dependent restoration.
```

Chronicle examples:

Clean cargo delivery:

```text
The player carried the missing piece through floodwater and gave the pipe a body again.
```

Panic Drop damage:

```text
The conduit arrived dented, but the water could not wait for perfect metal.
```

Manifest discrepancy exposed:

```text
The player found that the crate had lied before the pipe ever leaked.
```

---

# 9. Final Additions to Principles

Add these to the cargo design principles:

```text
A manifest mismatch is a playable dispute.

Cold-chain logistics are care under time pressure.

A cartridge is a witness you carry.

Cargo condition must propagate into repair quality.

The first cargo hazard should teach vulnerability, not punishment.

A contested item becomes infrastructure before installation.
```

Final line:

```text
The settlement did not only need the part.
It needed someone to carry it honestly.
```
