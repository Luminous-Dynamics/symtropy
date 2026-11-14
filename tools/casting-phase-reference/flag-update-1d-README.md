# 1D sparse flag-update enthalpy reimplementation

This standalone Rust benchmark independently implements the 1D isothermal flag-update algorithm described in Peters et al. (2025), IJHMT 249, 127166, Sections 4.1–4.4. It is **not** a port of the authors' MATLAB supplementary source.

## Numerical method

At each implicit Euler step, the shifted enthalpy is split as E = S + gamma(f) v, where S is latent enthalpy, v is the Kirchhoff potential and f ∈ {-1,0,1} is a solid/mixed/liquid flag. The phase flag makes each step a linear block system. In 1D, mixed nodes set v=0 and the remaining non-mixed nodes form contiguous tridiagonal segments that can be solved with a Thomas sweep. The mixed-node latent enthalpy is then recovered from the PDE row. Flags are updated according to the paper's consistency rules until stable.

This implementation uses a tridiagonal-segment reduction rather than allocating a dense 2N×2N matrix. A test-only dense Gaussian-elimination oracle checks the reduced solve for several mixed/solid/liquid flag patterns.

## Grid convention

The paper's general notation gives N gridpoints and dx=D/(N-1), but its 1D table says N=48, D=6m, dx=0.125m. The benchmark therefore interprets N as intervals and uses N+1 endpoint-inclusive points to match the tabulated spacings. This is an explicit reproduction interpretation, not a claim that the article's notation is internally consistent or that the MATLAB implementation has been reproduced.

## Build and run

    rustc --edition=2024 -D warnings --test tools/casting-phase-reference/flag_update_1d.rs -o /tmp/flag-update-tests
    /tmp/flag-update-tests

    rustc --edition=2024 -D warnings tools/casting-phase-reference/flag_update_1d.rs -o /tmp/flag-update
    /tmp/flag-update --verify
    /tmp/flag-update --csv

## Metrics

Reports paper-style mean absolute temperature error and mean absolute interface error over all grid points/timesteps. The primary interface is the linear interpolation where latent component S=rho*L/2. Also reports final interface, mean/max flag iterations, tridiagonal segment solves, PDE residual, wall heat and global enthalpy conservation.

The paper explicitly reports that interface error can be zero by construction when a variable time-step pins the front at a node, while temperature error is nonzero. Therefore no single metric is treated as a substitute for the others.

## Claim ceiling

This is an independent 1D reimplementation from published equations. It is not a port or reproduction of the authors' supplementary MATLAB implementation, not physical foundry validation, not an alloy model, and not a production-qualified solver.


## First exact-head curve and literature gap

The first five-level campaign has now been frozen separately as `flag-update-1d-expected.csv` and is compared byte-for-byte in the dedicated workflow.

At N=768, Nt=1152 the Rust implementation reports:
- mean absolute temperature-field error: 7.310334766222e-4 °C;
- RMS temperature error: 5.781491776676e-3 °C;
- mean absolute interpolated interface error: 4.827953511591e-4 m;
- final-time interface error: 5.890537227409e-4 m;
- average flag iterations: 1.18663194;
- relative global energy residual: 1.015445146377e-12.

The published Peters et al. (2025) Table 6 reports 1.961e-5 °C and 8.919e-5 m for the same named N/Nt configuration. This independent implementation therefore does **not** yet reproduce the published accuracy (about 37.3× field error and 5.4× interface error). The dedicated internal checks validate the Rust code against its own discretized equations, not the paper's MATLAB implementation. See [CASTING-PHASE-001G #2965](https://github.com/Luminous-Dynamics/symtropy/issues/2965) for the one-step reproduction audit. No superiority claim is made.


## Grid-convention audit (separate from the canonical table run)

The paper contains two statements that cannot both be literally true for endpoint-inclusive nodes:
- Section 4.4 writes N points at x_i=(i−1)D/(N−1).
- Table 1 says N=48, D=6m, Δx=0.125m, implying Δx=D/N and 48 intervals.

The new `--grid-audit` mode executes both interpretations with the same nominal N and Nt:
- `table_N_intervals_dx_D_over_N`: N intervals, N+1 endpoint-inclusive gridpoints, dx=D/N (the existing frozen run).
- `section_4_4_N_endpoint_points_dx_D_over_N_minus_1`: N endpoint-inclusive gridpoints, N−1 intervals, dx=D/(N−1).

The audit emits both error metrics, PDE residual, energy residual and flag-update work. It is diagnostic only and does not change the canonical `--csv` or its golden file. The values should be compared with Table 6, but they do not resolve the separate initial-boundary and printed-matrix sign ambiguities.


### Dual grid-convention results (30 October 2026)

The new exact-head `--grid-audit` mode tests the two literal readings of the article:

| Nominal N | Convention | Actual points | Δx (m) | Field MAE (°C) | Interface MAE (m) |
|---:|---|---:|---:|---:|---:|
| 48 | Table: N intervals | 49 | 0.1250000 | 2.70345e-2 | 8.95321e-3 |
| 48 | Section 4.4: N endpoints | 48 | 0.1276596 | 2.79532e-2 | 9.08511e-3 |
| 96 | Table: N intervals | 97 | 0.0625000 | 1.13292e-2 | 4.42647e-3 |
| 96 | Section 4.4: N endpoints | 96 | 0.0631579 | 1.15043e-2 | 4.47592e-3 |
| 192 | Table: N intervals | 193 | 0.0312500 | 4.68880e-3 | 2.08298e-3 |
| 192 | Section 4.4: N endpoints | 192 | 0.0314136 | 4.75614e-3 | 2.07923e-3 |
| 384 | Table: N intervals | 385 | 0.0156250 | 1.86568e-3 | 9.91706e-4 |
| 384 | Section 4.4: N endpoints | 384 | 0.0156658 | 1.84220e-3 | 9.97161e-4 |
| 768 | Table: N intervals | 769 | 0.0078125 | 7.31033e-4 | 4.82795e-4 |
| 768 | Section 4.4: N endpoints | 768 | 0.00782269 | 7.41154e-4 | 4.84017e-4 |

The convention choice changes the finest-grid field error by about 1.4% and interface error by about 0.25%, so it does not explain the 37× field-error gap with Table 6. Both curves preserve energy and PDE residuals to roundoff. This is now a frozen golden artifact and will be byte-compared in the dedicated workflow.
