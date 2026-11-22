# Orbital truss as-built geometry benchmark v0.1

Parent protocol: [#4263](https://github.com/Luminous-Dynamics/symtropy/issues/4263). Executable child scope: [#6810](https://github.com/Luminous-Dynamics/symtropy/issues/6810).

This crate implements the first executable slice beneath Symtropy #4263 / #6810: **bounded 2D as-built geometric conformance**.

It is deliberately not a new truss solver, point-cloud engine, CAD kernel, GD&T engine, NDE/POD model, stiffness estimator or structural acceptance authority.

## Core rule

A common-mode position error must not disappear because the inspected article was best-fit aligned back onto its nominal geometry. V0.1 therefore accepts only a declared rigid transform derived from independent external datums and explicitly rejects the article-derived-fit path.

For every required feature:

`r = ||R x_{survey} + t - x_{nominal}||_2`

`u = u_{feature} + u_{registration}`

The uncertainty terms are added conservatively. No independence, RSS cancellation, Gaussian distribution or covariance model is assumed.

`lower = max(0, r-u)`, `upper = r+u`.

- `ACCEPT` only when `upper <= tolerance`.
- `REJECT` when `lower > tolerance`.
- Otherwise `INDETERMINATE_HUMAN_REVIEW`.

A definite feature violation produces REJECT even if a different required feature is unobserved. Incomplete or invalid evidence can never produce ACCEPT.

## Frozen synthetic corpus

Thirteen cases cover nominal acceptance, common-mode datum offset, rigid-frame metamorphism, reflection, scale, missing critical coverage, wrong serial, stale revision, expired calibration, guard-band overlap, article-derived registration, duplicate feature measurements, and a certain oversize feature.

The evaluator reports per-feature residuals, combined uncertainty radius, lower/upper error bounds, tolerance and reason codes. The receipt binds the input bytes and source commit and always carries:

- `authority_granted=false`
- `structural_acceptance_authorized=false`
- `flight_authorized=false`
- `physical_validation=NOT_ASSESSED`

## What this establishes

Only deterministic software semantics for the frozen synthetic cases. The provided SHA-256 and calibration-window checks do not authenticate the survey instrument, calibration laboratory, external datum network, physical article or producer. It does not prove metrology traceability or physical truth.

## Benchmark targets (proposed, not achieved)

- zero false accepts on held-out critical negatives;
- 100% explicit abstention for missing observations, stale revision, invalid calibration, invalid registration and uncertainty overlap;
- invariance under equivalent rigid-frame changes;
- independent oracle agreement on frozen and sealed holdout corpora;
- report false-accept / false-reject / indeterminate rates by failure family, not only one aggregate score.

## Comparative boundary

NIST STEP File Analyzer and Viewer, CAx-IF AP242 recommendations and commercial CAD/CMM systems remain the specialist reference points for product definition, PMI, inspection programming and conformance. This crate begins downstream of those systems: it evaluates a small, transparent, reproducible contract over already-normalized feature observations. It does not replace NIST SFA, PC-DMIS, CAD/CAM, QIF validators or a metrology system.

## Build

```sh
cargo fmt --manifest-path tools/orbital-truss-asbuilt-geometry-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-truss-asbuilt-geometry-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-truss-asbuilt-geometry-bench/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/orbital-truss-asbuilt-geometry-bench/Cargo.toml -- \
  docs/research/orbital-truss/fixtures/asbuilt-geometry-v0_1.json <40-hex-source-commit>
```

The Python reference implementation is an independent standard-library oracle; it is not part of the production evaluator.


## Standards relationship and metrology boundary

This v0.1 evaluator interprets `measurement_uncertainty_m` and `registration_uncertainty_m` as **declared hard upper bounds on radial error**, not standard uncertainties, standard deviations or coverage intervals. It combines them by worst-case addition because no justified independence model is supplied. A GUM Type A/Type B standard uncertainty must not be inserted directly into these fields.

The relevant industrial references are:

- [ISO 14253-1:2017](https://www.iso.org/standard/70137.html), confirmed current in 2023, defines conformity/nonconformity decision rules for dimensional specifications while taking measurement uncertainty into account. This evaluator is not an ISO 14253-1 implementation or compliance claim.
- [JCGM 100:2008 / GUM](https://www.bipm.org/en/doi/10.59161/jcgm100-2008e) sets general methods for evaluating and expressing measurement uncertainty; the BIPM/JCGM publication list now includes the 2026 amendment on nonlinearity and the GUMSuite:2026 collection. V0.1 is intentionally simpler than that framework.
- [NIST STEP File Analyzer and Viewer 5.41](https://www.nist.gov/services-resources/software/step-file-analyzer-and-viewer) supports AP242, syntax checks and semantic/graphic PMI conformance reporting against CAx-IF recommended practices. It is a product-definition/interoperability baseline, not the same task as this geometry-only disposition evaluator.
- [CAx-IF Recommended Practices](https://www.mbx-if.org/home/cax/recpractices/) include Cross-Domain Exchange for Downstream Uses v1.0 and STEP/QIF trace-information Test Cases 7 and 8. These are the preferred rights-cleared downstream interop references for the next integration layer.
- [Hexagon PC-DMIS 2026.2](https://hexagon.com/about/news-media/media-alerts/2026/pc-dmis) adds stronger complex-geometry inspection and STEP AP242 semantic-PMI to QIF planning / UUID traceability. It is a mature commercial comparator to integrate with, not replace.
- NASA's [Design for Inspection](https://ntrs.nasa.gov/citations/20240003953) and [Inspectability Metric](https://ntrs.nasa.gov/citations/20240007593) put defect findability and variability into early design optimization. Our geometry-only evaluator does not reproduce that metric.

## Competitive benchmark ladder

A fair comparison must use identical geometry, feature definitions, coordinate frames, tolerance limits, raw observations, calibration scope, uncertainty interpretation and sealed train/holdout partitions.

1. **Software semantics:** this evaluator vs independent hand-enumerated cases, a minimal checklist, and equivalent adapters. Score critical false accepts, false rejects, abstentions, identity/revision rejection, and transformation invariance by failure family.
2. **Interoperability:** consume rights-cleared CAx-IF AP242/QIF Test 7/8 packages through NIST SFA and instrumentable CAD/CMM adapters. Score supported semantic assertions, silent loss, wrong feature/serial joins and manual reconciliation time; retain unsupported/ambiguous fields instead of guessing.
3. **Metrology:** use the same physical article measured by two independently calibrated systems; freeze traceable datum realization, instrument uncertainty budgets, registration procedure and reference artefacts before testing. Compare measured deviations and conformity decisions under the agreed ISO 14253-1/JCGM decision profile.
4. **Structural validation:** only after geometry and metrology are qualified, compose with the separate truss stiffness/load-response and external FEM qualification owners. Geometry ACCEPT is not structural ACCEPT.
5. **Industrial deployment:** report inspection planning time, setup/re-registration time, operator hours, rework, cost per accepted article and data portability across tool vendors. Compare matched cases and report uncertainty/coverage, not marketing feature counts.

### V0.2 research targets (proposed)

- 3D rigid-body registration with independently surveyed external datum networks and a full pose-error budget.
- Covariance-aware and/or Monte Carlo uncertainty propagation tied to a named ISO 14253-1/JCGM decision rule; preserve common-mode and correlated terms explicitly.
- Semantic GD&T checks for position, orientation, profile and datum reference frames through AP242 PMI → QIF characteristics → measured results.
- Joint/member/bay/assembly tolerance propagation, critical-feature coverage and design-for-inspection trade-offs.
- Independent reference agreement and zero false acceptance on a pre-registered, sealed critical-negative set.

These are research targets, not measured performance. This branch does not establish metrology traceability, physical article conformance, structural capacity or flight qualification.
