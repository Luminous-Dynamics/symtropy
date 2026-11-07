# CityKit synthetic corpus v0.1.2

Parent: Symtropy [CITYKIT-SYNTH-001 #3768](https://github.com/Luminous-Dynamics/symtropy/issues/3768) and Sol-Atlas [CITY-MAKER-001 #549](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/549).

File: `CITYKIT_SYNTH_CORPUS_V0_1.json`  
Corpus ID: `CITYKIT-SYNTH-CORPUS-V0.1.2`

## Status

This is a candidate synthetic fixture set for implementation and independent-oracle review, not measured city, supplier, utility, factory or robot data. All numeric quantities are deliberately synthetic and cannot be used as project estimates or engineering inputs.

The corpus contains 18 bounded cases covering service capacity/evidence, resource balance and unit consistency, owner-native productive-capability assessment consumption, shared workcell capacity, supplier disruption outcomes and evidence currentness.

## Ownership and semantics

- **CityKit does not implement a generic AND/OR productive-capability closure engine.** Symthaea CIV-BOOT owns that theorem and its closure outputs. CityKit consumes an owner-native assessment receipt and checks the receipt contract / claim consistency.
- Each case declares `capability_assessments`, including target, disposition, operational/local-closure booleans where known, evidence state, source-owner/contract identity, revision, exact scenario subject digest and synthetic receipt ID.
- Receipt subject digest in these fixtures is `synthetic-subject:<case_id>`; this tests subject-binding semantics, not a cryptographic or real owner-issued receipt.
- Duplicate assessment target IDs invalidate that target; record order must not select a favorable winner.
- `LocallyClosed` requires operationally reachable = true and locally closed = true.
- `ImportDependent` requires operationally reachable = true and locally closed = false.
- `Unavailable` requires operationally reachable = false and locally closed = false.
- `Unknown` cannot carry favorable reachability / closure booleans.
- Imported capability may support present operations but must never be presented as locally reproducible.
- A local replacement component does not establish productive-equipment reproduction.
- CityKit service and resource checks require compatible declared units and current evidence.
- Each work-package resource demand must declare the same unit as its declared finite resource capacity.
- Work-package time intervals are half-open `[start_tick, end_tick)`; concurrent resource demand is summed against declared capacity.
- Stale, revoked, unknown and missing evidence cannot be promoted to favorable states.
- Dispositions remain distinct: `FeasibleForSyntheticCase`, `Infeasible`, `Unknown`, `Unsupported`.

The synthetic assessment fields are adapter fixtures, not genuine Symthaea receipts. Real integration must bind exact artifact identity, schema revision, source digest, subject identity, currentness and independent verification status.

## Qualification boundary

The `expected` fields are test-vector expectations, not authority. The candidate evaluator and separately implemented oracle recompute disposition and reason codes from `input`; neither reads expected output as its decision path. A differential harness compares independently computed outcomes against these frozen expectations.

Hostile controls include duplicate/dangling IDs, missing/incompatible units, duplicate assessment targets, invalid status/boolean combinations, subject mismatch/digest mismatch, semantically irrelevant input permutations, stale/revoked receipts and expected-label substitution.

Before claiming corpus qualification, run exact-head tests and independently review every expected output. A successful JSON parse is not evaluator validation.
