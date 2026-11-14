# Two-phase superheated Stefan explicit finite-volume baseline

This is a conservative explicit cell-centred finite-volume enthalpy baseline for the one-dimensional two-phase Stefan benchmark in Peters et al. (2025), IJHMT 249, 127166. It is not the published implicit flag-update algorithm.

## Published parameter set

rho=1 kg/m³; cp_s=cp_l=2.5e6 J/(kg K); k_s=k_l=2 W/(m K); L=1e8 J/kg; Tw=-10 °C; Tm=0 °C; T∞=2 °C; D=6 m; end time 288 h.

The article tabulates N=48,96,192,384,768 and dx=D/N, although the methods section also gives a generic N-gridpoint formula using D/(N-1). This implementation declares N to be the number of finite-volume cells and uses dx=D/N. It matches the tabulated spacings without claiming source-code-level reproduction of the authors' MATLAB experiment.

## Numerical method

- Shifted enthalpy H=E-rho*L: liquid H>0, mixed phase -rho*L<=H<=0, solid H<-rho*L.
- Conservative cell-centred fluxes; the cold wall uses half-cell distance and the far boundary has zero heat flux.
- Explicit time integration with effective Fourier number <=0.2. The solver chooses all stable substeps; it does not use the paper's implicit Nt values.
- Interface estimate is the linearly interpolated location where solid fraction is 0.5, equivalent to the paper's latent-component threshold S=rho*L/2 for this shifted enthalpy.
- Also reports integrated solid-equivalent length sum(fs*dx).
- Temperature-field error follows the paper's mean absolute form: mean over sampled times and cells of |T_num-T_exact|. Interface error is mean absolute front-position difference over the same times. At most 1,152 evenly spaced solution states are sampled for scoring; all explicit substeps execute.
- RMS temperature error is an additional metric. Global wall heat / enthalpy closure, effective Fourier number and far-boundary state are reported too.

The paper demonstrates why interface error alone is insufficient: its variable-timestep variant pins the interface to grid locations and therefore has zero interface error by construction, while its temperature-field error is substantially larger. Compare field, interface, energy and cost together.

## Build and run

    rustc --edition=2024 -D warnings --test tools/casting-phase-reference/two_phase_fv.rs -o /tmp/two-phase-fv-tests
    /tmp/two-phase-fv-tests
    rustc --edition=2024 -D warnings tools/casting-phase-reference/two_phase_fv.rs -o /tmp/two-phase-fv
    /tmp/two-phase-fv --verify
    /tmp/two-phase-fv --csv

## Claim ceiling

This is an explicit 1D numerical baseline against an analytical mathematical problem. It is not an exact reproduction of the authors' MATLAB implementation, an alloy/foundry model, physical validation, or production qualification.


## Exact-head baseline results

The first GitHub Actions execution of the explicit baseline produced the following deterministic five-grid curve. The exact-head golden CSV is checked byte-for-byte by the dedicated workflow.

| Cells | Explicit steps | Mean absolute temperature error (°C) | RMS temperature error (°C) | Mean absolute half-solid-front error (m) | Relative energy residual |
|---:|---:|---:|---:|---:|---:|
| 48 | 266 | 2.5899e-2 | 1.4184e-1 | 1.00238e-2 | 4.10e-16 |
| 96 | 1,062 | 1.1395e-2 | 6.4899e-2 | 4.61307e-3 | 3.27e-15 |
| 192 | 4,247 | 4.4603e-3 | 2.7579e-2 | 2.12523e-3 | 2.86e-15 |
| 384 | 16,987 | 1.7049e-3 | 1.2039e-2 | 1.00450e-3 | 3.87e-15 |
| 768 | 67,948 | 6.2729e-4 | 5.1098e-3 | 4.83459e-4 | 3.47e-15 |

The effective Fourier number remains at or below 0.2. The insulated far-boundary cell remains liquid at approximately 1.99998 °C. These are mathematical verification results for the stated two-phase Stefan case, not foundry measurements.

At the finest mesh, the mean temperature error is about 32 times the paper's Peters flag-update value at N=768 (1.961e-5 °C in Table 6), and our explicit run takes 67,948 steps versus the paper's 1,152 implicit time increments for that reported configuration. The hardware and execution protocols are not matched, so this is not a runtime comparison. It clearly identifies the next algorithmic gap: implicit methods and adaptive resolution must reduce cost per validated accuracy while preserving temperature-field, interface and energy metrics.
