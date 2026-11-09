# BH reference fixtures

This crate defines a strict JSON exchange contract for independently produced Kerr / Teukolsky reference data.

It is deliberately not a Teukolsky solver. Its purpose is to make cross-engine comparisons well-posed before a numerical discrepancy is computed.

## Initial contract

- Pin the field sector `(s, ell, m)`, signed `a/M`, mass, and dimensionless complex `M omega`.
- Pin Fourier sign, Boyer–Lindquist radial coordinate, frequency normalization, angular-eigenvalue branch, and producer-described `In` / `Up` normalizations.
- Store complex values as explicit objects with named `re` and `im` components.
- Require finite radial samples outside the outer horizon, strictly increasing in `r/M`.
- Preserve producer version, source revision, artifact digest, timestamp, and precision.

`ReferenceFixture::validate` checks structural/domain integrity. `assess_compatibility` checks whether two fixtures describe the same pinned problem. A `Matched` compatibility result does not mean the numerical values agree.

A Fourier-sign, coordinate, branch, frequency-normalization, or boundary-normalization mismatch returns `Unmatched`; it is not treated as a solver failure.

## Scope limits

The first schema covers radial reference fixtures and a separate Kerr QNM-sequence record. It does not yet encode source orbits, fluxes, higher derivatives, arbitrary charts, branch-cut metadata, or complete sourced waveforms. These require explicit typed definitions and fixtures. Unknown fields are rejected to prevent silent schema drift.

## Payload identity

The CLI prints a `payload_sha256` digest for every validated radial fixture and QNM sequence. It is computed from deterministic JSON bytes produced by this crate:

- object keys are sorted recursively;
- array order is preserved;
- negative zero is normalized to positive zero;
- non-finite numbers are rejected by the fixture validators;
- `serde_json` is pinned in `Cargo.toml` so its number formatting is reproducible.

This is a schema-defined deterministic JSON profile, not a claim of general RFC 8785/JCS conformance. The producer artifact digest and the payload digest are distinct: one identifies the generating code/artifact, the other identifies the actual serialized fixture content. A change in any fixture value or convention changes its payload digest.

QNM sequence records preserve sequence position independently of an optional overtone index. A prolate/oblate family label requires a multi-mode asymptotic evidence record, but the schema only checks that evidence is present and finite. It does not prove the family classification or assess whether the measured residuals are small enough. Unresolved and Ambiguous remain first-class outcomes.

Reference fixtures are data, not executable instructions. Do not load untrusted notebooks or code from a fixture payload.

## CLI

The crate builds a small command-line validator:

```sh
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- validate-radial fixture.json
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- validate-qnm qnm-sequence.json
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- compare-radial reference-a.json reference-b.json 1e-14 1e-9
```

`compare-radial` validates both inputs, then compares only when problem identity, conventions, radial sample nodes and per-channel coverage match. It performs no interpolation or normalization fitting. It reports per-channel maximum absolute error, maximum tolerance-scaled error, sample count and count of out-of-tolerance values. The explicit absolute and relative tolerances are required CLI arguments.

Statuses are `unmatched` (problem/convention mismatch), `unresolved` (different radial nodes or data coverage), `numerically_agreeing`, and `numerically_disagreeing`.

Exit codes: `0` numerical agreement, `2` malformed input/invalid fixture or tolerance, `4` problem or convention mismatch, `5` numerical disagreement, `6` comparison unresolved.

## WKB and mode-family identity

`KerrQnmSequenceFixture` preserves the conventional overtone index `n` separately from the high-overtone genus-one quantization integer `N`. The Hatsuda normal-prolate method is represented as `KerrNormalProlateGenusOneWkb` and requires `N`, an expansion truncation order, an explicit cycle ID, and a normal-prolate oscillator level. The method is rejected on an `Oblate` family label.

Do not hard-code the prolate angular scaling limit to one. In the normal-prolate expansion, the expected leading ratio is `eta = 2 Lbar + 1`, where `Lbar` is the equatorial oscillator level; for scalar fields `Lbar = ell - |m|`. For nonzero spin weight, the mapping from spherical labels to `Lbar` must be established by branch continuation. Each family-evidence record therefore stores its expected angular-scaling limit explicitly.

The genus-one WKB result is asymptotic and its coefficients grow factorially. Store the chosen truncation order and, when available, the least-term / best-truncation order. A larger truncation order is not automatically more accurate. The outer-scattering cycle is an input, not inferred from the elliptic curve.

## Angular eigenvalue conventions

Radial and QNM fixtures name the angular quantity explicitly with `AngularEigenvalueConvention`: `spin_weighted_spheroidal_a`, `teukolsky_radial_lambda`, or `other`. The last form requires a producer-authored definition string. These values are not interchangeable; an adapter must document any conversion before comparing radial solutions. The angular branch ID is a separate piece of metadata and cannot substitute for the eigenvalue convention.

Generic prolate-family classifications do not infer an equatorial oscillator label from `(s, ell, m)`. The optional normal-prolate oscillator level is required only when a mode is explicitly tagged with the normal-prolate genus-one WKB method. For scalar fields the cited expansion uses `Lbar = ell - |m|`; for nonzero spin weight a mapping must be established by branch continuation before the WKB method is used.

## Method applicability and WKB reproducibility

Each QNM mode stores a producer-reported `method_applicability`: `within_documented_regime`, `extrapolated`, `known_breakdown`, or `unresolved`. This metadata does not certify the claim; it preserves the producer's stated validity assessment so a near-extremal potential-peak WKB failure is not hidden by an aggregate error score.

The normal-prolate genus-one WKB record requires:
- positive sub-extremal spin in the method's declared fixed-spin domain, `0 < a/M < 1`;
- a branch-continuation path ID, explicit cycle ID, quantization integer `N` separate from overtone `n`, and truncation order;
- both elliptic cycle periods `J0` and `J1`, the quantization residual, and a producer-estimated truncation error;
- an explicit normal-prolate oscillator level. The mapping from `(s, ell, m)` is not inferred for nonzero spin weight.

`KerrQnmSequenceFixture::normal_prolate_wkb_small_spin_parameter` exposes the non-uniformity parameter `chi^(-4/3)/(N + 1/2)`. It is a diagnostic, not a universal pass/fail threshold; publish accuracy against this parameter, spin, and truncation order.

The exact-extremal WKB method is a distinct method record and the current fixture validation limits it to scalar-field records. It must not be conflated with the fixed-spin, non-extremal genus-one expansion.

## Solver independence and initial-guess provenance

QNM records keep `solver_method` separate from the mandatory `initial_guess_source`. Sources distinguish an explicit independent guess, BHPT interpolation-table seed, Python `qnm` cache, asymptotic fallback, previous-spin continuation, previous-overtone continuation, `NoInitialGuessRequired`, and explicit `Unknown`. A numeric initial guess cannot be tagged as `NoInitialGuessRequired`; an unknown source remains visible rather than being treated as independent.

This matters for BHPT: its current documentation says `SpheroidalEigenvalue` and `IncidenceAmplitude` use the `Interpolation` result as the default initial guess, falling back to asymptotic expansions where tables are absent. Those methods are algorithmically different but may share seed data. `InterpolationFromPythonQnmCache` is an explicit method label, not an independent numerical confirmation.

The method enum also preserves BHPT's Schwarzschild-only `Spectral1D`, `Large-l Asymptotic`, and `Large-n Asymptotic` routes separately. Their limited background/regime validity should remain visible in each comparison.
## Branch-aware QNM family analysis

`analyze-qnm` independently recomputes asymptotic family diagnostics from a validated sequence. It does not trust the producer's `family_classification`, overtone labels, or prefilled `family_evidence`.

```sh
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- analyze-qnm qnm-sequence.json family-targets.json
```

The `family-targets.json` file supplies **independent** optical/WKB frequency-spacing predictions and optional acceptance tolerances:

```json
{
  "targets": {
    "prolate_spacing_m": { "re": 0.0, "im": -0.255364 },
    "oblate_spacing_m": { "re": 0.0, "im": -0.208309 }
  },
  "config": {
    "minimum_consecutive_modes": 4,
    "maximum_angular_residual": 0.05,
    "maximum_frequency_spacing_residual": 0.05,
    "maximum_angular_drift": 0.05,
    "maximum_frequency_spacing_drift": 0.05,
    "maximum_solver_residual": 1e-8
  }
}
```

The example spacing values are the published asymptotic Kerr references for `a/M = 0.7`; they are not universal constants. Supply targets for the exact spin and convention being tested.

The prolate diagnostic compares `A/(i (a/M) M omega)` with `2 Lbar + 1` for the declared normal angular branch. The oblate diagnostic compares `A/(-(a/M)^2 (M omega)^2)` with 1. Both are checked across the final consecutive-mode window; complex frequency differences are independently compared with the supplied spacings.

The analyzer requires a continuous angular branch, angular ODE residuals, and at least one radial/continued-fraction/incidence residual per mode. It returns `Unresolved` if a mode is marked `KnownBreakdown` or `Unresolved`, branch identity changes, or neither family satisfies both angular and frequency tests. `Ambiguous` is returned when both candidate models pass the caller's declared tolerances. It uses sequence position, not the optional conventional overtone index `n`.

These are asymptotic diagnostics over the supplied window, not a proof that all modes belong to a family, a completeness theorem, or a finite-overtone remainder bound. Report the chosen window, targets, residuals and tolerances with every family claim.

Exit codes: `0` qualified Prolate/Oblate result, `6` Ambiguous/Unresolved, `2` invalid input or unsupported convention.

## QNM analytic sheets and extremal spin traces

The Kerr QNM sequence schema is now version 2. Each mode records a stable `mode_lineage_id`, `riemann_sheet_kind`, `riemann_sheet_id`, `continuation_class`, and `frequency_cut_relation`. These fields are independent of `family_classification`: prolate/oblate is an angular asymptotic family, while conventional/unconventional and Class I/II identify analytic continuation structure.

A separate `KerrQnmSpinContinuationTrace` follows one mode lineage along a strictly monotonic signed `a/M` path on one declared sheet. A branch-cut crossing must start a new trace linked to its parent. Exact extremality is stored as a sample with `|a/M| == 1`; it is never inferred from a near-extremal extrapolation.

Finite-damping and zero-damping classifications require both an exact-extremal sample and a subextremal fit-evidence record. The record includes the fitting interval, number of fit samples, estimated damping limit, uncertainty, fit residual, optional damping-versus-surface-gravity exponent, and the exact-extremal detuning from the signed superradiant bound.

The dimensionless extremal bound is `M omega_SR = sign(a) m/2`. Therefore the detuning is `M omega - sign(a) m/2`; for negative `m` and positive spin the bound itself is negative. The validator recomputes this detuning from the exact sample and rejects an inconsistent producer value. For gravitational (`s != 0`) exact-extremal samples, the explicit double-confluent Heun method is required; a nonextremal solver result cannot stand in for a direct extremal solve.

Validate a spin trace with:

```sh
cargo run --manifest-path tools/bh-reference-fixtures/Cargo.toml -- validate-continuation qnm-spin-trace.json
```

The trace validator checks identity, monotonic spin order, angular-branch continuity, per-sample residuals, exact-extremal method selection and superradiant detuning consistency. It does not itself prove that the damping-limit fit is physically correct; that requires the numerical reproduction and uncertainty analysis tracked by #4415.
