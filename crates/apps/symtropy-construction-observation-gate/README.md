# Construction observation coverage gate v0.1

Issue: [#3558](https://github.com/Luminous-Dynamics/symtropy/issues/3558). Research-only Rust oracle; no sensor I/O, cryptographic verification, robot actuation or physical validation.

## What this crate decides

Given a frozen policy and upstream observation records, it returns one of four states:
- `verified_clear`: every required source and zone cell has a fresh, authenticated, policy-qualified clear observation;
- `occupied_or_changed`: at least one policy-qualified source positively observes a changed/occupied cell;
- `unknown_or_unobserved`: a required source/cell is missing, stale, untrusted, degraded, uncalibrated, occluded, out of timebase, or otherwise insufficient;
- `conflicting`: qualified sources disagree on a zone cell, or a source sequence/digest integrity conflict is detected.

Any state other than `verified_clear` requires hold / epoch invalidation. `occupied_or_changed` is never averaged away by other clear sources. A positive and clear report for the same cell is `conflicting`. Empty detections do not imply clear.

## Trust boundary

The caller supplies an upstream-authenticated assertion, sensor health, calibration result, uncertainty, reference-independence assessment and monotonic timestamp/timebase. This crate does not perform those measurements or verify signatures/identities. The `independent_under_profile` value must come from the owning independence service (Symthaea #4918); do not fork a dependency graph here. Each source policy pins an externally trusted baseline sequence and digest; every observation must link to its predecessor digest. A sequence gap latches unknown, while rollback or predecessor/same-sequence digest divergence latches conflict. Recovery requires an external re-baseline and a new gate instance. The gate does not create or verify that baseline.

## Privacy

Cells are anonymous work-zone geometry identifiers. There is no worker identity, biometric, intent, emotion or individual productivity field.

## Integration boundary

This crate owns upstream observation sufficiency and missed-change handling. The worksite epoch oracle in [PR #3542](https://github.com/Luminous-Dynamics/symtropy/pull/3542) owns monotonic epoch updates, permission leases, safe hold and requalification. The gate does not create an epoch or authorize motion; an upstream event router must translate any non-clear result into the corresponding #3542 invalidation path.

## Limits

Passing unit tests prove only this deterministic software contract against declared inputs. They do not prove a sensor detects real people, that zone geometry is correct, that upstream authentication is true, or that physical stopping completes in time. Physical validation needs independent ground truth, metrology, HIL, supervised field sampling, and claim-specific admissibility via [PR #3531](https://github.com/Luminous-Dynamics/symtropy/pull/3531).
