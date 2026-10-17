# CIV-INFRA Measurement Compatibility v0.1

A small, standalone Rust evaluator for deriving **possible measurement outcomes** from typed state intervals, a declared measurement transform, and versioned outcome bins. It is a follow-on to the partition-based decision planner in PR #1799 and implements the first conservative core for issue #1800.

## Run

\`\`\`sh
cargo fmt --manifest-path tools/civ-infra-measurement-compatibility/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civ-infra-measurement-compatibility/Cargo.toml
cargo run --locked --manifest-path tools/civ-infra-measurement-compatibility/Cargo.toml -- \
  tools/civ-infra-measurement-compatibility/fixtures/compatibility-v0.1.json \
  /tmp/civ-infra-compatibility-report.json
\`\`\`

The public byte API hashes the exact raw JSON bytes itself, parses those same bytes, and binds their digest to the report; callers cannot provide a replacement digest. Inputs above 4 MiB are rejected before parsing.

## Core semantics

- Input ranges and observation values are typed as meters, millimeters or dimensionless quantities. Values are integer fixed-point ticks: 1 tick unit = 1e-6 of the declared unit.
- Length intervals are normalized to nanometer ticks so meter/millimeter comparisons do not rely on floating point.
- \`identity\` is supported for compatible units. A bounded, non-negative \`square_non_negative_dimensionless\` transform demonstrates conservative nonlinear propagation: output endpoints are rounded outward. It is intentionally not a general expression language.
- Intervals and outcome bins are **closed**. If an output interval touches two bins at the same boundary, both outcomes are possible. Overlapping bins and one state mapping to multiple outcomes are intentional.
- A state interval not wholly inside the model's declared domain is marked out-of-domain and remains compatible with every declared outcome. It is not silently clipped.
- Every model carries an explicit, typed additive discrepancy interval and a basis statement. That envelope widens the transformed output interval with checked arithmetic; overflow fails closed. The fixture's zero-discrepancy value is a synthetic assumption, not calibration evidence.
- If the predicted output interval extends into a region not covered by any outcome bin, the report flags the uncovered region.
- If an observation matches no declared bin, or matches a bin but no state envelope supports it, the status becomes \`out_of_model_support\`; the prior state set is preserved. Worst-case metrics over declared bins are conditional on declared support and are reported separately from the all-prior-states support-failure bound.
- Cost or outcome likelihoods are not inferred. The report is a possibility/compatibility result, not a probability model.
- The report labels worst-case ambiguity across declared bins as conditional on the declared support model. It also reports the support-failure branch explicitly: no state is eliminated, and the full prior decision-signature ambiguity remains.

## Evidence and tests

- Frozen four-state, three-outcome synthetic fixture with meter/millimeter conversion and a supported observation that leaves multiple states possible.
- 25 authored Rust tests for unit conversion, nonlinear outward rounding, overlapping boundaries, unsupported observations, out-of-domain state handling, model-support gaps, provenance syntax, order invariance, and a direct interval-overlap oracle.
- Generated suite: 100 seeded worlds × 20 states × 5 outcome bins = 10,000 state/outcome evaluations. Fifty worlds use length identity (meter/millimeter-compatible) and fifty use the nonlinear dimensionless square; 50 worlds also carry non-zero synthetic additive discrepancy envelopes. The independent oracle compares the full state/outcome edge set, independently propagating square bounds with outward rounding and independently applying the declared discrepancy interval.
- CI writes full generated inputs, SHA-256 digests, reports, reference edge counts and an artifact checksum manifest. The workflow is scoped to this crate plus its protocol and checks out the exact PR head.
- Pre-allocation cap: 100,000 state/outcome combinations, deliberately conservative for v0.1 memory use.

## Claim ceiling

Model and upstream-report SHA-256 values are syntax-checked and preserved, but v0.1 does not retrieve or verify those artifact bytes. Model-discrepancy bounds are declared inputs, not empirical error bounds unless their basis is independently qualified. A model reference is a traceability field, not proof of model authenticity. The model family is intentionally small: only identity and a bounded non-negative dimensionless square are implemented. Open endpoints, arbitrary nonlinear expressions, correlated multivariate inputs, censored readings, calibration likelihoods, missingness mechanisms and Bayesian updates are not supported. Unsupported behavior must be rejected or remain conservatively ambiguous, never silently approximated.

All fixtures are synthetic. This crate is not a hydraulic, utility, terrain, sensor, laboratory or metrology-qualified model. A CI pass would establish only the declared software semantics.

## Research anchors

- BIPM/JCGM GUM-6:2020: developing and using measurement models — https://doi.org/10.59161/JCGMGUM-6-2020
- BIPM/JCGM 100:2008/Amd.1:2026: amendment on nonlinearity in measurement models — https://doi.org/10.59161/PPDI3267
- NIST (5 October 2026): risk-based model credibility, VVUQ and uncertainty quantification — https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing
