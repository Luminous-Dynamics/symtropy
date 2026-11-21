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
- exact M1 subgoal ID, active action ID, monotonic attempt number and expected M1 predicate ID;
- operational design domain and mode revisions;
- authority identity, epoch, scope and revocation state;
- monotonic clock and boot epoch;
- navigation profile, service level, frame, freshness and sequence;
- source-acquisition time that is explicitly qualified into the live comparison clock.

Navigation freshness is **not** computed by comparing an arbitrary provider timestamp or local receipt tick with the platform clock. The upstream Mycelix time/correlation owner must supply a qualified time mapping into the exact live clock/boot epoch. Cross-clock mappings require a correlation commitment. Unknown, unqualified or missing source time fails closed; receipt time remains a distinct fact and cannot silently stand in for source-acquisition time.

The caller must re-evaluate the ticket against a newly acquired live context immediately before dispatch. A ticket is rejected at its expiry boundary (now >= expires_at), on boot/clock discontinuity, or when any bound configuration, ODD, mode, authority or mission generation changes.

The contract returns **applicability**, not permission to actuate. A domain-specific, independently qualified runtime-assurance layer and local interlocks remain mandatory.

## Separate physical effect from transport

A transport acknowledgement is not physical completion. The physical-outcome validator requires a receipt bound to:

- an externally computed ticket commitment and caller-computed canonical outcome content digest;
- exact M1 mission/subgoal/action/attempt/predicate identity inherited from the bound ticket;
- exact platform configuration and expected effect predicate;
- independent observer role and source identity distinct from the executing controller;
- monotonic source sequence and same clock/boot epoch;
- an explicit freshness-policy clock/boot identity matching the ticket and outcome receipt;
- observation frame, calibration and freshness;
- a declared outcome (Satisfied, NoEffect, PartialEffect or Unknown).

Observer independence is now a ticket-bound policy commitment. The outcome policy declares the executor's failure-domain path and the exact expected observer dependency path for the selected source. The receipt's observer path must exactly equal that policy path; it must not omit or add dependencies. Empty/duplicate/mismatched paths, policy-version mismatch, or any shared domain between observer and executor cause rejection. Different source IDs or an `IndependentSensor` label alone are not enough. The policy digest and dependency inventory must be authenticated by upstream platform/trust owners; this crate checks structural consistency and set separation but cannot prove that the inventory is complete or truthful.

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

The action ticket now carries the exact M1 subgoal ID, action ID, attempt and expected predicate ID, and the live context must match the active M1 ticket at dispatch. The observer receipt also carries its caller-computed canonical content digest so it can be projected into M1's receipt envelope without substituting the ticket commitment for the observation digest. A future adapter must not cast `observed_at_ticks` to M1's `observed_at_ms`: it must obtain a qualified unit/basis conversion from Mycelix Position #435, and preserve unknown/unqualified source time rather than fabricating a timestamp.

## Benchmark protocol artefacts

`BENCHMARK_PROTOCOL_v0.1.md` defines the L0–L5 CDAAB assurance ladder, matched baselines, non-compensatory metrics, statistical rules, version-pinned external comparators and claim ceiling.

The `protocol/` directory provides machine-readable v0.1 schemas:

- `scenario_manifest.schema.json`: controller-visible scenario inputs and budgets only. It explicitly excludes evaluator truth, hidden fault identities and expected outcomes.
- `evaluation_plan.schema.json`: candidate/baseline build identities, frozen manifest/evaluator digests, sampling plan, cluster dimensions, metrics and mandatory early-stop rules.
- `run_receipt.schema.json`: exact source/artifact/runtime/evaluator/scenario digests, per-episode event/evaluator receipt digests, raw numerator/denominator counts, exclusions and run status.

The evaluator's private truth set must be held separately from the controller-visible manifest. A JSON Schema syntax pass is not a semantic benchmark validation; the independent evaluator must additionally enforce numerator ≤ denominator, nonzero denominators for reported rates, valid cluster analysis, and that an incomplete/aborted/invalidated run cannot be labelled a pass. No hidden evaluator labels or private test answers are checked into the public controller-facing corpus.

## Tests

The unit-test corpus contains positive controls and table-driven negative controls for:

- valid admission;
- configuration and mission-generation mismatch;
- ODD/mode revision changes;
- authority revocation, epoch and scope mismatch;
- expiry and clock/boot discontinuity;
- navigation profile, integrity, frame, freshness and sequence downgrade;
- active M1 subgoal, action, attempt and expected-predicate mismatch;
- unknown/unqualified/missing navigation source time, mismatched comparison clock and absent cross-clock correlation;
- acknowledgement versus independent physical effect;
- self-report and executor-as-observer rejection;
- missing/duplicate/mismatched observer dependency paths, ticket-bound policy revision mismatch, and common-cause overlap with executor dependencies;
- wrong ticket, configuration, predicate, calibration and frame;
- stale, future-dated, low-confidence and out-of-order outcome evidence;
- outcome freshness evaluated under a mismatched clock or boot epoch;
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
