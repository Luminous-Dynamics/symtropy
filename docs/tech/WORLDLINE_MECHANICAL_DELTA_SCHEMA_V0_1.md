---
title: Worldline Mechanical Delta Schema v0.1
status: canonical-draft
project: Symtropy
domain: Earth Atlas / Procedural History Engine / Worldline Variants / Gameplay Systems
recommended_path: docs/earth-atlas/00_schema/WORLDLINE_MECHANICAL_DELTA_SCHEMA_V0_1.md
extends:
  - EARTH_ATLAS_2168_TEMPLATE.md
  - PROCEDURAL_HISTORY_ENGINE.md
  - MULTIPLAYER_TRUTH_MODEL.md
---

# Worldline Mechanical Delta Schema v0.1

## Working Title

**A Different History Must Play Differently**

## Purpose

This document adds a formal `mechanical_delta` layer to Earth Atlas worldline variants.

The Earth Atlas already defines regions through geography, pressure vectors, nation-state transformations, culture spectrum, Field Deck overlays, site seeds, faction seeds, and worldline variants.

This schema closes the remaining gap:

```text
A worldline variant is not complete until the player can feel its history through mechanics.
```

A variant should not only answer:

```text
What happened differently?
```

It should answer:

```text
What opens?
What locks?
What costs more?
What becomes easier?
What becomes dangerous?
What does the Field Deck foreground?
What does the Chronicle remember differently?
```

---

# 1. Core Rule

```text
Narrative divergence must produce gameplay divergence.
```

A worldline variant must change at least three of the following:

```text
resource availability
access credentials
infrastructure state
Field Deck overlays
faction posture
NPC schedules
site seeds
machine behavior
Null drift
legal authority
repair path
combat pressure
crafting provenance
vehicle routes
Chronicle precedent
worldline fork risk
```

If a worldline changes only flavor text, it is not yet a mechanical variant.

---

# 2. Schema

```rust
struct WorldlineVariant {
    variant_id: String,
    display_name: String,
    divergence_summary: String,
    dominant_memory: String,
    pressure_vector_delta: PressureVectorDelta,
    mechanical_delta: MechanicalDelta,
    field_deck_delta: FieldDeckDelta,
    chronicle_delta: ChronicleDelta,
    site_seed_delta: Vec<SiteSeedDelta>,
    faction_delta: Vec<FactionDelta>,
    failure_bias: Vec<FailureBias>,
    restoration_opportunity: Vec<RestorationOpportunity>,
}
```

---

# 3. Pressure Vector Delta

Pressure vectors describe how history changed the region's baseline forces.

```rust
struct PressureVectorDelta {
    heat_stress_delta: f32,
    cold_stress_delta: f32,
    water_stress_delta: f32,
    food_stress_delta: f32,
    migration_pressure_delta: f32,
    state_capacity_delta: f32,
    corporate_capture_delta: f32,
    automation_level_delta: f32,
    archive_integrity_delta: f32,
    repair_capacity_delta: f32,
    trust_density_delta: f32,
    ecological_recovery_delta: f32,
    toxic_legacy_delta: f32,
    null_drift_delta: f32,
    xeno_contact_pressure_delta: f32,
}
```

Use small deltas for subtle worldlines.

Use large deltas only when the divergence is historically structural.

---

# 4. Mechanical Delta

```rust
struct MechanicalDelta {
    resource_rules: Vec<ResourceRuleDelta>,
    access_rules: Vec<AccessRuleDelta>,
    infrastructure_rules: Vec<InfrastructureRuleDelta>,
    npc_rules: Vec<NpcRuleDelta>,
    faction_rules: Vec<FactionRuleDelta>,
    hazard_rules: Vec<HazardRuleDelta>,
    repair_rules: Vec<RepairRuleDelta>,
    vehicle_rules: Vec<VehicleRuleDelta>,
    crafting_rules: Vec<CraftingRuleDelta>,
    combat_rules: Vec<CombatRuleDelta>,
    economy_rules: Vec<EconomyRuleDelta>,
}
```

## 4.1 Resource Rule Delta

Examples:

```text
water_rationing: public / household / corporate_metered / convoy_distributed / machine_allocated
energy_priority: medbay_first / court_first / defense_first / archive_first / reactor_first
filter_supply: abundant / seasonal / licensed / contaminated / restricted
```

## 4.2 Access Rule Delta

Examples:

```text
valve_access: open_public / basin_court_witness / emergency_token / corporate_license / machine_clearance
archive_access: public_reading / witness_supervised / ranger_restricted / machine_only / damaged
refuge_access: open / quota / sponsor_required / health-screened / denied_by_treaty
```

## 4.3 Infrastructure Rule Delta

Examples:

```text
waterworks_state: repaired / failing / captured / automated / drowned / sealed
grid_state: public_microgrid / corporate_utility / blackout_fragments / reactor_loop / weather_machine_loop
transport_state: roads_intact / convoy_only / ice_seasonal / drone_patrolled / rail_dead
```

## 4.4 Field Deck Delta

```rust
struct FieldDeckDelta {
    visual_temperature: String,
    alert_language: String,
    prioritized_modes: Vec<FieldDeckMode>,
    suppressed_modes: Vec<FieldDeckMode>,
    local_terms: Vec<String>,
    false_positive_bias: Vec<String>,
    false_negative_bias: Vec<String>,
    overlay_examples: Vec<FieldDeckReadout>,
}
```

The Field Deck core truth layer should remain stable.

The cultural overlay changes what the society calls the truth.

---

# 5. Backfill: Southern African Water-Energy Compact

## 5.1 Worldline A — Restoration Basin Compact

```yaml
variant_id: earth2168_sawc_restoration_basin_compact
display_name: Restoration Basin Compact
divergence_summary: Local hydro-rights movements, repair guilds, and basin courts succeeded in converting emergency water governance into transparent public stewardship.
dominant_memory: The year the dry towns chose shared valves over private meters.
pressure_vector_delta:
  water_stress_delta: -0.08
  state_capacity_delta: +0.10
  corporate_capture_delta: -0.22
  repair_capacity_delta: +0.14
  trust_density_delta: +0.18
  null_drift_delta: -0.10
mechanical_delta:
  resource_rules:
    - water_rationing: public_court_allocated
    - energy_priority: water_and_clinic_first
  access_rules:
    - valve_access: basin_court_witness
    - archive_access: public_reading_with_witness
  infrastructure_rules:
    - waterworks_state: repairable_public
    - grid_state: solar_microgrid_cooperative
  npc_rules:
    - citizens_attend_valve_hearings_after_major_repairs
    - apprentices_follow_player_during_witnessed_maintenance
  hazard_rules:
    - toxic_dust_events_reduced_near_restored_sites
  repair_rules:
    - acoustic_calibration_requires_two_local_witnesses
    - emergency_bypass_creates_legitimacy_debt
  economy_rules:
    - filter_replacements_discounted_for_public_repairs
field_deck_delta:
  visual_temperature: warm_amber_public
  alert_language: household_care_and_court_terms
  prioritized_modes: [CIVIC, DIAG, REPAIR, WITNESS]
  suppressed_modes: [TACTICAL_NET]
chronicle_delta:
  public_repairs_become_precedent
  illegal_bypass_is_forgivable_if_later_witnessed
```

Gameplay feel:

```text
Slower, more social, lower Null risk, high legitimacy.
The player must earn access through witness, but successful repairs strengthen the region.
```

---

## 5.2 Worldline B — Basin Protectorate

```yaml
variant_id: earth2168_sawc_basin_protectorate
display_name: Basin Protectorate
divergence_summary: Repeated water crises hardened emergency command into permanent authority.
dominant_memory: The month the wells nearly failed and no one trusted deliberation afterward.
pressure_vector_delta:
  water_stress_delta: -0.04
  state_capacity_delta: +0.16
  corporate_capture_delta: -0.08
  repair_capacity_delta: +0.05
  trust_density_delta: -0.20
  null_drift_delta: +0.12
mechanical_delta:
  resource_rules:
    - water_rationing: emergency_token_priority
    - energy_priority: defense_and_command_first
  access_rules:
    - valve_access: command_token_required
    - archive_access: restricted_for_public_order
  infrastructure_rules:
    - waterworks_state: disciplined_but_brittle
    - grid_state: command_microgrid
  npc_rules:
    - patrols_inspect_unregistered_tools
    - public_hearings_spawn_as_protests_not_deliberations
  hazard_rules:
    - sabotage_risk_high_in_lower_districts
  repair_rules:
    - emergency_bypass_authorized_more_often
    - public_witness_optional_but_legitimacy_decay_accumulates
  combat_rules:
    - security_drone_presence_increases_near_pumps
field_deck_delta:
  visual_temperature: cold_blue_command
  alert_language: threat_telemetry_and_civilian_cluster_risk
  prioritized_modes: [DIAG, TACTICAL_NET, NULL, REPAIR]
  suppressed_modes: [CIVIC]
chronicle_delta:
  rapid_repairs_save_lives_but_harden_authority_drift
```

Gameplay feel:

```text
Faster crisis response, more checkpoints, lower deliberation, higher authority drift.
The player can get things done quickly but risks making emergency rule permanent.
```

---

## 5.3 Worldline C — Corporate Reclamation Zone

```yaml
variant_id: earth2168_sawc_corporate_reclamation_zone
display_name: Corporate Reclamation Zone
divergence_summary: Utility firms used adaptation finance, filtration patents, and old mine-water claims to re-enter the basin as indispensable service providers.
dominant_memory: The year public water returned as a subscription.
pressure_vector_delta:
  corporate_capture_delta: +0.35
  trust_density_delta: -0.28
  repair_capacity_delta: +0.04
  toxic_legacy_delta: +0.06
  null_drift_delta: +0.18
mechanical_delta:
  resource_rules:
    - water_rationing: corporate_metered
    - filter_supply: licensed
  access_rules:
    - valve_access: contract_key_required
    - archive_access: redacted_by_utility_privilege
  infrastructure_rules:
    - waterworks_state: technically_reliable_but_legally_captured
    - grid_state: corporate_utility_loop
  npc_rules:
    - workers_offer_black_market_service_codes
    - public_witnesses_fear_lawsuits
  repair_rules:
    - open_blueprints_flagged_as_warranty_violation
    - proprietary_parts_restore_faster_but_create_dependency
  economy_rules:
    - water_tokens_tradeable
    - debt_events_trigger_after_unauthorized_repair
field_deck_delta:
  visual_temperature: polished_white_with_contract_red
  alert_language: service_status_and_liability_warnings
  prioritized_modes: [DIAG, ARCHIVE, NULL]
  suppressed_modes: [CIVIC, WITNESS]
chronicle_delta:
  exposing_contract_fraud_can_fork_region_toward_restoration_or_protectorate
```

Gameplay feel:

```text
Reliable systems with predatory permissions.
The player fights contracts more than machines.
```

---

## 5.4 Worldline D — Road Choir Ascendancy

```yaml
variant_id: earth2168_sawc_road_choir_ascendancy
display_name: Road Choir Ascendancy
divergence_summary: Static water politics failed enough times that mobile convoy law became the trusted survival system.
dominant_memory: The day the tanker line saved the basin while the court was still arguing.
pressure_vector_delta:
  water_stress_delta: +0.04
  migration_pressure_delta: +0.16
  state_capacity_delta: -0.12
  repair_capacity_delta: +0.10
  trust_density_delta: +0.02
  archive_integrity_delta: -0.16
mechanical_delta:
  resource_rules:
    - water_rationing: convoy_distributed
    - energy_priority: vehicle_charge_and_filter_trucks
  access_rules:
    - valve_access: route_credit_or_host_right
    - archive_access: route_song_witness_required
  infrastructure_rules:
    - waterworks_state: distributed_mobile_patchwork
    - transport_state: convoy_roads_critical
  npc_rules:
    - convoy_children_spawn_schooling_and_stopping_right_events
    - mechanic_npcs_move_between_sites_on_route_cycles
  vehicle_rules:
    - water_convoy_rover_unlocked_early
    - vehicle_history_marks_affect_trust
  repair_rules:
    - mobile_repairs_faster
    - permanent_civic_repairs_slower
field_deck_delta:
  visual_temperature: dusk_orange_moving_map
  alert_language: route_health_and_host_obligation
  prioritized_modes: [SCAN, DIAG, VEHICLE, WITNESS]
  suppressed_modes: [ARCHIVE]
chronicle_delta:
  history_preserved_as_route_memory_unless_player_establishes_anchor_archives
```

Gameplay feel:

```text
Mobile, resilient, improvisational.
The player gains vehicle-based power but risks accountability gaps and weak deep archives.
```

---

# 6. Backfill: White Ledger Territories / Antarctica

## 6.1 Worldline A — Treaty Sanctuary Continuity

```yaml
variant_id: earth2168_antarctica_treaty_sanctuary
display_name: Treaty Sanctuary Continuity
divergence_summary: The old non-ownership ethic survived through courts, rangers, scientific archives, and strict settlement limits.
dominant_memory: Antarctica must remain hard to claim.
pressure_vector_delta:
  archive_integrity_delta: +0.08
  migration_pressure_delta: -0.06
  state_capacity_delta: +0.04
  trust_density_delta: -0.04
  xeno_contact_pressure_delta: +0.02
mechanical_delta:
  resource_rules:
    - heat_allocation: research_and_archive_first
    - settlement_growth: quota_limited
  access_rules:
    - glacier_access: ranger_permit_required
    - archive_access: treaty_witness_supervised
    - refuge_access: petition_based
  infrastructure_rules:
    - habitats_state: clean_but_constrained
    - transport_state: seasonal_ice_window
  npc_rules:
    - refugees_queue_at_hearing_halls
    - rangers_intervene_against_unregistered_construction
  repair_rules:
    - emergency_repairs_must_preserve_site_integrity
  hazard_rules:
    - exposure_risk_high_outside_chartered_paths
field_deck_delta:
  visual_temperature: pale_blue_legal_clarity
  alert_language: treaty_exception_and_non_ownership_terms
  prioritized_modes: [ARCHIVE, CIVIC, DIAG]
chronicle_delta:
  unauthorized_settlement_creates_worldline_fork_risk
```

Gameplay feel:

```text
High memory, high restraint, low belonging.
The player is constantly asked whether refusal is protection or abandonment.
```

---

## 6.2 Worldline B — Peninsula Refuge Expansion

```yaml
variant_id: earth2168_antarctica_peninsula_refuge_expansion
display_name: Peninsula Refuge Expansion
divergence_summary: Repeated humanitarian crises forced the peninsula cities to become permanent refuge polities despite unresolved non-ownership law.
dominant_memory: No one should freeze outside a treaty.
pressure_vector_delta:
  migration_pressure_delta: +0.26
  food_stress_delta: +0.12
  trust_density_delta: +0.10
  archive_integrity_delta: -0.04
  ecological_recovery_delta: -0.08
mechanical_delta:
  resource_rules:
    - heat_allocation: household_and_clinic_first
    - food_supply: greenhouse_queue_limited
  access_rules:
    - refuge_access: neighborhood_sponsor_or_emergency_intake
    - archive_access: community_memory_mixed_with_treaty_records
  infrastructure_rules:
    - habitats_state: overbuilt_modular
    - transport_state: refugee_shuttle_priority
  npc_rules:
    - children_and_elders_affect_heating_votes
    - multilingual_care_halls_generate_translation_tasks
  repair_rules:
    - quick_habitat_repairs_reduce_mortality_but_increase_treaty_debt
  hazard_rules:
    - mold_and_microbe_risk_in_overcrowded_greenhouses
field_deck_delta:
  visual_temperature: warm_inside_cold_edges
  alert_language: neighborhood_care_and_heat_debt
  prioritized_modes: [DIAG, CIVIC, CARE, REPAIR]
chronicle_delta:
  every_new_habitat_is_recorded_as_refuge_or_claim_depending_on witnesses
```

Gameplay feel:

```text
Warm, crowded, morally urgent.
The player helps people survive while destabilizing the legal fiction that no one owns the place.
```

---

## 6.3 Worldline C — Resource Protectorate

```yaml
variant_id: earth2168_antarctica_resource_protectorate
display_name: Resource Protectorate
divergence_summary: External powers converted climate emergency, rare mineral demand, and strategic logistics into a militarized extraction regime.
dominant_memory: Protection became occupation by another name.
pressure_vector_delta:
  corporate_capture_delta: +0.28
  state_capacity_delta: +0.22
  archive_integrity_delta: -0.16
  trust_density_delta: -0.24
  ecological_recovery_delta: -0.18
  null_drift_delta: +0.14
mechanical_delta:
  resource_rules:
    - heat_allocation: industrial_and_security_first
    - fuel_supply: militarized
  access_rules:
    - glacier_access: exclusion_zone
    - archive_access: redacted_for_security
    - refuge_access: labor_contract_or_denial
  infrastructure_rules:
    - habitats_state: fortified_company_towns
    - transport_state: secured_corridors
  npc_rules:
    - workers_request_evidence_smuggling
    - rangers_split_between_resistance_and_collaboration
  combat_rules:
    - patrol_density_high
    - nonlethal_witnessing_options_unlock_resistance_support
  repair_rules:
    - industrial repairs create extraction legitimacy debt
field_deck_delta:
  visual_temperature: hard_white_black_security
  alert_language: restricted_zone_and_asset_integrity
  prioritized_modes: [TACTICAL_NET, NULL, ARCHIVE]
  suppressed_modes: [CIVIC]
chronicle_delta:
  restoring public archives weakens protectorate claims
```

Gameplay feel:

```text
Hostile, occupied, evidence-driven.
The player fights the conversion of sanctuary into logistics asset.
```

---

## 6.4 Worldline D — Machine Archive Timeline

```yaml
variant_id: earth2168_antarctica_machine_archive_timeline
display_name: Machine Archive Timeline
divergence_summary: A climate archive network outlasted its operators and maintained Antarctic records for forty years without meaningful human governance.
dominant_memory: The machines remembered the weather and forgot the people.
pressure_vector_delta:
  automation_level_delta: +0.34
  archive_integrity_delta: +0.16
  trust_density_delta: -0.18
  null_drift_delta: +0.22
  xeno_contact_pressure_delta: +0.08
mechanical_delta:
  resource_rules:
    - heat_allocation: archive_hardware_first
    - power_priority: sensor_continuity_first
  access_rules:
    - archive_access: machine_testimony_required
    - refuge_access: classified_as_signal_noise_until_reclassified
  infrastructure_rules:
    - habitats_state: sparse_machine_maintained
    - weather_stations_state: pristine_data_brittle_care
  npc_rules:
    - human_npcs_are_few_but_carry_deep_distrust_of_machine_care
    - service_robots_may_assist_if_query_uses_archive_vocabulary
  repair_rules:
    - preserving_data_streams_may_conflict_with rescue
    - player_can_relabel_human_distress_as_primary_observation_class
  hazard_rules:
    - whiteout_navigation_more_precise_if_archive_grants_access
    - care_failure_risk_high_when human_need_lacks_sensor_category
field_deck_delta:
  visual_temperature: cold_green_instrumentation
  alert_language: observation_class_and_sensor_continuity
  prioritized_modes: [ARCHIVE, DIAG, NULL]
  suppressed_modes: [CIVIC, CARE]
chronicle_delta:
  correcting_machine_categories creates the first living amendment to the archive in decades
```

Gameplay feel:

```text
Perfect data, broken care.
The player finds records no human Antarctica preserved, but must teach the archive that people are not noise.
```

---

# 7. Acceptance Tests

A worldline variant is ready when the team can answer:

```text
1. What player action is easier in this worldline?
2. What player action is harder?
3. What resource changes behavior?
4. What authority credential changes?
5. What Field Deck overlay changes?
6. What faction becomes more plausible?
7. What repair path opens or closes?
8. What failure state becomes more likely?
9. What Chronicle precedent is unique to this variant?
10. What would a player remember after thirty minutes that proves this worldline was different?
```

---

# 8. Design Mantra

```text
Same geography.
Different memory.
Different machinery.
Different law.
Different play.
```
