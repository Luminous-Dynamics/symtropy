---
title: Symtropy Earth Atlas 2168 Template
status: canonical-draft
version: v0.1
scope: earth geography, nation-state transformation, regional pressure vectors, culture spectrum, Field Deck overlays, procedural history hooks
recommended_path: docs/earth/00_canon/EARTH_ATLAS_2168_TEMPLATE.md
companion_docs:
  - WORLD_TIMELINE_2000_2168.md
  - SPACE_HISTORY_2000_2168.md
  - PROCEDURAL_HISTORY_ENGINE.md
  - SOCIAL_SYSTEMS_AND_CHARTERS.md
  - SYMTROPY_DARK_CULTURES_CODEX_V0_2.md
  - SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_1.md
  - Symtropy Architecture Design Bible.md
  - HOSTILE_FACTIONS_AND_THREAT_ECOLOGY.md
  - NONHUMAN_GAME_THEORY_AND_AGENCY.md
---

# Symtropy Earth Atlas 2168 Template

## Working Title

**The Map Is a Memory of Maintenance**

## Core Thesis

The Earth Atlas is not a lore gazetteer.

It is a playable causality layer.

Every region of Earth in *Symtropy* should explain:

```text
what geography made possible
what history damaged
what infrastructure still matters
what authority failed
what cultures adapted
what the player can repair, inherit, expose, or worsen
```

The atlas should not answer only:

```text
What does this place look like in 2168?
```

It should answer:

```text
What happened here, who still claims it, what keeps people alive, and what would break if the player intervenes?
```

Design rule:

```text
Geography becomes gameplay when history creates constraints.
```

---

# 1. Why the Earth Atlas Exists

Symtropy should not be set in a generic global collapse.

Earth in 2168 is a mosaic of survival, adaptation, loss, repair, capture, memory, refusal, and renewed settlement. Some nation-states survive. Some federalize. Some hollow out. Some cities armor themselves. Some deltas drown. Some inland regions boom. Some infrastructure keeps running after legitimacy dies. Some regions rebuild from below through charters, repair guilds, water compacts, Archive Witness networks, ecological restoration, and local machine law.

The Earth Atlas converts that macro-history into a design tool.

It connects:

```text
planetary geography
↓
regional pressure vectors
↓
nation-state transformation
↓
settlement and faction archetypes
↓
culture spectrum
↓
site history
↓
Field Deck readings
↓
Chronicle consequences
```

The Atlas is the missing middle layer between global timeline and playable site.

---

# 2. Prime Design Rules

## 2.1 Atlas Entries Are Interaction Catalogs

Do not write regions as static lore entries.

Each regional entry must define mechanical constraints, Field Deck behavior, site-generation hooks, faction pressures, cultural overlays, and Chronicle outcomes.

Bad:

```text
The region is dry and politically unstable.
```

Better:

```text
Water authority is controlled by basin courts using analog acoustic locks.
Digital overrides are unreliable. Players must negotiate water credits, physically calibrate hydro-gates, and decide whether to recognize old national water claims or community-led hydro-rights.
```

## 2.2 Pressure Vectors Bias History; They Do Not Dictate It

A pressure vector should never become a simple deterministic faction switch.

Bad:

```text
if water_stress > 0.8:
    region = Water Dictatorship
```

Better:

```text
high water_stress
  → raises probability of rationing, basin courts, water militias, watershed commons, aqueduct principalities, refugee compacts, sabotage economies, and emergency powers
  → specific outcomes depend on state capacity, archive integrity, trust density, repair capacity, corporate capture, and player/worldline history
```

Design rule:

```text
Pressure creates crisis.
Culture interprets crisis.
Politics chooses survival logic.
The Chronicle records what survival became.
```

## 2.3 Every Culture Needs a Structural Paradox

Every regional culture should solve one suffering while creating another danger.

Examples:

```text
Hearth Commons
  solves: loneliness, hunger, domestic abandonment
  risks: intimacy pressure, soft surveillance, difficulty exiting the community

Road Choirs
  solves: territorial capture, static citizenship, route isolation
  risks: weak deep archives, unstable child education, fragile long-term repair obligations

Festival Republics
  solves: despair, civic alienation, post-collapse joylessness
  risks: spectacle replacing deliberation, beauty gatekeeping, political performance metrics

Cold Perimeter Holds
  solves: raids, uncertainty, external threat
  risks: permanent emergency posture, outsider suspicion, childhood militarization
```

Structural paradox is not the same as hidden evil.

A culture can be beautiful, sincere, and worth defending while still carrying a failure mode.

## 2.4 Field Deck Core Remains Stable; Culture Changes the Overlay

The Field Deck must retain a universal diagnostic grammar:

```text
SCAN
DIAG
ARCHIVE
CIVIC
NULL
REPAIR
WITNESS
TACTICAL_NET
```

Regional culture may alter:

```text
mode ordering
warning language
iconography
color temperature
what gets foregrounded
what gets suppressed
alert metaphors
trust labels
local terms for legitimacy
```

But it should not destroy the player's ability to understand the instrument.

Design rule:

```text
Field Deck Core = standardized diagnostic truth.
Culture Overlay = local moral vocabulary.
Faction Filter = what this society wants noticed.
Null Distortion = what the system has learned to misclassify.
```

## 2.5 Nation-States Transform, But Do Not Vanish Uniformly

By 2168, sovereignty is layered:

```text
old nation-state law
regional compacts
watershed law
energy districts
settlement charters
archive legitimacy
machine authority
identity/source-chain systems
worldline continuity claims
```

Do not write a world where every state collapses the same way.

Each region should identify which old political forms still matter, which have become ceremonial, which remain militarily real, which survive as archive claims, and which are contested by new settlement charters.

## 2.6 Earth History Must Remain Human

Alien presence, deep-time artifacts, machine ecologies, and worldline anomalies may intersect Earth history, but they should not steal human agency.

Do not imply:

```text
aliens built human civilization
aliens authored human culture
aliens secretly controlled all history
```

Use instead:

```text
aliens observed
aliens misread
aliens quarantined
aliens left instruments
humans adapted, failed, repaired, exploited, protected, and remembered
```

---

# 3. Regional Data Model

## 3.1 EarthAtlasRegion

```rust
struct EarthAtlasRegion {
    region_id: String,
    display_name: String,
    atlas_version: String,
    geography_class: Vec<GeographyClass>,
    old_sovereignties: Vec<OldSovereignty>,
    current_sovereignty_layers: Vec<SovereigntyLayer>,
    primary_pressure_vector: RegionPressureVector,
    secondary_pressure_vectors: Vec<RegionPressureVector>,
    nation_state_outcomes: Vec<NationStateOutcome>,
    dominant_culture_spectrum: Vec<CultureSpectrumEntry>,
    mechanical_modifiers: RegionMechanicalModifiers,
    field_deck_overlays: Vec<FieldDeckCultureOverlay>,
    site_generation_rules: Vec<SiteGenerationRule>,
    signature_sites: Vec<SignatureSiteSeed>,
    faction_seed_pool: Vec<FactionSeed>,
    ecology_profile: EcologyProfile,
    threat_profile: ThreatProfile,
    chronicle_hooks: Vec<ChronicleHook>,
    worldline_variants: Vec<WorldlineVariant>,
}
```

## 3.2 RegionPressureVector

Pressure vectors describe historical and systemic forces.

Suggested scalar range:

```text
0.0 = absent / negligible
0.5 = meaningful / region-shaping
1.0 = defining / existential
```

```rust
struct RegionPressureVector {
    heat_stress: f32,
    cold_stress: f32,
    water_stress: f32,
    food_stress: f32,
    energy_stress: f32,
    migration_pressure: f32,
    sea_level_pressure: f32,
    storm_pressure: f32,
    state_capacity: f32,
    corporate_capture: f32,
    archive_integrity: f32,
    repair_capacity: f32,
    trust_density: f32,
    automation_level: f32,
    null_drift: f32,
    ecological_recovery: f32,
    toxic_legacy: f32,
    militarization: f32,
    xeno_contact_pressure: f32,
}
```

## 3.3 NationStateOutcome

```rust
enum NationStateOutcome {
    AdaptationState,
    FederalizedMaintenanceState,
    ArmoredCoastalCityState,
    HollowState,
    ArchiveState,
    CorporateUtilityZone,
    WatershedCompact,
    SettlementCharterFederation,
    MachineManagedTrust,
    TreatySanctuary,
    RefugeeCompact,
    DisputedRestorationZone,
}
```

## 3.4 CultureSpectrumEntry

```rust
struct CultureSpectrumEntry {
    culture_family: String,
    emotional_lane: EmotionalLane,
    core_good_life_claim: String,
    best_side: String,
    structural_paradox: String,
    dark_failure_mode: String,
    field_deck_tone: FieldDeckTone,
    gameplay_expression: Vec<String>,
}
```

Suggested emotional lanes:

```text
cozy_domestic
nomadic_convoy
education_apprenticeship
spiritual_nonmagical
performance_beauty
hardcore_survival
merchant_logistics
deep_ecological
machine_positive
ordinary_democratic
pleasure_care
restraint_discipline
archive_memory
law_truth
craft_labor
security_conflict
offworld_starward
```

## 3.5 RegionMechanicalModifiers

```rust
struct RegionMechanicalModifiers {
    construction_constraints: Vec<String>,
    water_logic: Vec<String>,
    power_logic: Vec<String>,
    travel_logic: Vec<String>,
    access_logic: Vec<String>,
    device_bus_modifiers: Vec<String>,
    field_deck_modifiers: Vec<String>,
    combat_modifiers: Vec<String>,
    social_modifiers: Vec<String>,
    ecology_modifiers: Vec<String>,
    chronicle_modifiers: Vec<String>,
}
```

Examples:

```text
hydro_logic:
  Settlement power and water are linked through basin-level allocation gates.

null_prevention:
  High-authority water systems use analog acoustic checksums rather than wireless command.

archive_logic:
  Public testimony requires both machine logs and elder basin witness.
```

## 3.6 FieldDeckCultureOverlay

```rust
struct FieldDeckCultureOverlay {
    overlay_id: String,
    associated_culture: String,
    visual_temperature: String,
    alert_language: String,
    prioritized_modes: Vec<FieldDeckMode>,
    suppressed_modes: Vec<FieldDeckMode>,
    local_terms: Vec<LocalDiagnosticTerm>,
    failure_bias: String,
    sample_readout: String,
}
```

Examples:

```text
Hearth Common overlay:
  SECURITY WARNING → Neighborhood Care Alert
  POWER DEFICIT → Shared Warmth Shortfall
  CIVIC DISPUTE → Household Witness Needed

Cold Perimeter overlay:
  Neighborhood Care Alert → Civilian Cluster Vulnerability
  Trust Decline → Cohesion Failure
  Unknown Visitor → Unclassified Approach Vector
```

---

# 4. Required Atlas Entry Template

Use this format for every major Earth region.

---

## 4.1 Region Identity

```yaml
region_id:
display_name:
atlas_version:
canonical_status:
recommended_doc_path:
geography_class:
old_sovereignties:
current_sovereignty_layers:
primary_biomes:
primary_settlement_forms:
primary_culture_families:
```

Questions:

```text
What real geography anchors this region?
What old political claims still matter?
What new forms of legitimacy have replaced or layered over them?
What would a player immediately see, hear, and need?
```

---

## 4.2 Core Regional Thesis

One paragraph plus one design rule.

Template:

```text
This region is about [core survival pressure] becoming [political/cultural structure].

Design rule:
[short rule that tells designers how to make the region playable]
```

Example:

```text
This region is about water scarcity becoming constitutional memory.

Design rule:
Every water pipe should also be a legal argument.
```

---

## 4.3 Historical Arc

Use era modules, not year-by-year exhaustive timelines.

```text
2000–2035: Platform Acceleration
2035–2050: Adaptation Shock
2050–2075: Settlement Turn
2075–2100: Automation Legitimacy Crisis
2100–2130: After-Platform World
2130–2150: Ghost Civilization Formation
2150–2168: Seed Age
```

For each era:

```yaml
era:
dominant_crisis:
dominant_adaptation:
dominant_failure_mode:
infrastructure_legacy:
visible_scars:
site_generation_hooks:
```

---

## 4.4 Pressure Vector

```yaml
pressure_vector:
  heat_stress:
  cold_stress:
  water_stress:
  food_stress:
  energy_stress:
  migration_pressure:
  sea_level_pressure:
  storm_pressure:
  state_capacity:
  corporate_capture:
  archive_integrity:
  repair_capacity:
  trust_density:
  automation_level:
  null_drift:
  ecological_recovery:
  toxic_legacy:
  militarization:
  xeno_contact_pressure:
```

Also include:

```text
Primary pressure:
Secondary pressures:
Contradictory pressures:
Pressure that outsiders misunderstand:
Pressure that locals consider sacred:
```

---

## 4.5 Nation-State Transformation

```yaml
old_nation_state_role:
current_state_form:
state_capacity_pattern:
remaining_state_functions:
failed_state_functions:
archive_claims:
settlement_charter_relationship:
regional_compacts:
corporate_claims:
armed_claims:
```

Design question:

```text
Does the old state still protect, merely record, extract, arbitrate, threaten, or haunt?
```

---

## 4.6 Culture Spectrum

Each major regional culture should use this mini-template:

```yaml
culture_family:
emotional_lane:
core_good_life_claim:
best_side:
structural_paradox:
dark_failure_mode:
what_it_makes_beautiful:
what_it_makes_shameful:
who_gets_access:
who_is_excluded_or_delayed:
field_deck_overlay:
primary_gameplay_loop:
reform_path:
```

---

## 4.7 Mechanical Modifiers

```yaml
mechanical_modifiers:
  water_logic:
  power_logic:
  food_logic:
  travel_logic:
  access_logic:
  construction_logic:
  device_bus_logic:
  field_deck_logic:
  social_logic:
  ecology_logic:
  combat_logic:
  chronicle_logic:
```

These modifiers should be implementable by systems design.

---

## 4.8 Field Deck Interaction Layer

Every region should define at least three Field Deck readouts.

Required:

```text
1. SCAN / DIAG readout for physical infrastructure
2. CIVIC / ARCHIVE readout for legitimacy conflict
3. NULL readout for regional failure mode
```

Optional:

```text
4. REPAIR readout for mission-specific intervention
5. WITNESS readout for Chronicle recording
6. TACTICAL_NET readout for hostile pressure
```

---

## 4.9 Procedural Site Seeds

Each region should provide reusable site seeds.

```yaml
site_seed:
  site_id:
  site_type:
  old_authority:
  current_claimants:
  primary_lock:
  visible_scar:
  hidden_history:
  repair_paths:
  legitimacy_debt:
  null_risk:
  chronicle_outcomes:
```

Suggested minimum per region:

```text
1 water site
1 power site
1 archive site
1 care site
1 market/logistics site
1 hostile/remnant site
1 ecological restoration site
1 culture-specific civic site
```

---

## 4.10 Faction Seed Pool

Do not create one fixed faction list.

Create seed archetypes that the procedural faction system can weight differently by worldline.

```yaml
faction_seed:
  seed_id:
  archetype_vector:
  sacred_value:
  origin_wound:
  fear_pattern:
  unacceptable_behavior:
  negotiation_possible:
  reform_possible:
  hostility_triggers:
  field_deck_flags:
```

---

## 4.11 Ecology and Earth Species

```yaml
ecology_profile:
  primary_biomes:
  damaged_ecologies:
  restoration_species:
  protected_species:
  invasive_risks:
  trophic_matrix_slots:
  biosecurity_conflicts:
  nonhuman_agency_hooks:
```

Design rule:

```text
Species are not decoration. They must alter water, soil, food webs, toxin load, settlement resilience, civic rights, or biosecurity.
```

---

## 4.12 Threat Ecology

```yaml
threat_profile:
  human_threats:
  machine_threats:
  corporate_threats:
  state_remnant_threats:
  ecological_threats:
  null_forms:
  noncombat_tension_devices:
```

Every hostile actor should have:

```text
one understandable reason
one unacceptable behavior
one possible reform or boundary condition
```

---

## 4.13 Worldline Variants

Each region should support multiple historical outcomes.

```yaml
worldline_variant:
  variant_id:
  divergence_point:
  dominant_outcome:
  dominant_culture:
  region_state:
  primary_conflict:
  visual_difference:
  mechanical_difference:
  chronicle_signature:
```

Example variants:

```text
restoration timeline
corporate capture timeline
hollow-state timeline
watershed commons timeline
machine-infected timeline
archive-state timeline
militarized timeline
xeno-contact timeline
```

---

## 4.14 Concept Art and Audio Profile

```yaml
art_profile:
  silhouette_language:
  material_language:
  color_temperature:
  lighting_logic:
  weather_profile:
  signage_language:
  clothing_language:
  vehicle_language:
  architecture_families:
```

```yaml
audio_profile:
  ambient_bed:
  civic_sounds:
  machinery_sounds:
  warning_sounds:
  cultural_music:
  silence_logic:
```

---

## 4.15 Mission and Chronicle Hooks

```yaml
mission_hooks:
  repair_mission:
  diplomacy_mission:
  archive_mission:
  logistics_mission:
  ecological_mission:
  combat_mission:
  civic_vote_mission:
  worldline_mission:
```

```yaml
chronicle_hooks:
  public_repair_record:
  exposed_abuse_record:
  treaty_precedent:
  rights_floor_expansion:
  machine_testimony_record:
  ecological_witness_record:
  worldline_fork_record:
```

---

# 5. First Recommended Anchor Regions

The first Earth Atlas pass should define twelve anchor regions.

```text
1. Southern African Water-Energy Compact
2. White Ledger Territories / Antarctica
3. Amazon Basin Restoration and Extraction Front
4. Nile / East African High Basin Compacts
5. South Asian Delta Federations
6. North American Interior Repair Belt
7. Gulf Coast Pump-State and Floodline Territories
8. Mediterranean Dry Belt and Solar Charter Cities
9. Siberian Thaw Belt and Methane Archive Zones
10. Pacific Island Archive States and Ocean Commons
11. Drowned Megacity Cluster
12. Lunar-Linked Launch Corridor Networks
```

Each anchor should demonstrate one major combination of:

```text
geography
nation-state transformation
culture spectrum
Field Deck overlay
site-generation hooks
Chronicle consequence
```

---

# 6. Implementation Guidance

## 6.1 Minimum Viable Atlas Entry

For early production, an atlas entry is valid if it includes:

```text
region identity
core thesis
pressure vector
nation-state transformation
3 culture spectrum entries
5 mechanical modifiers
3 Field Deck readouts
5 site seeds
3 worldline variants
```

## 6.2 Do Not Over-Explain In-Game

The Atlas is a design backend.

Players should not receive an encyclopedia dump.

They should experience region history through:

```text
locked infrastructure
local words in the Field Deck overlay
what NPCs argue about
which repairs require witness
which roads are trusted
which records count
what the map refuses to simplify
what the Chronicle preserves
```

Design rule:

```text
The player should meet lore as resistance, not exposition.
```

## 6.3 First Playable Earth Atlas Vertical Slice

Recommended first showcase:

```text
Southern African Water-Energy Compact
```

Why:

```text
It is grounded.
It connects water, energy, repair, law, community, extraction, and post-state governance.
It supports analog/lithic anti-Null mechanics without requiring extreme polar or off-world habitats.
It can host hopeful, ordinary, dark, nomadic, archive, machine, and ecological cultures in one region.
```

Recommended second showcase:

```text
White Ledger Territories / Antarctica
```

Why:

```text
It asks who may settle a continent humanity promised not to own.
It connects archive sanctuaries, treaty law, closed-loop habitats, geothermal heat politics, refugee pressure, and planetary memory.
```

Together:

```text
Southern Africa asks:
Who owns water where people already live?

Antarctica asks:
Who may settle a continent humanity promised not to own?
```

That pairing defines the moral range of Earth 2168.

