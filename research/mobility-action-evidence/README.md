# Mobility action evidence contract v0.1

**Tracking:** [MOBILITY-ACTION-EVIDENCE-001 #6503](https://github.com/Luminous-Dynamics/symtropy/issues/6503)  
**Parent reference:** [MISSION-SUPERVISOR-001 #6498](https://github.com/Luminous-Dynamics/symtropy/issues/6498) / [M1 reference PR #6499](https://github.com/Luminous-Dynamics/symtropy/pull/6499)  
**Status:** isolated Rust contract and deterministic unit tests; software research only.

This crate defines the narrow M2 hand-off between mission-level action proposals and a live mobility-platform context. It checks whether a previously proposed action is still applicable at commit time, and separately validates an independently observed physical outcome.

It does **not** issue actuator commands, authenticate identities, calculate cryptographic digests, estimate position, determine source independence, implement a safety controller, or grant operational authority.

## Paradigm: configuration-bound, expiring action commitments

A mission action is not a timeless command. Its applicability is conditional on exact platform identity/configuration, mission generation and active attempt, ODD/mode revisions, authority epoch/scope, monotonic clock/boot epoch, navigation profile/service/frame/source/freshness, and explicitly qualified source time.

Navigation freshness is **not** computed by comparing an arbitrary provider timestamp or local receipt tick with the platform clock. The upstream Mycelix time/correlation owner must supply a qualified mapping into the exact live clock/boot epoch. Unknown, unqualified or missing source time fails closed; receipt time remains distinct.

The caller must re-evaluate the ticket against a newly acquired live context immediately before dispatch. A ticket is rejected at its exclusive expiry boundary (now greater than or equal to expires_at), on boot/clock discontinuity, or when any bound configuration, ODD, mode, authority or mission generation changes.

The contract returns **applicability**, not permission to actuate. A domain-specific, independently qualified runtime-assurance layer and local interlocks remain mandatory.

## Separate physical effect from transport

A transport acknowledgement is not physical completion. The physical-outcome validator requires evidence bound to the exact ticket/action attempt, platform configuration, expected effect predicate, observer identity and declared failure domains, source sequence/time, frame, calibration and freshness.

Observer independence is a ticket-bound policy commitment. M2 checks exact declared observer dependency paths and disjointness from executor failure domains, but cannot prove that the inventory is complete or truthful. Upstream platform/trust owners must authenticate those claims.

Outcomes remain distinct:
- **Accepted:** matching evidence establishes the expected effect.
- **RecoveryRequired:** an independent observer establishes no effect.
- **ReconciliationRequired:** partial/unknown effects remain unresolved.

No branch automatically retries an action. Recovery requires the M1 supervisor's separately approved recovery path.

## Ownership and integration

This crate references, but does not reimplement, the existing owners:
- Mycelix Mobility Commons #3709: platform/configuration lifecycle.
- Mycelix Position #416/#435/#458 and navigation claim v2 PR #6412: frame, qualified time and navigation service.
- Mycelix #5899/#5904: provenance and common-mode dependencies.
- Symthaea #3779/#10012/#10747/#11185: domain runtime assurance and authority boundaries.
- Symtropy #3893: maritime ODD/mode applicability.
- Symtropy #6498/#6499: mission/subgoal state machine.

The M1 mission supervisor remains the owner of subgoal transitions, ambiguity holds, reconciliation and approved recovery. This crate does not duplicate its mission state machine.

## Benchmark protocol artefacts

BENCHMARK_PROTOCOL_v0.1.md defines the L0–L5 CDAAB assurance ladder, matched baselines, non-compensatory metrics, statistical rules, version-pinned external comparators and claim ceiling.

CDAAB_RESEARCH_AND_TARGETS_v0.2.md compares external baselines and research targets. It does not claim Symtropy has run robot episodes or achieved any proposed target.

CDAAB_RESEARCH_AND_TARGETS_v0.3.md adds the authority-linearizability benchmark definition, updated external comparisons, matched assurance ablations and explicit research targets.

The protocol directory includes:
- action_ticket.schema.json: action ticket and expected observer dependency path.
- scenario_manifest.schema.json: controller-visible scenario inputs and budgets, without evaluator truth.
- evaluation_plan.schema.json: preregistered builds, evaluator, sampling plan, cluster dimensions and metrics.
- run_receipt.schema.json: source/artifact/runtime/evaluator/scenario digests and per-episode receipts.
- authority_linearization_cases_v0_1.json and authority_linearization_cases.schema.json: 23 public positive/negative race fixtures for the native mutation boundary.

The evaluator's private truth set must be held separately from the controller-visible manifest. JSON Schema syntax validation is not complete semantic validation; the independent evaluator must also enforce complete planned-case accounting, numerator no greater than denominator, nonzero denominators, cluster-aware inference, and that incomplete/aborted/invalidated runs cannot be labelled a pass.

## Active assurance: observation is a control action

src/active_assurance.rs implements a deterministic observation selector. It requires the candidate to target the same predicate, carries calibration/risk-model/time-correlation/independence commitments, rejects observer/executor common-cause overlap, and enforces residual-risk, latency, energy and cost ceilings as separate hard constraints. It returns the non-dominated Pareto frontier rather than a weighted aggregate. If no candidate satisfies all gates, the result is Hold.

The selector accepts caller-supplied conservative risk forecasts; it does not compute, calibrate or authenticate them, execute a sensor query, or change mission state. A domain owner must validate the forecast model and source qualification before treating a returned frontier as decision evidence.

## New work: authority-linearized mutation

The next integration issue is [MOBILITY-MUTATION-BOUNDARY-001 #6669](https://github.com/Luminous-Dynamics/symtropy/issues/6669). It addresses the remaining time-of-check/time-of-use gap between M2's pre-dispatch applicability check and a native controller's eventual application of a queued command.

The proposed shift is from **validated-before-queue** to **authority-linearized mutation**: the native application boundary must atomically compare the full current M1/M2/platform binding, check the controller generation and qualified monotonic lease, consume the exact action attempt once, and invoke the native sink. A wrapper mutex is not enough if a process, driver or second queue can bypass it.

The design, dependency-free reference boundary, history checker and formal finite-state model are under:
- AUTHORITY_MUTATION_BOUNDARY_V0.1.md
- the sibling mobility-mutation-boundary directory
- HISTORY_CHECKER_AND_TLA_MODEL_V0.1.md: operation-history linearizability checking, explicit PASS/FAIL/INCOMPLETE semantics, and a bounded TLA+/TLC protocol model.

The current reference now also models the M1 single-unresolved-mutation rule. Every native application occupies the pending slot until an independently qualified outcome is reconciled. Unknown retains the hold; NoEffect and PartialEffect require a typed recovery-permit reference bound to the failed attempt, authority epoch and approved plan digest. Recovery-permit IDs are consumed once.

The reference model validates the shape and scope of the permit reference but does not authenticate it. M1 remains the authoritative permit issuer and allowlist owner.
- AUTHORITY_MUTATION_BOUNDARY_V0.1.md
- the sibling mobility-mutation-boundary directory.

The reference model is intentionally an in-process software conformance artifact. It does not authenticate binding commitments, fence external processes or hardware, survive restart, establish physical effect or provide safety-rated control.

## Validation commands

```sh
cargo fmt --manifest-path research/mobility-action-evidence/Cargo.toml -- --check
cargo test --manifest-path research/mobility-action-evidence/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-action-evidence/Cargo.toml --all-targets --locked -- -D warnings
```

## Claim ceiling

Passing M2 tests demonstrates only deterministic behavior of this software contract for declared inputs. It does not demonstrate sensor truth, cryptographic authentication, actual source independence, physical effect accuracy, real-time deadlines, vehicle safety, simulator fidelity, regulatory compliance or authorization to operate a road vehicle, vessel or aircraft.
