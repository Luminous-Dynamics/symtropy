# Construction layout input-integrity benchmark v0.1

**Issue:** [#3561](https://github.com/Luminous-Dynamics/symtropy/issues/3561)  
**Status:** synthetic contract proposal and reference validator; no physical robot test

## Why it exists

A layout robot can print precisely in the wrong place when the CAD/BIM export, coordinate transform, datum, units, trade revision or site context is wrong. A 2025 peer-reviewed multi-trade layout case study reported substantial gains for one project, but also documented approximately one hour of erroneous layout caused by a drywall CAD file using the wrong coordinate system, followed by realignment and reprinting. It also counted about five working days per floor for VDC file preparation and coordination. These results are a single case study, not a universal productivity estimate.

Source: Kam et al., *Comparison of dusty robotics and traditional layout methods*, Construction Robotics (2025-09-15): https://link.springer.com/article/10.1007/s41693-025-00163-z

## Core contract

The validator evaluates an exact input manifest against a separate trusted project/site policy. The manifest cannot supply its own expected revision, policy, decision, reason codes or approval result.

It verifies:
- exact source model ID, revision and digest against the trusted policy;
- export lineage to that exact source revision, exporter identity/version/configuration and artifact digest format;
- current project, site, work-package, task, work-zone, site epoch and change-sequence binding;
- coordinate-frame identity, horizontal reference, vertical datum, coordinate epoch where required, units, axis order, handedness, transform bundle and resource digests;
- exact trade/discipline revision set and feature inventory;
- references and digests for the upstream #1607 survey result and adjustment;
- separation between fit-point IDs and independent check points;
- checkpoint target-frame binding, independence assessment references, spatial spread, residuals and U95 relative to the predeclared decision margin.

The v0.1 manifest cannot self-authorize feature exclusions. All expected critical features must be present in the export; exclusions need a future policy-approved mechanism with explicit reasons and authority references.

## Decision semantics

- `admissible_for_named_layout_input`: all software-evaluable constraints in this named project policy are met.
- `inadmissible`: explicit revision, frame, transform, feature inventory, checkpoint or measured-margin mismatch.
- `indeterminate`: required survey references, independent evidence, sample count or uncertainty qualification is missing/insufficient.

This result means only that the manifest meets the named pre-print *input-admission contract*. It does not prove source bytes, signatures, issuer competence, design correctness, survey truth, or robot safety. It is not a robot command, dispatch permit, legal/professional design approval, or authorization to print.

## Existing owners composed, not duplicated

- [#1607](https://github.com/Luminous-Dynamics/symtropy/issues/1607) owns survey sessions, CRS/datum/frame/transform semantics, control adjustment, calibration, uncertainty, and held-out checkpoints. The new validator consumes exact references and results; it does not compute a transform or adjust a survey.
- [#3523 / #3524](https://github.com/Luminous-Dynamics/symtropy/pull/3524) own layout robot task profile and common independent referee benchmark.
- [#3522](https://github.com/Luminous-Dynamics/symtropy/pull/3522) owns physical trial receipts and raw-artifact identity.
- [#3531](https://github.com/Luminous-Dynamics/symtropy/pull/3531) owns claim-specific evidence admissibility.
- [#3542](https://github.com/Luminous-Dynamics/symtropy/pull/3542) owns site epoch invalidation and requalification.
- [#3558](https://github.com/Luminous-Dynamics/symtropy/pull/3558) owns observation coverage, freshness and unknown-versus-clear.
- [#1834](https://github.com/Luminous-Dynamics/symtropy/issues/1834) owns robot capability-to-work-package matching.

No CRS engine, BIM clash detector, event store, survey adjustment engine, site ledger, safety controller or robot dispatch path is added.

## Frozen synthetic adversarial cases

The initial fixtures cover:
- matched manifest under the synthetic trusted policy;
- wrong axis order;
- stale source model revision;
- fit point reused as an independent checkpoint;
- independent checkpoint reported in the wrong target frame;
- manifest-authored feature exclusion;
- checkpoint residual plus U95 beyond the decision margin;
- wrong vertical datum.

Unit tests also cover missing checkpoint evidence, U95 too wide to resolve the margin, stale site epoch, source receipt digest malformed, caller-supplied decision, and excessive policy checkpoint complexity. The checkpoint policy has a hard 128-point maximum to bound geometry validation work.

Every identifier, artifact digest, survey reference, checkpoint, coordinate and result in the fixtures is invented. Fixture tests qualify only the software contract.

## Industry comparison: benchmark the complete chain

HP SitePrint publishes ±2 mm layout tolerance only under its specified high-accuracy setup (SMR prism, High Accuracy Print Mode, 1-inch total station, 5–30 m). Dusty FieldPrinter 2 publishes ±1.6 mm given accurate surveyed control. These are vendor-reported product conditions, not independent matched accuracy or safety results. The #3524 common-referee track should compare only identical task geometry, site/ODD strata, control network and acceptance criteria.

The end-to-end productivity denominator must include BIM/VDC preparation, trade coordination, model correction, survey/control setup, robot setup, operator/spotter, printing, quality assurance, rework and recovery. Do not compare robot print speed alone against manual total-workflow labor.

Reference product pages:
- HP SitePrint: https://www.hp.com/us-en/printers/site-print/layout-robot.html
- Dusty FieldPrinter 2 specs: https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs

## Prospective targets (not achieved results)

- Zero false-ready decisions on the frozen deterministic coordinate/revision/inventory corpus.
- 100% exact source/export/revision/transform/receipt traceability for a named input admission.
- 100% critical feature reconciliation before print eligibility.
- Zero fit/check-point reuse.
- Zero commands accepted under a stale or mismatched design/worksite epoch (enforced by downstream #3542, not by this crate).
- Every injected coordinate/frame/revision defect rejected or marked indeterminate; no best-effort coordinate guess.
- Physical geometry targets remain separate: #3524's candidate p95 1.0 mm and p99 2.0 mm hypotheses require independent referee metrology and are not proven by input validation.

These are research targets, not measured performance, universal tolerances, regulatory thresholds or certifications.

## Future work

- Add a policy-approved exclusion model if the project owner needs declared non-layout features.
- Consume a signed #1607 survey result and independently validated checkpoint assessment; current references are opaque inputs, not authenticated.
- Add multi-discipline federation manifests with authoritative clash/coordination results, without duplicating the clash engine.
- Bind the validator output into the #3522 receipt and #3542 epoch/qualification flow.
- Conduct actual vendor-configuration reproduction and common-referee trials only after hardware, site approval, calibration and independent adjudication exist.
