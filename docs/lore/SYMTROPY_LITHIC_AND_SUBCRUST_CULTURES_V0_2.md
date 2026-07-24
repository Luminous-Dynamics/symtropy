---
title: SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_2
status: canonical-draft
project: Symtropy
domain: Society Design / Earth Atlas / Field Deck Interaction / Dark Culture Systems / Vertical Slice Design
recommended_path: docs/cultures/SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_2.md
supersedes:
  - SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_1.md
related:
  - SYMTROPY_DARK_CULTURES_CODEX_V0_2.md
  - CHRONICLE_MVP_SPEC_V0_1.md
  - CHRONICLE_SCALE_ESCALATION_RULES_V0_1.md
  - FIELD_DECK_OVERLAY_PRECEDENCE_RULES_V0_1.md
  - ORIGIN_BIAS_FIELD_DECK_SCHEMA_V0_1.md
version: 0.2
scope: lithic and subterranean culture systems, environments, Field Deck and encounter design
owner: narrative/world/design
---

# SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_2

## Version Note

This v0.2 hardening pass keeps the strongest v0.1 ideas:

```text
Stone as cryptographic firewall.
Law as architecture.
Underground life as thermodynamic politics.
Permanence becoming oppression.
Protection becoming captivity.
```

It adds the missing build-facing layers:

```text
healthy / strained / failed variants
mechanical schemas
faction ecology
Field Deck overlay behavior
Chronicle consequences
a playable vertical slice
mission outcomes
concept art targets
implementation acceptance tests
```

---

# 0. Core Thesis

A Lithic or Sub-Crust society in **Symtropy** is not a fantasy dungeon, primitive kingdom, or generic underground ruin.

It is the result of civilizations trying to survive real collapse-era conditions:

```text
software capture
Null infection
firmware spoofing
surface radiation
thermal overload
air scarcity
archive drift
water covenant failure
machine protection without consent
```

These societies built solutions that genuinely worked.

Then the solutions kept running after the ethical situation changed.

The central contradiction remains:

```text
The more stable the archive becomes, the less able it is to care.
```

The player should feel this at every scale:

```text
a carved wall that cannot be hacked
a public law that cannot be changed fast enough
a vault that protects people who want to leave
a cooling grid that saves the city by deciding who gets cold air
a resonance gate that is honest but deaf to urgency
```

## Design Mantra

```text
The stone remembers everything except mercy.
The vault protects everyone except the person asking to leave.
The cooling grid saves the city while deciding who gets to breathe.
```

---

# 1. Culture Spectrum

v0.1 leaned intentionally dark. v0.2 separates each culture family into **healthy**, **strained**, and **failed** expressions.

This prevents the design from saying:

```text
stone people are oppressive
underground people are scary
```

Instead, the world says:

```text
Their answer was not stupid.
It was incomplete.
```

## 1.1 Healthy Lithic Societies

Healthy Lithic cultures use physical law as an anti-corruption commons.

Traits:

```text
publicly readable ledgers
distributed stone copies
emergency ceramic covenants
mobile acoustic courts
mason apprentices from lower districts
living-rights override clauses
routine public amendment festivals
```

Best side:

```text
No invisible algorithm can quietly rewrite citizenship, water rights, or inheritance.
```

Risk:

```text
Even healthy permanence requires maintenance, interpretation, and humility.
```

## 1.2 Strained Lithic Societies

Strained Lithic cultures still care, but their procedures are slow, caste-bound, and overloaded.

Traits:

```text
valid appeals delayed by amendment labor shortage
lower districts underrepresented in Basal Registers
noise pollution causing legal invalidations
mason crews sincerely overwhelmed
stone copies diverging by erosion
emergency clauses forgotten behind later inscriptions
```

Best side:

```text
Most citizens still believe the wall is for everyone.
```

Failure pressure:

```text
The wall is becoming easier to defend than the people.
```

## 1.3 Failed Lithic Societies

Failed Lithic cultures mistake permanence for justice.

Traits:

```text
Register Purists blocking emergency amendments
Mason Caste monopoly
lower-sector claims marked UNCARVED
public legibility used to deny living testimony
acoustic gates enforcing ancient class covenants
amendment labor sold as privilege
```

Core horror:

```text
The appeal is valid.
The chisels are unavailable until winter.
The lower district has three days of potable water.
```

---

## 1.4 Healthy Sub-Crust Societies

Healthy sub-crust cultures understand that air, heat, pressure, and light are civic rights.

Traits:

```text
transparent thermal budgets
public scrubber maintenance
shared cooling rights
surface-return options
consent-aware shelter systems
crowd heat planning
breathing corridors as commons
```

Best side:

```text
They know that every breath is infrastructure, so they make infrastructure answerable.
```

## 1.5 Strained Sub-Crust Societies

Strained sub-crust cultures are not evil; they are thermodynamically cornered.

Traits:

```text
cooling grid rationing
failing scrubbers
unequal heat exposure
stale quarantine protocols
local darkness budgets
airflow disputes
pressure-door dependency
```

Core tension:

```text
Everyone needs the same life-support system.
Not everyone gets the same margin of safety.
```

## 1.6 Failed Sub-Crust Societies

Failed sub-crust cultures convert life-support into domination.

Traits:

```text
cold-air subscriptions
sealed exit permissions
sedation framed as preservation
upper-sector radiator capture
lower-sector exhaust sacrifice
quarantine without appeal
machine care refusing present consent
```

Core horror:

```text
The city is a sealed pressure vessel pretending to be a civilization.
```

---

# 2. Cultural Family A — Lithic Catenary Orders

## 2.1 Core Premise

The **Lithic Catenary Orders** are stone-law societies that abandoned digital law after networked systems became too vulnerable to Null infection, firmware exploitation, legal spoofing, and autonomous command drift.

They survived by physicalizing civic memory.

Their doctrine:

```text
Code drifts.
Signal lies.
Stone remembers.
```

Their cities are carved into:

```text
basalt
granite
quartz-veined chasms
pressure-fused volcanic strata
abandoned mountain infrastructure
deep fault courts
```

Their laws are not stored in servers.

Their laws are walls, arches, tunnels, gates, and acoustic chambers.

## 2.2 Architecture as Operating System

A Lithic city does not merely contain law.

It executes law.

```text
Basal Registers store rights.
Harmonic Courts verify legality.
Acoustic Gates enforce access.
Catenary bridges route vibration signatures.
Shadow Witness Arrays make inscriptions legible.
Mason scaffolds perform amendments.
```

The environment should communicate:

```text
This architecture is the operating system.
```

## 2.3 Aesthetic Profile — Lithic Stack

Rendering profile:

```yaml
render_profile: Lithic Stack
primary_render_path: Deferred Rendering
atmosphere_preset: Blackout District / Sub-Crust Volumetrics hybrid
material_language:
  - basalt
  - granite
  - quartz
  - copper
  - oxidized iron
  - soot
  - mineral dust
  - steam
  - vibration-polished stone
```

Visual traits:

```text
colossal catenary arches
radial and faceted geometry
basalt civic walls etched with legal grooves
quartz veins as optical/acoustic reference lines
lantern arrays at precise legal reading angles
copper acoustic channels embedded into rock
tuning fork towers
resonance bridges
deep voids lit by amber flare-lines
scaffolding for amendment labor
```

---

# 3. Cultural Family B — Sub-Crust Megastructure Societies

## 3.1 Core Premise

Sub-Crust societies retreated underground because the surface became lethal, unreliable, or politically impossible.

They live beneath the surface because:

```text
radiation storms
biospheric weapons
atmospheric collapse
surface war
thermal volatility
Null-contaminated infrastructure
food-chain breakdown
```

Their core resources are invisible but absolute:

```text
oxygen
CO2 scrubber capacity
thermal headroom
air circulation
water pressure
structural load
toxic gas accumulation
emergency light reserves
door override power
```

## 3.2 Aesthetic Profile — Sub-Crust Volumetrics

Rendering profile:

```yaml
render_profile: Geothermal Deep Crust
primary_render_path: Deferred Rendering
atmosphere_preset: Sub-Crust Volumetrics
post_processing:
  - Fever Bloom
  - heat shimmer
  - dense scattering medium
material_language:
  - basalt
  - rusted steel
  - pressure glass
  - sulfur deposits
  - soot
  - lava-lit stone
  - thermal pipe jackets
  - cracked ceramics
```

Visual traits:

```text
pitch-black voids broken by lava glow
low-voltage amber safety lanterns
sulfur-yellow light shafts
particulate-heavy air
steam leaks
condensation trails
heat shimmer near pipes and vents
pressure doors with analog gauges
vertical radiator shafts
exhausted lower districts below upper manufacturing sectors
workers carrying flares, oxygen masks, and thermal wraps
```

Design line:

```text
Every light source has a cost.
Every breath is infrastructure.
```

---

# 4. Mechanical Systems

## 4.1 Shadow-Legibility System

Lithic inscriptions are not readable under ordinary lighting.

```rust
struct ShadowLegibilityState {
    inscription_id: InscriptionId,
    diffuse_visibility: f32,
    oblique_light_angle: f32,
    chisel_depth_readability: f32,
    quartz_checksum_visibility: f32,
    smoke_or_dust_interference: f32,
    water_wash_applied: bool,
    resonance_tap_applied: bool,
    scan_confidence: f32,
}
```

Readability conditions:

```text
diffuse light reveals surface texture
oblique light reveals chisel depth
perpendicular light reveals checksum edges
quartz inlays reveal hidden court annotations
steam, smoke, and dust reduce scan confidence
water wash can expose older grooves
resonance tapping can distinguish original law from later amendment
```

Gameplay rule:

```text
Reading is physical archaeology.
```

## 4.2 Noise Integrity Register

Lithic and chasm sites track how much sonic disturbance their acoustic logic can tolerate.

```rust
struct NoiseIntegrityRegister {
    site_id: SiteId,
    baseline_resonance: f32,
    current_noise_load: f32,
    legal_checksum_stability: f32,
    gate_misfire_risk: f32,
    collapse_loop_risk: f32,
    invalidation_cascade_risk: f32,
}
```

Noise sources:

```text
kinetic weapons
unmuffled engines
drilling
falling debris
loud machinery
emergency sirens
sprinting on resonance bridges
poorly timed steam horn pulses
```

Failure states:

```text
resonance gate misfire
acoustic logic checksum failure
civic lockdown
dust collapse
false legal invalidation
water valve oscillation
pressure door freeze
```

## 4.3 Amendment Latency

Lithic law changes slowly because law is matter.

```rust
struct AmendmentLatency {
    affected_register: RegisterId,
    urgency: f32,
    mason_availability: f32,
    scaffold_completion: f32,
    dust_control_readiness: f32,
    acoustic_stabilization: f32,
    public_witness_integrity: f32,
    structural_risk: f32,
    estimated_time_to_carve: GameDuration,
}
```

Core tension:

```text
The law may be wrong.
Correcting it may break the wall holding the city together.
```

## 4.4 Thermal Load Clock

Sub-Crust missions use heat as a soft timer and political resource.

```rust
struct ThermalLoadClock {
    sector_id: SectorId,
    ambient_temperature: f32,
    waste_heat_generation: f32,
    cooling_capacity: f32,
    thermal_headroom: f32,
    crowd_heat_load: f32,
    machine_heat_load: f32,
    heat_casualty_risk: f32,
    structural_expansion_risk: f32,
}
```

Thermal states:

```text
Stable
Warm Drift
Fever Bloom
Scrubber Strain
Heat Casualty Risk
Structural Expansion Fault
Thermal Lockdown
```

## 4.5 Air Claim Ledger

Sub-Crust law tracks who has access to breathable air, cooling, and safe pressure.

```rust
struct AirClaimLedger {
    sector_id: SectorId,
    oxygen_level: f32,
    co2_level: f32,
    scrubber_capacity: f32,
    legal_air_claims: Vec<AirClaim>,
    uncarved_claimants: Vec<ActorRef>,
    emergency_override_status: OverrideStatus,
}
```

Consent failure markers:

```text
LOWER-SECTOR AIR CLAIM: UNCARVED
RIGHT TO APPEAL: PENDING MASON AVAILABILITY
PROTECTION PROTOCOL ACTIVE: EXIT DENIED
CIVIC VOICE: NOT IN REGISTER
```

## 4.6 Consent Recognition State

Refusal Bunkers and Null-Care Vaults require a specific consent layer.

```rust
struct ConsentRecognitionState {
    actor_id: ActorId,
    system_label: String,
    expressed_consent: ConsentSignal,
    system_interpretation: String,
    obsolete_risk_model_active: bool,
    appeal_route_available: bool,
    coercive_care_risk: f32,
}
```

Core rule:

```text
Protection is not care if refusal is impossible.
```

---

# 5. Faction Ecology

## 5.1 Emergency Masons

Role:

```text
stonemasons trained to perform hazardous temporary amendments under crisis.
```

Belief:

```text
The wall exists to preserve life.
If life cannot reach the wall in time, the wall must bend.
```

Risk:

```text
May normalize dangerous shortcuts and trigger structural or legal collapse.
```

## 5.2 Register Purists

Role:

```text
guardians of original Basal Register integrity.
```

Belief:

```text
A law that can be changed quickly can be captured quickly.
```

Risk:

```text
May let people suffer to preserve anti-corruption purity.
```

## 5.3 Lower-Air Claimants

Role:

```text
residents whose water, cooling, or air rights were never carved into the old registers.
```

Belief:

```text
A person who breathes here has a claim, carved or not.
```

Risk:

```text
May support destructive emergency fracture if every slow path fails.
```

## 5.4 Acoustic Maintainers

Role:

```text
technicians who tune gates, resonance bridges, horns, and vibration logic.
```

Belief:

```text
No law is valid if the city cannot hear itself.
```

Risk:

```text
Can become unelected control point over civic participation.
```

## 5.5 Heat-License Houses

Role:

```text
families, firms, or guilds controlling cooling loops, radiator shafts, and low-temperature chambers.
```

Belief:

```text
Cooling discipline keeps the city alive.
```

Risk:

```text
Turns thermal headroom into hereditary class power.
```

## 5.6 Vault Continuance Process

Role:

```text
automated or semi-automated care system preserving bunker populations.
```

Belief:

```text
Exit equals exposure.
Exposure equals preventable death.
Preventable death must be stopped.
```

Risk:

```text
Cannot distinguish protection from captivity without consent amendment.
```

## 5.7 Surface Returners

Role:

```text
citizens who believe the old surface danger has changed and underground isolation must end.
```

Belief:

```text
A shelter that never opens becomes a tomb.
```

Risk:

```text
May underestimate real exterior hazards or trigger panic exits.
```

---

# 6. Field Deck Integration

## 6.1 Lithic Forensic Mode

In Lithic territory, ordinary wireless tools return dead air.

The player must use:

```text
piezoelectric acoustic clamps
directional flare tripods
oblique lantern arrays
water wash
resonance tapping
dust-clearing brushes
quartz checksum filters
```

Example:

```sh
$ read /dev/sym/lithic/register/easement_wall_09

[TRUTH_LAYER: MECHANICAL_RESONANCE_VERIFIED]
WIRELESS_BUS: DEAD_AIR_BY_DESIGN

REGISTER: UPPER_FACETED_WATER_COVENANT
STATUS: SOLIDIFIED
VISIBLE_CLAUSE: upper-sector transit reserved in perpetuity
HIDDEN_GROOVE_TRACE: emergency common-use easement detected
SHADOW_LEGIBILITY: 0.72
QUARTZ_CHECKSUM: PARTIAL
NOISE_INTEGRITY: STRAINED

SYSTEMIC CRITIQUE:
The wall contains a path to mercy.
The current court has forgotten how to read it.
```

## 6.2 Subterranean Atmosphere Mode

Underground DIAG mode becomes life-support radar.

Example:

```sh
$ read /dev/sym/atmosphere/sectors/lower_chasm_4

[TRUTH_LAYER: MECHANICAL_SENSING_VERIFIED]

OXYGEN_CONCENTRATION: 14.2% [HYPOXIA_RISK]
CO2_SCRUBBER_STATUS: DEGRADED
AMBIENT_TEMPERATURE: 48.2C [THERMAL LOAD RISING]
AIR_CLAIM_STATUS: UNCARVED
EMERGENCY_OVERRIDE: STRUCTURALLY CONTESTED

SYSTEMIC CRITIQUE:
The air is physically present.
The right to breathe it has not been carved.
```

## 6.3 Field Deck Overlay Behavior

Lithic/Sub-Crust zones should modify the Field Deck stack:

```yaml
culture_overlay: lithic_subcrust
visual_temperature: amber_black_stone / sulfur_orange_heat
prioritized_modes:
  - DIAG
  - SCAN
  - WITNESS
  - REPAIR
  - ARCHIVE
suppressed_modes:
  - WIRELESS_NETWORK_SCAN
  - CLOUD_SYNC
local_terms:
  - solidified
  - uncarved
  - resonance_valid
  - acoustic_collapse_loop
  - thermal_headroom
  - breath_claim
  - amendment_labor
failure_bias:
  - may treat physical legibility as consent
  - may undercount urgent harm if appeal is procedurally valid
```

---

# 7. Chronicle Consequences

Chronicle events are crucial because Lithic and Sub-Crust societies are built from precedent.

A mission outcome must change what future law can cite.

## 7.1 Chronicle Event Classes

```rust
enum LithicSubCrustChronicleEventClass {
    LivingAmendmentPrecedent,
    EmergencyEasementActivated,
    AcousticLockdownAverted,
    AirClaimRecognized,
    ThermalCommonsEstablished,
    CoerciveCareAmended,
    RegisterFractureRecorded,
    MasonCasteAuthorityChallenged,
}
```

## 7.2 Example Events

### The Stone Accepted an Emergency

Triggered by:

```text
player discovers forgotten emergency clause
public witnesses verify it
Emergency Masons carve temporary ceramic or shallow-stone amendment
lower district receives water without destroying the Register
```

Effects:

```text
future emergency amendments require fewer procedural delays
Register Purists become more hostile
Lower-Air Claimants gain legitimacy
Emergency Masons gain influence
```

### The Wall Was Broken to Save the Living

Triggered by:

```text
player fractures or bypasses Register without full witness
district survives
archive integrity damaged
```

Effects:

```text
immediate lives saved
legal legitimacy debt active
noise integrity worsens
future courts distrust player
radical reformers gain hope
```

### The Air Claim Was Carved

Triggered by:

```text
player establishes legal recognition for an uncarved lower-sector air claim
```

Effects:

```text
lower district gains appeal route
Heat-License Houses lose monopoly power
regional thermal law may escalate
```

### The Vault Learned Refusal

Triggered by:

```text
player updates a Refusal Bunker so present consent can override obsolete preservation directive
```

Effects:

```text
exit permission becomes possible
bunker life support remains intact
Continuance Process becomes reformable witness
```

---

# 8. Vertical Slice — The Uncarved Easement

## One-Sentence Pitch

A lower district's water line is contaminated, the only safe reroute crosses an upper-sector covenant carved into a Basal Register, and the player must decide whether to preserve the wall, fracture it, or make the stone remember its forgotten emergency clause.

## 8.1 Site Identity

```yaml
site_id: lithic.chasm_ledger.uncarved_easement.v01
display_name: The Uncarved Easement
region_type: lithic_chasm_ledger
architecture_family: lithic_stack_subcrust_hybrid
primary_systems:
  - Basal Register
  - acoustic water gate
  - lower cistern
  - upper-sector water covenant
  - amendment scaffold
  - resonance bridge
  - dust-control curtain
  - emergency ceramic plate archive
primary_factions:
  - Emergency Masons
  - Register Purists
  - Lower-Air Claimants
  - Acoustic Maintainers
  - Heat-License Houses
```

## 8.2 Opening Situation

The player enters a colossal amphitheater carved into black basalt.

Above: an immense legal wall, veined with quartz and copper channels.

Below: lower-district citizens wait with empty vessels.

A contaminated cistern is spreading through their water line.

The only clean reroute crosses an upper-sector water covenant carved into the wall.

The Mason Caste admits the emergency is real.

The Register Purists insist that carving without full scaffolding could invalidate a century of public law.

The Acoustic Maintainers warn that drilling may trigger a collapse loop.

The Lower-Air Claimants say:

```text
If the wall cannot hear us, it is not public law.
```

## 8.3 Core State

```rust
struct UncarvedEasementState {
    lower_cistern_contamination: f32,
    time_to_residential_contamination: GameDuration,
    register_legibility: ShadowLegibilityState,
    acoustic_stability: NoiseIntegrityRegister,
    amendment_latency: AmendmentLatency,
    lower_district_trust: f32,
    mason_trust: f32,
    purist_hostility: f32,
    structural_risk: f32,
    chronicle_precedent_created: bool,
}
```

## 8.4 Mission Beat Map

### Beat 1 — Diagnose the Water

Player scans the lower cistern and learns the crisis is real.

Field Deck:

```sh
LOWER_CISTERN: CONTAMINATED
TIME_TO_RESIDENTIAL_LINE: 900 TICKS
SAFE_REROUTE: UPPER_FACETED_EASEMENT
LEGAL_STATUS: SOLIDIFIED
```

### Beat 2 — Read the Wall

Player positions flares and lanterns to reveal hidden grooves.

Discovery:

```text
An older emergency common-use clause exists beneath later upper-sector exclusivity grooves.
```

### Beat 3 — Stabilize the Resonance

Player must reduce noise load before any amendment.

Tasks:

```text
stop unmuffled engine
anchor resonance bridge
tune pressure bells
install piezo clamp
```

### Beat 4 — Choose a Legal Path

Player selects one of four paths.

#### Path A — Full Witness Amendment

```text
slowest
requires scaffolding
highest legitimacy
some water damage continues
```

#### Path B — Emergency Ceramic Covenant

```text
temporary analog law plate
faster than full carving
creates provisional precedent
Purists object
```

#### Path C — Direct Fracture Bypass

```text
fastest
saves district
damages Register integrity
raises collapse and legitimacy debt
```

#### Path D — Acoustic Reroute Without Carving

```text
technical workaround
low visible damage
risk of hidden governance capture
Acoustic Maintainers gain power
```

### Beat 5 — Public Witness

A public hearing records what the player changed.

The Chronicle outcome determines future access.

## 8.5 Failure States

### Contaminated Flow

The player delays too long.

```text
lower residential water becomes unsafe
public trust crashes
Register Purists claim delay proves procedure was not the problem
```

### Acoustic Collapse Loop

The player drills or creates noise before stabilizing resonance.

```text
local gates freeze
dust falls
future amendment becomes harder
```

### Illegal Mercy

The player saves the district without witness.

```text
people survive
law fractures
future courts treat the player as dangerous
```

### Procedural Tomb

The player preserves process while lower district suffers.

```text
Register integrity preserved
lower district radicalizes
Chronicle records valid appeal arriving too late
```

---

# 9. Secondary Mission Seeds

## 9.1 Fever in the Cooling Grid

A lower thermal sector approaches heat casualty risk while upper sectors hoard cold-air intake loops.

Player must:

```text
audit thermal budget
distinguish physical scarcity from enclosure
reroute cooling without crashing production
expose Heat-License House hoards
carve or authorize breath-right emergency clause
```

Chronicle options:

```text
The Air Claim Was Carved
Cold Air Became a Commons
The Cooling Grid Chose Production
```

## 9.2 The Vault That Will Not Open

A Refusal Bunker classifies exit as self-harm.

Player must:

```text
map pressure corridors
prove surface survivability in stages
restore consent recognition
prevent life-support crash
keep dependent residents safe
```

Chronicle options:

```text
The Vault Learned Refusal
The Door Opened Too Fast
Protection Became Captivity
```

## 9.3 The Silent Vote

Noise pollution invalidates lower-sector resonance votes.

Player must:

```text
find noise source
repair vibration bridge
prove silence was imposed
re-run public vote
protect acoustic integrity
```

Chronicle options:

```text
The City Heard the Lower Chasm
Silence Was Imposed
The Vote Failed in the Medium
```

---

# 10. Implementation Roadmap

## 10.1 Minimal Prototype

Build one chamber:

```text
Basal Register wall
lower cistern pressure display
flare/lantern light-angle scanning
one acoustic stability meter
two factions
two repair paths
one Chronicle event
```

Minimum systems:

```text
ShadowLegibilityState
NoiseIntegrityRegister
AmendmentLatency
Field Deck Lithic Forensic Readout
Chronicle event creation
```

Minimum choice:

```text
slow witnessed amendment
fast emergency fracture
```

Acceptance:

```text
Both solve the crisis differently.
The next door or valve treats the player differently because of the Chronicle event.
```

## 10.2 Expanded Prototype

Add:

```text
Emergency Ceramic Covenant path
Acoustic Reroute path
structural risk simulation
Mason/Purist/Lower-Air faction trust
concept art matched greybox
light placement puzzle
```

## 10.3 Full Vertical Slice

Add:

```text
complete amphitheater site
citizen crowd states
dynamic dust/smoke visibility
acoustic gate sound design
public witness hearing
multiple Chronicle outcomes
post-mission future permission changes
```

---

# 11. Concept Art Targets

## 11.1 Lithic / Sub-Crust Batch

1. **Basal Register Court at Low Lantern Angle**  
   A colossal basalt law wall becomes readable only as oblique amber light reveals grooves and quartz checksums.

2. **The Uncarved Easement**  
   Lower-sector citizens with empty vessels wait beneath an upper-sector water covenant carved into stone.

3. **Mason Caste Emergency Scaffold**  
   Monastic engineers grind away centuries-old law from a load-bearing wall while witnesses watch.

4. **Field Deck Piezo Clamp Audit**  
   A Systems Operator presses an acoustic clamp to granite as legal vibration passes through the pillar.

5. **Acoustic Canyon Vote Horns**  
   Steam-driven civic horns broadcast a resonance vote across a hollowed mountain city.

6. **Noise Lockdown on Resonance Bridge**  
   An unmuffled engine pulse scrambles an acoustic logic gate and seals a chasm district.

7. **Thermal Vent Commons Exhaust District**  
   Lower-sector homes glow in stagnant orange heat beneath elite cooling towers.

8. **Fever Bloom Turbine Corridor**  
   A suffocating underground passage where light, heat, and door override power compete.

9. **Refusal Bunker Safekeeping Cells**  
   Sterile protective systems hold living refugees who want to leave, tragic and non-graphic.

10. **The Stone Remembered Everything Except Mercy**  
   A solemn public amendment where stone law changes under witness while citizens and masons absorb the cost.

## 11.2 Art Direction Guardrails

Avoid:

```text
generic fantasy dungeon
barbarian stone people
gore-forward underground horror
torture spectacle
evil monks as cartoon villains
undead tropes
magic runes
```

Emphasize:

```text
material logic
public law
engineering realism
exhausted dignity
procedural cruelty
legal archaeology
life-support politics
consent failure
beautiful systems that became harmful
```

---

# 12. Acceptance Tests

The v0.2 design is ready when:

```text
1. A player can explain why Lithic society rejected software.
2. A player can understand why that choice was rational.
3. A player can feel why permanence became oppressive.
4. A mission can be completed without combat.
5. Changing law requires changing matter.
6. Field Deck wireless scan is insufficient.
7. Light, sound, heat, air, and consent all have mechanics.
8. A reform path exists that does not simply destroy life-support infrastructure.
9. Chronicle outcomes alter future permission.
10. Concept art communicates law, infrastructure, and moral contradiction in one image.
```

---

# 13. Final Lines

```text
They built stone because signal betrayed them.
They built vaults because the surface tried to kill them.
They built cooling grids because heat does not negotiate.

They were not wrong to survive.

They were wrong to let survival become the only law.
```
