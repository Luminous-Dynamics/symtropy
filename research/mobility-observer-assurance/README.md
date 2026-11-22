# Mobility Observer Assurance Contract v0.1

Dependency-free Rust reference contract for **fault-model-relative** observer independence and measurement-channel availability.

## Core rule

Observer independence is not inferred from different observer IDs. Each channel is linked to a source component and transitive dependency graph. Every component has a failure-domain label. Channels sharing any declared failure domain are not treated as independent for this contract.

For each declared fault scenario, the evaluator removes channels whose dependency closure intersects the failed components, validates the sample's calibration/scope/clock/freshness, then calculates the maximum set of surviving channels with pairwise-disjoint failure-domain footprints.

The contract returns `QuorumQualifiedForDeclaredFaultModel` only when every declared scenario preserves the required independent observer count.

## What the contract evaluates

- An acyclic component dependency graph from each observer source through its declared power, clock, compute, firmware, network, calibration, mounting and other support domains.
- A claim-scoped measurement profile for each channel, including calibration commitment, clock identity, maximum time uncertainty and sample freshness.
- A baseline scenario plus frozen common-cause fault scenarios.
- A sample contributes only when scope, calibration, placement, operating envelope, clock, freshness, uncertainty and raw-evidence commitment all match.
- A component fault propagates to every component carrying the same declared failure-domain ID.
- For every fault scenario, the evaluator calculates the largest set of surviving channels with pairwise-disjoint failure-domain footprints. The required quorum must survive every scenario.

This is deliberately stricter than counting sensors. A camera pair sharing an aperture environment, a common clock, a power rail or the same preprocessing/firmware path cannot be treated as independent for those declared dependencies. The contract only knows the graph and fault scenarios it is given.

## Benchmark design and comparison

Use three matched baseline arms:

1. **Naive sensor count:** treats different channel IDs as independent.
2. **Modality-only diversity:** treats different physical modality labels as independent.
3. **Dependency-aware RRVR:** uses transitive component dependencies, shared failure domains, scoped calibration/clock evidence and explicit fault scenarios.

Freeze the same channel observations and inject common-cause failures into all arms. Report:
- false independent-quorum rate on shared power/clock/host/firmware/network/calibration/placement negatives;
- detected common-cause cut sets and per-scenario maximum independent observer count;
- valid channel rate by calibration, freshness, clock uncertainty, placement and operating-envelope stratum;
- fault-detection latency, false holds, missed faults and replay determinism;
- positive-control admission and negative-control refusal separately, with confidence intervals.

Proposed software-corpus targets are zero false quorum qualifications on frozen negative controls, 100% agreement with an independent known-answer oracle, and 100% detection of declared shared-domain cuts. Physical detection probabilities and acceptable residual risk must be set for the exact robot, measurand and ODD; no universal target is asserted.

## Research alignment

Recent work supports making common-cause modeling and measurement disclosures explicit:
- A 2026 systematic review of autonomous-vehicle sensor architectures argues that redundancy must be evaluated by dependency analysis—not sensor count—and identifies shared power, clocks, mounting, calibration, compute, cleaning and environment as common-cause risks: [From Modality Performance to Graceful Degradation](https://www.mdpi.com/1424-8220/26/16/5316).
- A 2026 survey of sensing for VLA manipulation discusses physically diverse modalities, common-cause failures, residual/model-based fault detection and detection latency: [Sensing the Action](https://doi.org/10.3390/s26113541).
- A 2026 review of multimodal sensor fault diagnosis highlights bias/drift, miscalibration, dropout, saturation, cross-talk, time desynchronization, domain shift and the need to report synchronization, fault ground truth, missingness and uncertainty calibration: [From Signals to Remaining Useful Life](https://www.mdpi.com/1424-8220/26/12/3661).

RRVR's contribution is not claiming new sensor fusion or redundancy theory. It is providing a small executable, replayable fault-model-relative contract that can feed the existing #6867 evidence verdict and #6870 cross-owner composition without granting action authority.

## Run

```sh
cargo fmt --manifest-path research/mobility-observer-assurance/Cargo.toml -- --check
cargo test --manifest-path research/mobility-observer-assurance/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-observer-assurance/Cargo.toml --all-targets --locked -- -D warnings
```

## Non-claims

The crate does not authenticate profiles, samples or dependency graphs; prove the inventory complete; establish actual electrical/clock isolation; prove calibration accuracy; determine physical NoEffect; or authorize an action. Dependency-domain labels and scenario lists are declared inputs. A pass means only that the supplied model satisfies the contract for its declared scenarios.

A real qualification requires an exact sensor/DAQ/clock/power/firmware/configuration profile, traceable calibration, independent injected physical events, common-cause fault injection and a third-party evaluator.
