# Shipyard scan metrology VVUQ contract v0.1

Tracking: [SHIPYARD-SCAN-METROLOGY-001 #4872](https://github.com/Luminous-Dynamics/symtropy/issues/4872)

## Paradigm

A scan pipeline's `registered=true` flag is not sufficient evidence for an inter-pass decision. V0 qualifies the measurement receipt independently: source identity and bytes, transform identity, coordinate frames, calibration/configuration revision, profile counts, residual distribution, coverage, occlusion, uncertainty and freshness must all be explicit.

The result is a synthetic-only metrology admission contract. It is not a physical laser-profiler certification and does not run against BAM raw data.

## Implemented exact transform oracle

The dependency-free Rust library fits a 2D proper rigid transform (rotation + translation) using exact point correspondences and least-squares Procrustes/Kabsch in two dimensions. It rejects too few/non-finite/degenerate correspondences and emits RMS, P95 and maximum residuals.

Four asymmetric known-answer cases freeze identity, positive rotation/translation, negative rotation/translation, and large translation. Tolerance is 1e-10 to 1e-9 in the synthetic coordinate units. Reflections, scale, shear and malformed homogeneous transforms are rejected by the receipt validator.

This does not implement robust point-cloud correspondence search, 3D ICP, scan segmentation or calibrated physical metrology. Those are later adapters and must be separately benchmarked.

## Receipt contract

A valid receipt binds:
- workpiece, joint revision, layer, predecessor pass, scan ID;
- source scan SHA-256 and registration receipt SHA-256;
- source and target frame IDs;
- calibration and algorithm identities/revisions;
- capture tick relative to decision time;
- expected/acquired/matched/rejected/occluded profile counts;
- coverage and occlusion in permille;
- residual RMS/P95/max in declared integer microunits;
- a non-zero 95% uncertainty bound and declared confidence;
- a proper rigid source-to-target homogeneous transform.

The evaluator is fail-closed. It rejects measured/simulated evidence in v0.1, malformed or mismatched digests, frame/identity mismatch, future or stale capture, inconsistent counts, insufficient coverage, excessive occlusion/residual/uncertainty, insufficient confidence, reflection, scale, shear, and invalid homogeneous transforms.

Digests in the unit fixtures are synthetic identifiers. The v0.1 code checks format and equality to the independently expected context; it does not claim to hash unavailable BAM bytes or canonicalize a physical transform receipt.

## Benchmark structure

- 4 known-answer positive 2D rigid transforms.
- 33 negative mutation controls.
- 38 Rust unit tests including transform recovery and fail-closed receipt admission.
- Independent cross-language replay is required before merging.
- The future 3D physical measurement campaign must report calibration traceability, repeatability/reproducibility, registration residual distribution, uncertainty coverage and decision impact by workpiece/layer/scan quality stratum.

## Industry and standards fit

The nearest 2026 adaptive multi-pass GMAW study already measures profiles before and after individual beads and reallocates residual volume; its system includes NDT and macrosection feedback. The new metrology work is not a competing bead planner. It qualifies the measurement evidence that adaptive planners consume.

Relevant published standards context:
- ISO/TR 23247-101:2026: manufacturing digital-twin use case for robotic multilayer/multipass GMAW.
- ISO/TS 8182:2026: arc-energy parameter reporting between WPQR and WPS.
- ISO 3834-2:2021 confirmed current in 2026.
- ISO 15614-1 Edition 3 is still FDIS/final approval at this date; do not claim compliance with the unfinished edition.
- AWS D1.1/D1.1M:2025 is the current US structural-steel code reference where contractually applicable.

These standards do not certify this experimental verifier or replace qualified metrology, procedure qualification or inspection authority.

## Metrics and future physical target

- 100% rejection of the frozen 33 negative controls.
- Exact transform recovery within the frozen synthetic tolerance.
- Zero false acceptance when scan/frame/calibration/predecessor identity changes.
- On permissioned physical data, empirical 95% interval coverage consistent with its stated uncertainty meaning, with interval width and per-WID coverage reported.
- Registration repeatability, spatial coverage, occlusion sensitivity and decision-impact quantified separately.
- Report abstention recall on unusable scans and unnecessary-abstention rate on admissible scans.

## Claim ceiling

PASS means only that the synthetic rigid-transform oracle and synthetic measurement receipt validator satisfy their frozen software contract. It does not establish physical scan accuracy, BAM data quality, weld geometry, process quality, NDT acceptance, WPS qualification or robot safety.
