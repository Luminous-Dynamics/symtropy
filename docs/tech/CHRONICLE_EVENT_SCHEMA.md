# CHRONICLE_EVENT_SCHEMA.md

# Symtropy Chronicle Event Schema

## Version 0.1 — Public History as an Append-Only Repair Ledger

## Purpose

This document defines the local Chronicle event format for **Symtropy: Seedworks**.

The Chronicle records consequential player actions, repair outcomes, faction memories, legitimacy changes, and historical precedents.

It is not only a save file.

It is the game’s public memory.

```text id="uhmyxn"
The player does not merely complete quests.
The player writes history that future systems can cite.
```

## Core Thesis

The Chronicle is where action becomes precedent.

A repair path should not disappear after the pump starts working.

It should become a record that factions, NPCs, machines, settlements, and future worldline systems can remember.

```text id="sd4pp4"
The pump doesn't care who fixes it.
The settlement does.
```

## Design Principle

```text id="77nov8"
If history is public, the log must be readable.
```

The first Chronicle backend must be human-readable, auditable, deterministic, and append-only.

For v0.1, use canonical JSONL.

Binary formats may come later as derived indexes, snapshots, caches, or network payloads.

---

# Format Decision

## v0 Source of Truth

```text id="as2ppj"
Format: canonical JSONL
Storage: local append-only file
One line: one Chronicle event
Integrity: hash chain
Authenticity: signature field, placeholder allowed in v0
Replay: deterministic event order
```

## File Layout

```text id="c5e9to"
chronicle/
  manifest.json
  events.jsonl
  snapshots/
  indexes/
  signatures/
```

## Why JSONL First?

JSONL is chosen because it is:

```text id="jj595d"
human-readable
easy to diff
easy to test
easy to replay
easy for modders and agents to inspect
compatible with later hash-chaining
compatible with later signatures
compatible with later binary indexing
```

Do not use a database first.

Do not use byte-packed binary first.

Do not hide public history behind an opaque format.

## Future Binary Layer

Binary may be added later for:

```text id="ow6emu"
fast replay
network replication
Merkle segment shipping
compressed snapshots
large worldline archives
mobile/offline storage
```

But binary should be derived from the canonical event stream.

```text id="fzuiqx"
Canonical source: events.jsonl
Derived acceleration: binary indexes/snapshots
```

---

# Canonicalization Rules

To hash and sign events consistently, Chronicle events must be serialized deterministically.

## Required Rules

```text id="cagf14"
UTF-8 encoding
sorted object keys
stable integer formatting
stable floating-point policy
no insignificant whitespace in canonical signed bytes
one canonical event per JSONL line
schema_version required
event_id required
prev_hash required
hash required
signature required
```

## Hash Rule

The `hash` field is computed over the canonical JSON event **without**:

```text id="bbah49"
hash
signature
```

The resulting hash is then inserted into the event envelope.

## Signature Rule

The `signature` signs either:

```text id="z55j4j"
the event hash
```

or:

```text id="m8sxcn"
the canonical event bytes without hash and signature
```

For v0.1, a placeholder signature is allowed:

```text id="qyvzl2"
"signature": "placeholder"
```

But the schema must preserve the field from day one.

## Genesis Rule

The first event uses:

```text id="7y1r56"
"prev_hash": "GENESIS"
```

All subsequent events reference the previous event’s `hash`.

---

# Event Envelope

## Rust Shape

```rust id="gasyki"
struct ChronicleEventEnvelope {
    schema_version: String,
    event_id: String,
    worldline_id: String,
    site_id: Option<String>,
    actor_id: String,
    logical_time: u64,
    event_type: String,
    prev_hash: String,
    payload: serde_json::Value,
    hash: String,
    signature: String,
}
```

## Field Definitions

### schema_version

Identifies the event schema.

Example:

```text id="m8sv17"
chronicle.event.v0
```

### event_id

Stable event identifier.

Example:

```text id="obslmb"
evt_00000001
```

For v0.1, sequential IDs are acceptable.

Later versions may use ULIDs, UUIDs, content hashes, or deterministic worldline IDs.

### worldline_id

The worldline context.

Example:

```text id="tshmb3"
seed_age_firstlight
```

### site_id

Optional location or system site.

Example:

```text id="wv1fj4"
old_waterworks
```

### actor_id

Who caused or committed the event.

Examples:

```text id="vvst5e"
player_local
npc_ivo_technician
machine_pump_1
faction_continuance_local
```

### logical_time

Monotonic event counter.

This is not wall-clock time.

Example:

```text id="ttgg7f"
1
2
3
4
```

### event_type

The type of event.

Example:

```text id="izqk28"
DeadAuthorityLockInspected
```

### prev_hash

Hash of the previous event.

Example:

```text id="nr4s9y"
GENESIS
```

or:

```text id="a7wgpc"
sha256:8f...
```

### payload

Event-specific data.

This must remain readable and schema-aware.

### hash

Hash of this event’s canonical pre-hash form.

### signature

Signature over this event or its hash.

Placeholder allowed in v0.

---

# Required v0 Event Types

The first playable slice should support at least these events:

```text id="ldk2dn"
FieldDeckRaised
FieldDeckModeChanged
DeadAuthorityLockInspected
RepairPathPreviewed
RepairPathCommitted
WaterworksOutcomeRecorded
ChroniclePrecedentCreated
```

Minimum required for first implementation:

```text id="msqwr1"
FieldDeckRaised
DeadAuthorityLockInspected
RepairPathPreviewed
WaterworksOutcomeRecorded
```

---

# Event Type: FieldDeckRaised

## Purpose

Records the first meaningful use of the Field Deck.

Do not record every raise/lower action forever unless needed for debugging.

## Payload

```rust id="ypih3f"
struct FieldDeckRaisedPayload {
    deck_id: String,
    origin_profile: Option<String>,
    visor_assist_enabled: bool,
    location_hint: Option<String>,
}
```

## Example

```json id="pye4y8"
{"schema_version":"chronicle.event.v0","event_id":"evt_00000001","worldline_id":"seed_age_firstlight","site_id":"firstlight_basin","actor_id":"player_local","logical_time":1,"event_type":"FieldDeckRaised","prev_hash":"GENESIS","payload":{"deck_id":"field_deck_mk0","origin_profile":null,"visor_assist_enabled":false,"location_hint":"water_queue"},"hash":"...","signature":"placeholder"}
```

---

# Event Type: FieldDeckModeChanged

## Purpose

Optional diagnostic event.

Useful for debugging soft-reveal origin and mode usage.

Do not necessarily keep in final public Chronicle.

## Payload

```rust id="y0wxu9"
struct FieldDeckModeChangedPayload {
    from_mode: String,
    to_mode: String,
    target_id: Option<String>,
}
```

## Example

```json id="82wpv4"
{"schema_version":"chronicle.event.v0","event_id":"evt_00000002","worldline_id":"seed_age_firstlight","site_id":"old_waterworks","actor_id":"player_local","logical_time":2,"event_type":"FieldDeckModeChanged","prev_hash":"...","payload":{"from_mode":"DIAG","to_mode":"ARCHIVE","target_id":"pump_1"},"hash":"...","signature":"placeholder"}
```

---

# Event Type: DeadAuthorityLockInspected

## Purpose

Records inspection of a dead authority lock.

This is one of the first major Chronicle-worthy events.

## Payload

```rust id="y66tn4"
struct DeadAuthorityLockInspectedPayload {
    target_id: String,
    authority_state: String,
    tank_level_percent: u8,
    archive_trace: Vec<String>,
    null_loop_detected: bool,
    null_loop_duration: Option<String>,
}
```

## Example

```json id="slj5wj"
{"schema_version":"chronicle.event.v0","event_id":"evt_00000003","worldline_id":"seed_age_firstlight","site_id":"old_waterworks","actor_id":"player_local","logical_time":3,"event_type":"DeadAuthorityLockInspected","prev_hash":"...","payload":{"target_id":"pump_1","authority_state":"DEAD_AUTHORITY_LOCK","tank_level_percent":12,"archive_trace":["Built 2048: Municipal drought adaptation works.","Modified 2087: Emergency Water Act automation.","Authority chain failed approximately 2113."],"null_loop_detected":true,"null_loop_duration":"55 years, 3 months, 12 days"},"hash":"...","signature":"placeholder"}
```

---

# Event Type: RepairPathPreviewed

## Purpose

Records that the player previewed a consequential repair path.

This can support future faction/NPC interpretation:

```text id="iiw4vx"
The player considered illegal bypass before choosing witness.
The player studied machine testimony but ignored it.
The player saw the warning and chose stabilization anyway.
```

## Payload

```rust id="x11ikn"
struct RepairPathPreviewedPayload {
    target_id: String,
    repair_path: String,
    visible_warnings: Vec<String>,
    predicted_outcome_class: String,
}
```

## Example

```json id="nsqvrh"
{"schema_version":"chronicle.event.v0","event_id":"evt_00000004","worldline_id":"seed_age_firstlight","site_id":"old_waterworks","actor_id":"player_local","logical_time":4,"event_type":"RepairPathPreviewed","prev_hash":"...","payload":{"target_id":"pump_1","repair_path":"TemporaryEmergencyStabilization","visible_warnings":["Temporary stabilization maintains emergency authority structure.","Null reinforcement loop will continue during stabilization period."],"predicted_outcome_class":"DeferredCrisis"},"hash":"...","signature":"placeholder"}
```

---

# Event Type: RepairPathCommitted

## Purpose

Records that the player committed to a repair path.

This is a WITNESS-worthy event.

## Payload

```rust id="w7fb14"
struct RepairPathCommittedPayload {
    target_id: String,
    repair_path: String,
    witness_mode_used: bool,
    origin_profile: Option<String>,
    charter_context: Option<String>,
}
```

## Example

```json id="slmtyb"
{"schema_version":"chronicle.event.v0","event_id":"evt_00000005","worldline_id":"seed_age_firstlight","site_id":"old_waterworks","actor_id":"player_local","logical_time":5,"event_type":"RepairPathCommitted","prev_hash":"...","payload":{"target_id":"pump_1","repair_path":"ArchiveWitnessOverride","witness_mode_used":true,"origin_profile":"ArchiveApprentice","charter_context":"FirstlightPublicRepairCharter"},"hash":"...","signature":"placeholder"}
```

---

# Event Type: WaterworksOutcomeRecorded

## Purpose

Records the outcome of the Old Waterworks encounter.

This is the main slice-ending event.

## Payload

```rust id="kat5ta"
struct WaterworksOutcomeRecordedPayload {
    outcome_class: String,
    repair_path: String,
    water_flow_state: String,
    authority_state_after: String,
    legitimacy_effect: String,
    null_drift_effect: String,
    faction_memory_flags: Vec<String>,
    chronicle_text: String,
}
```

## Outcome Classes

```text id="otgq05"
FullRepair
PartialRepair
ContestedRepair
IllegalRepair
EmergencyStabilization
DestructiveVictory
NegotiatedTruce
DeferredCrisis
NullExpansion
```

## Example — Archive Witness Success

```json id="sv5z05"
{"schema_version":"chronicle.event.v0","event_id":"evt_00000006","worldline_id":"seed_age_firstlight","site_id":"old_waterworks","actor_id":"player_local","logical_time":6,"event_type":"WaterworksOutcomeRecorded","prev_hash":"...","payload":{"outcome_class":"FullRepair","repair_path":"ArchiveWitnessOverride","water_flow_state":"RESTORED","authority_state_after":"PUBLIC_OVERRIDE_RESTORED_UNDER_WITNESS","legitimacy_effect":"LEGITIMACY_INCREASED","null_drift_effect":"NULL_DRIFT_REDUCED","faction_memory_flags":["old_waterworks_repaired_legitimately","archive_witness_respected","dead_authority_overturned"],"chronicle_text":"2168 — The Old Waterworks were restored under Archive Witness after the dead authority chain was overturned. Water returned with public legitimacy."},"hash":"...","signature":"placeholder"}
```

## Example — Manual Illegal Bypass

```json id="t0yyvw"
{"schema_version":"chronicle.event.v0","event_id":"evt_00000006","worldline_id":"seed_age_firstlight","site_id":"old_waterworks","actor_id":"player_local","logical_time":6,"event_type":"WaterworksOutcomeRecorded","prev_hash":"...","payload":{"outcome_class":"IllegalRepair","repair_path":"ManualIllegalBypass","water_flow_state":"RESTORED_FAST","authority_state_after":"UNRESOLVED","legitimacy_effect":"LEGITIMACY_DEBT_INCREASED","null_drift_effect":"NULL_DRIFT_UNRESOLVED","faction_memory_flags":["old_waterworks_bypassed_illegally","archive_witness_bypassed","open_valve_precedent_created"],"chronicle_text":"2168 — The Old Waterworks were restored through unwitnessed manual bypass. Water returned quickly, but the settlement inherited a new argument."},"hash":"...","signature":"placeholder"}
```

## Example — Temporary Emergency Stabilization

```json id="zysmsd"
{"schema_version":"chronicle.event.v0","event_id":"evt_00000006","worldline_id":"seed_age_firstlight","site_id":"old_waterworks","actor_id":"player_local","logical_time":6,"event_type":"WaterworksOutcomeRecorded","prev_hash":"...","payload":{"outcome_class":"EmergencyStabilization","repair_path":"TemporaryEmergencyStabilization","water_flow_state":"PARTIAL_FLOW","authority_state_after":"EMERGENCY_AUTHORITY_MAINTAINED","legitimacy_effect":"LEGITIMACY_UNRESOLVED","null_drift_effect":"NULL_REINFORCEMENT_CONTINUES","faction_memory_flags":["old_waterworks_stabilized_temporarily","dead_authority_remained_in_command","continuance_precedent_created"],"chronicle_text":"2168 — The Old Waterworks resumed partial flow under temporary emergency stabilization. The settlement drank, but the dead authority remained in command."},"hash":"...","signature":"placeholder"}
```

---

# Event Type: ChroniclePrecedentCreated

## Purpose

Records that an event can be cited later by factions, NPCs, machines, charters, or worldline systems.

## Payload

```rust id="eyb4g0"
struct ChroniclePrecedentCreatedPayload {
    precedent_id: String,
    source_event_id: String,
    summary: String,
    cited_by: Vec<String>,
    future_argument_hooks: Vec<String>,
}
```

## Example

```json id="p8hvwf"
{"schema_version":"chronicle.event.v0","event_id":"evt_00000007","worldline_id":"seed_age_firstlight","site_id":"old_waterworks","actor_id":"chronicle_system","logical_time":7,"event_type":"ChroniclePrecedentCreated","prev_hash":"...","payload":{"precedent_id":"old_waterworks_illegal_bypass_precedent","source_event_id":"evt_00000006","summary":"The player restored public water through unwitnessed manual bypass.","cited_by":["OpenValveAbsolutists","ArchiveWitnessOrder","ContinuanceReformers"],"future_argument_hooks":["You broke the seal for water. Why not break the gate for food?","You returned water, but taught the settlement that witness is optional.","If law may be bypassed in crisis, command authority may also expand in crisis."]},"hash":"...","signature":"placeholder"}
```

---

# Faction Memory Flags

Faction memory flags are small identifiers derived from events.

They should be readable and stable.

## Old Waterworks v0 Flags

```text id="ztrxg9"
old_waterworks_repaired_legitimately
old_waterworks_bypassed_illegally
old_waterworks_machine_testimony_used
old_waterworks_stabilized_temporarily
old_waterworks_logs_destroyed
dead_authority_overturned
dead_authority_remained_in_command
archive_witness_respected
archive_witness_bypassed
open_valve_precedent_created
continuance_precedent_created
machine_memory_preserved
machine_memory_destroyed
null_loop_isolated
null_reinforcement_continues
```

## Design Rule

Faction memory flags should not replace Chronicle text.

They are machine-readable handles for future systems.

The Chronicle text remains the human-readable public memory.

---

# Outcome Text Register

Chronicle writing should be:

```text id="w7mftm"
civic
historical
understated
specific
consequence-aware
```

Good:

```text id="s45gcx"
Water returned quickly, but the settlement inherited a new argument.
```

Good:

```text id="rnnho3"
The settlement drank, but the dead authority remained in command.
```

Good:

```text id="xkg70g"
The Old Waterworks spoke through its diagnostic memory.
```

Bad:

```text id="j7uzgj"
Quest complete! Pump fixed!
```

Bad:

```text id="zk6dff"
You made the good moral choice and gained +10 reputation.
```

Metrics may exist behind the scenes.

The Chronicle should read like public history.

---

# Replay Rules

Chronicle replay should be deterministic.

Given the same event stream, the same derived state should be produced.

## Derived State Examples

```text id="4urryr"
current waterworks state
authority status
faction memory flags
legitimacy debt
Null drift level
available precedents
unlocked charter amendments
```

## Replay Order

Events replay in `logical_time` order.

The hash chain verifies that no event was removed, inserted, or reordered.

## Replay Failure

If replay detects broken hash continuity:

```text id="plwefg"
mark Chronicle as corrupted
stop applying derived consequences after the broken event
surface warning in dev builds
preserve corrupted file for inspection
do not silently repair
```

---

# Manifest

## chronicle/manifest.json

The manifest identifies the Chronicle stream.

Example:

```json id="rj009c"
{
  "schema_version": "chronicle.manifest.v0",
  "worldline_id": "seed_age_firstlight",
  "created_by": "symtropy_seedworks",
  "event_log": "events.jsonl",
  "hash_algorithm": "sha256",
  "signature_algorithm": "placeholder",
  "canonicalization": "sorted_keys_utf8_no_insignificant_whitespace_v0"
}
```

---

# Error Handling

## Invalid Event JSON

Action:

```text id="n39kcc"
stop replay at invalid line
emit developer warning
do not delete the file
```

## Hash Mismatch

Action:

```text id="afedts"
stop trusted replay at mismatch
mark later events as untrusted
preserve file
```

## Unknown Event Type

Action:

```text id="6slwdz"
skip if marked forward-compatible
otherwise stop replay in strict mode
```

## Missing Signature

Action:

```text id="bfocqc"
allowed only in dev mode if schema permits placeholder
rejected in strict signed mode later
```

## Duplicate Event ID

Action:

```text id="ja3sp2"
reject second event during replay
surface warning
```

---

# Privacy and Save Boundaries

The Chronicle should record game-world history, not sensitive real-world player data.

Do not put into Chronicle:

```text id="0iaqqf"
real user name
real device username
real file paths
real email address
real OS account data
external account IDs
private system information
```

Use in-world actor IDs:

```text id="qj05qf"
player_local
field_deck_mk0
pump_1
firstlight_basin
```

---

# Implementation Milestones

## Milestone 1 — Local Event Writer

Create a local append-only JSONL writer.

Acceptance:

```text id="urtdlz"
creates chronicle/events.jsonl
writes one event per line
uses schema_version
uses logical_time
uses prev_hash
```

## Milestone 2 — Hash Chain

Compute deterministic hash of canonical event pre-hash form.

Acceptance:

```text id="yv9q9f"
first event prev_hash is GENESIS
second event prev_hash equals first event hash
test verifies continuity
```

## Milestone 3 — Old Waterworks Events

Write events:

```text id="yro1pt"
FieldDeckRaised
DeadAuthorityLockInspected
RepairPathPreviewed
WaterworksOutcomeRecorded
```

## Milestone 4 — Outcome Text

Write human-readable Chronicle text into outcome payload.

Acceptance:

```text id="gd7n37"
events.jsonl can be opened and understood by a human
outcome text matches selected repair path
```

## Milestone 5 — Replay Test

Add replay test that derives:

```text id="yjn5ad"
waterworks outcome class
authority state
faction memory flags
```

## Milestone 6 — Precedent Event

Create `ChroniclePrecedentCreated` after consequential outcomes.

Acceptance:

```text id="kq4rjv"
illegal bypass creates bypass precedent
temporary stabilization creates continuance precedent
archive witness creates legitimate repair precedent
```

---

# Out of Scope for v0

Do not implement yet:

```text id="t2uw81"
real cryptographic identity
network signing
Holochain/Mycelix integration
binary event storage
database backend
Merkle segment sync
multi-agent conflict resolution
cloud save
full Chronicle UI
```

Use placeholder signatures until the local event shape is stable.

---

# Testing Requirements

## Unit Tests

```text id="9ue67r"
canonical event serialization is stable
event hash excludes hash/signature fields
hash chain continuity succeeds
hash chain detects tampering
duplicate event IDs rejected
unknown event handling works
```

## Golden Test Vector

Create one known event and expected hash.

Purpose:

```text id="y6c607"
prevent accidental canonicalization drift
```

## Manual Inspection Test

A developer should be able to open:

```text id="nhr8ih"
chronicle/events.jsonl
```

and understand:

```text id="bq8dp3"
what happened
where it happened
who acted
what outcome occurred
what precedent was created
```

---

# Agent Implementation Rules

```text id="k4brpj"
Do not use git add .
Do not use git commit --no-verify.
Do not stage unrelated files.
Do not edit sibling workspaces.
Do not introduce database dependency.
Do not introduce networking.
Do not introduce Holochain/Mycelix runtime dependency yet.
Use rg for search.
Keep Chronicle v0 local and deterministic.
```

Required check before and after:

```text id="t5k7nr"
cargo check -p symtropy-bevy-core --example old_waterworks_micro_slice
```

If the check fails because of unrelated workspace contamination, document the exact blocker and do not hide it.

---

# Final Principle

The Chronicle is not a save file.

It is the settlement’s memory of what the player made possible.

```text id="s33904"
Symtropy does not ask what future you prefer.
It asks what future your choices are already building.
```
# CHRONICLE_EVENT_SCHEMA.md — v0.2 Addendum

# Session Monotonicity, File-State Handling, and Schema Evolution

## Purpose

This addendum tightens the Chronicle v0 schema for production implementation.

It clarifies:

```text
logical_time behavior across sessions
first-run versus corrupted-log handling
manifest reader compatibility
unknown-event behavior
Milestone 1 implementation placement
```

These rules should be added before implementing the first local Chronicle writer.

---

# 1. logical_time Across Sessions

## Rule

```text
logical_time is globally monotonic across sessions.
```

It is derived from the count or maximum `logical_time` of committed events in the Chronicle log.

It is not derived from:

```text
wall-clock time
session count
frame count
save slot open time
system clock
```

## Required Behavior

When the player quits and reloads:

```text
logical_time continues from the last committed event.
Session restart does not reset logical_time.
```

Example:

```text
Session 1:
evt_00000001 logical_time = 1
evt_00000002 logical_time = 2
evt_00000003 logical_time = 3

Player quits.

Session 2:
next event logical_time = 4
```

## Replay Rule

During replay, the Chronicle reader must verify that `logical_time` is strictly increasing.

If a later event has a duplicate or lower `logical_time`, replay must treat the stream as corrupted or untrusted from that point forward.

## Design Principle

```text
History does not restart because the session restarted.
```

---

# 2. Chronicle File-State Handling

The Chronicle reader must distinguish between:

```text
file missing
file empty
file valid
file corrupted
```

These states require different behavior.

## File Missing

Meaning:

```text
First run, new save, or Chronicle has not been created yet.
```

Required behavior:

```text
create chronicle/ directory if missing
create manifest.json
create events.jsonl
initialize the next event with prev_hash = "GENESIS"
do not treat this as corruption
```

## File Empty

Meaning:

```text
Chronicle file exists but has no committed events.
```

Required behavior:

```text
treat as fresh Chronicle
initialize the next event with prev_hash = "GENESIS"
do not treat this as corruption
```

## File Valid

Meaning:

```text
all events parse
hash chain verifies
logical_time is strictly increasing
required fields exist
```

Required behavior:

```text
load last event hash
set next logical_time to last logical_time + 1
append future events normally
```

## File Corrupted

Corruption includes:

```text
invalid JSON
hash mismatch
duplicate event_id
non-monotonic logical_time
missing required fields
invalid schema in strict mode
truncated event line
```

Required behavior:

```text
surface warning
do not silently repair
do not overwrite events.jsonl
create a backup copy, such as events.jsonl.corrupted.<timestamp>
start fresh only after explicit player or developer action
preserve corrupted file for inspection
```

## Important Rule

A missing or empty file is not corruption.

A malformed existing file is corruption.

## Design Principle

```text
First history may be empty.
Broken history must not be silently rewritten.
```

---

# 3. GENESIS Handling

There is no separate `Genesis` event required in v0.

The first committed event in `events.jsonl` should use:

```json
"prev_hash": "GENESIS"
```

Example:

```json
{"schema_version":"chronicle.event.v0","event_id":"evt_00000001","worldline_id":"seed_age_firstlight","site_id":"firstlight_basin","actor_id":"player_local","logical_time":1,"event_type":"FieldDeckRaised","prev_hash":"GENESIS","payload":{"deck_id":"field_deck_mk0","origin_profile":null,"visor_assist_enabled":false,"location_hint":"water_queue"},"hash":"...","signature":"placeholder"}
```

Future versions may add an explicit `ChronicleInitialized` event, but v0 does not require it.

---

# 4. Manifest Compatibility Fields

Add reader compatibility metadata to `chronicle/manifest.json`.

## Required New Fields

```json
"min_reader_version": "0.0.1",
"writer_version": "0.0.1"
```

## Updated Manifest Example

```json
{
  "schema_version": "chronicle.manifest.v0",
  "worldline_id": "seed_age_firstlight",
  "created_by": "symtropy_seedworks",
  "writer_version": "0.0.1",
  "min_reader_version": "0.0.1",
  "event_log": "events.jsonl",
  "hash_algorithm": "sha256",
  "signature_algorithm": "placeholder",
  "canonicalization": "sorted_keys_utf8_no_insignificant_whitespace_v0"
}
```

## Field Meaning

### writer_version

The Symtropy build or Chronicle writer version that created the stream.

### min_reader_version

The minimum Chronicle reader version expected to safely replay this stream.

## Reader Rule

If the current build’s Chronicle reader is older than `min_reader_version`, it must warn and avoid trusted replay.

If the current reader is newer, it may replay the stream using migration or compatibility rules.

---

# 5. Schema Evolution Rules

Each event carries:

```json
"schema_version": "chronicle.event.v0"
```

This remains correct.

However, new builds may introduce new event types.

Example future event:

```text
FactionMemoryConsolidated
```

## Unknown Event Handling

The reader must support two modes:

```text
strict mode
forward-compatible mode
```

## Strict Mode

Used for tests, verification, and signed canonical replay.

Unknown event type behavior:

```text
stop replay
surface warning
mark stream unsupported
```

## Forward-Compatible Mode

Used for normal player-facing loads when safe.

Unknown event type behavior:

```text
preserve event
verify hash chain if possible
skip derived-state application for unknown event
continue replay only if event schema declares safe_skip or the manifest allows forward-compatible replay
```

## Future Field Recommendation

Future event envelopes may add:

```json
"forward_compatible": true
```

or:

```json
"unknown_event_policy": "safe_skip"
```

Do not require this in v0.

## Design Principle

```text
Unknown history must not be destroyed.
Unknown history must not be blindly trusted.
```

---

# 6. Milestone 1 Implementation Placement

## Rule

Do not make Chronicle v0 part of the public `symtropy-bevy-core` engine API yet.

`symtropy-bevy-core` is currently a permissive, generic Bevy integration crate. Chronicle v0 is game-specific Seedworks logic.

For Milestone 1, implement Chronicle v0 as either:

```text
example-local module inside old_waterworks_micro_slice.rs
```

or:

```text
example-local sibling module used only by old_waterworks_micro_slice
```

Preferred initial target:

```text
crates/symtropy-bevy-core/examples/old_waterworks_chronicle_v0.rs
```

or, if keeping everything self-contained:

```text
mod chronicle_v0;
```

inside:

```text
crates/symtropy-bevy-core/examples/old_waterworks_micro_slice.rs
```

## Dependency Rule

If needed, add only dev-dependencies to `symtropy-bevy-core`:

```toml
[dev-dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
```

Do not add Chronicle dependencies to the public crate dependency surface yet unless the team deliberately promotes Chronicle into a reusable crate.

## Future Promotion Path

After the local writer proves useful, promote it to a dedicated game/runtime crate, not automatically to `symtropy-bevy-core`.

Possible future crate:

```text
symtropy-chronicle
```

or game-specific module:

```text
symtropy-seedworks/chronicle
```

## Design Principle

```text
Prototype public history locally.
Promote only after the shape is proven.
```

---

# 7. Updated Milestone 1 — Local Event Writer

## Mission

Implement a local append-only Chronicle JSONL writer for the Old Waterworks micro-slice.

## Required First Event

Write:

```text
FieldDeckRaised
```

with:

```json
"prev_hash": "GENESIS"
```

when the Field Deck is first raised.

## Required Behavior

```text
create chronicle/ if missing
create manifest.json if missing
create events.jsonl if missing
continue logical_time across sessions
append one canonical JSON event per line
compute hash excluding hash and signature
set signature to "placeholder"
```

## Required File-State Behavior

```text
missing file → initialize fresh Chronicle
empty file → initialize fresh Chronicle
valid file → continue from last event
corrupted file → warn, preserve, do not overwrite silently
```

## Acceptance Criteria

```text
FieldDeckRaised event is written to chronicle/events.jsonl
first event has prev_hash = "GENESIS"
second event references first event hash
logical_time continues across reloads
events.jsonl is human-readable
manifest.json includes min_reader_version
unit test verifies hash-chain continuity
unit test detects hash tampering
```

## Out of Scope

```text
real signatures
network sync
Holochain integration
binary storage
database backend
full Chronicle UI
event migration framework
```

---

# 8. Updated Testing Requirements

Add tests for:

```text
missing events.jsonl initializes fresh stream
empty events.jsonl initializes fresh stream
valid events.jsonl continues logical_time
corrupted events.jsonl is not overwritten silently
logical_time is globally monotonic across sessions
manifest includes min_reader_version
unknown event type behavior is explicit
```

## Golden Test Vector

Keep one known event with expected canonical hash.

This prevents accidental canonicalization drift.

---

# Final Principle Addendum

The Chronicle is not a save file.

It is the settlement’s public memory.

```text
History does not restart because the session restarted.
First history may be empty.
Broken history must not be silently rewritten.
```
