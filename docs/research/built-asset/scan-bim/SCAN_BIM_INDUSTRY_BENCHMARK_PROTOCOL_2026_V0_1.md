# Scan-to-BIM: industry benchmark protocol and 2026 competitive position v0.1

**Status:** research/benchmark plan; no comparative product trial has been executed.  
**Issue:** #4116 — SCAN-BIM-REPLAY-001.  
**Related:** #4188 replay receipt, #4205 directed accuracy, #4208 decision-sufficient capture, #4225 surface sampling, #4227 residual uncertainty, #4229 area overlap.  
**Date:** 2026-11-08.

## Strategic position

Do not compete with commercial reality-capture platforms by claiming that a point cloud can be turned into a mesh. Those workflows already exist and have mature operational features. Build a different, complementary advantage:

**An evidence-qualified as-built state that can be independently replayed, challenged, updated and used to decide whether to accept a component, request another measurement, repair it or escalate it.**

The competitive boundary is:
1. what was directly observed versus inferred or hidden;
2. geometry, semantics, topology and identity as separate claims;
3. covariance and correlation that are not collapsed into one scalar confidence;
4. exact provenance from observation through IFC/graph revision;
5. task-conditioned decisions and measurement planning;
6. independent re-verification after a physical change.

This is a product/research thesis, not a measured superiority claim.

## 2026 commercial baseline

### Leica Cyclone 3DR

The current product documentation describes BIM tolerance checking, deviation analysis, progress monitoring, clash detection, visual reporting and BCF exports. The workflow:
- compares each BIM sub-element against matched point-cloud data;
- exposes a tolerance, an exclusion zone used to associate points to parts, and a minimum coverage threshold;
- presents within/out-of-tolerance/no-data-style statuses;
- allows a person to review or modify results;
- exports PDF reports and BCF 2.1 topics, including per-element results.

The vendor's 2026 exercise explicitly warns that points can be linked to the wrong part and that editing tolerance parameters can reset previous statuses. These are concrete benchmark cases for us: association error, coverage denominator, stale result invalidation, reviewer intervention and decision reproducibility.

Sources:
- [Cyclone 3DR BIM Analyses overview](https://rcdocs.leica-geosystems.com/v1/docs/cyclone-3dr-cyclone-3dr-which-bim-analysis-feature-to-use)
- [Cyclone 3DR Tolerance Checking](https://rcdocs.leica-geosystems.com/v1/docs/cyclone-3dr-tolerancechecking)
- [Cyclone 3DR 2026 tolerance-checking exercise](https://rcdocs.leica-geosystems.com/cyclone-3dr/2026.0/Ex_ToleranceChecking)
- [Leica BIM analysis product overview](https://leica-geosystems.com/products/laser-scanners/software/leica-cyclone/leica-cyclone-3dr/powerful-bim-analysis)

### Trimble RealWorks and Trimble Business Center Buildings Edition

Trimble RealWorks publicly describes point-cloud registration, analysis, modeling, large-data processing, clash analysis, deviation reports and office-to-field collaboration. Its 2026.10 release notes add noise reduction for X7/X9 scans and registration refinement using targets, points and point clouds in one operation.

Trimble Business Center Buildings Edition's Pro plan includes scan registration, georeferencing to project survey control, scan-to-model deviation inspection, wall-verticality inspection and indoor scan classification. It supports IFC model import and connected delivery through Trimble Connect.

These are relevant baselines for registration productivity, field/office continuity, point-to-model deviation inspection and deliverable generation. Do not assume every feature is included in every RealWorks edition or license.

Sources:
- [Trimble RealWorks](https://www.trimble.com/en/products/building-construction-field-systems/trimble-realworks)
- [RealWorks 2026.10 release notes](https://help.fieldsystems.trimble.com/realworks/2026.10.htm)
- [Trimble Business Center Buildings Edition](https://www.trimble.com/en/products/building-construction-field-systems/trimble-business-center-buildings-edition)

### Autodesk ReCap

The cited ReCap scan-to-mesh service converts point clouds to meshes for use in downstream Autodesk design products. Its current documentation states that only the visible/selected point-cloud area is meshed after limit-box, clip, region and deletion selections, and the service requires cloud processing tokens.

Compare it as a registration/preparation/mesh-delivery workflow, then compare downstream authoring and IFC fidelity separately. Do not claim that this one service is a complete semantic Scan-to-BIM authoring system.

Sources:
- [Autodesk ReCap Scan to Mesh (Cloud)](https://help.autodesk.com/view/RECAP/ENU/?contextId=scan_to_mesh)
- [Autodesk ReCap 2026 Scan-to-BIM role](https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/Role-of-Autodesk-ReCap-in-scan-to-BIM-workflows.html)

## Research frontier: what the strongest new work changes

### Correlated stochastic registration

Zhou, Gu & Dong (2026), *Point cloud registration with enhanced accuracy via a synthetic stochastic model*, Expert Systems with Applications 302, article 130539, DOI 10.1016/j.eswa.2025.130539, argue that diagonal/block-diagonal covariance can be over-optimistic when point-cloud errors are correlated. Their full synthetic variance-covariance matrix decomposes non-correlating, functionally-correlating and stochastic-correlating errors and couples this with Gauss–Helmert estimation in which both source and reference clouds are stochastic. The paper reports up to about 70% lower parameter RMSE in its specific simulations and improvements over five simplified baselines in an indoor TLS case; that is not a universal registration improvement.

Symtropy response:
- consume #1607 survey covariance and adjustment receipts;
- maintain cross-covariance rather than assume independent scans;
- report transform uncertainty as unknown when only horizontal/vertical scalar 95% accuracy exists;
- benchmark residual uncertainty against independent checkpoints and exact matched registrations.

Source: [Zhou et al. (2026)](https://www.sciencedirect.com/science/article/pii/S0957417425041545).

### Point-level uncertainty prediction

Xu, Wysocki & Holst (2026), *From Propagation to Prediction: Point-level Uncertainty Evaluation of MLS Point Clouds under Limited Ground Truth*, ISPRS Annals XI-1-2026, 463–471, DOI 10.5194/isprs-annals-XI-1-2026-463-2026, use optimal neighbourhood estimation and geometric features to predict point-level uncertainty. They report XGBoost accuracy comparable to Random Forest at approximately three times the speed in their evaluated dataset. The target is C2C-derived point uncertainty under limited ground truth, not instrument calibration or absolute survey truth.

Symtropy response:
- treat learned uncertainty as a second, predictive channel alongside physical covariance propagation;
- calibrate both on building-disjoint independent checkpoints;
- compare calibration, interval coverage and selective risk, not only regression error or ROC-AUC;
- retain sensor, range, incidence, density and surface class as shift strata.

Source: [Xu et al. (2026)](https://isprs-annals.copernicus.org/articles/XI-1-2026/463/2026/).

### Active reconstruction and task-specific next-best-view

Active3D (AAAI 2026) combines multi-level uncertainty, an Expected Hybrid Information Gain objective and risk-sensitive view planning for high-fidelity reconstruction. It reports SOTA results on its evaluated 3D reconstruction benchmarks; these are not automatically Scan-to-BIM or as-built acceptance results.

Zhu, Sellán & Terenin (SIGGRAPH 2026) formulate next-best-view as Bayesian task-specific decision-making over uncertain implicit surfaces. Their paper evaluates semantic classification, segmentation and PDE-guided simulation, and reports better task performance with fewer views than general uncertainty-reduction baselines.

The important paradigm shift is that another scan is valuable only when it can change the downstream task decision. Compose with #1892 SPATIAL-VOI-001 and #1922 SPATIAL-ADAPTIVE-001 rather than recreating them.

Sources:
- [Active3D, AAAI 2026](https://doi.org/10.1609/aaai.v40i8.37585)
- [Bayesian task-specific next-best-view, SIGGRAPH 2026](https://arxiv.org/abs/2605.05095)

### Representation and information continuity

A 2026 review of the move from measured geometry to editable BIM highlights a recurring gap: measurement uncertainty, provenance, element correspondence, topology and semantic information can be lost as point clouds become surfaces, objects and IFC entities. This supports an evidence-preserving model-evolution architecture, not merely another representation conversion.

Symtropy response: every transition should preserve or explicitly supersede the source evidence, uncertainty, identity and model revision. IFC/IDS validation remains separate from geometric and topological verification; IDS is not a geometric accuracy evaluator.

Source: [From Measured Geometry to Editable BIM (2026 review)](https://www.mdpi.com/2075-5309/16/20/4095).

## Fair comparison protocol

A vendor or research comparison is only meaningful when these controls are frozen:
- identical raw scan, reference/model revision, coordinate control and visibility assumptions;
- exact scanner/instrument, field configuration and acquisition trajectory;
- product edition/license, software build, options and hardware;
- same tolerance, exclusion distance, minimum-coverage rule, element set and task definition;
- same operator expertise and training, with operator identity blinded where feasible;
- timed acquisition, registration, cleanup, authoring, review, correction, report and handoff phases;
- independent checkpoint/reference set unavailable to the candidate;
- exact source/configuration/output hashes and per-scene/per-building results.

Do not compare a vendor's advertised accuracy range against a research result measured on another scanner or benchmark. Do not count repeated captures of one building as independent buildings. Treat TUB1/TUB2 as different acquisitions of one building group.

## Benchmark scorecard and target ladder

Targets below are proposed internal gates, not achieved results.

| Gate | Primary measures | Comparison / target |
|---|---|---|
| G0 replay integrity | Exact manifest, source/build/model/config/input/output hashes, frame/unit/tolerance/metric binding, leakage canaries | 100% deterministic replay; every injected digest/frame/unit/stale-reference/path leak fails closed |
| G1 historical geometry | ISPRS wall-only completeness, correctness and accuracy curves; published 10 cm values | Reproduce all accessible published values to their displayed rounding precision; unresolved differences remain explicit |
| G2 element geometry | Surface distance median/p95/p99, dimension/plane error, coverage and completeness/correctness denominators | Non-inferior on each native metric against matched published baselines; do not trade one metric silently for another |
| G3 topology and identity | Room adjacency/connectivity precision/recall, opening-boundary consistency, GlobalId correspondence, split/merge accuracy | No geometry-only pass when graph topology or identity is wrong; separate per-class and per-building errors |
| G4 uncertainty | Covariance/transform propagation, independent-checkpoint residuals, 90/95/99% interval coverage, interval width, calibration error, selective risk/coverage | Nominal 95% intervals should empirically cover at a rate statistically consistent with 95% on held-out buildings; publish width and failures, not coverage alone |
| G5 decision-sufficient capture | Accepted task decisions per scan-hour, cost per independently accepted element, scans needed to resolve a decision | Beat fixed-grid and uncertainty-max baselines in decision quality per acquisition cost; no extra scan unless it can change a declared decision |
| G6 operational comparison | Expert correction person-minutes, false acceptance, false alarms, unresolved issues, BCF/IFC interoperability, report time | Internal target: >=20% less expert correction time at matched independently accepted quality, with no increase in false acceptance. Re-baseline if manual/vendor control does not support the comparison |
| G7 lifecycle | Physical change precision/recall, stale-model rejection, evidence continuity, identity continuity, re-verification latency | Every accepted mutation is bound to the exact before/after evidence and stable asset identity; stale or superseded receipts cannot authorize a new acceptance |

### Industry workflow challenge matrix

| Workflow capability | Current commercial baseline | Symtropy's testable differentiation |
|---|---|---|
| Per-element point/model deviation and tolerance status | Leica Cyclone 3DR; Trimble RealWorks / TBC Buildings Pro | Same per-element outputs, plus visibility-aware denominators, measurement covariance, explicit unknowns and independently replayable thresholds |
| Point assignment / exclusion / coverage | Leica exposes exclusion-zone and minimum-coverage controls; its docs warn of wrong-part matches | GlobalId/geometry identity, wrong-part adversarial cases, ambiguity and unmatched coverage are explicit outputs |
| Scan registration and georeferencing | Trimble supports target/point/cloud refinement and survey-control alignment | Exact transform, control/checkpoint lineage, frame/epoch/unit evidence and propagated residual uncertainty |
| Scan-to-mesh/preparation | ReCap cloud mesh from selected visible point-cloud region | Retain source-to-mesh evidence map and quantify information/uncertainty loss at each conversion |
| Reporting and collaboration | Leica BCF/PDF; Trimble cloud/office-field exchange | Signed/revision-bound evidence receipts, machine-verifiable BCF/IFC/graph links and after-repair re-verification |
| Active capture | Active3D and task-specific Bayesian NBV research | Choose views by expected building/task decision value and cost, not only global entropy or visual completeness |

## How to surpass the field

The winning target is not “best point cloud mesh.” It is a Pareto frontier with independent dimensions:
- geometry and area completeness;
- correctness / over-modelled surface;
- topology and asset identity;
- calibrated residual uncertainty;
- capture/review cost;
- human correction effort;
- traceability and lifecycle change reliability.

A proposed system should only be called superior when it improves one dimension at comparable levels of the others on the same held-out buildings, or produces a strictly better frontier at equal independently accepted quality. No single composite score should conceal an increase in false acceptance, hidden-surface claims or unqualified uncertainty.

## Immediate execution order

1. Finish and independently review the sampler (#4225) and covariance adapter (#4228).
2. Implement #4229 area-overlap/completeness/correctness with a named surface-compatibility policy and analytic area ledger.
3. Connect survey covariance and independent checkpoint identities from #1607; do not recreate its survey adjustment.
4. Add a matched commercial trial protocol and recruit or record operator-based control data before declaring a 20% productivity lead.
5. Integrate #4208 decision-sufficient capture with #1892/#1922, and measure task quality per scan-hour.
6. Bind accepted changes to #3331 component identity, #1226 evidence receipts and #4043 lifecycle verification.

## Claim ceiling

This document describes current public product workflows, research comparators and proposed internal targets. No product trial, SOTA reproduction, commercial superiority, 20% time saving, or calibrated uncertainty result has been achieved by this document.
