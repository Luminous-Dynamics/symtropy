# Historical physics experiment catalog v0.1

**Status:** roadmap and provenance inventory, not a claim that each listed experiment is already implemented or reproduced.
**Home:** Symtropy Physics Experiment Atlas; Symthaea experiment design companion: https://github.com/Luminous-Dynamics/symthaea/pull/8034

The ambition is a *growing, qualification-gated collection of history-defining experiments*, not a claim that a finite checklist exhausts physics. “Perform” has two meanings here and must be reported distinctly:

- **Virtual performance:** run a declared model of the physical process, apparatus/readout and analysis under chosen interventions.
- **Evidence reproduction:** reanalyse sourced historical or modern observations with documented transcription, units, calibration and uncertainty.

A virtual result does not become an experimental replication just because the scene looks right. Some results depend on real apparatus, source data, specialised facilities, or measurements we cannot reconstruct from an image of the historical apparatus.

## Catalog and readiness states

| Era / experiment | Central question and measured observable | First useful simulation and decisive benchmark | Evidence route / readiness |
|---|---|---|---|
| Archimedes — buoyancy | Does displaced fluid volume predict the buoyant force? | Hydrostatic force balance; check \(F_b=\rho g V\), equilibrium and unit scaling | Analytic first-principles fixture first; real object/container measurements later |
| Torricelli / Boyle — pressure | How do pressure, height, and gas volume relate under controlled conditions? | Hydrostatic column plus ideal-gas/isothermal model; recover declared limit laws and flag leakage/temperature assumptions | Need trustworthy apparatus data for historical reproduction |
| Galileo — inclined plane / falling bodies | Does acceleration follow a law independent of the object's mass in the idealised limit? | Constant-acceleration solution, step-halving convergence, nuisance friction and uncertainty sweep | Synthetic analytic kernel exists; primary observations not digitised |
| Newton — prism dispersion | Can a single white-light spectrum be explained by superposition of wavelength-dependent refraction? | Wavelength-resolved ray / spectral-response model; recover prism geometry and an instrument-convolved spectrum | Optical model first; primary-source data and apparatus geometry required |
| Newton / Kepler — orbital motion | Can inverse-square dynamics recover conic orbits and area-law behavior? | Two-body solver; conservation drift vs timestep, orbit-period and periapsis reference; add N-body perturbations only with an appropriate ephemeris target | Circular two-body invariant fixture exists; no ephemeris fit yet |
| Cavendish — torsion balance | Can a measured twist under known masses infer \(G\)? | Separate gravity field, torsion-fiber response, calibration and reading noise; recover injected \(G\) with uncertainty coverage | Inverse-square ratio check exists; torsion apparatus and historical data not modeled |
| Foucault — pendulum | Does the swing plane precess with latitude as expected in the rotating Earth frame? | Earth-rotation model; equator zero, pole rate, hemisphere sign, then damping/ellipticity nuisance sweeps | Latitude formula check exists; no measured trace is imported |
| Eötvös — equivalence principle | Do test masses accelerate alike under gravity across material/composition changes? | Differential-acceleration estimator with apparatus drift and null controls; test bias and interval coverage | Requires sourced historical measurements or a real test apparatus |
| Young — double slit | Does coherent superposition produce fringes, and how does path information affect visibility? | Finite-aperture wave model; recover fringe spacing/visibility under wavelength, separation and detector-resolution sweeps | Ideal two-path visibility control exists; no finite-aperture or which-path apparatus model yet |
| Coulomb / Faraday cage | Do electric fields superpose, and how does a conductor screen the interior field? | Electrostatic boundary-value solution; field residual and boundary-condition convergence | Not yet implemented in this atlas |
| Ohm / Kirchhoff | Can measured circuit response be inferred from resistance, connectivity and source characteristics? | Circuit network solve with sensor loading and tolerances; current conservation, power balance, parameter recovery | Symthaea already has circuit/engineering directions; a history-specific measurement protocol remains to be added |
| Faraday / Lenz — induction | Does changing magnetic flux generate the sign and magnitude of induced EMF? | Coupled circuit–field experiment; verify \(\mathcal{E}=-d\Phi_B/dt\), orientation reversal, energy consistency and discretisation convergence | P0 next-model candidate; no physical induction model in this atlas yet |
| Joule — mechanical equivalent of heat | Does measured mechanical work produce the predicted thermal energy increase? | Coupled work/heat balance with calorimeter losses and heat capacity uncertainty; energy residual and blind parameter recovery | Requires apparatus-loss calibration for evidence reproduction |
| Maxwell / Hertz — electromagnetic waves | Do time-varying electric and magnetic fields support waves with the predicted propagation speed? | Maxwell-wave solver plus source/antenna and receiver model; speed convergence and energy-flux checks | Analytical solver comparison first; historical signals need provenance |
| Michelson–Morley | Does interferometer fringe displacement match the competing orientation-dependent predictions? | Explicit interferometer phase model with an instrumental drift/noise layer; recover injected shifts and demonstrate power to detect a declared counterfactual | Ideal counterfactual/null calculations exist; historical fringe data and nuisance model not ingested |
| Blackbody radiation — Stefan/Wien/Planck | Which spectrum fits a measured thermal-radiation curve across temperature? | Spectral radiance model plus detector bandpass/background; test peak scaling, integrated radiance and held-out model discrimination | Requires a pinned spectrum and uncertainty; don't use a generated Planck curve as independent evidence |
| Millikan — oil drop | Are inferred drop charges consistent with a shared elementary-charge unit after drag/viscosity correction? | Drop dynamics, buoyancy, air-viscosity correction, field calibration, integer-lattice fit and uncertainty propagation | Synthetic lattice estimator exists; historical raw values and correction trail not digitised |
| Photoelectric effect | How do stopping potential, light frequency, intensity, and material work function affect electron emission? | Photon-energy and stopping-voltage model plus threshold/detector response; estimate \(h/e\) and work function with noise and held-out frequencies | Noiseless slope-recovery fixture exists; measured source data not imported |
| Rutherford / Geiger–Marsden — scattering | Does angular scattering support a concentrated positive nucleus over diffuse-charge alternatives? | Differential cross-section and detector acceptance; fit angular counts, test large-angle tail and model-selection stability | Classical angle invariant exists; real scattering counts not imported, quantum/nuclear effects omitted |
| Bragg — X-ray diffraction | Do diffraction peaks reveal lattice spacing and crystal structure? | Bragg geometry and peak broadening/noise; recover known spacing under angular-calibration perturbations | Needs sourced diffraction patterns and instrument response |
| Franck–Hertz | Do electron energies show discrete inelastic-excitation losses in a gas? | Electron energy-loss / collision model plus contact-potential and detector effects; recover peak spacing with uncertainty | Need a pinned lab trace or published table |
| Stern–Gerlach | Does a prepared spin state yield discrete measurement outcomes and axis-dependent probabilities? | Quantum state evolution / measurement model, finite magnet and detector response; Born-rule limits and statistical coverage | Ideal spin-1/2 probability check exists; no finite-magnet/wave-packet model yet |
| Compton scattering | Does scattered photon wavelength shift depend on angle as predicted by momentum-energy conservation? | Photon–electron kinematics plus spectral-resolution response; angular-shift residual and parameter recovery | Need a sourced spectrum with calibration and uncertainty |
| Davisson–Germer — electron diffraction | Do electron scattering peaks show matter-wave diffraction? | Electron wavelength and crystal lattice scattering; peak-angle scaling with voltage and lattice spacing | Need a citable digitised experimental series |
| Electron / molecular single-particle interference | Do accumulated individual detection events recover the predicted interference distribution? | Monte Carlo sampling from a wave probability distribution; frequency residuals, seed replay, and distinguish the sampling model from any claimed trajectory interpretation | Can simulate distributions; historical raw counts and the apparatus/decoherence model remain separate evidence requirements |
| Bell / CHSH — entanglement tests | Do measured correlations violate a declared Bell inequality under stated assumptions? | Exact state/correlation predictions and detection/setting model; confidence interval, finite-sample effects, setting dependence and loophole register | Never imply a classical visual model performs a loophole-free experiment; use a pinned published dataset for reproduction |
| Pound–Rebka — gravitational redshift | Is a small frequency shift consistent with the gravitational potential difference? | Frequency/energy shift model plus source, absorber and instrument calibration; recover injected shift under drift | Requires source measurements and systematic-error ledger |
| Muon lifetime / time dilation | Does observed lifetime change with relativistic speed as predicted? | Proper-time model plus production/selection/detection efficiency; fit survival curves with background and held-out intervals | Need published decay counts / detector corrections |
| Eddington / modern light deflection and lensing | Does light deflection track mass distribution as predicted in weak-field gravity? | Ray tracing in a declared metric or lens model plus image/astrometric response; recover injected deflection and quantify parameter degeneracies | Modern astrometric data may be available; historical eclipse plates need careful provenance and uncertainty |
| Atomic spectra / line splitting | Can quantized energy levels predict wavelength lines and their changes in fields? | Level calculation and line-shape/instrument model; residuals vs resolution, isotope, Doppler and field nuisance parameters | Pin a reference spectrum before fitting; NIST atomic data is a candidate resource |
| Atomic clocks / gravitational redshift / relativistic timing | Do clock-frequency comparisons follow predicted velocity and potential shifts? | Relativistic clock model with oscillator noise and link delay; hold-out prediction on sourced clock-comparison series | Requires precision measurement records, not just a GPS-themed animation |
| Cosmic microwave background / FIRAS | Is the measured sky spectrum consistent with a nearly blackbody spectrum and constrained departures? | Fit a pinned spectrum with foreground and calibration nuisance models; held-out residual and uncertainty coverage | Real mission data needed; not yet a bundled dataset |
| Neutrino oscillation | Do flavour-transition rates vary with baseline and energy under mass-squared-difference mixing? | Oscillation probability plus flux, cross-section, efficiency and matter effects as needed; reproduce held-out energy/baseline spectra | Use collaboration releases and their own uncertainty/covariance data; a toy sinusoid is not a reproduction |
| Gravitational-wave interferometry — GW150914 and successors | Does calibrated strain contain a coherent transient consistent with a compact-binary waveform rather than noise-only controls? | Inject-recover calibration plus strain preprocessing, matched filtering and background trials; false-alarm accounting and cross-detector consistency | GWOSC publishes open event data and tutorials; use the exact release and provenance, not only a generated chirp |
| Higgs and collider particle discoveries | Do collision-event distributions support a signal over background for a declared channel and analysis selection? | Reproduce documented object selection, backgrounds and statistical test on a pinned release | CERN Open Data provides collision data, software and documentation; data level, licenses and required reconstruction must be respected |
| Modern precision constants and metrology | Can independently measured observables jointly constrain constants while preserving covariance? | Covariance-aware parameter estimation, units, leave-one-source-out checks and sensitivity to correlated inputs | Pin the CODATA edition and source observations; NIST's current public page lists the 2022 recommended values and describes the planned 2026 adjustment |

## Evidence status (required on every entry)

Use exactly one top-level label per run:

- **Analytic reference:** compared against a mathematical solution or invariant. Does not establish agreement with a real historical experiment.
- **Synthetic recovery:** parameter/data generator is known and separate from the estimator; generated sample settings and random seed are declared.
- **Transcribed historical data:** every point traces to a cited table/figure/paper, with digitization method and estimated reading uncertainty.
- **Open modern data reproduction:** immutable release/version, DOI or permanent URL, license, processing chain and analysis configuration are pinned.
- **Physical measurement:** actual apparatus, calibration certificate, raw readout, clock/instrument provenance, uncertainty budget and operator/run metadata are preserved.

Do not collapse these labels into a single number called “reproduced.” The highest earned label is attached to each model/data version, and evidence never silently upgrades because a newer renderer or AI planner uses it.

## Release gates and benchmark targets

### Gate 1 — mathematical model fidelity
- Dimensional consistency and unit-change tests on every model.
- Analytic solution/invariant at limiting cases, including zero input and sign reversal where appropriate.
- Wrong-law negative controls must fail (e.g. deliberately use inverse-cube instead of inverse-square; reverse Faraday's sign; replace a Planck spectrum with Rayleigh–Jeans outside its limit).
- At least three step sizes; observed convergence order matches the scheme's documented order.
- Solver budgets, singular states, non-finite results and domain violations fail closed.

### Gate 2 — apparatus and inference fidelity
- Simulated sensor output must be distinguishable from latent physical state and carry units/calibration.
- Ingest real data only after source and license review; preserve raw, cleaned, transformed and fitted layers separately.
- On synthetic recovery tests, report bias, variance, interval coverage and failure rate across parameter/noise sweeps.
- On sourced data, declare likelihood/error model, covariance, fit window, background and excluded points; retain a held-out score where possible.

### Gate 3 — experiment selection quality
- Ranking invariant to row ordering and deterministic on a pinned input.
- When two models make identical predictions for a candidate, its score must be zero for that pair.
- Increase declared uncertainty or nuisance-model disagreement: scores should not become unjustifiably more decisive.
- Compare heuristic ranking to a calibrated likelihood/EIG reference on synthetic known-truth cases; report decision regret, ranking agreement and compute/cost, not a single composite score.

### Gate 4 — Symtropy scene fidelity
- Same physics receipt across graphical and headless modes.
- UI framerate must not change simulation time or numeric outputs.
- Reset/replay on the same revision reproduces the same deterministic run receipt.
- Visual appeal is judged separately from model accuracy and evidence grade.

### Gate 5 — scientific-loop quality
- Symthaea must preregister the hypothesis pair, intervention, predicted distribution, uncertainty, falsifier and stopping rule before a scored run.
- Score on held-out interventions that were not used to tune the model.
- Preserve failed hypotheses and negative results; independent deterministic code—not Symthaea—sets acceptance.
- Report calibration, false-discovery rate, sample efficiency, decision regret, compute, and analyst/operator cost separately.

## Near-term execution order

1. Stabilise v0.1 analytic/synthetic atlas and its shared, digest-pinned Galileo experiment-design fixture.
2. Add Faraday/Lenz induction and coupled oscillators with energy/current invariants and sign-reversal negative controls.
3. Add the Joule energy-balance/calorimeter model and Planck blackbody spectrum with synthetic recovery and nuisance sweeps.
4. Import one historical table/curve end-to-end (start with a primary-source measurement where the transcription can be independently checked); preserve source image, digitised points, units, read-off uncertainty and license.
5. Add one open-data modern reproduction with a manageable analysis path, such as GWOSC strain or a CERN educational collision dataset.
6. Connect Symtropy's headless model runner to the Symthaea planner through the versioned JSON handoff. Freeze predictions before execution and let an independent scorer evaluate the outcome.

## Sources and open-data starting points

- NIST CODATA recommended values and measurement bibliography: https://physics.nist.gov/constants — use editioned constants and source-level uncertainty rather than unversioned literals. As of this catalog revision, the public site lists the 2022 recommended set and describes the next 2026 adjustment.
- NIST history on Millikan and historical determinations: https://physics.nist.gov/cuu/Constants/historical1.html
- Millikan, *On the Elementary Electrical Charge and the Avogadro Constant* (1913 record): https://authors.library.caltech.edu/records/sfc90-k2g07
- NIST Cavendish/torsion balance explainer: https://www.nist.gov/how-do-you-measure-it/how-do-you-measure-strength-gravity
- UNSW Physics, Foucault pendulum derivation: https://newt.phys.unsw.edu.au/~jw/pendulumdetails.html
- PhET, *Photoelectric Effect*: https://phet.colorado.edu/en/simulations/photoelectric — pedagogical control reference, not a source of experimental historical data.
- GWOSC/GW150914 open-data entry point: https://gwosc.org/s/events/GW150914/GW150914_tutorial.html — this old tutorial points to data files; before use, select a current release/tutorial and pin the data DOI/URL and processing chain.
- CERN Open Data: https://opendata.cern/ and https://opendata.cern/docs/about — collision data, software and documentation are released at multiple data levels under declared licenses.

These sources define starting points, not proof that any Symtropy/Symthaea implementation matches them. Each future evidence bundle must cite the exact source object used.
