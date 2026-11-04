# WPT passivity-preserving spatial matrix atlas v1

**Status:** research software; synthetic fixture only. No hardware, FEM source matrix, or physical WPT route has been qualified by this tool.

## Problem

Dynamic WPT simulation often evaluates coupling or inductance at many positions. A dense reference sweep can be expensive, while fitting each mutual-inductance entry independently can yield a globally inadmissible matrix between samples. The benchmark should reduce expensive field solves without hiding physical invalidity or uncertainty.

This crate accepts already-qualified, complete reciprocal inductance matrices and interpolates the **whole matrix** inside the sampled position domain. It does not run FEM, estimate the matrix, verify hardware identity, or validate receipt signatures.

## Mathematical invariant

For endpoint matrices \(L_0,L_1\succeq0\) and \(0\leq\alpha\leq1\),

\[
\widehat L(\alpha)=(1-\alpha)L_0+\alpha L_1
\]

is PSD because for every current vector \(i\),

\[
i^T\widehat L(\alpha)i=(1-\alpha)i^TL_0i+\alpha i^TL_1i\geq0.
\]

If the true position-dependent matrix differs from the interpolant by \(E(x)\) and a validated bound establishes \(\lVert E(x)\rVert_2\leq\epsilon\) over the entire segment, Weyl's inequality gives

\[
\lambda_{\min}(L(x))\geq(1-\alpha)\ell_0+\alpha\ell_1-\epsilon,
\]

where each endpoint lower bound \(\ell_j\) already includes its matrix-entry uncertainty and numerical guard.

This is not permission to invent \(\epsilon\). The segment bound must be backed by a source/validation receipt and an allowed method. The code distinguishes:
- `FORMAL_INTERVAL_BOUND` and `ANALYTIC_DERIVATIVE_BOUND`: a continuous-segment proof may be claimed, **conditional on external verification of the receipt and its derivation**;
- `HELD_OUT_DENSE_REFERENCE`: establishes empirical residuals at tested held-out positions only. The output is `EMPIRICAL_MARGIN_POSITIVE`, never a continuous passivity certificate;
- negative or unresolved lower bound: `UNRESOLVED`.

The CLI does not verify cryptographic signatures or retrieve external receipts. A digest is a content identifier, not proof of authorship or correctness.

## Endpoint validation

Before interpolation, the crate independently recomputes a conservative endpoint eigenvalue lower bound from:
1. The complete symmetric H-valued matrix.
2. Its nonnegative entrywise absolute-error bounds, combined using a Frobenius-norm upper bound.
3. A deterministic Jacobi eigensolver residual bound.
4. The declared upstream numerical tolerance.

It rejects non-finite/asymmetric matrices, invalid coil identity, non-positive self-inductance lower bounds, non-PSD endpoint envelopes, and any upstream claimed eigenvalue lower bound more optimistic than this independent recomputation. It never applies diagonal jitter, clips a negative eigenvalue, flips polarity, or projects a matrix into the PSD cone.

The entrywise matrix errors and segment interpolation error are reported separately: the latter is a **matrix Frobenius-norm bound**, not another per-entry tolerance.

## Adaptive refinement

`refinement_plan` ranks segment midpoints by normalized interpolation-error excess and passivity-margin deficit; `refinement_candidates` preserves the simpler position-only API. This is a transparent first deterministic policy, not an optimal acquisition or value-of-information function. It currently does not consume circuit-energy sensitivity, spatial curvature estimates or compute budgets. The next research layer should compare midpoint refinement against error-gradient, curvature, matrix-spectrum, downstream energy-sensitivity and expected value-of-information policies.

## Input contract

- 2–32 fixed, unique coil IDs and one polarity convention implicit in the already-qualified symmetric matrices;
- matrix kind `SMALL_SIGNAL` or `DIFFERENTIAL` at one fixed frequency/temperature/bias operating point;
- strictly increasing position samples;
- complete square H-valued matrix and entrywise deterministic absolute-error bound for every sample;
- independently checkable nonnegative eigenvalue lower bound plus numerical tolerance;
- source, matrix and upstream validator receipt digests;
- one segment residual bound and validation receipt for each adjacent pair.

The JSON Schema is a structural contract. Runtime validation additionally enforces cross-field dimensions, increasing position, adjacent segment indices, matrix symmetry and independently recomputed eigenvalue consistency.

## CLI

```sh
cargo run --locked -p wpt-matrix-atlas -- benchmarks/wpt-matrix-atlas/v1/synthetic-two-coil-atlas.json 5.0
```

Out-of-domain requests fail with `EXTRAPOLATION_REJECTED`. No extrapolation or silent extrapolation fallback is implemented.

## Service-aware field-solve allocation

The crate also accepts external `ServiceSensitivitySignal` records from a validated circuit/route-energy evaluator. The input contract is frozen in `service-sensitivity-signal.schema.json`, with a two-segment known-truth fixture in `synthetic-service-sensitivity-signals.json`. Every atlas segment must have exactly one versioned signal binding a conservative route-energy uncertainty upper bound in Wh, an acceptable Wh budget, a strictly positive estimated fractional uncertainty reduction from one additional field solve, an estimated solve cost in seconds, a circuit-model digest and a sensitivity-receipt digest. Missing or duplicate signals fail closed.

The service-aware score is a transparent heuristic:

[
\text{score}=
\frac{\max(r_{matrix},r_{margin},r_{energy})\;\rho_{reduction}}
{t_{solve}},
\qquad
r_{energy}=\frac{\epsilon_{E,Wh}}{E_{budget,Wh}}.
]

A segment can therefore be selected even when its matrix interpolation error is small, if circuit sensitivity maps that error to excessive route-energy uncertainty. The API does not derive the Wh uncertainty from henries; that mapping must come from a separately validated circuit sensitivity receipt. It checks receipt-digest syntax, not signatures or derivation correctness.

The reduction fraction and solve-cost estimate are planning inputs, not guarantees. The score is **not** an optimal expected value-of-information policy. It should be benchmarked against uniform refinement, midpoint refinement, matrix-residual refinement and a frozen held-out oracle. Missing/invalid sensitivity evidence must be treated as missing—not as zero energy risk.

## Benchmark protocol

Use a frozen dense reference position grid and tune only on training samples. Hold out positions and preserve source FEM/measurement mesh, B–H data, coil order, polarity, frequency, gap and solver settings. Compare:
1. Dense reference FEM.
2. Uniform sparse sampling.
3. Pairwise element interpolation.
4. Unconstrained cubic/spline interpolation.
5. Whole-matrix convex interpolation.
6. Adaptive whole-matrix convex interpolation.
7. A passivity-preserving parametric model-order-reduction baseline.

Metrics:
- expensive solver count and wall time;
- full-matrix relative Frobenius and spectral-norm error;
- P50/P95/max entrywise coupling error;
- minimum eigenvalue lower bound at every held-out position;
- passivity false-pass count (target zero);
- integrated receiver Wh/pass and spatial energy-curve error;
- peak-position error, qualified coverage and out-of-domain detection;
- receiver fairness where the topology has multiple independent receivers.

### Proposed targets (not achieved)

- reduce a 61-position reference to at most 18 expensive FEM/measurement solves (≥70% fewer);
- held-out full-matrix relative Frobenius error P95 ≤1%, maximum ≤2%;
- route-integrated receiver energy relative error ≤0.5%;
- peak-energy-position error ≤2 cm;
- zero false-pass outcomes;
- never extrapolate without an independently qualified extension;
- preserve geometry, source model, solver and receipt digests for every atlas version.

These are proposed research goals. They are not results or industry standards.

## Research and prior art

The 2026 dynamic-WPT study by Garrofy, Nikulshin and Wolfus compares 3–7 vertical receiver coils using coupled FEM/circuit co-simulation at 85 kHz, 30 cm air gap and a 3 m pass. It publishes five aligned-position normalized coupling matrices in Appendix A and describes a 61-position route sweep. Its nonlinear ferrite FEM solve cost is source-reported at about 90 seconds per position. Its five-coil optimum is geometry-specific and based on route-integrated harvested energy, while another coil count can maximize an instantaneous peak.

Passivity-preserving parametric model reduction and matrix interpolation are established prior art; Samuel et al. (2012) describe common-projection and positive interpolation schemes for guaranteed passive parametric reduced models. Therefore this crate does not claim the convexity theorem or passive interpolation generally as novel. The opportunity is WPT-specific evidence integration: complete H-valued matrix identity, geometry-conditioned trajectory atlas, source-linked error bounds, adaptive field-solve allocation, endpoint energy validation and strict out-of-domain behavior.

## Claim ceiling

A positive result is conditional on endpoint and segment receipt verification. It is a mathematical model-envelope result, not proof that measured geometry, FEM material parameters, coil fabrication, receiver alignment, circuit compensation, exposure, thermal behavior or standards compliance are correct. Empirical held-out residuals never become a formal continuous-segment proof merely by adding more sample points.
