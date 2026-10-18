# Physics Experiment Atlas v0.1 — qualification protocol

**Status:** research prototype / synthetic-and-analytic checks only  
**Executable:** tools/physics-experiment-atlas  
**Reproduce:** cargo test --locked --manifest-path tools/physics-experiment-atlas/Cargo.toml  
**Receipt:** cargo run --locked --manifest-path tools/physics-experiment-atlas/Cargo.toml -- --json

## Thesis: reproduce the experiment, not only its appearance

Symtropy should become a deterministic, inspectable laboratory for landmark physics experiments. A scene that looks like a Cavendish balance or double-slit bench is useful presentation, but does not validate the equations, measurements, or inference. Every experiment must bind together:

1. **Historical question:** what hypothesis or law was tested, and what alternative predictions existed?
2. **Model contract:** explicit equations, assumptions, units, boundary conditions and omitted effects.
3. **Apparatus model:** geometry and sensor/readout model kept separate from the underlying physical process.
4. **Interventions:** controls that change one causal variable, including negative controls.
5. **Observables:** the quantities the real experiment would record, not only visually appealing state variables.
6. **Inference:** a disclosed estimator, uncertainty treatment and parameter identifiability check.
7. **Evidence labels:** synthetic output, digitized historical data, modern reference data, or new physical measurement are distinct and never silently substituted.
8. **Replay receipt:** source revision, model parameters, initial state, integrator/solver, tolerances, seed (if stochastic), output digest and exact validation command.

The initial Rust tool is a small independent reference layer. It does not depend on Bevy or the live scene engine. That makes visual integration unable to silently define the expected physics.

## First release: ten experiment families

The CLI emits **19 checks** in a stable order. All are analytic or synthetic tests; none are re-digitized historical measurements.

| Experiment family | Model / observable | Initial qualification target | Explicit limit |
|---|---|---|---|
| Galileo inclined plane | constant-acceleration displacement; time-step convergence | analytic displacement within \(10^{-12}\) m for the declared fixture; first-order error ratio of 2 when \(\Delta t\) halves | no rolling geometry, friction, drag, or reconstruction of the original apparatus |
| Newton / Cavendish | point-mass inverse-square force ratio | \(F(r)/F(2r)=4\) within \(10^{-12}\) | does not calculate torsion-fiber response or infer \(G\) from real measurements |
| Kepler orbital dynamics | velocity-Verlet two-body orbit | maximum relative specific-energy drift below \(10^{-8}\); angular-momentum drift below \(10^{-12}\) on the frozen circular-orbit fixture | no third bodies, oblateness, relativistic corrections, atmospheric drag, or ephemeris fitting |
| Photoelectric effect | stopping potential vs frequency; least-squares slope | recover the fixture's declared \(h/e\) slope within \(10^{-27}\,\mathrm{eV\,s}\) | noiseless generated points; not a new measurement of Planck's constant |
| Millikan oil drop | infer the elementary-charge lattice from noisy charge observations | recover the synthetic charge unit within \(10^{-4}\times 10^{-19}\,\mathrm C\); RMS lattice residual below \(1.5\times10^{-3}\times10^{-19}\,\mathrm C\) | synthetic values, grid-search prior \([1.50,1.70]\times10^{-19}\,\mathrm C\); no drop-dynamics, viscosity or apparatus inference |
| Foucault pendulum | signed precession rate from latitude and Earth rotation | zero at equator, about \(-15.0411^\circ/\mathrm h\) at the north pole, and equal-and-opposite rates at \(\pm45^\circ\) | ideal small-angle limit and explicit sign convention (clockwise viewed from above is negative); no damping, ellipticity or drive mechanism |
| Michelson–Morley | classical-aether counterfactual vs Lorentz-invariant null model | reproduce the declared classical shift (about 0.4406 fringe for fixture parameters) and exact zero invariant-model shift | no historical fringe digitization, arm dynamics, thermal drift, vibration, or real instrument noise |
| Double slit | two-path interference law with visibility control | unit visibility for ideal equal paths; zero visibility removes the predicted fringe contrast | not a full wave-equation / finite-aperture / decoherence solver |
| Stern–Gerlach | ideal spin-\(1/2\) Born probabilities | \(P_+(0)=1\), \(P_+(\pi/2)=0.5\), \(P_+(\pi)=0\) | no finite magnet field, spatial wave-packet, detector efficiency, or preparation error |
| Rutherford scattering | unscreened classical Coulomb scattering angle | invariant \(b\tan(\theta/2)=k/(2E)\) within \(10^{-12}\) for frozen parameters | not a quantum cross-section, screened potential, nuclear structure, or detector model |

A check passing only means that the coded fixture agrees with the stated analytic/model invariant within its tolerance. A passing suite is not proof that the underlying law is complete or that a historical experiment has been reconstructed faithfully.

## Shared Symthaea experiment-design fixture

The transport fixture at `tools/physics-experiment-atlas/fixtures/galileo-discrimination-v0.1.json` is mirrored in Symthaea's `crates/domains/symthaea-experiment-design/fixtures/galileo-discrimination-v0.1.json`. It gives the same two Galileo model descriptors and uncertainty descriptors to the experiment-design component, with SHA-256 digests checked by the Symthaea planner. These digests verify the descriptor bytes; they are **not** an attestation that a simulation binary executed the equations. Both focused workflows pin the fixture's Git object fingerprint (`0548cf58562517e69c93e5f810bfee822acee1a2`), so unreviewed fixture drift in either repository fails that repo's gate. The fingerprint is a drift check, not an executable/model attestation. A coordinated change must review and update both copies and both pins.

## Architecture: three separate truth layers

**Symtropy — experiment scene and physical state.** Render apparatus, measurement readouts, particle/field state and intervention controls. Rendering must be a consumer of model state, never the source of truth. Record deterministic time steps, dimensionality, units, solver identity and reset behavior.

**Symthaea — hypothesis and experiment design assistant.** Propose preregistered hypotheses, parameter sweeps, discriminating interventions and candidate explanations. It must not edit the reference oracle or validation thresholds during a scored run. Preserve failed predictions and negative results. Compare its prediction against held-out cases rather than using the reference result as an input.

**Independent reference layer — oracle and evidence.** Evaluate conservation laws, dimensional consistency, analytic solutions, convergence order, parameter recovery and cross-solver differences. Store each result with exact source/model digests. The first executable here is intentionally independent of Bevy and Symthaea so later integration cannot create circular validation.

No result about \(\Phi\), cognition, agent capability or consciousness follows from rendering or passing these physics checks. Symthaea's role is scientific workflow support, not an authority that can certify its own hypotheses.

## Paradigm-shift benchmark ladder

### Gate A — equation and unit integrity

- Every model declares SI or explicit dimensionless units, parameter domains, initial/boundary conditions and omitted physics.
- Use dimensional-analysis tests and scale transformations: convert all lengths by a declared factor and verify dimensionless outcomes or appropriately transformed outputs.
- Negative controls must fail the intended assertion (e.g. inverse-square force under a deliberately wrong exponent, or an incorrect photon-frequency slope).
- Invalid, non-finite, singular, or out-of-domain inputs fail closed with a typed error; no silently clamped parameters.

### Gate B — numerical qualification

- Exact-solution comparisons at representative and adversarial parameter values.
- At least three decreasing step sizes; measured convergence order must agree with the integrator's documented order on well-conditioned problems.
- Conservation/invariant drift versus step size and simulation duration, not a single visually plausible trajectory.
- Stiffness, scale and near-singular sweeps; resource caps and explicit nonconvergence receipts.
- Independent reference calculation or a second numerical method for each high-value model.
- Deterministic replay on an exact code revision and fixed seed; tolerance and solver settings are part of the receipt.

### Gate C — measurement and inference qualification

- Keep generated observations clearly synthetic. For real data, archive source citation, license/provenance, transcription/digitization method, units, calibration and uncertainty.
- Report residuals, confidence/credible intervals where justified, sensitivity, parameter correlations and identifiability; do not report point estimates alone.
- Fit on declared calibration data and score on held-out data. A model does not earn accuracy by regenerating its own training fixture.
- Inject realistic noise and systematic errors, then verify uncertainty coverage and failure/abstention behavior.
- Compare against simple baselines before any AI-designed model or inference is called an improvement.

### Gate D — Symtropy/Symthaea closed-loop scientific workflow

- Preregister hypothesis, intervention, predicted direction/magnitude, falsifier and stopping rule.
- Randomize intervention order where order effects are plausible; run blinded or held-out cases when practical.
- Preserve raw measurement, transformed series, model output and rendered frame as separate artifacts.
- Symthaea may request new experiments, but an independent deterministic harness decides pass/fail.
- Score prediction accuracy, calibration, counterfactual discrimination, sample efficiency, robustness to nuisance parameters, false discoveries, reproducibility and wall-clock cost separately. No composite score can hide a safety-critical miss.

## Release roadmap

**v0.1 — analytic/synthetic kernel:** the eight families and bounded equation/invariant checks above.

**v0.2 — mechanics and electromagnetism:** pendulum and coupled oscillators; Faraday/Lenz induction; Coulomb field superposition; charge-lattice inference for Millikan-style observations. Add damping and nuisance parameters with explicit assumptions.

**v0.3 — quantum demonstrations:** finite-aperture double slit; blackbody spectrum; Franck–Hertz; Compton scattering; Stern–Gerlach with state preparation and detector model. Distinguish amplitude evolution, Born-rule sampling and detector response.

**v0.4 — precision and modern experiments:** Bell/CHSH with local-realistic control models; gravitational-wave open data; precision atomic spectra; neutrino-oscillation baselines. These require trustworthy external datasets, licensing/provenance review, uncertainty-aware analysis and independent reference pipelines.

**v1.0 — historical evidence bundles:** pin and cite the primary paper or a trusted archive for every historical data point, carry full uncertainty and calibration provenance, permit independent re-running, and label each experiment by the highest evidence gate actually passed.

The collection should grow by adding qualified experiments, not by declaring a finite list to be “all of physics.” Some experiments cannot be faithfully performed in software alone; for those, Symtropy should support data replay and instrument twins and explicitly state what still requires real apparatus or access to external facilities.

## References

- NIST, *2022 CODATA recommended values of the fundamental physical constants*: https://physics.nist.gov/cuu/Constants/ and https://physics.nist.gov/cuu/pdf/all.pdf
- NIST, *How Do You Measure the Strength of Gravity?* (Cavendish/torsion balance context): https://www.nist.gov/how-do-you-measure-it/how-do-you-measure-strength-gravity
- NIST, *Introduction to the constants for nonexperts, 1900–1920* (historical charge and photoelectric \(h/e\) measurement context; documents Millikan's early bias traced to air-viscosity error): https://physics.nist.gov/cuu/Constants/historical1.html
- Millikan, *On the Elementary Electrical Charge and the Avogadro Constant* (1913 primary-source record): https://authors.library.caltech.edu/records/sfc90-k2g07
- UNSW Physics, *The Foucault pendulum — the physics (and maths) involved*: https://newt.phys.unsw.edu.au/~jw/pendulumdetails.html
- American Physical Society, *Robert Millikan Reports His Oil Drop Results*: https://www.aps.org/apsnews/2006/08/robert-millikan-oil-drop-results
- PhET Interactive Simulations, *Photoelectric Effect* (pedagogical reference for frequency/intensity/voltage/material controls): https://phet.colorado.edu/en/simulations/photoelectric

The references motivate the historical selection and constant values. They do not validate this implementation, its synthetic fixtures, or any future Symtropy scene.
