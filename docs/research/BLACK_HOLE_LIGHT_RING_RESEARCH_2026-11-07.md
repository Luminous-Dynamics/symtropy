# Black-Hole Light-Ring and Non-QNM Research Addendum

**Research date:** 2026-11-07  
**Status:** competing-model research specification; not a statement of consensus.

## Why this changes the programme

The GW250114 Nature paper reported a post-merger component whose frequency and damping were interpreted through Kerr horizon angular velocity and surface gravity. That result is reproducible research and must remain in the comparison set.

Several later 2026 works materially complicate the horizon interpretation:

1. Kankani & McWilliams, *Direct waves in black hole binary mergers: Insights from the backward one-body model*, Phys. Rev. D 114, 084025 (2026-10-07), DOI [10.1103/kjjd-nk8f](https://doi.org/10.1103/kjjd-nk8f). They connect merger-ringdown evolution to the backward-one-body (BOB) model and light-ring dynamics. Their rational-filter residual frequency has a different spin dependence from the horizon frequency and tracks the news frequency near peak news amplitude.

2. Dyer et al., *Moving Beyond Quasinormal Modes and Reinterpreting Direct Waves in Binary Black Hole Merger-Ringdowns*, arXiv:2610.25176 (2026-10-18), [arXiv link](https://arxiv.org/abs/2610.25176). The authors present three strands of evidence for a light-ring interpretation of the early non-QNM component: (a) a spin-dependent comparison using 125 SXS simulations spanning mass ratios, eccentric/quasicircular configurations and precessing/non-precessing systems, plus 43 anti-aligned-remnant simulations; (b) agreement with a light-ring-based BOB model; and (c) a similar non-QNM residual in perturbative ringdown around a horizonless ultracompact object that has light rings.

3. Dyer, Chung & Moore, *Modeling direct waves in binary black hole ringdowns*, Phys. Rev. Lett. accepted 2026-10-27, DOI [10.1103/xftt-2n6t](https://doi.org/10.1103/xftt-2n6t). They report extracting a direct-wave component over a range of ringdown start times but find that its frequency deviates from the horizon value, limiting its use as a horizon probe.

The Nature GW250114 analysis remains a published result with public data and code; this addendum does not discard it. Instead, it separates the empirical existence/extraction of a non-QNM component from claims about what physical structure the component uniquely measures.

## Four distinct scientific claims

Do not merge these into one binary "direct wave yes/no" claim:

1. A reproducible early-time non-QNM residual exists after the specified QNM removal/filtering procedure.
2. Its instantaneous frequency and damping evolution can be estimated under a specified model.
3. Those properties encode a Kerr horizon quantity, a light-ring quantity, peak-news/orbital dynamics, or some combination.
4. The observable uniquely discriminates a spacetime with an event horizon from a horizonless ultracompact object with a comparable exterior light ring.

The horizonless perturbative result challenges uniqueness of the horizon diagnosis. It is **not** evidence that a nonlinear binary merger of horizonless objects has been observed.

## Competing models to compare

| Model family | Physical organizing idea | Main validation question |
|---|---|---|
| Linear QNM / orthonormal-QNM | Remnant's linear perturbation spectrum | Is mode support stable under basis, start-time, and injection changes? |
| Horizon-linked component | Kerr horizon mode, schematically (m\Omega_H-i\kappa) | Does its spin/time dependence actually track horizon observables out of sample? |
| Light-ring / BOB | Unstable circular null orbit and associated dynamics | Does the component track prograde/retrograde light-ring frequency and instability? |
| Peak-news model | Frequency near peak Bondi-news amplitude | Does it generalize across SXS simulations and observed events? |
| Flexible non-QNM residual | Phenomenological early-time component | Is the residual reproducible and predictive beyond its fitting interval? |
| Noise / transient / model mismatch | Detector disturbance or incomplete signal model | Do null, off-source, and detector-coherence controls reject this explanation? |

The light-ring picture is also a hypothesis to evaluate, not a replacement dogma. Use held-out simulation families and independent codes.

## Horizon uniqueness experiment

Use the horizonless ultracompact-object perturbative calculation as a specificity control:

- Determine which residual properties depend on horizon boundary conditions.
- Determine which can be reproduced from the exterior potential / light-ring structure.
- Compare black-hole and horizonless perturbative ringdowns at matched light-ring properties where the available model permits.
- Quantify whether any proposed observable discriminates horizons, not merely compact objects with a light ring.
- Explicitly separate perturbative ringdown of a stationary horizonless object from a fully nonlinear binary merger involving horizonless objects.

A similar residual in a horizonless perturbative spacetime would falsify the claim that the residual *alone* uniquely establishes a horizon. It would not establish the behavior of a real horizonless binary merger.

## Reproducibility baselines

Pin exact code/data versions and processing choices:

- Nature data products: [Zenodo 10.5281/zenodo.20017347](https://doi.org/10.5281/zenodo.20017347).
- Rational QNM filter: [Sizheng-Ma/qnm_filter](https://github.com/Sizheng-Ma/qnm_filter).
- Direct-wave modeling code: [Richardvnd/direct_waves](https://github.com/Richardvnd/direct_waves).
- Light-ring analysis code: [Richardvnd/direct_waves_LR](https://github.com/Richardvnd/direct_waves_LR).
- SXS waveforms: [SXS waveform database](https://data.black-holes.org/waveforms/).

For every simulation, record SXS ID, extrapolation/CCE product, resolution, mode content, Bondi news versus strain, QNM removal set, filter implementation SHA, fitting interval/start time, remnant mass/spin posterior, and result uncertainty.

## Symtropy light-ring reference layer

The new Symtropy work item is [BH-LIGHTRING-001 #4078](https://github.com/Luminous-Dynamics/symtropy/issues/4078).

The research branch now contains `light_ring.rs`, implementing the analytic Kerr equatorial prograde/retrograde circular-null-orbit radii and signed coordinate angular frequencies. Authored tests cover:
- Schwarzschild photon-sphere radius/frequency;
- positive-spin prograde vs retrograde ordering;
- negative-spin orientation reversal;
- extremal Kerr limits;
- near-extremal numerical finiteness.

The next light-ring gate is the radial Lyapunov exponent under a declared coordinate-time convention, followed by comparison with the eikonal QNM correspondence. The eikonal relation is high-angular-index asymptotics; it must not be presented as exact for the observed ℓ=m=2 mode.

## Benchmarks

1. Prograde and retrograde Kerr photon-orbit radius and frequency over signed spin χ in [-1,1].
2. Radial Lyapunov exponent and impact parameter, with independent derivations from null radial potential and linearized geodesics.
3. Eikonal QNM frequency/damping comparison, explicitly respecting high-ℓ assumptions.
4. Non-QNM frequency/damping vs horizon, light-ring, and peak-news predictors on held-out SXS runs.
5. Generalization across mass ratio, precession, eccentricity, aligned/anti-aligned remnant spin, resolution and independent NR implementations.
6. Horizonless perturbative controls that test diagnostic specificity.

Primary success metric: calibrated out-of-sample prediction and model discrimination, not maximum fit quality on GW250114.
