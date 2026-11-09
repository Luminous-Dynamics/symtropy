# Aero energy-information dual control v0

Status: exact finite-scenario composition of a qualified probe choice with a downstream non-anticipative energy-service choice. It is not a pose estimator, probe selector, aircraft controller, battery model, trajectory planner or safety permit.

## Purpose

The central question is whether a probe's information improves the downstream mission-energy decision enough to pay for its energy, time and service disturbance.

Existing owners remain authoritative:
- #4214/#4241 select and propagate qualified probe outcomes/actions;
- #4115 owns the observation and pose semantics;
- #4286 owns deterministic replenishment paths;
- #4317 owns non-anticipative service recourse;
- #3472 owns source-to-battery metrology;
- #4157 owns benchmark receipts and joins.

This crate composes caller-supplied finite probe and service options. It does not regenerate the probe outcome model.

## Information structures

- NO_PROBE: the initial observation class remains unresolved; service must be common across all scenarios in it.
- QUALIFIED_PROBE_RECOURSE: a digest-bound probe produces the declared observation labels; scenarios with the same label must choose the same downstream service.
- PERFECT_INFORMATION_NON_DEPLOYABLE: hidden scenario identity chooses a service action. This is an optimistic upper bound only.

No scenario probabilities, expected utility, CVaR or conditional correctness are computed.

## Input

The exact 34-column TSV header appears in synthetic-probe-service-scenarios.tsv.

Every row binds case metadata, a scenario, one probe-action profile and one service action. For each probe action, every scenario must have the same receipt/partition identity and the same declared action costs/quality bounds; only the resulting observation label and scenario-specific service outcome may differ.

probe_receipt_id, probe_receipt_sha256 and observation_partition_sha256 are required for a real probe. NO_PROBE retains the upstream initial-observation partition. The tool syntax-checks these IDs/digests and exact row consistency; it does not fetch the referenced bytes or verify the physical observation model.

Probe and service each contribute:
- duration slots;
- aircraft battery-terminal energy draw;
- service battery-terminal accepted energy;
- source-side real energy at the common site meter plane;
- ripple, ZVS margin and service disturbance values.

Hard gates:
- probe ripple <= max_ripple_ppm;
- probe ZVS margin >= min_zvs_margin_ppm;
- probe/service disturbance <= max_service_disturbance_ppm;
- all selected actions available;
- every intermediate battery state remains in [reserve, capacity];
- probe + service duration <= mission deadline.

Service energy and battery-swap pack energy must be reported at the same declared source-side boundary. No bearer gets free energy.

Receiver asset mass is the union of assets needed by any downstream branch of the policy, not the mass of only the branch that happened to execute in one hidden scenario.

## Output metrics

The evaluator reports each policy separately: probe ID, observation-to-service action map, worst-case time, probe aircraft draw, total aircraft draw, source energy, minimum terminal battery and installed asset mass. It distinguishes policy classes and never collapses them into a weighted scalar.

## Run

    cargo test --locked --manifest-path tools/aero-energy-information-dual-control/Cargo.toml
    cargo run --locked --manifest-path tools/aero-energy-information-dual-control/Cargo.toml -- research/aero-energy-information-dual-control-v0/synthetic-probe-service-scenarios.tsv

## Claim ceiling

A pass proves exact finite composition and arithmetic over synthetic caller-supplied actions. It does not validate probe energy, sensor support, partition completeness, battery physics, actual service availability, physical WPT, flight safety or operational authority.
