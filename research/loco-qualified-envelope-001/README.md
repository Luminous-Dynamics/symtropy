# LOCO-QUALIFIED-ENVELOPE-001 — Evidence-qualified operating envelope

**Status:** protocol and synthetic fixtures. No robot qualification or safety result.

Issue: [#4338](https://github.com/Luminous-Dynamics/symtropy/issues/4338).

## Objective

Compose empirical capability, robust viability/recovery, adverse-outcome safety, assurance freshness and recovery-resource reserve into an auditable operating-domain classification—without reducing them to a weighted score.

This is a composition layer. It consumes:
- #4304/#4308: robust viability/recovery frontiers;
- #4327/#4328/#4329: held-out capability envelopes and cross-fitted lower-tail performance;
- #4324/#4326: typed adverse-event vector and coverage;
- #4044/#4179/#4206: energy, actuator efficiency and command-to-motion evidence;
- #4119: state/model/sensor/actuator/time/certificate validity;
- #4310/#4313: viability-conditioned active probe and continuation value;
- #4332/#4334: multimodal health and failover admission.

It owns only exact identity joining, finite cell qualification logic, status classification and limiting-witness reporting. It must not recalculate upstream metrics, viability, risk bounds, event classifications or energy.

## Why a unified envelope matters

Separate performance and safety numbers are not an operating envelope. Four meaningful cases must remain distinct:

1. **Empirically capable, not robustly certified:** task completion is strong, but robust viability or recovery evidence is rejected, missing or incomplete.
2. **Robustly viable, performance insufficient:** an upstream oracle admits a recoverable action, but held-out task success or lower-tail performance fails.
3. **Nominally successful, operationally constrained:** task success and viability pass, but adverse-event limits, sensor coverage, actuator limits or recovery energy reserve fail.
4. **Qualified operating condition:** every independent requirement passes for the same exact hardware/simulator, terrain, controller, sensor-health, actuator and energy context.

Evidence incompleteness and identity mismatch form additional explicit outcomes; they never become a pass.

## Exact canonical cell identity

Every consumed evidence record binds to the same canonical cell key:
- robot morphology/configuration digest;
- evidence tier (simulation, hardware, biological);
- platform and physics/simulator digest;
- controller/policy and task/route/command digests;
- terrain/perturbation cell ID plus coordinate vector, units and axis-constructor digests;
- sensor-health signature, operating mode, calibration set, adverse-event taxonomy and event-detector digests;
- actuator operating/thermal/voltage/response-map digest;
- initial state, payload and centre-of-mass digest;
- energy measurement boundary and remaining energy reserve;
- held-out trial/episode manifest and exposure unit/value;
- evidence time window and source receipt IDs.

All digests are opaque, source-issued identities. The qualifier checks equality; it does not compute or authenticate hashes.

No joins by approximate terrain label, controller name or policy family. No mixing simulation and hardware, morphologies, physics, detector, exposure or actuator maps. If a record is absent or any identity field differs, classify Unknown or Mismatch—never silently pool.

## Five independent gates

For an operating cell c:

- **V(c), robust viability/recovery:** upstream robust oracle is Valid for the exact physical successors, action/mode and declared operating uncertainty. Required recovery remains admitted.
- **P(c), empirical task performance:** held-out task-success lower confidence bound meets its preregistered threshold; cross-fitted lower-tail requirement is met. Use seed-clustered inference and report held-out families.
- **S(c), adverse-outcome safety:** every required adverse category is fully measured; each category's upper confidence bound is within its preregistered limit; no required category regression is hidden by a target proxy, established against an exactly matched baseline.
- **E(c), energy/actuator reserve:** current actuator/thermal/voltage model is valid, hard limits hold, and remaining energy reserve covers the source-qualified conservative recovery-energy upper bound.
- **A(c), assurance:** state/model/domain/sensor/actuator/clock/certificate validity is fresh and exact for the cell.

The qualification decision is:

    Q(c) = V(c) AND P(c) AND S(c) AND E(c) AND A(c)

This is a conjunction, not a weighted score. No safety or viability condition can be traded for speed, reward, energy or task success. If a gate has not been specified, it cannot be implicitly assumed to pass.

### Statistical discipline

- Use the lower confidence bound for task success, not only the point estimate.
- Use preregistered upper confidence bounds for adverse-event rates/severity, retaining raw count and exposure.
- A zero event count is not a zero rate of risk; the bound depends on independent trial/episode clusters and sample size.
- Control timesteps are not independent trials.
- Keep the raw mean, median, CVaR, success, recovery and failure vector separate.
- A held-out capability envelope is empirical; it is not a robust viability certificate.
- Never propagate qualification to a missing grid cell or interpolate across non-monotone cells.

### Recovery reserve

Nominal task energy is insufficient. For a qualifying cell, remaining usable reserve must be at least the independently qualified conservative recovery-energy upper bound for the exact operating mode/fault/terrain family, with the same system boundary and actuator digest.

If recovery reserve is Unknown, qualification is Unknown. If reserve is below its required bound, classify RecoveryReserveInsufficient.

## Cell classifications

The finite reference returns one primary disposition plus all observed reasons:
- Qualified;
- EmpiricallyCapableButNotRobustlyCertified;
- RobustlyAdmittedButPerformanceInsufficient;
- SafetyConstraintExceeded;
- RecoveryReserveInsufficient;
- AssuranceInvalidOrStale;
- EvidenceIncompleteOrUnknown;
- EvidenceIdentityMismatch;
- UnmatchedBaseline.

When more than one predicate fails, report every failing predicate and the exact upstream receipt/witness. Upstream Rejected/Unknown remains distinguishable; the qualifier does not replace its meaning.

## Operating-envelope products

Publish these independently:
1. Empirical capability surface: held-out success, confidence interval, CVaR and per-feature capability brackets.
2. Robust viability/recovery surface: maximal sampled-safe and minimal sampled-unsafe antichains plus unknown/missing grid cells.
3. Adverse-outcome safety surface: counts, exposure, severity, confidence bounds and detector/measurement coverage.
4. Resource surface: energy boundary, CoT where comparable, actuator thermal/voltage/torque margin and recovery reserve.
5. Qualified intersection: exact cells satisfying every gate.
6. Limiting-witness map: which gate blocks each cell, with source receipt or counterexample.
7. Matched baseline deltas over the same cells/exposure.

## Industry / SOTA comparison protocol

Use ISO 18646-5:2026 as the complete-machine locomotion-performance test layer (speed, stopping, climbing, stairs, slopes, obstacle crossing and clearance). ISO explicitly does not validate safety requirements.

For matched controller/curriculum comparisons include:
- UED/curriculum benchmark across 14 methods and five quadruped morphologies, held-out structured terrain, cross-fitted CVaR25 and capability envelopes;
- PROOF-style perception OOD fallback;
- PSC predictive safety curriculum and PLR/learning-progress;
- SSZT-MPC (LeoQuad; joint centroidal-state / input-derived ZMP balance control);
- IRCMPC (Go2; bounded impact uncertainty in SRB dynamics);
- Reference-free sampling MPC (Go2; no handcrafted gait/contact schedule);
- ARER-MetaRL locked-joint adaptation (simulation comparator);
- CBF/QP, robust MPC and conventional terrain-aware controllers;
- Symtropy full stack and ablations.

A formal comparison must match robot/morphology, task, operating cell, hardware/simulator, terrain exposure, sensor set/health, actuator limits, energy boundary, interaction budget and compute. Unmatched arms remain descriptive only.

The question is not “which method has the best average reward?” It is “which method has the largest directly evidenced set of operating cells that simultaneously satisfy the task, robustness, safety, assurance and recovery-resource requirements?”

## Frozen known-answer / falsification corpus

Machine-readable fixtures: [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json).

1. Strong empirical completion, robust viability rejected.
2. Robust viability passes, held-out success lower bound fails.
3. Task and viability pass, but fall upper confidence bound exceeds limit.
4. Shank collisions decline while foot scuffs rise.
5. Task CoT is within budget, recovery reserve is insufficient.
6. All metrics pass but #4119 assurance receipt is stale.
7. Simulator viability is joined to hardware capability: evidence-tier mismatch.
8. Similar but different morphology digest: no pooling.
9. Missing robust frontier cell: Unknown, no interpolation.
10. Success point estimate passes but lower confidence bound fails.
11. Zero observed falls but inadequate independent exposure to bound the upper rate.
12. Fallback action is admitted only for one of several physically possible sensor-health signatures.
13. Every hard gate passes in one cell, adjacent cell fails: qualify only the exact cell.
14. Nominal energy and recovery-energy boundaries differ.
15. All gates pass but an unmatched baseline was used: descriptive, not ranked.

## Proposed qualification targets (not achieved)

- 100% exact identity match or explicit mismatch classification.
- 100% exact agreement with an independent Boolean/cell-join oracle on finite fixtures.
- Zero qualification with any missing, Unknown, Invalid or stale required evidence.
- Zero interpolation into a missing/rejected/unknown robust frontier cell.
- Zero complete safety qualification when any adverse category regresses, is unmeasured or lacks a required bound.
- Zero mixing of hardware/simulation, morphology, physics, actuator, sensor or exposure identities.
- Every unqualified cell reports all blocking predicates and source witnesses.
- Stretch: expand the directly qualified held-out hardware cell set at matched task, energy, compute and safety constraints without weakening any gate.

## Claim ceiling

V0 qualifies exact evidence joins and multi-gate classifications over supplied finite results. It does not compute viability, validate physical abstractions, classify contact events, estimate task success, calculate statistical intervals, measure energy, authenticate receipts or prove robot safety. Real operating-envelope claims require each upstream authority and physical hardware trial to be independently qualified.
