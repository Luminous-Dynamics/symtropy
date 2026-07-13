---

title: Substrate Systems Implementation Tickets
status: implementation
milestone: seedworks-v0.1
scope: engineering tickets
owner: engineering
depends_on:

* DEVICE_BUS_SUBSTRATE_SYSTEMS.md
* SEEDWORKS_ARCHITECTURE.md
  recommended_path: docs/seedworks/04_engine/SUBSTRATE_SYSTEMS_IMPLEMENTATION_TICKETS.md

---

# Symtropy Engineering Doc: Substrate Systems Implementation Tickets

## Purpose

This document converts the Device Bus Substrate Systems design into narrow implementation tickets for the Old Waterworks micro-slice.

The goal is not to build a full power-grid simulator, audio-analysis engine, or trade economy.

The goal is to add three small substrate signals to the first playable slice:

```text
power stress
machine sound
proof of repair
```

These should reinforce the existing Old Waterworks loop without expanding the milestone beyond v0.1.

Core rule:

```text
Add substrate truth only where it makes the first pump more meaningful.
```

---

# Ticket S0 — Hygiene Gate

## Mission

Do not begin substrate feature work unless the Old Waterworks check lane is clean or the blocker is explicitly documented.

## Required Command

```sh
cargo check -p symtropy-bevy-core --example old_waterworks_micro_slice
```

## Acceptance Criteria

```text
check passes
or unrelated blocker is documented
no unrelated workspace edits are made
no git add .
no --no-verify
```

## Design Principle

```text
Do not add substrate systems to a broken pipe.
```

---

# Ticket S1 — Device Bus Substrate Node Types

## Mission

Add minimal substrate node types for power, audio, and labor.

## Suggested Rust Types

```rust
#[derive(Clone, Debug)]
pub struct PowerNodeState {
    pub node_path: String,
    pub status: PowerStatus,
    pub voltage_percent: u8,
    pub thermal_bleed: ThermalBleed,
    pub connected_devices: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum PowerStatus {
    Stable,
    Stressed,
    Overloaded,
    Brownout,
    Offline,
}

#[derive(Clone, Debug)]
pub enum ThermalBleed {
    Low,
    Medium,
    High,
}
```

```rust
#[derive(Clone, Debug)]
pub struct AudioNodeState {
    pub node_path: String,
    pub source_node: String,
    pub status: AudioMachineStatus,
    pub frequency_hz: f32,
    pub vibration_amplitude: f32,
    pub rhythm_pattern: RhythmPattern,
}

#[derive(Clone, Debug)]
pub enum AudioMachineStatus {
    Stable,
    Stressed,
    Chatter,
    Silent,
}

#[derive(Clone, Debug)]
pub enum RhythmPattern {
    Steady,
    IrregularTriplet,
    RepeatingRelay,
}
```

```rust
#[derive(Clone, Debug)]
pub struct ProofOfRepairReceipt {
    pub receipt_id: String,
    pub site: String,
    pub node: String,
    pub work_type: String,
    pub repair_grade: String,
    pub authority_basis: String,
    pub chronicle_event_id: Option<String>,
    pub witness_status: String,
    pub transferability: String,
}
```

## Acceptance Criteria

```text
types compile
types are local to micro-slice or appropriate Seedworks module
no full Device Bus refactor required
no network integration required
```

---

# Ticket S2 — Power Node Readout Stub

## Mission

Expose one power node readout in the Old Waterworks.

## Required Node

```text
/dev/sym/power/transformer_2
```

## Required Reading

```text
NODE: /dev/sym/power/transformer_2/load
STATUS: OVERLOADED
VOLTAGE: 84%
LINE_LOSS: HIGH
THERMAL_BLEED: HIGH

CONNECTED_DEVICES:
- /dev/sym/water/pump_1            draw: 45 kW
- /dev/sym/logistics/flooded_crate_0 reader    draw: 1 kW
```

## Behavior

For v0.1, this may be hardcoded.

Optional behavior:

```text
after repair, voltage improves from 84% to 91%
or remains overloaded if player selects temporary stabilization
```

## Acceptance Criteria

```text
Field Deck DIAG can display power readout
readout uses /dev/sym/power/* path
power status can be referenced by pump restart output
```

## Out of Scope

```text
real electrical graph solver
full cable placement
full regional pressure vector
WASM runtime slowdown
```

---

# Ticket S3 — Deterministic Clock Drift Flag

## Mission

Represent voltage sag as a deterministic script-latency flag.

## Required Fields

```text
voltage_percent
script_clock_state
transaction_delay_ticks
```

## Example Output

```text
SCRIPT_CLOCK: DEGRADED
TRANSACTION_LATENCY: +3 TICKS
```

## Suggested Enum

```rust
#[derive(Clone, Debug)]
pub enum ScriptClockState {
    Normal,
    Warning,
    Degraded,
    Brownout,
    Offline,
}
```

## Acceptance Criteria

```text
voltage band maps deterministically to ScriptClockState
no real-time nondeterministic slowdown
pump initialization can display delay note
```

## Out of Scope

```text
actual WASM fuel accounting
real multi-tick transaction queue
network replication
```

---

# Ticket S4 — Pump Audio State Stub

## Mission

Make the Old Waterworks pump expose one audio diagnostic state.

## Required Node

```text
/dev/sym/audio/pump_1
```

## Required States

```text
stable_hum
stressed_knock
relay_chatter
```

## Field Deck Outputs

### Stable

```text
SCAN:
Pump vibration stable.
Primary resonance within expected range.
```

### Stressed

```text
SCAN:
Pump vibration irregular.
Primary resonance below expected range.

DIAG:
Valve drag or bearing wear likely.
```

### Relay Chatter

```text
NULL:
Relay cadence repeats without adaptive response.
Possible command chatter loop.
```

## Acceptance Criteria

```text
one pump audio state variable exists
Field Deck can display different text based on state
no full audio engine required
```

## Optional

```text
play different looping audio clips per state
origin-specific note for Basin-Born Technician
```

---

# Ticket S5 — Origin-Specific Audio Note

## Mission

Add one origin-specific audio note to reinforce player background.

## Required Origin

```text
Basin-Born Technician
```

## Required Text

```text
ORIGIN NOTE:
The pump is knocking too low.
That valve is dragging against load.
```

## Acceptance Criteria

```text
origin note appears only when player origin is Basin-Born Technician
note appears during SCAN or DIAG of pump audio state
```

## Out of Scope

```text
all origins
audio skill tree
expert-listening progression
```

---

# Ticket S6 — Proof-of-Repair Receipt v0

## Mission

Generate one Proof-of-Repair receipt after a successful Old Waterworks outcome.

## Required Receipt Fields

```text
receipt_type
receipt_id
site
node
work_type
repair_grade
authority_basis
chronicle_event
witness_status
transferability
```

## Example

```json
{
  "receipt_type": "ProofOfRepair",
  "receipt_id": "por_old_waterworks_v0_0001",
  "site": "Old Waterworks",
  "node": "/dev/sym/water/patch_conduit_alpha",
  "work_type": "temporary_public_water_restoration",
  "repair_grade": "RoughEmergencySeal",
  "authority_basis": "ArchiveWitnessCartridge",
  "chronicle_event": "evt_00000004",
  "witness_status": "partial_valid",
  "transferability": "non_transferable_reputation"
}
```

## Acceptance Criteria

```text
receipt is created after outcome commit
receipt is displayed to player
receipt can be serialized
receipt references Chronicle event ID if available
```

## Out of Scope

```text
merchant redemption
cross-settlement trade
receipt disputes
cryptographic signing beyond placeholder
```

---

# Ticket S7 — Chronicle Event: ProofOfRepairIssued

## Mission

Add a Chronicle event type for Proof-of-Repair issuance.

## Required Event Type

```text
ProofOfRepairIssued
```

## Example Chronicle Payload

```json
{
  "receipt_id": "por_old_waterworks_v0_0001",
  "site": "Old Waterworks",
  "node": "/dev/sym/water/patch_conduit_alpha",
  "repair_grade": "RoughEmergencySeal",
  "authority_basis": "ArchiveWitnessCartridge",
  "witness_status": "partial_valid"
}
```

## Chronicle Line

```text
The settlement did not only receive water. It received proof of who carried the repair.
```

## Acceptance Criteria

```text
event appends to JSONL Chronicle
hash chain remains valid
receipt can reference event_id
```

---

# Ticket S8 — Integrated Field Deck Substrate Page

## Mission

Add a single optional Field Deck page that summarizes substrate conditions.

## Example Display

```text
SUBSTRATE SUMMARY

POWER:
Transformer overloaded.
Voltage: 84%.
Pump restart may delay.

AUDIO:
Pump vibration irregular.
Valve drag likely.

LABOR:
No Proof-of-Repair issued yet.
```

After repair:

```text
SUBSTRATE SUMMARY

POWER:
Transformer stressed but stable.
Voltage: 91%.

AUDIO:
Pump vibration improved.

LABOR:
Proof-of-Repair committed.
Receipt: por_old_waterworks_v0_0001
```

## Acceptance Criteria

```text
summary page exists
uses existing substrate state values
does not require new UI framework
```

## Out of Scope

```text
full dashboard
graph visualization
spectral audio UI
market UI
```

---

# Ticket S9 — Substrate Test Fixtures

## Mission

Add simple tests or fixtures for substrate state mapping.

## Required Tests

```text
voltage 100 maps to Normal
voltage 84 maps to Degraded
power node serializes
audio node serializes
ProofOfRepairReceipt serializes
```

## Acceptance Criteria

```text
tests run locally
no flaky timing behavior
no dependency on live Bevy scene
```

---

# Ticket S10 — Documentation Hook

## Mission

Reference Device Bus Substrate Systems from the Seedworks README or canonical scope doc.

## Required Note

```text
Substrate systems are implementation-support systems.
They are not allowed to expand v0.1 scope beyond the Old Waterworks pump loop.
```

## Acceptance Criteria

```text
README points to DEVICE_BUS_SUBSTRATE_SYSTEMS.md
v0.1 scope warning included
deferred items clearly marked
```

---

# Build Order

Recommended sequence:

```text
S0 Hygiene Gate
S1 Node Types
S2 Power Readout Stub
S3 Clock Drift Flag
S4 Pump Audio State Stub
S6 Proof-of-Repair Receipt v0
S7 Chronicle Event
S8 Field Deck Substrate Page
S5 Origin Audio Note
S9 Test Fixtures
S10 Documentation Hook
```

Reason:

```text
types first
readouts second
receipt third
Chronicle integration fourth
polish last
```

---

# Success Criteria

The substrate ticket set succeeds if the player can understand:

```text
the pump is power-constrained
the pump sounds mechanically sick
the repair creates a labor receipt
the receipt becomes part of the player’s verified history
```

The ticket set fails if:

```text
power becomes a full simulator before the pipe works
audio becomes cosmetic only
Proof-of-Repair becomes generic currency
substrate work breaks the micro-slice check lane
```

---

# Final Engineering Principle

```text
Do not build the grid.
Make one transformer matter.

Do not build the acoustic world.
Make one pump sound sick.

Do not build the economy.
Make one repair leave proof.
```

Final line:

```text
The substrate is real when the first pump cannot lie about power, sound, or who repaired it.
```
