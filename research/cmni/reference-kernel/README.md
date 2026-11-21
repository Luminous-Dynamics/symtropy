# CMNI reference kernel

This is a **research-only numerical prototype** for a homogeneous screened D–D thick-target yield integral.

It deliberately does not:
- implement first-principles nuclear physics or a validated nuclear-data library;
- supply a real astrophysical S-factor or material stopping-power dataset;
- represent spatially heterogeneous defects, implantation or hydrogen transport;
- reproduce any published experiment;
- estimate power gain, calorimetric excess heat, or net energy.

The caller must inject sourced, versioned S-factor and stopping-power functions, target density, screening model, and uncertainty metadata. Current tests use synthetic constants only to verify numerical behavior and input rejection.

Run with:

```sh
cargo test --manifest-path research/cmni/reference-kernel/Cargo.toml
```

The kernel is intentionally outside the production workspace pending independent physics review, source-data reconstruction, and numerical verification. See [the physical twin design](../../docs/research/cmni/PHYSICS_TWIN.md).


## Current research-kernel capabilities

- Homogeneous screened thick-target yield integral with explicit center-of-mass energy and stopping-power frame contract.
- Relative-yield normalization (Y(E)/Y(E_{ref})).
- Closed-form homogeneous ALR enhancement and Eq. 13 two-environment mixture.
- Separate literal-Eq.16 and prose/Fig.3c-direction sigmoid variants for the 2026 model reproducibility audit.
- Weighted log-space objective.
- First-order prompt D⁺/D₂⁺ dissociation component mapper with ion-current fractions converted to deuteron-flux fractions.

The D₂⁺ routine is a conditional sensitivity model, not an observed beam decomposition. It does not model dissociation depth or species-specific stopping. The code has not been executed with Cargo in the current work environment; the tests are pending a Rust-capable runner.


## Current research-kernel capabilities (updated)

- Homogeneous screened thick-target yield integral with explicit center-of-mass energy and stopping-power frame contract.
- Relative-yield normalization (Y(E)/Y(E_{ref})).
- Closed-form homogeneous ALR enhancement and Eq. 13 two-environment mixture.
- Separate literal-Eq.16 and prose/Fig.3c-direction sigmoid variants for the 2026 model reproducibility audit.
- Weighted log-space objective.
- D⁺/D₂⁺ prompt-dissociation component mapper, with ion-current fractions converted to deuteron-flux fractions.
- Generic weighted beam-spectrum yield function accepting an arbitrary discrete measured or hypothesized energy distribution; no Gaussian assumption is made.

The D₂⁺ routine is a conditional sensitivity model, not an observed beam decomposition. It does not model dissociation depth or species-specific stopping. The generic spectrum interface can represent a measured distribution only once the species-resolved energy histogram is supplied with provenance and uncertainty.

There are **21 Rust tests authored** in the current source. They have not been executed in the current environment because `cargo`, `rustc`, and `nix` are unavailable. The prototype remains outside the production workspace pending Rust-capable execution, independent physics review, source-data reconstruction, and numerical verification.


## EJ-309 detector observation scaffold

- Source metadata fixture: `research/cmni/fixtures/chen-2025-ej309-detector-published-metadata.json`.
- Pure PSD gate helper: `neutron_psd_gate_accepts`, using caller-supplied 5-keVee slice calibration values, `mu_gamma + 5*sigma_gamma`, upper offset, and minimum light-output threshold.
- The helper tests gate arithmetic and invalid calibration handling only. It does not model scintillation, PMT waveforms, detector efficiency, MCNP/Geant4 transport, or GRAVEL unfolding.
- **26 Rust tests are authored but not executed** in this environment because `cargo`, `rustc`, and `nix` are not installed.


## New statistical and spatial-model functions

- shared_reference_log_covariance: constructs the common-denominator covariance for log relative yields, assuming independent numerator counts and one shared reference yield.
- correlated_weighted_log_space_chi_square: generalized least-squares score using Cholesky factorization; rejects non-symmetric and non-positive-definite covariance.
- total_deuteron_rate_per_second: converts a singly charged D+/D2+ ion-current mixture into deuteron arrival rate.
- GaussianBeamFootprint: current-normalized axisymmetric Gaussian radial flux density, explicitly a baseline that must be replaced by a measured 2-D beam map when available.
- Literal Eq. 16 sigmoid enhancement is implemented separately from the prose-direction sigmoid and Eq. 13 mixture.

There are now **42 authored Rust tests** in the prototype. They remain unexecuted in the current environment because cargo, rustc, and nix are unavailable. Independent calculations in this session checked the shared-reference covariance algebra, diagonal GLS limit, Gaussian profile normalization and both sigmoid asymptotes; these checks are not a Rust test pass.
