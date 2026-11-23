# CMNI spatiotemporal material twin

**Status:** model architecture, v0  
**Tracking:** https://github.com/Luminous-Dynamics/symtropy/issues/6518

## 1. Objective

Represent the irradiated metal hydride as a local, evolving state field rather than one bulk-average target with one screening parameter.

The 2026 study reports an approximately 1 cm² beam spot, target-bias-dependent focusing, D+ implantation depths of roughly 4.4–67.5 nm across 0.25–6.5 keV center-of-mass energy, and 250 µm membranes. It also discusses ion-created vacancies/defects, deuterium trapping, diffusion, replenishment and surface losses.

The active reaction region is a tiny near-surface fraction of the membrane. A depth-only model is a useful intermediate approximation, but it cannot represent beam-profile or lateral nonuniformity.

Source: https://doi.org/10.1038/s41467-026-74421-1

## 2. State and input fields

Coordinates: x,y lateral position; z depth into the target; t time.

Material state:
- c_D(x,y,z,t): deuterium number density / occupancy;
- d(x,y,z,t): defect/trap density or calibrated proxy;
- T(x,y,z,t): temperature, only where a calibrated thermal model exists;
- phase, strain, composition and surface/overlayer state;
- sample-specific preparation and irradiation history.

Beam input:
- J_s(x,y,E,t): incident ion current density by species and energy;
- measured species fractions and per-species energy distributions;
- beam footprint, profile uncertainty, spot drift and alignment;
- total charge measured by Faraday cup and independent current diagnostics.

For a singly charged D+ / D2+ mixture with ion-current fractions f1,f2, the deuteron rate is proportional to (I/e)(f1+2f2). Keep the electrical current fraction and deuteron number flux distinct.

## 3. Coupled state evolution

A reduced transport equation may take the form:

∂c_D/∂t = ∇·(D_D ∇c_D) + S_impl(J_s,E,z) + S_echem - L_surface - L_sputter - L_beam + R_trap

A defect-state model may be written:

∂d/∂t = G_damage(J_s,E,z) - k_anneal(T)d - R_recombination

These are model templates, not fully specified constitutive laws. Diffusion coefficients, source terms, boundary conditions, sputtering yields, trap exchange and defect annealing must come from validated specialist models or independent measurements. Unknown quantities remain latent with uncertainty.

Boundary conditions must distinguish:
- the beam-facing vacuum surface;
- the electrochemical-side membrane boundary;
- the bulk foil;
- the actual membrane thickness and any catalytic overlayer.

## 4. Reaction-yield integration

For a local per-deuteron energy distribution, a schematic reaction event rate is:

dN/dt = integral over V of n_D(x,y,z,t) times the integral over E of Phi_D(x,y,z,E,t) sigma(E,U_local) dE dV

The actual implementation must:
- integrate species-resolved energy distributions;
- account for energy loss and stopping with depth;
- avoid double-counting both molecular flux and constituent deuterons;
- distinguish a phenomenological local effective screening parameter from a microscopic measured field;
- propagate uncertainties in c_D, d, stopping, beam profile and detector response.

Detector observation remains a separate forward model after reaction-product generation.

## 5. Spatial beam profiles

The research kernel includes an axisymmetric Gaussian profile as a normalized synthetic baseline:

J_D(r) = F_D,total / (2*pi*sigma^2) * exp(-r^2/(2*sigma^2))

Its integral over the plane equals total incident deuteron rate. The sigma parameter must be supplied, not inferred silently from a reported approximate spot area. A measured 2-D beam map or calibrated radial profile takes precedence.

The 2026 paper describes electrostatic focusing and changes in current capture with target bias. Therefore record spot profile and uncertainty at each energy/target-bias condition. Total charge alone does not uniquely determine local fluence.

## 6. Counterfactual experiments and benchmarks

- Same integrated charge, different beam-footprint width/profile.
- Same peak current density, different total charge.
- Same total charge and spot profile, different species/energy distributions.
- Same bulk-average D loading, different near-surface D profiles.
- Same defect average, different spatial defect distributions.
- Same local material state, different beam raster/drift.

Required tests:
- spatial-profile integral equals measured total beam current/deuteron flux;
- uniform-profile limit recovers the depth-only model;
- depth-only limit recovers existing one-dimensional calculations;
- diffusion manufactured solutions and mass-balance tests;
- mesh/time refinement;
- energy and particle conservation across adapters;
- held-out prediction across materials, spot profiles and campaigns.

No SOTA claim should be made until the richer model beats depth-only and bulk-average baselines on preregistered held-out data at matched input/compute budgets.

## 7. Scope boundary

Symtropy orchestrates and qualifies solver outputs. It does not replace SRIM/TRIM-class ion stopping, DFT, molecular dynamics, defect-kinetics or validated nuclear-data calculations. Physical experiments and beam diagnostics must be performed by appropriately qualified facilities. No lab actuation is included.


## 8. Current isolated transport implementation

A standalone research module, research/cmni/reference-kernel/src/material_state.rs, now provides:
- explicit 1-D cell-centred finite-volume diffusion with zero-flux boundaries;
- a conservative inventory update and a timestep bound dt <= dx²/(2D);
- fail-closed validation for unstable steps, negative/non-finite concentration, and invalid coefficients;
- an exact local constant-source / first-order-loss update.

This is a numerical kernel, not a complete hydride model. It has no calibrated Pd/Ti diffusion coefficients, no beam implantation source profile, no surface Robin boundary, no defect-trapping constitutive law, and no coupled thermal/phase solver. The current manufactured tests cover a uniform invariant, inventory conservation, smoothing, a discrete Neumann cosine mode, stability rejection and exact source/loss limits.

There are 8 additional Rust tests in this module. The combined crate currently has 42 authored tests, still unexecuted in this environment because the Rust toolchain is absent.


## 9. Arbitrary measured 2-D beam-map reconciliation

The research kernel now includes `BeamProfileCell` and `reconcile_measured_beam_profile`. The helper:
- accepts arbitrary cells with x/y coordinates, area and combined singly charged ion-current density;
- integrates area × current density across cells;
- compares the map current to an independent Faraday-cup total;
- fails closed when the relative mismatch exceeds a caller-declared tolerance;
- converts the reconciled cell currents to deuteron rates using separately supplied D+ / D2+ ion-current fractions;
- returns per-cell deuteron rate, normalized deuteron-flux fraction and areal flux density.

It does not silently renormalize an inconsistent map to force agreement. The synthetic fixture `research/cmni/fixtures/arbitrary-2d-beam-profile-faraday-reconciliation-synthetic.json` demonstrates a nonuniform map whose integrated current equals 1.0 µA and a 50%-mismatch negative control that should be rejected.

Current limitations: this helper assumes a combined D+ / D2+ current-density map with uniform species fractions across cells. A production measured input should preserve species-resolved maps when available and support spatially varying species composition and energy distributions.


## 10. Instrument response kernels and isotope-aware observability

The reference crate now contains a separate `measurement.rs` module:
- `DepthResponseKernel` stores a nominal sample depth and a caller-supplied normalized nonnegative response over latent depth cells.
- `apply_depth_observation_operator` maps a latent profile to instrument-level observations.
- The operator supports arbitrary row kernels; it does not assume a Gaussian or shift-invariant blur.

This is deliberately an observation operator, not a simulator of any particular instrument. The instrument adapter must supply calibrated response rows, and separately model calibration, isotope selectivity, efficiency, background and covariance.

Be careful with isotope selectivity:
- The 6.385-MeV `1H(15N,αγ)12C` resonant NRA reaction measures **protium**, not deuterium.
- A D-specific reaction such as `D(3He,p)alpha` has a separate energy/stopping/matrix response.
- SIMS can distinguish H and D with suitable mass resolution, standards and matrix calibration.
- Slow-positron depth response is an energy-dependent Makhovian distribution; lateral beam point-spread is a separate dimension.

The approximately 50-µm PAS number cited in a 2025 Pd methods discussion is a spatial-resolution value, not a depth-resolution value. The current model docs now explicitly keep these axes separate.
