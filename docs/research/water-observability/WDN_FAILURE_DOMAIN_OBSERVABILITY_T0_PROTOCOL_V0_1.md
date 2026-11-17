# WDN failure-domain observability T0 protocol v0.1

**Tracking issue:** #5047 WATER-OBSERVABILITY-T0-EXECUTOR-001  
**Contract issue:** #5045 WATER-OBSERVABILITY-QUALIFICATION-001  
**Parent diagnostic benchmark:** #4963 PRESSURIZED-WATER-LEAK-DIAGNOSTICS-BENCH-001  
**Date:** 17 November 2026  
**Status:** synthetic known-answer contract only.

## 1. Paradigm shift

A sensor count and a topology-only Boolean are not operational observability. The evaluator separates four layers:

1. **Structural certificate validity:** does the proof bind the exact topology revision/digest, complete critical-state mapping and current surviving sensor set?
2. **Direct evidence availability:** is each measurement current, correctly bound, calibrated, time-valid and outside every active failure domain?
3. **Estimator qualification:** does the estimator match the live topology and surviving sensor set, and have the required calibration/held-out/OOD/validity evidence and numeric gates?
4. **Claim eligibility:** what is the strongest state the available evidence actually supports?

A sensor is not an independent backup merely because it has a different device identifier. Its physical interface, calibration lineage, power, communications, clock and measurement principle must each be recorded. A failed shared domain invalidates every channel attached to it.

## 2. Frozen state dispositions

Each of four synthetic pressure-state IDs receives exactly one disposition:

- `directly_observed_current`
- `estimator_qualified_for_declared_scope`
- `structurally_observable_only`
- `unobservable_under_declared_faults`
- `topology_invalidates_certificate`
- `indeterminate_evidence`

No structural proof may upgrade stale instruments, a failed common-cause group or an unqualified estimator. If clock evidence is invalid, the result is indeterminate—not normal, healthy or confidently unobservable.

Direct readings are preserved even when a topology certificate is stale. A stale or incomplete certificate is marked unusable and is never used to infer missing states.

## 3. Failure-domain contract

Every sensor supplies exactly one identifier for each domain:
- physical interface;
- calibration lineage;
- power supply;
- communications path;
- clock domain;
- measurement principle.

For every run, compute surviving sensor IDs from:
- healthy device condition;
- valid state/asset binding;
- current calibration;
- current time status;
- no active failed dependency domain.

A common-cause failure is applied to all dependent channels simultaneously. The evaluator never counts channels with the same failed domain as independent redundancy.

## 4. Structural-certificate reuse

A structural certificate is reusable only if all of the following match the current input:
- topology revision;
- 64-character topology digest;
- complete four-state mapping;
- exact set of currently surviving sensor IDs;
- non-empty proof reference.

An empty surviving sensor set is not structural support. An incomplete mapping or stale digest/revision invalidates the certificate. A valid certificate may identify a state as structurally observable only; it cannot create a direct reading or a numerically qualified estimate.

## 5. Synthetic estimator gate

The `synthetic_qualified_contract` state may be emitted only when:
- model scope, parameter scope, calibration, held-out, OOD and validity references are all present;
- `qualification_claimed=false`;
- estimator topology revision and digest match;
- estimator sensor IDs exactly equal the currently surviving sensor IDs;
- the state is explicitly inside estimator state scope;
- sample count is at least 100;
- empirical interval coverage is between 85% and 95%;
- interval width is positive and no more than 20%;
- condition number is positive and no greater than 1,000,000.

These are synthetic contract gates, not physical estimator qualification limits or regulatory standards. Even a passing case keeps receipt qualification at `NotRun`.

## 6. Frozen challenge pack

Twelve synthetic scenarios cover:
1. nominal independent direct channels;
2. single-channel dropout with an independent backup;
3. shared calibration-lineage failure;
4. shared power failure;
5. shared communications failure;
6. common clock failure;
7. stale topology certificate with direct readings preserved;
8. ill-conditioned estimator that must not be qualified;
9. incomplete critical-state mapping;
10. alternate independent measurement principle survives a domain failure;
11. missing structural proof;
12. fully referenced synthetic estimator contract with physical claim disabled.

The oracle contains 48 per-state dispositions. Tests additionally attack duplicate sensor IDs, missing and duplicate dependency domains, out-of-set sensor bindings, claimed physical qualification, missing OOD references, out-of-range interval coverage and altered oracle dispositions.

## 7. Reproducible receipt

The exact-revision receipt binds:
- input scenario pack;
- input Draft 2020-12 schema;
- separate known-answer oracle;
- Cargo.toml;
- Cargo.lock;
- evaluator implementation;
- exact checkout revision.

The evaluator is read-only. It does not run EPANET, solve hydraulics, estimate a live state, optimize a sensor layout, write SCADA, or actuate pumps/valves.

## 8. Acceptance and nonclaims

Acceptance is 12/12 exact cases and 48/48 state dispositions, with rustfmt, locked Rust tests, strict Clippy, JSON Schema validation and an exact-revision receipt.

A PASS proves only software behavior against synthetic inputs. It does not prove observability of Johannesburg or any live network, qualify physical sensors or estimators, establish a service floor, establish leak detection/localization performance, or authorize operational action.
