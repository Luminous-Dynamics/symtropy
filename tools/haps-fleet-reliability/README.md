# HAPS fleet reliability and service availability tools

Standalone Rust tools for synthetic-first HAPS fleet reliability and service-continuity qualification.

## Static common-cause reliability

`cargo run --locked --manifest-path tools/haps-fleet-reliability/Cargo.toml --bin haps-fleet-reliability -- <case.json>`

Exact Bernoulli enumeration of independent platform states, common-cause events and shared dependencies. It is static probability arithmetic, not a semi-Markov model.

## Deterministic service schedule replay

`cargo run --locked --manifest-path tools/haps-fleet-reliability/Cargo.toml --bin service-replay -- <case.json>`

Replays declared platform state segments, maintenance windows, common-cause event windows, reserve launch/recovery/reposition events and service-capacity thresholds. It is a deterministic schedule evaluator, not a stochastic reliability process.

## Fleet rotation capacity lower bound

`cargo run --locked --manifest-path tools/haps-fleet-reliability/Cargo.toml --bin rotation-capacity -- <case.json>`

Calculates the idealized lower-bound fleet size from required simultaneous service units and the service-active / sequential-turnaround duty fraction. It assumes perfect staggering and unlimited operations resources; it does not prove a feasible zero-outage schedule.

## Qualification boundaries

All fixtures currently labelled synthetic are arithmetic or scheduling examples only. They are not provider reliability inputs, repair distributions, vehicle performance data, maintenance measurements, flight authorization, commercial service SLAs or airworthiness evidence.

The HAPS fleet research and fixture set is documented in [HAPS_FLEET_RESILIENCE_001.md](../../docs/research/aviation/HAPS_FLEET_RESILIENCE_001.md).
