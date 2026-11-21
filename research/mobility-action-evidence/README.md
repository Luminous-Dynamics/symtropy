# Mobility action evidence contract v0.1

**Tracking:** [MOBILITY-ACTION-EVIDENCE-001 #6503](https://github.com/Luminous-Dynamics/symtropy/issues/6503)  
**Parent reference:** [MISSION-SUPERVISOR-001 #6498](https://github.com/Luminous-Dynamics/symtropy/issues/6498), [M1 reference PR #6499](https://github.com/Luminous-Dynamics/symtropy/pull/6499)  
**Status:** isolated Rust contract and deterministic unit tests; software research only.

## Purpose

This crate defines the narrow M2 hand-off between mission-level action proposals and a live mobility-platform context. It checks whether a previously proposed action is still applicable at commit time, and separately validates an independently observed physical outcome.

It does **not** issue actuator commands, authenticate identities, calculate cryptographic digests, estimate position, determine source independence, implement a safety controller, or grant operational authority.

## Paradigm: configuration-bound, expiring action commitments

A mission action is not a timeless command. Its applicability is conditional on:

- exact platform identity and physical configuration commitment;
- mission identity and generation;
- operational design domain and mode revisions;
- authority identity, epoch, scope and revocation state;
- monotonic clock and boot epoch;
- navigation profile, service level, frame, freshness and sequence.

The caller must re-evaluate the ticket against a newly acquired live context immediately before dispatch. A ticket is rejected at its expiry boundary (now >= expires_at), on boot/clock discontinuity, or when any bound configuration, ODD, mode, authority or mission generation changes.

The contract returns **applicability**, not permission to actuate. A domain-specific, independently qualified runtime-assurance layer and local interlocks remain mandatory.

## Separate physical effect from transport

A transport acknowledgement is not physical completion. The physical-outcome validator requires a receipt bound to:

- an externally computed ticket commitment;
- exact platform configuration and expected effect predicate;
- independent observer role and source identity distinct from the executing controller;
- monotonic source sequence and same clock/boot epoch;
- observation frame, calibration and freshness;
- a declared outcome (Satisfied, NoEffect, PartialEffect or Unknown).

The result is one of:

- **Accepted:** the expected effect has independent, matching evidence;
- **RecoveryRequired:** an independent observer establishes no effect;
- **ReconciliationRequired:** partial/unknown outcomes remain unresolved.

No branch automatically retries an action. Recovery requires the M1 supervisor's separately approved recovery path.

## Ownership and integration

This crate references, but does not reimplement, the existing owners:

- Mycelix Mobility Commons #3709: platform/configuration lifecycle;
- Mycelix Position #416/#435/#458 and navigation claim v2 PR #6412: frame, qualified time and navigation service;
- Mycelix #5899/#5904: provenance and common-mode dependencies;
- Symthaea #3779/#10012/#10747/#11185: domain runtime assurance and authority boundaries;
- Symtropy #3893: maritime ODD/mode applicability;
- Symtropy #6498/#6499: mission/subgoal state machine.

The M1 mission supervisor remains the owner of subgoal transitions, ambiguity holds, reconciliation and approved recovery. This crate does not duplicate its mission state machine.

## Tests

The unit-test corpus contains positive controls and table-driven negative controls for:

- valid admission;
- configuration and mission-generation mismatch;
- ODD/mode revision changes;
- authority revocation, epoch and scope mismatch;
- expiry and clock/boot discontinuity;
- navigation profile, integrity, frame, freshness and sequence downgrade;
- acknowledgement versus independent physical effect;
- self-report and executor-as-observer rejection;
- wrong ticket, configuration, predicate, calibration and frame;
- stale, future-dated, low-confidence and out-of-order outcome evidence;
- no-effect, partial-effect and unknown-outcome disposition.

## Run

```sh
cargo fmt --manifest-path research/mobility-action-evidence/Cargo.toml -- --check
cargo test --manifest-path research/mobility-action-evidence/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-action-evidence/Cargo.toml --all-targets --locked -- -D warnings
```

The crate is deliberately isolated from the main Symtropy workspace. The targeted workflow, not a broad workspace build that excludes this crate, is the relevant Rust validation path.

## Claim ceiling

Passing these tests demonstrates only the deterministic behavior of this software contract for its declared inputs. It does not demonstrate sensor truth, cryptographic authentication, actual source independence, physical effect accuracy, real-time deadlines, vehicle safety, simulator fidelity, regulatory compliance or authorization to operate a road vehicle, vessel or aircraft.
