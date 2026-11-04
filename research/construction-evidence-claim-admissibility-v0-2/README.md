# Claim Admissibility V0.2 — Evidence Vectors, Not Evidence Ladders

**Issue:** [CONSTRUCTION-ROBOT-EVIDENCE-005 / #3526](https://github.com/Luminous-Dynamics/symtropy/issues/3526)  
**Status:** research proposal and schema draft  
**Existing receipt:** [#3522 — physical trial receipt v0.1](https://github.com/Luminous-Dynamics/symtropy/pull/3522)

## Core change

A single evidence level is not a general model of evidence quality. It mixes origin, metrology, reference independence, provenance, custody, adjudication, ODD coverage and replication.

This proposal represents those as **separate axes** attached to one or more evidence units. Claims are then evaluated separately against frozen, claim-specific requirements profiles. There is no global evidence rank, composite confidence score or generic PASS.

## Evidence origins

The proposed origin axis is limited to where/how an evidence unit was produced:

- `synthetic`: authored/frozen inputs and expected outputs.
- `simulated`: model-generated or simulator output.
- `bench_measured`: physical measurement on an instrumented bench/rig.
- `hardware_in_loop`: real controller/hardware components exercised in a controlled setup.
- `field_observed`: observation under a declared real worksite ODD.

Independent assessment is deliberately **not** an origin. A reviewer can examine synthetic, simulated, bench, HIL or field evidence; review does not change where the evidence came from.

## Orthogonal evidence axes

Each evidence unit records:

| Axis | Question answered | Example states |
|---|---|---|
| Origin | Where did this evidence come from? | synthetic, simulated, bench, HIL, field |
| Reference independence | Is the physical reference independent under an explicit claim profile? | not assessed, independent under profile, partial, common mode, indeterminate, same path |
| Metrology | Are instrument calibration and measurement uncertainty qualified for this claim? | not applicable, not assessed, declared, calibration verified, uncertainty reviewed, independently audited |
| Provenance/authentication | Are bytes and issuer identities established? | digest absent/digest only/bytes verified; unsigned/signed/issuer trusted |
| Custody | Is evidence custody controlled and audited? | local, declared, audited |
| Adjudication | Who reviewed the claim? | none, internal, independent assessor, competent authority |
| ODD coverage | Which conditions were actually measured? | declared only, partial, measured by stratum |
| Replication | What is the independent experimental scope? | one trial, repeat same configuration, independent site, independent configuration |

These states are descriptive. They must not be summed into one trust/confidence score or treated as a total ordering. A calibrated bench measurement may be stronger than a poorly controlled field observation for one claim, while the field observation may answer a different question entirely.

## Independence ownership: reuse Symthaea #4918

The reference-independence axis must consume [Symthaea #4918 — ENG-EVID-INDEP-001](https://github.com/Luminous-Dynamics/symthaea/issues/4918) for claim-relative independence profiles, dependency lineage, common-mode roots, calibration roots, clock/timebase dependencies and repeatability versus replication.

Symtropy must not implement a competing common-mode dependency graph. Its claim packet carries profile and assessment references plus the task-specific physical evidence. The generic lineage owner remains authoritative for dependency ancestry.

## Claim-specific admissibility

The same evidence unit can support one claim and not another. Examples:

- A synthetic frozen trace can support a claim about deterministic software-contract behavior, but cannot support a physical detection or stopping-distance claim.
- A calibrated bench measurement can support a bounded stopping-distance claim for the exact machine/payload/surface/mode tested; it does not establish field performance.
- A field human-detection claim requires independent target ground truth, calibration/uncertainty, encounter sampling and measured ODD strata. Being labelled field-observed is insufficient.
- A field productivity claim requires accepted output and matched task/design/ODD/acceptance baselines. It does not imply safety qualification.
- Cross-site generalization requires independently held-out sites/configurations; repeated trials at one site are repeatability, not independent-site replication.

The JSON profile file contains illustrative profiles for these claim classes. Numeric sample targets are explicitly protocol hypotheses and must be justified by the actual risk assessment/statistical plan. Null sample targets are intentionally left to the task-specific plan where a universal count would be misleading.

## Claim decisions

Each claim gets one decision:

- `admissible_for_named_claim`
- `inadmissible_for_named_claim`
- `indeterminate`
- `not_evaluated`

There is no universal PASS. A packet may admit a productivity claim while leaving a human-detection claim not evaluated. Missing dimensions do not erase other measurements, but they block only the claims that require them.

## Composed artifacts

- `schema/claim-admissibility-packet-v0.2.schema.json`: closed Draft 2020-12 schema for claim packets and evidence vectors.
- `profiles/claim-requirements-v0.2.json`: illustrative per-claim requirement profiles.
- `fixtures/valid/synthetic-contract-only.json`: synthetic input may support a synthetic claim only.
- `fixtures/valid/bench-stopping-distance-claim.json`: fixture showing a bounded bench claim.
- `fixtures/invalid/independent-review-does-not-make-synthetic-physical.json`: an independent reviewer cannot change evidence origin.
- `fixtures/invalid/field-observation-without-reference-ground-truth.json`: field label alone cannot admit a detection claim.

All fixtures and hashes are invented. No physical measurement, vendor result, assessor action or calibration is asserted.

## Ownership boundaries

- #3522 remains the physical trial receipt.
- #3505/#3506 remain the physical protocol, ODD and metrology method.
- #1607 remains surveying/coordinate-frame/adjustment/uncertainty.
- #1300 remains blind challenge commitment/custody.
- Symthaea #4918 remains generic evidence-independence lineage.
- #3317/#3499/#2672/#1595/#1172/#3331 remain snapshot, temporal, mission, scenario, replay and component/as-built owners.

## Migration rule

v0.1 values are not silently re-ranked into v0.2. A named migration tool must preserve the old scalar verbatim as legacy metadata, map only facts explicitly represented in the old receipt, mark unknown axes as unknown, and produce no new admissible claims by default.
