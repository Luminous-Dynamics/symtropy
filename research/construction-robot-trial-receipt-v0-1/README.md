# Construction Robot Physical Trial Receipt V0.1

**Status:** draft schema and reference semantic validator  
**Owner:** [CONSTRUCTION-ROBOT-EVIDENCE-004 / #3521](https://github.com/Luminous-Dynamics/symtropy/issues/3521)  
**Parent protocol:** [#3505 / #3506](https://github.com/Luminous-Dynamics/symtropy/pull/3506)

This package defines a machine-readable receipt for one construction-robot qualification trial. It is a record of claims and evidence references, not an authority that makes those claims true.

## Composition boundaries

- #3505/#3506 owns physical trial, ODD, reference metrology and staged qualification protocol.
- #1607 owns surveying provenance, coordinate frames, transforms, calibration and uncertainty semantics. This receipt references those artifacts; it is not a survey adjustment engine.
- [Symthaea #4918 — ENG-EVID-INDEP-001](https://github.com/Luminous-Dynamics/symthaea/issues/4918) owns generic claim-relative independence profiles and common-mode dependency lineage. This receipt performs only narrow declared-domain consistency checks; it does not implement a second dependency graph or prove physical independence.
- #1300 owns blind challenge-set commitment, custody and post-run reveal. This receipt carries only an opaque challenge commitment and custodian reference; hidden seeds, answer keys and challenge preimages are not producer-visible.
- #3317 owns snapshot operating-state evaluation.
- #3499 owns temporal/per-cell event reduction.
- #2672 owns mission-level fault-to-disposition qualification.
- #1595 owns shared scenario identity and fault schedules.
- #1172 owns deterministic scenario execution and replay identity.
- #3331 owns component/as-built continuity where relevant.

## Contract layers

1. **Schema validity:** JSON is structurally valid against JSON Schema Draft 2020-12, including closed objects and exact enumerations.
2. **Semantic admissibility:** the reference validator checks cross-field constraints that JSON Schema does not express directly: claim-ceiling promotion, calibration coverage, reference independence declaration, measurement uncertainty versus decision margin, instrument references and sample denominator consistency.
3. **Independent adjudication:** an external assessor evaluates the actual instrument, calibration, source-byte custody, fault-domain independence and physical truth. The software cannot establish these from a receipt alone.

These statuses must never collapse into a generic PASS.

## Evidence classes and claim ceilings

The ordered classes are:
`synthetic < simulated < bench_measured < hil_measured < field_observed < independently_adjudicated`.

A claim ceiling may be lower than the available evidence class, never higher. The ordering is a conservative ceiling rule, not a substitute for a task-specific risk assessment. A hash string identifies expected bytes; it does not authenticate them or prove who produced them.

## Result vocabulary

- `measured_within_declared_criterion`
- `measured_outside_declared_criterion`
- `blocked_by_safety_gate`
- `indeterminate`
- `not_evaluated`
- `invalid_evidence`
- `unsupported_odd`

There is deliberately no bare `pass`, `safe`, `certified` or `compliant` result.

## Semantic rules

- A measured result requires a physical evidence class, raw evidence references, an independent reference declaration, a positive independent-encounter denominator and uncertainty-bearing measurements.
- Every subject-sensor and reference-instrument calibration interval must cover the complete trial interval. These time bounds are required for physical evidence classes; synthetic/NotEvaluated receipts must not fabricate physical calibration.
- Shared relevant sensing, power, clock, network or processing fault domains invalidate the claim of independent reference measurement.
- The validator also cross-checks declared reference-instrument fault-domain identifiers against the evaluated robot's sensor fault domains, and requires a distinct reference clock-domain identifier. These are checks of declared metadata, not proof that the real hardware is independent.
- The initial protocol gate requires U95 <= one-third of each decision margin. This is a proposed benchmark rule, not a requirement quoted from ISO.
- Measured values must identify units, sample counts and reference instrument IDs.
- Decision-margin units must equal measurement units; sample counts declare their unit (encounters, features, frames, sites, task cycles or robot-hours) so feature counts are not incorrectly compared with encounter denominators.
- A claim ceiling cannot exceed its evidence class.
- `blind_independent` challenge receipts require an independent custodian and a producer-output commitment. The schema has no field for hidden seeds or oracle answers and rejects unknown fields.
- Repeated frames are not independent encounters. Clustering and exclusions remain explicit.
- NotEvaluated and indeterminate outcomes remain first-class outcomes, not successes.

## Validation

The CLI validates all fixtures under `fixtures/valid` and `fixtures/invalid`, reports one stable line per fixture, and exits non-zero on a mismatch between expected and actual validity.

The crate uses a pinned JSON Schema validator version. A Cargo lockfile is required before this branch can be treated as dependency-reproducible; the dedicated workflow generates and exports the resolved lockfile for review, after which it must be committed and CI changed to `--locked`.

## Example

See `examples/bench-measured-stopping.json` and `examples/not-evaluated-synthetic.json`. They contain invented fixture values and digests for software tests only. They are not physical measurements, test results, calibration records, or claims about a commercial robot.

## Claim limits

The validator does not fetch raw artifacts, verify signatures, inspect calibration certificates, estimate measurement uncertainty from waveforms, prove fault-domain independence, verify a robot's physical stop, or grant authority to operate. It cannot issue commands, stop/reset equipment, authorize work or certify compliance with ISO/ANSI/IEC standards.

## Known v0.1 epistemic limitation

The v0.1 `evidence_class` / `claim_ceiling` ordering is a conservative anti-promotion guard, not a universal ranking of evidence quality. Evidence origin (synthetic/simulated/bench/HIL/field), reference independence, metrology, authentication/custody, adjudication, ODD coverage and replication are orthogonal dimensions. A weak field observation is not automatically stronger than a calibrated bench measurement; independent review of synthetic evidence does not make it physical.

The v0.2 migration is tracked by [CONSTRUCTION-ROBOT-EVIDENCE-005 / #3526](https://github.com/Luminous-Dynamics/symtropy/issues/3526). The proposed design removes the global rank and uses claim-specific requirements across explicit evidence dimensions. Do not interpret v0.1 ranks as certification, confidence or a transferable quality score.
