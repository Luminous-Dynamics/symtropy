---
title: Sawlog Benchmark Contract
version: 0.1
status: synthetic-validator
date: 2026-10-17
---

# Sawlog Benchmark Contract v0.1

## Why this tranche precedes a 3D optimizer

A cutting plan can be mathematically optimal against an inaccurate or incomplete model. The next useful capability is a reproducible contract for evidence a sawlog optimizer will consume and the observed outputs against which it must be evaluated.

The U.S. Forest Service's 3D sawing work combined geometry, internal defects, opening-face selection, sawing simulation and grade rules. Its reported value-recovery gains show that these interactions matter; they are not a promise about this project. A later validation of LORCAT on 42 hardwood logs found no statistically significant difference in total sawn volume, yet significant differences in board count linked to opening-face board-size targeting. This warns against volume-only scoreboards.

Research:
- Lin, Wang and Thomas (2011), 3D log sawing optimization: https://research.fs.usda.gov/treesearch/39811
- Thomas, Buehlmann and Bennett (2024), LORCAT validation: https://research.fs.usda.gov/treesearch/68788
- USDA Forest Service (2025), LORCAT v4, including by-product estimates: https://research.fs.usda.gov/treesearch/69793
- FAO, circularity in forest and wood industries: https://www.fao.org/forestry-fao/industries/98953/en/

## Paradigm shifts

1. **Yield forecast → evidence-grounded digital twin.** Represent cross-sections along the log, out-of-round shape, centerline offset/sweep, defects, measurement uncertainty and instrument calibration. Preserve the origin and quality of every field.
2. **Volume-only → product portfolio.** Score individual board count, dimensions/tolerances, grade mix, rejects/rework, by-products, runtime/energy where measured, and value only when prices are sourced and dated.
3. **Point prediction → robust plan.** Carry uncertainty through geometry and machine variation. Return feasible alternatives and confidence bounds; return indeterminate when evidence or compute budgets do not support a safe conclusion.
4. **Proprietary silo → permissioned reproducible corpus.** Use a versioned schema, explicit license, pseudonymous stable IDs, splits by mill/source, and immutable receipts. Never publish operational data without permission.
5. **Single output → circular cascade.** Evaluate whether by-products can become chips, panels, pulp, bio-based chemicals, energy or another qualified use; do not award environmental credit without measured process boundaries and defensible lifecycle methods.
6. **AI as authority → AI as proposal generator.** Symthaea may propose cuts, orientations, experiments and explanations. Deterministic feasibility checks and qualified human/standards workflows remain authoritative for execution and structural grading.

## What this PR implements

- A standalone Rust validator for synthetic-only sawlog evidence records.
- Section stations with major/minor diameters, centerline offsets, ellipse orientation and uncertainty.
- Defect extents, optional circumferential/radial measurements and evidence state.
- Machine setup and explicit grade-rule version.
- Per-board observed dimensions, grade evidence, intended application and inspection state.
- Output/by-product volume ledger with complete, partial and unknown modes.
- Strict parsing that rejects unknown keys, invalid units, unordered geometry, malformed defects and duplicate IDs.
- Structural-use boards require qualified grade evidence and a passing inspection.
- An ellipse-per-section trapezoid integrator independently estimates log volume from section stations and checks the declared gross-volume estimate against a separate geometry tolerance.
- Complete-ledger reconciliation within declared tolerance; partial outputs cannot exceed the input-volume envelope beyond tolerance; unknown observations cannot carry quantitative outputs.
- Deterministic receipt binding the exact corpus bytes and source commit.

This is **not** a geometry reconstruction engine or optimizer. v0.1 accepts only synthetic examples so real mill/operator data are not accidentally published. An overall `validated` receipt means the corpus and physical-accounting invariants passed; it does **not** mean every prediction met its accuracy target. Read `geometry_target_met` and the aggregate geometry metrics separately. The synthetic suite intentionally contains an out-of-target estimate, demonstrating why prediction error must be reported rather than filtered away. Passing schema validation means only that the fixture conforms to the contract and declared accounting gates.

## Proposed benchmark ladder

These targets are proposals, not standards or demonstrated results. Freeze them only with a sawmill/domain partner and enough held-out measured logs.

### Gate A — data integrity

- 100% rejection of malformed units, unordered stations, out-of-log defects, duplicate IDs and unknown schema fields in adversarial tests.
- 100% exclusion of structural-use claims unless grade evidence is qualified and inspection passes.
- Every receipt includes corpus digest, data revision, source commit, validator version and model/config version.
- No private/proprietary measurements in public corpora without permission and a compatible license.

### Gate B — physical/accounting correctness

- Every board volume is recomputable from integer dimensions.
- Geometry-derived volume (ellipse area per station integrated by a trapezoid rule) agrees with the declared gross-volume estimate within a separate predeclared tolerance.
- Complete-ledger cases close within predeclared measurement uncertainty; no silent balancing.
- Partial observations remain explicitly partial and are never reported as fully reconciled.
- Report volume, board count, dimensions, grade mix and residual/by-product totals per log.

### Gate C — prediction quality on held-out measurements

Candidate starting targets to validate with practitioners:
- Median absolute percentage error for sawn volume <= 5%, reported separately by mill/species/size regime.
- At least 90% of logs within +/-1 board or 5% of observed board count, using whichever tolerance is larger.
- Board dimension and grade confusion metrics reported separately; an aggregate score may not hide safety-relevant failures.
- Any value-recovery improvement must use paired per-log comparisons and uncertainty intervals on source/mill-held-out data, without increasing quality/safety violations.
- Report p50/p95 runtime and peak memory, including timeout rate. Timeouts are indeterminate, never success.

### Gate D — operational usefulness and circularity

- Measure setup time, operator overrides, rework, rejects, usable recovery and downtime in controlled trials before claiming savings.
- Quantify by-products only where measured; downstream reuse credits require traceable destinations and lifecycle boundaries.
- Track missingness and how quality changes when geometry stations, defect observations or calibration metadata are removed.

## Real-data leads researched (2026-10-17)

Do not treat these sources as one blended benchmark. They cover different tasks and should have separate split rules and metrics.

### Track A — log-to-board traceability and product quality

TreeTrace_Douglas is a published dataset for Douglas-fir logs and downstream products. Its paper describes 52 long logs, 156 short logs, 208 discs and 346 boards, including log-end images, volume estimates, board positions, machine-grading imagery and board-quality/mechanical measurements. The paper reports that boards were collected for a 32-short-log subsample and 152 of 156 short logs were sawn. That makes it the strongest candidate found here for a first log-to-board traceability pilot, while recognizing the modest count and single-species focus.

- Dataset DOI: https://doi.org/10.15454/YUNEGL
- Dataset metadata: https://metadata-afs.nancy.inra.fr/geonetwork/srv/fre/catalog.search#/metadata/d9eef6e4-f195-41f4-b6c2-2ab46adc637e
- Dataset paper: https://doi.org/10.1186/s13595-022-01163-7

The TreeTrace paper is CC BY 4.0, but the data repository records **Custom Dataset Terms: CC BY 2.0**. Follow the dataset terms and citation instructions rather than assuming the paper license applies to every data file; review third-party image notices where applicable. The repository files are not bundled in this PR.

### Cross-species extension — TreeTrace_spruce

TreeTrace_spruce complements TreeTrace_Douglas with 100 Norway spruce logs, log-end RGB imagery, CT/X-ray measurements, and terrestrial LiDAR scans. Its dataset record lists the Etalab Open License 2.0, described as compatible with CC BY 2.0. It is a useful independent species/protocol track for sensing, geometry and traceability. It does not itself provide the same matched end-to-end sawmill board outcome as TreeTrace_Douglas, so do not treat it as a direct board-recovery test set.

- Dataset record: https://doi.org/10.57745/WKLTJI
- Dataset paper: https://doi.org/10.1186/s13595-023-01178-8

Keep TreeTrace_spruce and TreeTrace_Douglas as separate evaluations because their acquisition protocols and conditions differ. Cross-species generalization should be an explicit held-out experiment, not an accidental result from randomly mixing samples.

### Track B — 3D log measurement and volume error

Stack-LAUT includes point clouds from 10 wood stacks captured with three LiDAR devices and sawmill reference measurements for individual logs. The authors mapped 280 marked logs to measurement records. This can test point-cloud/geometry extraction and volume agreement, but does not by itself establish board count, lumber-grade or value-recovery accuracy.

- Dataset record (published 2025-01-08): https://doi.org/10.5281/zenodo.14609648

The Zenodo record's visible Rights section currently shows a License label without a license value. Treat reuse/redistribution permission as unresolved and contact the depositor before using files or derived data; do not assume public download implies an open license. The record describes a large collection; start with metadata and contact/permission checks rather than downloading or mirroring the full corpus.

### Track C — board-surface defect perception (supporting only)

Public board-surface defect-image datasets can test knot/crack detection, but they are not substitutes for log geometry, sawing settings or matched mill outputs. Keep image-detection metrics (precision/recall, class confusion, localization and calibration) separate from recovery metrics. In particular, a knot classifier does not confer structural-grade eligibility; the applicable grading rules and inspection remain an independent gate.

### Pilot decision

Start with metadata and a small authorized sample from Tracks A and B. Keep original source units and identities, record separate data/license provenance, and split by tree/log and preferably site/mill to avoid related samples leaking between calibration and test sets. Do not publish any derived sample until its terms are reviewed. If no compatible redistribution rights exist, write a private adapter/report pipeline over locally obtained licensed data and publish only metrics and synthetic fixtures.

## Next sequence

1. Freeze the validator and synthetic adversarial suite.
2. Review the schema with a small sawmill/wood-products specialist.
3. Identify a legally shareable dataset or obtain explicit partner authorization.
4. Build an independent baseline report on real held-out logs before implementing a geometry-aware candidate generator.
5. Compare candidate plans on the exact same logs, grade rules, machine configuration and price assumptions.
6. Let Symthaea search cut hypotheses under uncertainty; let the deterministic evaluator reject infeasible or unsupported proposals.
7. Connect accepted products/residuals to Mycelix provenance/custody and Sol-Atlas spatial context, without letting provenance or spatial data imply harvest permission.

Related work:
- One-dimensional reference evaluator: https://github.com/Luminous-Dynamics/symtropy/pull/1744
- Geometry-aware measured-log benchmark tracker: https://github.com/Luminous-Dynamics/symtropy/issues/1745
- Cross-stack wood-products initiative: https://github.com/Luminous-Dynamics/symtropy/issues/1741
- Synthetic lifecycle benchmark: https://github.com/Luminous-Dynamics/symtropy/pull/1742
