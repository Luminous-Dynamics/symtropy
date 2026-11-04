# WPT spatial matrix atlas v1

**Status:** software research prototype with synthetic fixtures. It does not qualify hardware, validate an FEM solver, or independently certify upstream physical evidence.

## Purpose

The spatial matrix atlas reconstructs the complete real-valued multi-coil inductance matrix along a declared trajectory from already-qualified endpoint samples. It is designed to reduce expensive dense field solves without losing track of global matrix admissibility.

The critical design choice is to interpolate the **whole matrix**, not each transmitter/receiver coupling independently. For symmetric positive-semidefinite endpoint matrices (L_0,L_1), a convex interpolation

[
\widehat L(\alpha)=(1-\alpha)L_0+\alpha L_1,\qquad 0\leq\alpha\leq1
]

remains positive semidefinite because, for every current vector (i),

[
i^T\widehat L(\alpha)i
=(1-\alpha)i^TL_0i+\alpha i^TL_1i\geq0.
]

This invariant is lost if independently fitted pairwise coefficients are assembled into a matrix or an unconstrained cubic spline overshoots the positive-semidefinite cone.

## Required evidence contract

Each atlas freezes:
- schema version, atlas identifier, geometry digest and matrix kind;
- stable ordered coil IDs and common polarity convention (polarity is assumed to be part of the geometry digest / upstream receipt);
- one operating point: frequency, temperature and DC bias vector;
- strictly increasing positions and complete square H-valued matrices;
- deterministic entrywise absolute uncertainty bounds for each endpoint;
- upstream physical-validator minimum-eigenvalue lower bound, explicit numerical tolerance, matrix digest, source-artifact digest and validator-receipt digest;
- one segment error bound for each adjacent sample pair, stated in Frobenius norm and tied to a validation receipt.

Only `SMALL_SIGNAL` and `DIFFERENTIAL` matrices are admitted. `SECANT` is deliberately excluded. An atlas is not allowed to silently combine samples with different frequencies, bias, temperatures, coil order or geometry digests.

SHA-256 fields are provenance references, not signatures. The consumer must independently fetch, authenticate and verify the referenced matrix/source/validator artifacts. The crate validates their syntax and binds them into each interpolation result; it does not implement a trust root or signature system.

## Endpoint checks

The atlas does not blindly trust an endpoint's claimed passivity margin. It independently recomputes a symmetric Jacobi minimum-eigenvalue estimate and adds its off-diagonal residual Frobenius bound to the entrywise input-uncertainty Frobenius bound. The declared numerical tolerance is subtracted from the independently recomputed eigenvalue margin. The declared endpoint lower bound must be no more optimistic than this recomputed conservative lower bound. A negative conservative lower bound is rejected.

The Jacobi residual bounds the remaining off-diagonal perturbation in the rotated basis. The implementation still uses a floating-point guard and is not a formal interval-arithmetic proof; high-assurance qualification should compare against an independently implemented high-precision eigensolver and preserve that receipt upstream.

## Interpolation and uncertainty propagation

For a request at position (x) between endpoint positions (x_0,x_1), compute

[
\alpha=\frac{x-x_0}{x_1-x_0},\qquad
\widehat L(x)=(1-\alpha)L_0+\alpha L_1.
]

Let (\ell_0,\ell_1\) be the endpoint lower eigenvalue bounds, each already including endpoint matrix uncertainty. Let (\epsilon_{seg}\) be an independently validated upper bound on the spectral/Frobenius norm of the difference between the true full matrix over the complete segment and its convex interpolant. Since (\|\Delta L\|_2\leq\|\Delta L\|_F\), Weyl's inequality gives

[
\lambda_{min}(L(x))\geq
(1-\alpha)\ell_0+\alpha\ell_1-\epsilon_{seg}.
]

If the lower bound is non-negative and the segment method is `ANALYTIC_DERIVATIVE_BOUND` or `FORMAL_INTERVAL_BOUND`, the interpolated matrix is returned as `PASSIVE_CERTIFIED_CONDITIONAL`; endpoint and segment receipts still require external verification. If the method is `HELD_OUT_DENSE_REFERENCE`, the output is `EMPIRICAL_MARGIN_POSITIVE` because discrete held-out positions do not prove the entire continuous interval. If the bound crosses zero, the outcome is `UNRESOLVED`. The tool does not clip eigenvalues, add diagonal jitter, project to PSD, or relabel an unresolved result as a pass.

**The segment error bound is the critical evidence item.** A bound evaluated only at the midpoint is not a bound over the full interval. Acceptable declared methods are:
- `HELD_OUT_DENSE_REFERENCE`: compare against withheld dense FEM/metrology positions and justify why the bound covers the full segment (not just observed points);
- `ANALYTIC_DERIVATIVE_BOUND`: prove a full-segment derivative/curvature bound for the admitted model;
- `FORMAL_INTERVAL_BOUND`: interval or validated-numerics bound covering the full segment.

The current v1 prototype checks that the method label and validation receipt reference are present; it does not independently validate the mathematical proof contained in that receipt.

Extrapolation outside the sampled position domain is always rejected.

## Adaptive refinement

`refinement_plan` ranks segment midpoints by normalized error excess and passivity-margin deficit; `refinement_candidates` returns those candidate positions in priority order. The score is an engineering heuristic, not a formal value-of-information estimate. Candidate segments are those where:
- the segment error bound exceeds the configured maximum; or
- the conservative endpoint margin minus segment error is below the required passivity margin.

A production adaptive sampler should refine further using held-out matrix residuals, geometry curvature, downstream energy uncertainty, coil proximity/overlap, operating-state transitions, FEM cost and uncertainty in the ferrite B–H model. The current ranking uses only the interpolation-error threshold and endpoint passivity margin; it is a deterministic starting policy, not a globally optimal sampling algorithm.


### Service-aware refinement

`service_refinement_plan` consumes exactly one versioned circuit-derived signal per atlas segment: a route-energy uncertainty upper bound in Wh, allowable Wh budget, strictly positive expected fractional uncertainty reduction per added solve, estimated FEM/measurement solve cost, circuit-model digest and sensitivity-receipt digest. Missing or duplicate segment coverage fails closed. It ranks triggered segments by normalized model/service risk times expected reduction divided by estimated solve seconds.

This lets the atlas spend field solves where matrix error most affects useful battery energy, not merely where a raw matrix norm is largest. The atlas does not derive Wh uncertainty from inductance error; this requires a validated circuit sensitivity receipt. Digests are references, not signatures. JSON contract: `benchmarks/wpt-matrix-atlas/v1/service-sensitivity-signal.schema.json`; frozen two-segment fixture: `benchmarks/wpt-matrix-atlas/v1/synthetic-service-sensitivity-signals.json`. The ranking is a transparent heuristic rather than an optimal value-of-information algorithm.
## Synthetic known truth

`synthetic-known-truth.json` has two 2-coil endpoint matrices at 0 m and 10 m. At 5 m:
- interpolated matrix is `[[9.5e-6, 1.5e-6], [1.5e-6, 8.5e-6]] H`;
- endpoint lower eigenvalue bounds average to 7.2 µH;
- segment residual bound is 0.2 µH;
- resulting conservative lower eigenvalue bound is 7.0 µH;
- disposition is `PASSIVE_CERTIFIED_CONDITIONAL` because the fixture's segment-bound method must be externally verified before it can be treated as a continuous-segment certificate.

The fixture's source/receipt digests are synthetic placeholders; its segment method is labelled `ANALYTIC_DERIVATIVE_BOUND` only to exercise the conditional-status path. The result is not an actual derivative proof or measured WPT evidence.

## CLI

```sh
cargo run --locked -p wpt-matrix-atlas -- \
  benchmarks/wpt-matrix-atlas/v1/synthetic-known-truth.json 5.0 PASSIVE_CERTIFIED_CONDITIONAL
```

The optional third argument asserts an expected disposition; CI supplies `PASSIVE_CERTIFIED_CONDITIONAL` so an `UNRESOLVED` result cannot silently pass the smoke test. An `UNRESOLVED` result exits nonzero even when no expected disposition is provided. The CLI emits a JSON record containing the interpolated matrix, position fraction, segment bound, conservative lower eigenvalue margin and the upstream receipt digests used. Downstream code must branch on the explicit disposition and independently verify the referenced receipts.

## Research baseline and proposed targets

A 2026 dynamic-WPT coupled FEM/circuit study reports K(x) extraction at 61 positions along a 3 m pass, at 85 kHz / 30 cm air gap, with average nonlinear FEM solve time around 90 seconds per position. Its published experiment compares 3–7 receiver coils; five coils maximize integrated harvested energy for that one geometry. This is a source-reported simulation baseline, not a universal optimum or hardware timing guarantee. The study's normalized K matrices are kept in a separate audit dataset on the inductance-matrix validator branch because they are not typed as H-valued SMALL_SIGNAL or DIFFERENTIAL measurements.

Proposed atlas research targets (not achieved):
- no more than 18 expensive field solves instead of the 61-position reference (≥70% reduction);
- held-out full-matrix Frobenius relative error P95 ≤1%, max ≤2%;
- held-out absolute K error P95 ≤0.005;
- route-integrated receiver energy error ≤0.5% versus the dense reference;
- peak transfer-position error ≤2 cm;
- zero false-pass passivity outcomes; unresolved intervals trigger refinement or conservative fallback;
- no extrapolation outside the sampled position/operating envelope.

Every accuracy target must be measured on frozen held-out positions that were not used to choose refinement locations or stop the sampler.

## Scope and claim ceiling

V1 supports real-valued reciprocal small-signal/differential inductance matrices at a single fixed operating point per atlas. It does not model dynamic eddy-current memory, complex frequency-dependent impedance, active/nonreciprocal circuits, nonlinear hysteresis over a trajectory, EMC, exposure or thermal safety. For frequency-dependent multiport passivity use the separate #3546 workstream.

A passing synthetic atlas test proves only that the code follows its declared algorithm on that fixture. It does not prove the FEM model, measured data, physical device, continuous-band passivity, route-level efficiency, safety or standards compliance.
