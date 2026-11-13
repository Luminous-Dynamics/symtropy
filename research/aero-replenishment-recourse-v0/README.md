# Aero replenishment recourse v0

**Status:** exact finite-scenario policy enumeration for a two-stage mission replenishment decision. It is not a probabilistic risk model, aircraft dynamics model, battery model, scheduler, controller or permit.

## The new question

#4286 compares deterministic complete replenishment paths. This workstream asks whether one policy remains feasible under a finite declared set of service outcomes, while making decisions only from the observations actually available.

Three information structures are evaluated separately:
- OPEN_LOOP_ROBUST: one common stage-1 action for every scenario.
- OBSERVATION_RECOURSE: one stage-1 action per declared observation label; all scenarios sharing that label must use the same action.
- PERFECT_INFORMATION: one stage-1 action per hidden scenario ID. This is a non-deployable optimistic upper bound, never an admissible controller policy.

No probabilities, expected values or failure rates are inferred from scenario counts.

## Input contract

One TSV row per scenario/stage/action alternative. Exact header:

    case_id evidence_class deadline_slots capacity_mwh initial_battery_mwh reserve_mwh site_energy_plane_id battery_energy_plane_id start_node observation_node terminal_node scenario_id observation_label stage action_id from_node to_node is_terminal edge_kind mode_id available duration_slots battery_gain_mwh aircraft_draw_mwh site_energy_mwh asset_id installed_mass_g observation_source_receipt_id observation_source_receipt_sha256 observation_partition_sha256

All rows repeat the same case metadata. Each scenario must have an identical stage-0 action set and an identical stage-1 action ID/topology set. Availability and energy outcomes may differ by scenario. A service action unavailable in a scenario remains explicitly present with `available=false` and zero energy transfer; it is never silently removed from the uncertainty set.

Stage-0 action is selected once and shared across every scenario. Its declared observation label becomes available at the decision node. The observation equivalence partition is bound to one upstream observation/probe receipt ID and two SHA-256 digests: receipt content and partition artifact. The finite policy evaluator syntax-checks these identifiers and requires them to be identical on every input row; it does not fetch or verify the referenced bytes. Stage-1 actions are then selected under one of the three information structures above. Non-anticipativity is conditional on the upstream partition actually representing information available to the decision-maker. The three provenance columns are exact, digest-bound references; v0 syntax-checks them and requires them to be identical on all rows, but does not resolve or verify the referenced bytes.

Battery transition is caller-supplied:

    next_mWh = current_mWh + battery_gain_mWh - aircraft_draw_mWh

Reserve/capacity and deadline are hard constraints at every stage. Source energy is accumulated separately at the common site-side real-energy plane. Asset mass is charged once per unique aircraft asset ID across all possible branches of a policy (the hardware has to be carried even when only one branch occurs).

## Frozen fixture

Three synthetic scenarios:
- S1 and S2 share observation NORMAL. Contact works in both; battery exchange works only in S1.
- S3 has observation DEGRADED. Contact and battery exchange are unavailable; IPT remains available.

The solver must not choose SWAP for S1 and CONTACT for S2 under OBSERVATION_RECOURSE, because both have the same NORMAL observation. The PERFECT_INFORMATION result may show that optimistic branch, explicitly labelled non-deployable.

## Metrics

Each feasible policy reports worst-case elapsed time, source energy, aircraft draw, minimum terminal battery, unique installed receiver mass and its observation-to-action map. Feasibility is all-scenario, not average. No scalar score hides reserve or capacity failures.

## Run

    cargo test --locked --manifest-path tools/aero-replenishment-recourse/Cargo.toml
    cargo run --locked --manifest-path tools/aero-replenishment-recourse/Cargo.toml -- research/aero-replenishment-recourse-v0/synthetic-scenarios.tsv

## Claim ceiling

A pass proves exact enumeration and non-anticipativity for the declared finite synthetic scenario table. It does not prove scenario completeness, field availability, physical transfer or mission reliability. The perfect-information result is not deployable.
