# Lock traffic scheduling v0.1

Dependency-free Rust policy evaluator for Symtropy #3362, composed with the
upstream passage-eligibility result from #3347 / PR #3349.

## Run

~~~sh
cargo test --locked --manifest-path benchmarks/lock-traffic-scheduling/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/lock-traffic-scheduling/Cargo.toml --all-targets -- -D warnings
~~~

## Implemented slice

- Single-lock, single-chamber, deterministic finite scheduling.
- FCFS, least-wait-slack (aging/fairness-oriented), and bounded directional-batch policies.
- Passage requests are admitted only when the upstream PassageEvaluation is
  EligibleForSyntheticPassage; unknown, inconsistent, blocked, or unsupported
  inputs are never relaxed into schedulable work.
- Request windows, arrival uncertainty, maximum wait, deadlines, planned lock
  downtime, direction-change time, water budget, and pump-energy budget are explicit.
- Emits recommendations and a non-collapsed objective/diagnostic vector:
  waits, maximum wait, inversions, service, lock occupancy, water and pump energy.
- Retains an outcome for every request, including requests not scheduled.
- Canonical output ordering is independent of request input order.

## Important limits

This is not a lock-control, gate-control, pump-control, vessel-traffic-authority,
or navigation system. It does not calculate hydraulics, derive water availability,
predict arrivals, prove the authenticity of an upstream receipt, or authorize a
passage. PassageEvaluation is consumed as an upstream engineering input; its
provenance and currentness remain the responsibility of #3347 and its evidence
admission layer. The scheduler uses declared synthetic resource envelopes only.

V0 is a single-chamber reference scheduler. Multi-chamber placement, serial-lock
routing, hydraulic transients, environmental-flow derivation, vessel dynamics,
and live AIS/LPMS integration remain out of scope. No superiority or SOTA claim
is made until matched operational data and published baseline formulations are
reproduced on a frozen held-out corpus.

All fixtures are synthetic. A schedule recommendation is not operator approval
or a physical passage.
