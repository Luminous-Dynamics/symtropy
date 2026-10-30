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
