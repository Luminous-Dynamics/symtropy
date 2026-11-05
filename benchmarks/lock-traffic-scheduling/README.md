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
- FCFS, projected least-slack, participant least-served, and bounded directional-batch policies.
- The participant least-served policy rotates toward the participant with the lowest
  accumulated delivered service units (within one declared comparable service-unit
  label). It is an auditable fairness-oriented baseline, not a max-min guarantee
  across heterogeneous goods, an equity proof, or a reproduction of published FSR.
- Least-slack ordering includes the request's worst-case arrival bound, authorized
  latest-start time, maximum-wait limit, completion deadline, service duration,
  direction-switch delay, lock downtime and planning window.
- Every arrival interval binds source/model revision, calibration/validation receipt,
  the exact lower/upper interval bounds, operating regime, observation time, expiry,
  and an explicit evidence state. A bound mismatch invalidates the interval.
  Future-dated, stale, missing, disputed or revoked arrival evidence is rejected or
  routed to review; intervals are conservatively scheduled at their latest arrival.
  The crate checks identifier presence, state, observation time and expiry; it does
  not authenticate the receipt or independently prove calibration/coverage. Those
  claims remain with the source owner and its verifier.
- Passage requests are admitted only when the upstream PassageEvaluation is
  EligibleForSyntheticPassage; unknown, inconsistent, blocked, or unsupported
  inputs are never relaxed into schedulable work.
- Request windows, arrival uncertainty, maximum wait, deadlines, planned lock
  downtime, direction-change time, water budget, and pump-energy budget are explicit.
- Retains exactly one reason-coded outcome for each request, including accepted
  passages and every non-admission.
- Reports independent metrics, not one opaque score: service units, total and
  maximum wait, nearest-rank p50/p90/p95/p99 wait distributions, inversions,
  direction changes, service ticks, direction-change occupancy, total lock-occupied time, comparable arrival pairs and inversion counts, water and pump energy. Wait distributions are
  also reported separately by participant and service group.
- Includes a separate exhaustive test-only oracle that reimplements tiny
  scheduling feasibility, enumerates request permutations (at most eight
  requests), and constructs a non-dominated objective frontier without using
  production scheduler or metric helpers.
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
and live AIS/LPMS integration remain out of scope. The tiny oracle is not a
published-algorithm reproduction, and it is not a proof of global optimality for
larger cases. No superiority or SOTA claim is made until matched operational
data and published baseline formulations are reproduced on a frozen held-out
corpus.

All fixtures are synthetic. A schedule recommendation is not operator approval
or a physical passage.
