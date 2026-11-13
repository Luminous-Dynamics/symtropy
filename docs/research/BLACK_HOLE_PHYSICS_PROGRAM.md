# Extreme Spacetime Physics: Black-Hole Research Program

**Status:** research charter; implementation work is tracked in the four linked repository issues.  
**Last researched:** 2026-11-07.  
**Scope:** black-hole physics as a cross-system proving ground for scientific reasoning, validated computation, evidence provenance, and astronomical observation.

## North-star objective

Build an evidence-qualified research loop that can move from a scientific question to a mathematically explicit model, an executable computation, an observable prediction, a comparison with public data, and an independently reproducible conclusion.

The objective is not to accumulate black-hole facts or claim a new solver prematurely. The objective is to make every scientific result inspectable: exact assumptions, equations, units, coordinates, inputs, code revision, numerical and statistical uncertainty, comparison method, and claim ceiling.

## Current work items

- [Symtropy — BH-REL-001](https://github.com/Luminous-Dynamics/symtropy/issues/3947): Lorentzian spacetime foundation and analytic black-hole benchmark kernel.
- [Symthaea — BH-REASON-001](https://github.com/Luminous-Dynamics/symthaea/issues/10812): distinguish semantic GR encodings from validated physics; build evidence-grounded reasoning.
- [Mycelix — BH-EVIDENCE-001](https://github.com/Luminous-Dynamics/mycelix/issues/5903): federate reproducible scientific evidence using existing Knowledge, DeSci, and Space ownership.
- [Sol Atlas — BH-OBS-001](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/586): source-faithful black-hole observation and gravitational-wave explorer.

Existing Symtropy work to compose with rather than duplicate:
- [#98 — reference-frame graph and ephemeris-bound transforms](https://github.com/Luminous-Dynamics/symtropy/issues/98).
- [#108 — coordinate time, proper time, causal cuts, and future relativistic profiles](https://github.com/Luminous-Dynamics/symtropy/issues/108).

## Repository audit: what exists and what it does not mean

### Symthaea

`crates/core/symthaea-core/src/physics/general_relativity.rs` is explicitly an exploratory HDC representation module. It has useful vocabulary for spacetime, curvature, geodesics, horizons, Kerr, and gravitational waves. However:
- Schwarzschild/Kerr constructors create semantic hypervector encodings, not metric tensor fields.
- The Kerr representation uses a Schwarzschild length scale plus a frame-dragging concept embedding; it is not a Kerr metric implementation.
- Geodesic construction creates a GenesisSeed-derived vector, not an integrated trajectory.
- The binary-merger helper estimates characteristic frequency/strain; it does not generate an inspiral-merger-ringdown waveform.
- The Einstein-equation helper combines concept vectors rather than evaluating tensor equations.
- Current tests primarily exercise representation norms/similarities and heuristic scalar behavior, not Einstein residuals, geodesic conservation, or numerical convergence.

Keep HDC for semantic retrieval, concept association, and hypothesis organization. Do not let an HDC vector stand in for a physical metric, tensor, solution, observable, or proof. The distinction is a scientific-strengthening refactor, not a reason to discard the semantic layer.

### Symtropy

The current `symtropy-math` and `symtropy-physics` crates provide reusable N-dimensional geometric primitives and Euclidean rigid-body / collision dynamics. They are valuable substrate, but they are not currently a Lorentzian tensor calculus or a numerical-relativity code. Preserve their existing contracts. Introduce a separately scoped 4D Lorentzian physics domain rather than bending Euclidean collision abstractions into general relativity.

### Mycelix

The active repository already contains Knowledge, DeSci, and Space research areas. Reuse their canonical claim, dataset, publication, evidence, and provenance owners. A cryptographic digest proves identity/integrity of an artifact, not scientific correctness. Reputation and governance can route review or fund replication; neither establishes physical truth.

### Sol Atlas

Preserve the existing ownership boundary: Mycelix owns canonical claims/evidence/provenance; Symthaea performs analysis/hypothesis reasoning; Sol Atlas projects and renders qualified observations for human inspection. Add a celestial/astronomical projection domain rather than forcing astronomical frames into Earth-geospatial coordinates.

## Paradigm shifts

1. **From knowledge ingestion to executable scientific claims.** A claim should link to its equation/model, assumptions, code and data, a runnable protocol, and a verification result.
2. **From one solver to an independent-method ensemble.** Use distinct formulations, discretizations, and implementations. Agreement is meaningful only after shared assumptions and correlated failure modes are examined.
3. **From benchmark scores to error budgets.** Keep numerical truncation, discretization, model-form discrepancy, detector calibration, noise, astrophysical population uncertainty, and posterior uncertainty separate.
4. **From scalar confidence to typed epistemic state.** Distinguish established theory, analytic derivation, direct observation, inference, simulation, hypothesis, disputed claim, and retraction.
5. **From post-hoc explanation to preregistered falsification.** Freeze model family, parameter range, observable, competing explanation, null control, and failure criterion before inspecting held-out data.
6. **From raw compute to time-to-verified-accuracy.** Compare methods at a target error and report wall time, core/GPU count, memory, power if available, compiler, precision, and hardware.
7. **From image resemblance to measurement-space agreement.** For gravitational waves compare strain/waveform likelihoods; for VLBI compare visibility/closure quantities and polarization products, not only rendered-pixel similarity.
8. **From giant end-to-end models to hierarchical physics.** Use exact analytic solutions first, perturbation theory and reduced-order/surrogate models second, and full nonlinear numerical relativity only where required. Every surrogate carries an applicability envelope and an out-of-domain detector.
9. **From discovery theater to reproducible negative results.** Null controls, failed replications, inconclusive measurements, and retractions must remain first-class outputs.

## Scientific workstreams

### A. Relativistic geometry and analytic solutions

- Minkowski, Schwarzschild, Kerr; later Kerr-Newman, de Sitter/anti-de Sitter, and selected perturbed solutions.
- Lorentzian signature, coordinate charts, tetrads where needed, metric/inverse/determinant, connection, curvature, invariants.
- Event/apparent horizons, ergosurfaces, photon orbits, ISCOs, redshift, proper time, frame dragging.
- Coordinate singularity vs curvature singularity.
- Coordinate-invariant checks across charts.

### B. Geodesics and orbital dynamics

- Timelike and null geodesics; equatorial and generic Kerr orbits.
- Conserved quantities associated with Killing symmetries.
- Normalization, affine/proper-time parameterization, separability checks where applicable.
- Photon spheres, light bending, capture/scatter thresholds, circular-orbit stability.
- Adaptive integration, convergence, and invariant-drift budgets.

### C. Ringdown and black-hole spectroscopy

- Kerr quasinormal modes and mode identification.
- Fundamental mode, overtones, mode mixing, excitation amplitudes, and damping times.
- Bayesian/model comparison for ringdown-only and full-signal analyses.
- Injection/null controls to detect spurious tones.
- Explicitly separate a resolved mode from a model-predicted but unresolved component.

### D. Binary-black-hole dynamics and numerical relativity

- Initial data, constraint solving, BSSNOK/Z4c or generalized-harmonic formulations, gauge choices, outer boundaries, mesh refinement, horizon finding.
- Wave extraction, \\(\Psi_4\\), strain reconstruction, extrapolation and/or Cauchy-characteristic extraction.
- Resolution sequences and cross-formulation/cross-code comparisons.
- Do not start a bespoke production GPU solver before analytic, convergence, and reference-comparison gates close.

### E. Gravitational-wave observation and inference

- GWOSC/GWTC event metadata, detector strain, data quality, calibration, posterior samples, skymaps.
- Matched filtering and likelihoods; parameter estimation; population inference only after event-level validation.
- Injection recovery, posterior coverage, prior sensitivity, calibration uncertainty.
- Compare results against official public analyses without treating catalog medians as exact truths.

### F. Accretion, jets, and multi-messenger astronomy

- GRMHD, radiation transport, magnetically arrested disks, turbulence, reconnection, jet formation and polarization.
- EHT image/polarization products, variability, source-frame and observing-epoch metadata.
- Compare predictions in measurement space; preserve image reconstruction and model dependence.

### G. Quantum fields, thermodynamics, and information

- Bekenstein-Hawking entropy, Hawking temperature, laws of black-hole mechanics, Unruh/Hawking effects.
- Semiclassical assumptions and domains of validity.
- Page curves, islands, holography, AdS/CFT and information-paradox proposals as distinct theoretical frameworks.
- Clearly label conjecture, derivation, and empirical status. Do not present a proposed resolution as experimentally confirmed.

### H. Model discrimination and scientific discovery

- Compare GR and bounded alternative-gravity / exotic-compact-object models only where predictions are well-defined and observationally distinguishable.
- Test parameter identifiability, degeneracies, prior dependence, selection effects, and model misspecification.
- Discoveries require a novel prediction, held-out evidence, uncertainty accounting, and independent reproduction.

## Public reference corpus and external baselines

Use these as independently maintained reference sources, respecting their citation and data-use requirements:

- [GWTC-5.0 release documentation](https://gwosc.org/GWTC-5.0/) — O4b catalog, strain, data quality, posterior products, and release references. The LVK release dated 2026-05-26 adds 161 significant O4b signals and brings the cumulative detections to 390.
- [GWTC-5.0 / GW250114 LIGO announcement](https://www.ligo.caltech.edu/news/ligo20260526) — high-SNR event and precision tests.
- [GW250114 GR/ringdown science summary](https://ligo.org/science-summaries/GW250114_TGR/) — ringdown tones, full-signal inference, and tests of Kerr consistency.
- [SXS Gravitational Waveform Database](https://data.black-holes.org/waveforms/) — SpEC waveforms, multiple numerical resolutions, extrapolation orders, and error assessment.
- [Einstein Toolkit Hypatia release](https://einsteintoolkit.org/about/releases/ET_2026_05_announcement.html) — GPU-accelerated BSSNOK/Z4c paths, Kerr exact initial data, and related tests in the 31st release.
- [SpECTRE official release and overview](https://spectre-code.org/) — the current cited release is v2026.09.05; high-order spectral finite elements, massive parallelism, and petascale-scale astrophysics/gravitational physics. Record the exact SpECTRE version used in every comparison.
- [LALSuite 8.0.0 release](https://pypi.org/project/lalsuite/8.0.0/) — released 2026-10-31; versioned baseline for waveform modelling and gravitational-wave analysis components including LALSimulation and LALInference.
- [GW250114 detection summary](https://www.ligo.caltech.edu/news/ligo20260526) — catalog-level SNR 76.9; the separate [GW250114 area-law/Kerr analysis](https://dcc.ligo.org/LIGO-P2500421/public) reports network matched-filter SNR 80. These are analysis-specific reported statistics; preserve pipeline/statistic provenance instead of flattening them to one number.
- [GW250114 ringdown interpretation](https://ligo.org/science-summaries/gw250114_tgr/) — ringdown-only analysis favors the fundamental mode plus first overtone; full-signal analysis measures the fundamental more precisely and constrains a higher-frequency secondary tone. Do not describe all three as independently resolved in the same analysis.
- [CarpetX manual](https://einsteintoolkit.org/thornguide/CarpetX/CarpetX/documentation.html) — AMReX-based block-structured adaptive mesh refinement and parallel hardware infrastructure.
- [June 2026 SpEC waveform-accuracy study](https://doi.org/10.1103/dmpn-q3fk) — generalized frequency-weighted mismatch, relative amplitude difference, phase difference over time, and parameter-dependent errors.
- [EHT OJ 287 paper](https://doi.org/10.1051/0004-6361/202555831) — 2026 spatially resolved polarization and short-timescale jet variability.
- [EHT official announcement](https://eventhorizontelescope.org/news/2026/01/event-horizon-telescope-maps-twisting-magnetic-fields-near-oj287) — contextual overview of the OJ 287 result.

### External software comparison

| System | Strength to benchmark against | Our strategy |
|---|---|---|
| SpEC / SXS | High-accuracy spectral binary-black-hole simulations and public waveform corpus | Validate against their resolution/error envelope; target a narrow, documented niche rather than claim general superiority |
| Einstein Toolkit / CarpetX | Broad community framework, AMR, evolving GPU BSSNOK/Z4c and horizon infrastructure | Use as a numerical-relativity reference; compare time-to-error, integration, reproducibility, and data products |
| SpECTRE | High-order discontinuous-Galerkin multi-physics / numerical-relativity methods | Compare where discretization and multi-physics questions matter |
| LALSuite / Bilby / GWOSC ecosystem | Mature gravitational-wave signal modelling, likelihoods, inference, and open data practices | Reproduce public results and injection coverage before proposing new inference methods |
| EHT analysis ecosystem | Interferometric calibration, imaging, polarization, and model comparison | Use published products and measurement-space diagnostics; do not replace specialist pipelines prematurely |

Industry comparison should be evidence-based: most high-end black-hole numerical-relativity and observational pipelines are research collaborations and open scientific infrastructure, not commercial black-box products. Benchmark us against the actual research codes, catalogs, and inference methods rather than generic “industry” labels.

## Benchmark suite and measurable targets

### Gate 0 — contracts and dimensional correctness

- Unit tests for SI/geometrized-unit conversion, signature, index placement, coordinate charts, and domain handling.
- Every input/output quantity has an explicit unit/convention.
- Invalid spin, mass, radius, chart domain, and coordinate singularity cases return explicit errors.
- Coordinate-dependent components and coordinate-invariant quantities are never conflated.

### Gate 1 — exact analytic fixtures

- Schwarzschild (r_s=2GM/c^2), photon sphere (3GM/c^2), ISCO (6GM/c^2).
- Kerr (r_\pm=M\pm\sqrt{M^2-a^2}), zero-spin and extremal-spin limits, and ergosurface.
- Schwarzschild Kretschmann invariant (K=48M^2/r^6) in geometrized units.
- Vacuum Ricci/Einstein tensors vanish within a declared, conditioning-aware floating-point tolerance at valid sample points.
- Metric inverse, determinant and signature match analytic fixtures.
- Prefer ULP/relative-error assertions for well-conditioned exact fixtures; do not hide poor conditioning behind one global tolerance.

### Gate 2 — geodesics and orbital invariants

For every benchmark trajectory, report:
- maximum drift in (g_{\mu\nu}u^\mu u^\nu\) from the declared timelike/null normalization;
- drift in Killing energy and angular momentum where defined;
- observed convergence order under step refinement;
- sensitivity to integration tolerances and chart changes;
- failure/abstention behavior near invalid domains.

Initial target: for selected well-conditioned exterior fixtures, achieve normalization/conserved-quantity drift at or below (10^{-8}) over the declared integration interval, and demonstrate the expected convergence trend when the step is halved. This is a fixture-level engineering target, not a universal guarantee for all orbits and coordinates.

### Gate 3 — numerical-relativity and waveform cross-validation

For matching physical assumptions and comparable simulation domains, report:
1. generalized frequency-weighted mismatch;
2. relative amplitude difference;
3. phase difference as a function of time/frequency;
4. constraint violations and convergence order;
5. extraction, extrapolation, and gauge/systematic differences;
6. memory, wall time, hardware, precision, and energy where measurable.

Target: remain inside the published/high-resolution reference uncertainty envelope for the selected fixture. At equal hardware and target error, seek a measurable improvement in time-to-verified-accuracy on one bounded workload before broad claims.

### Gate 4 — observational inference

- Blind injection-recovery on public/open strain data and synthetic injections.
- Calibrated posterior coverage (nominal 50%, 90%, 95% intervals checked over enough injections to quantify sampling error).
- Bias, credible-interval width, prior sensitivity, and calibration-systematic sensitivity.
- Reproduce a declared GW250114 ringdown/full-signal summary within the published uncertainty and model assumptions.
- Include null injections and noise-only controls; report false-mode detections and abstentions.
- Treat event detection, source classification, parameter inference, and tests of GR as different tasks.

### Gate 5 — EHT / VLBI observation

- Compare in visibility and closure-quantity space where data are available.
- Preserve observing epoch, calibration, frequency, polarization convention, imaging regularization, and uncertainty.
- Compare polarimetric quantities and temporal variability, not only pixel-level image metrics.
- Keep measured visibility, reconstructed image, simulated image, and model interpretation distinct.

### Gate 6 — scientific discovery

A novel result is eligible for a discovery claim only if:
- its model, assumptions, prediction, prior/parameter range, and falsifier were recorded before the held-out comparison;
- the observation is independent of the data used to generate the hypothesis;
- uncertainty and alternative explanations are quantified;
- null controls are passed;
- an independent implementation/team can reproduce it;
- a qualified domain scientist can review the result and its claim ceiling.

## Performance comparison protocol

Every performance result must identify:
- exact source commit and build configuration;
- CPU/GPU model, driver, memory, core count, and precision;
- dataset and benchmark fixture digest;
- solver, integrator, tolerances, resolution, and stopping condition;
- warm-up and repeated-run policy;
- wall time, memory peak, throughput, numerical error, and uncertainty.

Plot **error vs wall time** and **error vs memory**. Do not compare a lower-accuracy answer with a higher-accuracy answer as if speed alone decides. For GPU paths, report transfer overhead and end-to-end time, not kernel timing alone.

## Execution order — no CI dependency for research design

1. Freeze conventions, analytic fixtures, benchmark manifests, and exact data source versions.
2. Audit and relabel Symthaea's current GR HDC module.
3. Implement the standalone Symtropy analytic Lorentzian kernel and property/negative-control tests.
4. Produce deterministic JSON benchmark output and evidence-package manifest.
5. Use public GWTC-5.0 and SXS data to test the observation/inference boundary.
6. Build Mycelix provenance and Sol Atlas projection adapters.
7. Only after analytic and observational slices are reproducible, decide whether a bounded numerical-relativity evolution kernel or GPU work is justified.

This sequence can be researched, specified, and partially implemented locally without waiting for CI runners. A queued CI job is not a pass; no benchmark status is upgraded until the exact source head and reproducible command have been executed and independently checked.

## Claim policy

Allowed claims: “analytic fixture reproduced,” “geodesic integrator meets the declared error budget on these test cases,” “public event parameters reproduced within stated uncertainty,” “cross-code waveform comparison completed under these assumptions.”

Not allowed without stronger evidence: “outperforms state of the art,” “solves black-hole physics,” “discovers quantum gravity,” “resolves the information paradox,” “proves a theory,” or “observes an event horizon.”

The success criterion is not the number of black-hole features. It is the number of scientific claims whose provenance, assumptions, computation, uncertainty, and independent verification can be inspected end to end.


## Implementation update — 2026-11-06

A first bounded Rust implementation slice has been added on branch `research/black-hole-physics-program`:

- `crates/domains/symtropy-relativity` is now a workspace member.
- Its initial API validates positive finite mass and signed dimensionless Kerr spin.
- It implements SI-valued gravitational radius; Schwarzschild horizon, photon sphere, and ISCO; Schwarzschild Kretschmann scalar; scoped static-observer lapse/redshift; Kerr inner/outer Boyer-Lindquist horizons; and angle-dependent ergosurface.
- Ten analytic/limiting-case/domain tests are authored, including zero-spin and extremal-spin Kerr limits, signed-spin preservation, Schwarzschild scale ratios, curvature scaling, and rejection of invalid inputs.

**Validation status:** tests are authored but not executed in this editing environment because no Rust toolchain is available here (`rustc` is absent). No compilation, test pass, CI pass, numerical-relativity result, observational validation, or performance comparison is claimed. Before merge, run `cargo test -p symtropy-relativity` on the exact PR head and review the API/math conventions. The new crate is an analytic scalar kernel only; it is not yet a metric/tensor implementation or geodesic solver.



## Research update — direct waves, orthonormal QNMs, and the new GPU-NR baseline (2026-11-07)

### A new observationally motivated black-hole channel: direct waves

A peer-reviewed Nature paper published 2026-06-24, *GW250114 reveals signatures of post-merger black-hole horizon*, reports evidence for a post-merger direct-wave component in GW250114. The paper describes a component oscillating near angular frequency (2\Omega_H), with increasing damping characterized by the Kerr horizon surface-gravity parameter (\kappa), and quotes 90%-credible matched-filter SNRs of (15.8^{+0.1}_{-0.5}) (Hanford) and (17.1^{+0.1}_{-0.4}) (Livingston).

Public replication inputs are unusually good:
- Analysis data products: [Zenodo DOI 10.5281/zenodo.20017347](https://doi.org/10.5281/zenodo.20017347).
- Rational QNM filter implementation: [Sizheng-Ma/qnm_filter](https://github.com/Sizheng-Ma/qnm_filter).
- Underlying direct-wave theory: [Oshita et al., arXiv:2509.09165](https://arxiv.org/abs/2509.09165).
- Nature paper: [doi:10.1038/s41586-026-10696-0](https://doi.org/10.1038/s41586-026-10696-0).

The Nature paper reports robustness checks across segment length, sampling frequency, detector peak times, sky location/timing, and remnant mass/spin marginalization. Its data availability and code availability make it a high-value replication target.

**Claim ceiling:** this is an important published interpretation based on one exceptionally loud event. Our systems must call it a reported direct-wave component / hypothesis under test, not an independently established generic black-hole emission law, a direct photograph of the horizon, or proof that every merger carries an observable component at the same significance.

### A substantive counteranalysis: direct-wave / horizon association is disputed

Kankani & McWilliams, arXiv:2607.02380 (2026-07-02), *The Direct Wave is Not a Meaningful Test of Horizon Properties*, directly challenges the proposed horizon interpretation using numerical-relativity strain from a selection of SXS comparable-mass, quasicircular, non-precessing simulations.

Their reported findings:
- The instantaneous direct-wave frequency does not track twice the Kerr horizon angular velocity over their remnant-spin sweep. They identify an incidental crossing near chi_f ≈ 0.7, close to GW250114's remnant spin (~0.68).
- The direct-wave damping evolves substantially over short intervals and does not track surface gravity kappa; a fixed-damping single sinusoid is not generally an adequate model.
- A time-evolving waveform model constrained by horizon frequency/surface gravity does not describe their high-spin SXS:BBH:0178 case (chi_f ≈ 0.95); a freely fitted constant-frequency model also differs from the horizon frequency by about 25% there.
- Mapping the fitted direct-wave frequency/damping to horizon parameters can produce apparent Hawking-area-law violations even when the true NR remnant horizon area obeys the area law.
- The authors limit their conclusion to comparable-mass cases and note that the extreme-mass-ratio regime may behave differently.

Primary reference: [Kankani & McWilliams, arXiv:2607.02380](https://arxiv.org/abs/2607.02380). This is a counteranalysis preprint, not a final adjudication of the subject. It is nevertheless a serious falsification result and must be treated as a primary competing scientific interpretation.

**Research consequence:** direct-wave extraction from GW250114 may be reproducible while the *interpretation* of that component as a measurement of horizon frequency/surface gravity remains unsupported. We must distinguish:
1. a non-QNM residual/component exists under a specified filter;
2. its instantaneous frequency/damping can be measured under a chosen model;
3. that frequency/damping are physically caused by / encode Kerr horizon observables;
4. it is suitable for testing Hawking's area law.

These are four different claims, each requiring separate evidence.

#### Mandatory direct-wave discrimination campaign

Use the actual SXS:BBH cases tabulated by Kankani & McWilliams (remnant spins 0.4942, 0.5478, 0.6215, 0.6864, 0.6921, 0.7447, 0.8029, 0.8574, 0.9075, 0.9499) and add other available SXS cases / NR codes. Prioritize dense coverage on both sides of chi_f≈0.7 and near-extremal spins.

Competing models must include:
- phenomenological direct-wave residual with freely evolving complex frequency;
- fixed-frequency/fixed-damping direct-wave sinusoid;
- horizon-tied frequency and surface-gravity damping;
- orbit/plunge-driven or other physically motivated time-varying frequency and damping;
- ordinary QNM/overtones and mode-mixing residual;
- detector-noise/transient alternatives.

Required measurements:
- correlation and calibration of Re(omega_DW) against 2*Omega_H across spin, mass ratio, precession, eccentricity, and fitting window;
- evolution of Im(omega_DW) and tau(t), compared with kappa and alternative dynamical timescales;
- frequency/damping bias and posterior coverage at low, intermediate, high, and near-extremal spin;
- sensitivity to rational-filter basis, removed QNM set, spherical-spheroidal mode mixing, numerical resolution, extraction radius/extrapolation, and start/end time;
- recovery of true horizon area from NR remnant mass/spin versus area inferred by mapping a direct-wave fit into Omega_H/kappa;
- out-of-sample generalization to events and NR simulations not used to tune the model.

Use blinded injections and negative controls, preregister all spin windows and fitting intervals, and compare model evidence under matched priors/likelihoods. Do not interpret a crossing near chi_f≈0.7 as a universal physical relation without a predicted mechanism and held-out evidence.

**Revised claim ceiling:** the direct-wave interpretation is a contested hypothesis. No system may state that the direct wave measures horizon frequency, surface gravity, or area unless the broad-parameter-space falsification campaign survives independent reproduction and calibrated null controls.

### A second, complementary ringdown development: orthonormalized QNMs

A separate Physical Review D paper, *Ringdown analysis of GW250114 with orthonormal modes* (published 2026-08-26), addresses an important identifiability problem: Kerr quasinormal modes are not orthogonal, and simultaneous fitting can induce strong correlations among subdominant modes. In its three-((\ell,m)=(2,2))-mode model through the second overtone, it reports the inferred support for the first overtone increasing from 86.2% in the compared non-orthogonal analysis to 99.9% with the orthonormalized-QNM framework. The earlier arXiv version reported a different baseline value (82.5%); preserve paper/version provenance rather than mixing those values.

Reference: [Suzuki et al., Phys. Rev. D 114, 044099 (2026), doi:10.1103/6x5z-q9ml](https://doi.org/10.1103/6x5z-q9ml); [arXiv:2605.03576](https://arxiv.org/abs/2605.03576).

This does not conflict with the LVK result by default: model basis, start time, included overtones, and inference procedure differ. We should compare methods as distinct analyses, preserving their model assumptions and posterior definitions.

### Required GW250114 comparison matrix

Do not collapse the following into one generic “ringdown” score:

| Track | Scientific object | Required comparison |
|---|---|---|
| LVK QNM analyses | Fundamental mode, first overtone, full-signal constraints and Kerr consistency | Reproduce published posterior / credible regions under the matching data and assumptions |
| Non-orthogonal QNM sum | Correlated damped-mode basis | Evidence, parameter covariance, start-time stability, injection recovery |
| Orthonormalized QNM basis | Reconditioned mode representation | Mode-support calibration, basis-construction sensitivity, held-out injection recovery |
| Rational-filter / mode-cleaning analysis | Filtered separation and evidence curves | Reproduce public qnm_filter pipeline, frequency/damping posteriors, detector consistency |
| Direct-wave model | Additional waveform component tied to (\Omega_H) and (\kappa) | Reproduce the published matched-filter result, test alternatives and null controls, validate out-of-event injections |

The campaign must:
1. Pin exact Zenodo record files, qnm_filter commit, analysis scripts, GWOSC strain release, detector calibration, and noise segment versions.
2. Reproduce the published pipeline before writing an independent implementation.
3. Keep QNM subtraction choices explicit (including which overtones and which remnant posterior are used).
4. Reconstruct the direct-wave template and reproduce the published SNR/posterior summaries in each detector.
5. Compare ordinary QNM, orthonormal-QNM, QNM-plus-direct-wave, and suitably flexible noise/transient alternatives using consistent likelihoods and model-complexity accounting.
6. Run signal injections, off-source time slides, phase/time scrambling, noise-only and wrong-remnant-parameter null controls.
7. Vary start time, segment length, sampling frequency, detector peak time, sky/timing uncertainty, remnant mass/spin, and calibration uncertainty.
8. Test cross-detector coherence and posterior predictive residuals; retain failed or inconclusive runs.
9. Only then attempt an independent Rust implementation of the stable mathematical core, keeping the published Python code as a reference/compatibility path rather than copying its implementation.

Primary success metric: calibrated false-positive rate and injection-recovery / posterior coverage across a preregistered set of realistic noise and signal conditions—not the highest SNR on the one target event.

### New analytic Kerr observables in Symtropy

The initial `symtropy-relativity` crate now includes:
- (\Omega_H = a c/(r_+^2+a^2)), in rad/s;
- (\kappa_t = c\sqrt{r_g^2-a^2}/(2r_g r_+)), in s(^{-1});
- (f_{2\Omega_H}=\Omega_H/\pi), the Hz reference for a component whose angular frequency is (2\Omega_H).

These are Kerr background reference scales. They do **not** constitute a direct-wave waveform model, nor do they imply that a measured feature has been attributed to the horizon. The next implementation must propagate remnant mass/spin posterior samples into correlated (\Omega_H), (\kappa), and reference-frequency distributions rather than evaluating only a best-fit point.

### Updated numerical-relativity state of the art: GRACE

An especially important new comparator appeared immediately before this research update: the GRACE paper was accepted by Physical Review D on 2026-11-02. GRACE is a public C++20/Kokkos/p4est GPU-portable framework that evolves ideal GRMHD coupled to Einstein's equations in Z4c, with constrained transport, AMR, and waveform diagnostics. It reports validation from shock tubes and Bondi accretion through perturbed spinning-puncture ringdown and binary-black-hole evolution, plus binary-neutron-star applications.

Primary references:
- [GRACE paper, arXiv:2607.09854](https://arxiv.org/abs/2607.09854).
- [GRACE source repository](https://github.com/GRACE-astro/grace).
- [GRACEpy post-processing repository](https://github.com/GRACE-astro/GRACEpy).
- [Physical Review D accepted paper, 2026-11-02](https://doi.org/10.1103/yq78-m749).

Its published single-device FMR TOV benchmark (reported as (10^6) zone-cycles/s) is:
| Device | Cowling evolution | Full spacetime evolution |
|---|---:|---:|
| AMD MI300A | 24.8 | 16.8 |
| NVIDIA A100 | 10.5 | 6.7 |
| Intel Xeon Gold 6148 (20 physical cores) | 0.33 | 0.18 |

Important comparability caveats: the A100 configuration used five refinement levels rather than six to fit memory; throughput tests ran 50 timesteps with output disabled except scalar reductions; these are zone-cycle throughput measurements, not time-to-validated-waveform-accuracy. Reported strong-scaling efficiency at the largest tested configurations was approximately 44% (unigrid) and 54% (FMR); weak-scaling efficiency was approximately 90% (unigrid, 2 to 128 devices) and 98% (FMR, 4 to 256 devices).

**Implication for Luminous Dynamics:** do not compete by starting a general-purpose GPU GRMHD solver from scratch. GRACE now makes that an even less credible first move. Use it as an external reproducibility baseline, study its published verification and bottlenecks, and focus our differentiators on:
- independent analytic/tensor/geodesic verification;
- posterior- and uncertainty-aware black-hole inference;
- reproducible, cross-method model discrimination;
- scientific evidence lineage;
- narrow, measured time-to-error or robustness improvements.

GRACE is GPL-3.0; our Apache/MIT analytic crate must not copy or vendor GRACE implementation code. Interoperability and independent validation are the appropriate first relationship.



## Implementation update — 2026-11-07

The Symtropy branch now contains a further bounded implementation increment:

- `metric.rs` provides Schwarzschild-coordinate and Kerr Boyer-Lindquist exterior covariant metric components, with explicit chart-domain checks.
- `MetricComponents4` provides finite/symmetric component validation, pivoted determinant and inverse operations, and an inverse-identity residual diagnostic. The generic matrix constructor does not certify Lorentzian signature.
- `KerrBlackHole` now exposes signed horizon angular velocity, the surface-gravity decay rate in s^-1, and a **nonnegative** Hz reference frequency for the angular-frequency magnitude (2|\Omega_H|). Retrograde spin keeps signed (\Omega_H) but does not produce a negative physical frequency.
- `posterior.rs` maps paired mass/spin posterior samples into horizon radius, signed (\Omega_H), (\kappa_t), and direct-wave reference frequency while preserving sample IDs, order, and mass-spin pairing. It performs no sampling, marginalization, or inference.
- Total authored Rust tests: 27 (13 scalar/root, 10 metric, 4 posterior). They remain unexecuted because this editing environment has neither `rustc` nor `cargo`.

The new posterior mapping is deliberately a deterministic transformation of already inferred remnant samples. It must not be described as a GW250114 analysis or a direct-wave detection. The next required validation is a local build/test on the exact PR head, followed by independent formula review and comparison against published Kerr/ringdown references.


### Third implementation slice — explicit Levi-Civita connection (2026-11-07)

The crate now also contains `connection.rs`:
- `MetricDerivatives4` stores (\partial_\alpha g_{\mu\nu}) with explicit index ordering and rejects non-finite/asymmetric derivative data.
- `LeviCivitaConnection4` computes (\Gamma^\rho{}_{\mu\nu}=\frac12 g^{\rho\sigma}(\partial_\mu g_{\sigma\nu}+\partial_\nu g_{\sigma\mu}-\partial_\sigma g_{\mu\nu})) from a metric and caller-supplied derivatives.
- The API intentionally does not hide finite-difference step sizes or silently differentiate a metric.
- An analytic Schwarzschild exterior derivative fixture and a Cartesian Minkowski zero-connection fixture are included.
- Tests compare selected Schwarzschild coefficients against closed forms, check lower-index symmetry, and reject invalid derivative/chart inputs.

Total authored Rust tests are now **32**: 13 scalar/root, 10 metric, 5 connection, and 4 posterior. They are still unexecuted because this environment has no `rustc` or `cargo`.

This advances the code from an analytic metric store to a first connection calculation. Riemann curvature, coordinate-invariant curvature checks, and geodesic integration remain future gates; no complete GR tensor solver or numerical-relativity claim is made.


## Next implementation gates — opened 2026-11-07

The next two mathematical stages are split into dedicated work items:
- [BH-CURVATURE-001 #3981](https://github.com/Luminous-Dynamics/symtropy/issues/3981): Riemann/Ricci/Einstein tensors, tensor symmetries, Bianchi identities, Schwarzschild/Kerr invariants, and cross-chart scalar checks.
- [BH-GEODESIC-001 #3982](https://github.com/Luminous-Dynamics/symtropy/issues/3982): timelike/null Schwarzschild and Kerr geodesics, conserved quantities, proper-time/affine parameter correctness, and independent Hamiltonian-vs-second-order formulations.

The sequence is deliberately: metric -> first derivatives -> connection -> curvature -> geodesics -> perturbations/waveforms -> NR / detector-data comparisons. Do not skip tensor identities or invariants to reach a visual simulator faster.


## Research update — waveform surrogates and direct-wave model discrimination (2026-11-07)

### 1. Direct-wave evidence now has three competing physical predictors

The Nature report on GW250114 is peer-reviewed evidence for an extracted component under its declared rational-filter analysis. It is not, by itself, proof that the component universally measures Kerr horizon properties.

The debate has materially advanced:

- **Nature report:** Lu et al., 24 June 2026, DOI [10.1038/s41586-026-10696-0](https://www.nature.com/articles/s41586-026-10696-0), report a component in GW250114 with a 90%-credible matched-filter SNR of 15.8 (+0.1/-0.5) in Hanford and 17.1 (+0.1/-0.4) in Livingston, interpreted near 2*Omega_H with damping characterized by kappa. The article itself says these properties create a new channel for probing strong gravity.
- **SXS counteranalysis:** Kankani & McWilliams, [arXiv:2607.02380](https://arxiv.org/abs/2607.02380), report no general correlation between extracted direct-wave frequency and horizon frequency / surface gravity over comparable-mass SXS systems with remnant spin |chi_f| >= about 0.5. They identify an incidental frequency crossing near chi_f≈0.7, note substantial damping evolution, and warn that mapping the component to horizon properties can create apparent area-law violations.
- **BOB interpretation:** Kankani & McWilliams, PRD 114, 084025 (published 7 October 2026), DOI [10.1103/kjjd-nk8f](https://doi.org/10.1103/kjjd-nk8f), report that the direct-wave frequency follows a different spin dependence from the horizon frequency and instead tracks the gravitational-wave News frequency at the time of peak News amplitude.
- **Independent modeling:** Dyer, Chung & Moore, PRL accepted 27 October 2026, DOI [10.1103/xftt-2n6t](https://doi.org/10.1103/xftt-2n6t), report extraction over multiple ringdown start times but find the direct-wave frequency deviates from the horizon value, limiting its utility as a horizon probe.

This changes the central experiment. Compare at least three distinct predictors:
1. Kerr horizon frequency, 2*Omega_H;
2. News frequency at the peak News-amplitude time;
3. Backwards-One-Body / prompt-plunge dynamics.

Also compare freely fitted, time-evolving direct-wave frequency and damping; standard QNM / overtone / mode-mixing residual; and detector/noise/transient alternatives. The answer may depend on mass ratio and on comparable-mass versus EMRI regimes. Do not generalize either way beyond the tested parameter domain.

### 2. Exact SXS spin-sweep cases and filter configuration

Kankani & McWilliams list the following equal-mass, quasicircular, non-precessing SXS simulations:

| SXS ID | Final spin chi_f |
|---|---:|
| SXS:BBH:2089 | 0.4942 |
| SXS:BBH:2088 | 0.5478 |
| SXS:BBH:3919 | 0.6215 |
| SXS:BBH:4434 | 0.6864 |
| SXS:BBH:0305 | 0.6921 |
| SXS:BBH:2095 | 0.7447 |
| SXS:BBH:3927 | 0.8029 |
| SXS:BBH:2099 | 0.8574 |
| SXS:BBH:3897 | 0.9075 |
| SXS:BBH:0178 | 0.9499 |

The paper's rational filter uses 7 prograde QNMs, 2 retrograde QNMs, and an additional prograde (l,m,n)=(3,2,0) mode to remove spherical-spheroidal mixing; analysis focuses on the (2,2) strain mode. It evaluates four fitting intervals around merger; exact interval bounds must be copied from the paper supplement before execution, never guessed.

Sources: [Kankani & McWilliams arXiv manuscript and supplemental table](https://arxiv.org/abs/2607.02380); [SXS waveform database](https://data.black-holes.org/waveforms/).

The corresponding machine-readable manifest is `crates/domains/symtropy-relativity/benchmarks/direct_wave_sxs_spin_sweep.json`.

### 3. New waveform-surrogate state of the art

Raw surrogate speed is advancing quickly, so building another uncalibrated MLP is not a credible differentiator.

| Baseline | Reported capability | Relevance / caveat |
|---|---|---|
| NRSur7dq4 NN surrogate, arXiv:2607.24960 | Validated on 10,000 waveforms; q=1..4, spin magnitudes <=0.8; median sky-averaged mismatch 8.0e-5 to 1.7e-4, 95th percentiles below 1e-3; around 1 ms/waveform on NVIDIA L40S, ~10x single waveform and ~140x batch-64 throughput vs LALSimulation | Fast differentiable inference surrogate; these are its paper-reported hardware/domain results, not Symtropy results |
| NRSur7dq4v2, arXiv:2609.07873 | Multi-domain inspiral/ringdown control, ell<=5 modes, 3-10x gains in remnant mass/spin estimates in reported QNM-fit tests, runtime complexity control, ~4x gwsurrogate package speedup | Strong ringdown-specific comparison; the 3-10x metric is remnant-parameter recovery, not a universal waveform mismatch reduction |
| SEOBNRv5PHM_NNSur7dq10, PRD 114, 084042 | Reduced-order NN surrogate of SEOBNRv5PHM for generically precessing quasicircular binaries up to mass ratio 1:10 | Wider mass-ratio domain; pin exact implementation and validation tables before numerical comparisons |
| AESur3dq8, PRD 114, 083047 | Autoencoder surrogate, mismatch of order 1e-4, reported millions of templates in under a second on modest resources | Different training lineage and domain; compare matched parameter slices |
| NRSurE_q4NoSpin_22, PR Research 8, 023362 | Eccentric nonspinning q~4 waveform surrogate; median mismatch 4e-5, maximum 6e-4 | Useful eccentric benchmark, but not directly comparable to generic precessing surrogates |
| SpEC accuracy study, arXiv:2610.24283 | Targeted domain/resolution choices reduce error; at high resolution floating-point error and disparate variable scales limit accuracy | Increasing resolution is not always monotonically beneficial; precision and nondimensionalization must be benchmarked |

References:
- [NRSur7dq4 NN](https://arxiv.org/abs/2607.24960)
- [NRSur7dq4v2](https://arxiv.org/abs/2609.07873)
- [SEOBNRv5PHM_NNSur7dq10](https://doi.org/10.1103/qt1n-gbv2)
- [AESur3dq8](https://doi.org/10.1103/qqln-f8vy)
- [Eccentric NR surrogate](https://doi.org/10.1103/5m8n-r746)
- [SpEC accuracy limits](https://arxiv.org/abs/2610.24283)

The versioned literature baseline lives at `crates/domains/symtropy-relativity/benchmarks/waveform_surrogate_baselines.json`.

### 4. Our better-than-SOTA strategy: error-certified scientific surrogates

Do not compete first on headline throughput. Compete on **time-to-posterior-accuracy with calibrated uncertainty**:

- explicit parameter-domain and mode coverage;
- held-out validation split by simulation family/configuration, not randomly adjacent parameter points;
- calibrated waveform error distributions including tail coverage;
- out-of-domain detection and safe abstention;
- cross-model disagreement (NRSur7dq4, v2, LALSimulation/EOB, BOB) as an uncertainty and active-learning signal;
- remnant mass/spin, QNM and ringdown consistency checks;
- propagate waveform/model discrepancy into posterior coverage;
- select the next expensive NR simulation where uncertainty or model disagreement most reduces posterior uncertainty;
- benchmark against random and space-filling simulation selection at the same simulation budget.

The target is not a universal surrogate. It is a bounded, independently validated surrogate whose applicability and uncertainty can be trusted.

A dedicated work item is [BH-SURROGATE-CERT-001 #3998](https://github.com/Luminous-Dynamics/symtropy/issues/3998).

### 5. New public data products for replication

In addition to GWOSC strain and SXS numerical-relativity waveforms, pin:
- [GWTC-5.0 Tests of General Relativity data release, Zenodo 21454847](https://zenodo.org/records/21454847), including FTI_TIGER, POL, pSEOB, QNMRF, RD and RES analysis products.
- [Orthonormal QNM GW250114 data release, Zenodo 19601938](https://zenodo.org/records/19601938), with a 9.9 GB ringdown archive; use its published README and file hashes.
- [Nature direct-wave analysis Zenodo package, 10.5281/zenodo.20017347](https://doi.org/10.5281/zenodo.20017347).

These datasets support exact reproduction of multiple independent pipelines. Their hashes and software revisions should be included in Mycelix evidence packages.


### Additional precision-GR direction — beyond-Teukolsky inference (2026-11-07)

Völkel & Franchini, *Constraining deviations from the Teukolsky equation with GW250114*, was accepted by Physical Review D on 2026-09-10 (DOI 10.1103/zzzw-vpgx). It maps theory-agnostic effective-potential deviations in the Teukolsky perturbation equation to QNM-spectrum shifts, using LVK remnant mass/spin information and public agnostic damped-sinusoid posterior data. The paper reports present bounds consistent with the Teukolsky equation under its declared deformation family and likelihood approximation.

Reference: [APS accepted paper](https://journals.aps.org/prd/accepted/10.1103/zzzw-vpgx).

This adds a third distinct GW250114 analysis axis:
1. mode-based QNM inference (ordinary / orthonormalized basis);
2. direct-wave residual interpretation (contested by the SXS spin-sweep counteranalysis);
3. deviations in the underlying perturbation equation / effective potential (beyond-Teukolsky).

These must not be conflated. A bound consistent with zero deviation constrains only the declared effective-potential deformation family and assumptions; it does not prove all of GR or exclude arbitrary modified gravity.

Execution issue: [Symthaea BH-TEUKOLSKY-001 #10874](https://github.com/Luminous-Dynamics/symthaea/issues/10874). Mycelix and Sol Atlas source/evidence requirements are recorded in #5903 and #586.

### Current mathematical implementation — 2026-11-07

The clean successor branch now carries the following Rust scope in `symtropy-relativity`:

- **Parameters and observables:** SI mass/spin validation, Schwarzschild/Kerr horizons, photon sphere, ISCO, ergosurface, redshift, Schwarzschild Kretschmann, Kerr (\Omega_H) and surface-gravity rate.
- **Metric charts:** Schwarzschild and Kerr exterior coordinate metrics, ingoing Eddington-Finkelstein Schwarzschild coordinates, and ingoing Kerr coordinates.
- **Linear algebra:** finite/symmetry checks, pivoted metric determinant/inverse, inverse-identity residual diagnostic.
- **Connection:** Levi-Civita connection from explicit metric derivatives; no hidden finite-difference step.
- **Curvature:** mixed Riemann, Ricci, scalar curvature, Einstein tensor, Kretschmann contraction, and algebraic Riemann symmetry / first-Bianchi residuals from explicit first/second derivatives.
- **Automatic differentiation:** fixed-size second-order forward-mode Jet2 values/gradients/Hessians for Schwarzschild, Kerr, ingoing Schwarzschild, and ingoing Kerr.
- **Posterior propagation:** deterministic paired mass/spin sample transformation into Kerr horizon observables preserving sample ID and sample pairing.

The current branch contains **50 authored Rust tests**: 13 scalar/root, 10 metric, 5 connection, 6 curvature, 12 automatic-differentiation/chart, and 4 posterior. They are not executed in this tool environment because rustc/cargo are unavailable.

Independent numerical calculations (separate from the Rust code) reproduce:
- Schwarzschild (K=48r_g^2/r^6) at a solar-mass-scale exterior point to floating-point precision;
- Kerr vacuum Ricci residual at approximately (7\times10^{-18}) in normalized mixed components for (chi=0.8), (r=7r_g), (	heta=0.8);
- Kerr analytic Kretschmann at the same fixture to floating-point precision;
- ingoing Kerr chart determinant (-\Sigma^2\sin^2\theta), vacuum Ricci, and analytic Kretschmann at, outside, and inside the outer horizon.

These independent numerical calculations are formula checks, not compilation or test passes. No solver performance or observational reproduction is claimed.

### Horizon-regular Kerr coordinate milestone

The ingoing Kerr chart avoids the Boyer-Lindquist horizon coordinate singularity. Its metric includes the expected (dv,dr), (dr,d\phi), and (dv,d\phi) couplings; its determinant is (-\Sigma^2\sin^2\theta). AD-generated curvature tests check the vacuum and Kretschmann identities at the horizon and inside it, and compare the scalar invariant against Boyer-Lindquist coordinates outside the horizon.

This is a significant coordinate-invariance check, but it is not yet a general coordinate-transform engine or a full regularity proof for every component / tetrad at all limiting points.


## Consolidated implementation update — 2026-11-07

The clean successor PR now combines the Schwarzschild/Kerr analytic geometry, AD curvature work, reproducible manifests, and the stronger Schwarzschild geodesic follow-on.

### Current source scope in `symtropy-relativity`

- **Analytic stationary geometry:** validated SI mass/spin, Schwarzschild radii/redshift/Kretschmann, Kerr horizons, ergosurface, horizon angular velocity, surface gravity, and sample-preserving Kerr posterior mapping.
- **Metric charts:** Schwarzschild and Kerr exterior charts; ingoing Eddington-Finkelstein Schwarzschild and ingoing Kerr charts.
- **Differential geometry:** explicit Levi-Civita connection and a normalized metric-compatibility diagnostic.
- **Curvature:** mixed Riemann, Ricci, scalar curvature, Einstein tensor, Kretschmann contraction, algebraic Riemann symmetries and first-Bianchi residuals; explicit second-derivative and second-order AD paths.
- **Automatic differentiation:** fixed-size second-order forward-mode Jet2 metric values, gradients and Hessians for Schwarzschild/Kerr and ingoing charts.
- **Geodesics (Schwarzschild exterior only):** second-order Christoffel RK4 and first-order canonical Hamiltonian RK4; step-doubling adaptive control for the coordinate formulation; component-wise dimensional tolerances; normalization, Killing energy, axial angular momentum, and Hamiltonian diagnostics.
- **Numerical hardening:** rationalized Kerr inner horizon for small spin; dimensionless surface-gravity expression near extremality; factored Kerr `Delta=(r-r_+)(r-r_-)` near the exterior horizon.

The integrated source currently has **63 authored Rust tests**:
- 15 scalar/parameter/horizon physics;
- 11 metric and matrix tests;
- 6 connection and metric-compatibility tests;
- 6 curvature tests;
- 12 AD/chart tests;
- 9 Schwarzschild geodesic tests;
- 4 posterior mapping tests.

These tests are **not executed** in the editing environment because `rustc` and `cargo` are unavailable. No build, test, CI, performance, waveform, or observational reproduction pass is claimed.

### Machine-readable benchmark manifests

The clean PR now carries five JSON manifests:
1. `analytic_reference_cases.json` — SI-valued Schwarzschild/Kerr parameter and horizon-observable reference grid.
2. `direct_wave_sxs_spin_sweep.json` — 10 high-spin SXS cases plus the 168-system / 13 CCE broader catalog manifest, competing horizon/News/BOB/light-ring/QNM/noise predictors and filter-artifact controls.
3. `geodesic_reference_cases.json` — Schwarzschild circular timelike and radial null fixtures, parameterization, conserved quantities, and reference endpoints.
4. `waveform_surrogate_baselines.json` — current NR surrogate, EOB, BOB, eccentric and SpEC accuracy literature comparators.
5. `waveform_surrogate_certification_v1.json` — leakage-resistant validation splits, calibrated waveform error, posterior coverage, OOD abstention and active NR-selection protocol.

All registries are specifications or source-pinned literature records. They are not execution results.

### Geodesic limitations and next gates

The new geodesic engine deliberately remains Schwarzschild-only and exterior-chart-only. It does not yet support Kerr geodesics, Carter-constant conservation, horizon crossing in ingoing charts, a symplectic integrator, or an adaptive Hamiltonian path. Its adaptive step-doubling controller estimates local error; it is not a rigorous global error bound.

Next:
- run the exact PR-head build/tests when a Rust toolchain is available;
- independently review the geodesic equations, Hamiltonian sign, units, and chart-domain handling;
- compare fixed-step vs adaptive convergence and invariant drift on held-out circular, radial, and near-critical trajectories;
- then implement Kerr equatorial geodesics and the Carter constant, followed by inclined Kerr orbits;
- maintain external NR / waveform comparisons as separate workstreams—this crate is not a numerical-relativity solver.

### PR consolidation

The older v3 draft supplied the adaptive / Hamiltonian Schwarzschild geodesic implementation and its reference manifest. Those are now carried into the clean successor PR, together with its analytic-reference and surrogate-certification manifests. The clean PR is the single continuation target; parallel earlier drafts are retained only as historical records and should not be merged independently.


## New implementation slice — noise-weighted waveform comparison (2026-11-07)

The clean branch now includes `crates/domains/symtropy-relativity/src/waveform_metrics.rs` and exports it from the crate.

The implementation provides:
- discrete one-sided PSD inner product (4\Delta f\sum_k \overline{h_{\rm ref}}h_{\rm cand}/S_n(f_k));
- constant-phase-maximized overlap / mismatch, with the best constant phase offset returned explicitly;
- separate PSD-weighted relative-amplitude L2 error;
- reference-amplitude / PSD-weighted frequency-dependent phase residual RMS and per-bin diagnostics;
- strict finite, positive-PSD, uniform-grid and identical reference/candidate/PSD frequency-grid validation;
- no hidden time maximization, interpolation, resampling, whitening, windowing or PSD estimation.

The caller must pre-align waveforms under a pinned procedure. The metric deliberately does not hide time alignment, because doing so can disguise phase/model errors. A manual adversarial review caught and corrected a missing candidate-grid comparison before the branch was updated.

Ten synthetic Rust tests are authored for identical waveforms, constant phase rotation, amplitude rescaling, frequency-dependent phase error, PSD weighting, mismatched / nonuniform grids, invalid PSD, zero reference norm, and invalid frequency bands. The total suite is now **73 authored tests** (63 previous plus 10 waveform-metric tests). They remain unexecuted because the current editing environment has no `rustc` or `cargo`.

The waveform surrogate certification manifest now points to this implementation and specifies the next validation steps: independent NumPy formula comparison, a pinned SXS waveform pair, PSD/alignment sensitivity, and posterior-recovery injection with propagated model discrepancy. No real waveform comparison or surrogate result has yet been produced.

Dedicated work item: [BH-WAVEFORM-METRICS-001 #4048](https://github.com/Luminous-Dynamics/symtropy/issues/4048).

## New nonlinear ringdown axis — quadratic QNM coupling (2026-11-07)

Yang, Shi & Hu, *Constraining the Quadratic-mode Amplitude Coupling in GW250114*, arXiv:2609.28953 (submitted 2026-09-24): https://arxiv.org/abs/2609.28953.

The preprint analyzes the 220 fundamental, 221 first overtone, and a 220×220 quadratic mode. It infers a mode-specific amplitude-coupling coefficient and reports consistency with NR-fit / second-order perturbation predictions within 1.3 sigma, while noting that current data cannot distinguish the spin-dependent predictions.

This creates a fourth distinct GW250114 ringdown test axis:
1. linear QNM basis inference (ordinary / orthonormalized);
2. direct-wave residual extraction and its contested physical interpretation;
3. linear perturbation-equation / beyond-Teukolsky effective-potential deviations;
4. nonlinear second-order quadratic-mode amplitude coupling.

Do not conflate a quadratic model term with an independently resolved nonlinear mode. Required controls include extra linear overtones, mode mixing, flexible residuals, noise/transients, no-quadratic injections, parent-amplitude-product scaling, spin dependence, start-time stability, prior sensitivity and calibrated posterior coverage.

Dedicated execution item: [Symthaea BH-NONLINEAR-RINGDOWN-001 #10908](https://github.com/Luminous-Dynamics/symthaea/issues/10908). Mycelix and Sol Atlas provenance/display requirements are recorded in #5903 and #586.



## Eikonal QNMs, local light-ring geometry, and prior identifiability (2026-11-07)

De Simone, Völkel & Kokkotas, arXiv:2609.24824 (2026-09-21), “Constraining the geometry of rotating black holes with eikonal QNMs,” compares a global metric-specific Konoplya–Rezzolla–Zhidenko parameterization with a metric-agnostic representation of local metric values and radial derivatives at the light ring.

Important reported lessons:
- Uniform priors on global metric coefficients can induce strongly nonuniform and skewed priors on local light-ring quantities.
- One-parameter analyses can fail to recover injected local deviations and can bias global metric reconstruction.
- Multiparameter analyses are more robust to injections but expose substantial degeneracies in marginalized parameters.
- Their application to approximate GW250114 ringdown posteriors is consistent with Kerr within the declared model families.

Reference: [De Simone et al., arXiv:2609.24824](https://arxiv.org/abs/2609.24824).

This suggests an important paradigm shift: infer local physically observable quantities first, then reconstruct global metric families only where identifiable. A global model prior should not silently dictate a highly structured prior on the local geometry being tested.

Albuquerque & Völkel, arXiv:2608.01061, “Optimal frequency scales for probing black-hole geometries,” studies an information-theoretic tradeoff in a controlled scattering experiment: very short high-frequency pulses resolve small scales but transmit through the potential barrier; broad pulses reflect efficiently but resolve poorly. Their model finds an optimal pulse width set approximately by the inverse square root of the potential peak. This is a model-specific result, not a universal frequency prescription. Reference: [arXiv:2608.01061](https://arxiv.org/abs/2608.01061).

### Symtropy equatorial Kerr light-ring reference implementation

The clean branch now includes `src/light_ring.rs`, exported by the crate. It computes:
- prograde/retrograde equatorial Kerr photon-orbit radius using the closed-form solution;
- signed coordinate angular frequency in rad/s;
- impact parameter (b=L/E);
- local metric null-condition and radial circularity residuals;
- coordinate-time radial Lyapunov exponent from the equatorial null radial potential;
- explicit extremal-prograde horizon-limit status, where Boyer–Lindquist impact/Lyapunov values are unavailable rather than fabricated.

The Lyapunov calculation uses the (E=1) equatorial Kerr null radial potential
(R(r)=[r^2+a^2-ab]^2-\Delta(b-a)^2), including a factorized (\Delta=(r-r_+)(r-r_-)) for near-horizon stability. The eikonal interpretation remains asymptotic in angular mode number; these quantities are not exact finite-(\ell) QNM eigenfrequencies.

Eight Rust tests are authored for the Schwarzschild limit ((r_{ph}=3r_g), (lambda_{ph}=|\Omega_{ph}|)), orientation, prograde/retrograde ordering, local circular-null residuals, extremal retrograde limit, extremal prograde horizon limit, and near-extremal finite behavior. A 20-case independently generated reference grid is in `benchmarks/light_ring_reference_cases.json`. Total authored tests are **81**; all remain unexecuted because no Rust toolchain is available in this environment.

Dedicated work:
- [Symtropy BH-LIGHTRING-001 #4051](https://github.com/Luminous-Dynamics/symtropy/issues/4051)
- [Symthaea BH-EIKONAL-GEOMETRY-001 #10909](https://github.com/Luminous-Dynamics/symthaea/issues/10909)

### Time-domain perturbation theory as an independent validation axis

The frequency-domain QNM/eigenvalue description should not be the sole reference for ringdown science. Recent work provides complementary time-domain methods:

- De Simone et al., *Parametrized beyond-Teukolsky framework in the time domain*, PRD 114, 064060 (2026-09-15), DOI [10.1103/93dw-jlcg](https://doi.org/10.1103/93dw-jlcg), performs 2+1D Gaussian-wavepacket scattering and follows prompt response, ringdown and late-time tails. The paper reports good agreement with frequency-domain eigenvalues at low azimuthal number, but mode mixing limits higher-m extraction; near-horizon potential deformations can change amplitude/phase and the onset of tail behavior.
- De Simone, *Discrete symmetries of modified Teukolsky equations*, PRD 114, 084063 (2026-10-21), DOI [10.1103/dbzz-jhjb](https://doi.org/10.1103/dbzz-jhjb), studies how potential deformations change discrete symmetries, break an (m=0) QNM degeneracy, and can introduce nonphysical branches if frequency-domain modified potentials are naively evolved in time.
- Vaswani et al., *Time-domain framework for the Teukolsky equation with a particle source using comoving hyperboloidal coordinates*, PRD 114, 084073 (2026-10-23), DOI [10.1103/2t8s-g9h3](https://doi.org/10.1103/2t8s-g9h3), develops a 1+1D time-domain particle-source implementation with comoving spatially compactified hyperboloidal coordinates and jump conditions. It reports avoiding nonphysical growing modes in its tests and includes an (s=-2) Teukolsky test implementation.

The important capability gap is not “we need a production Teukolsky solver immediately.” It is a benchmarked bridge between:
1. stationary Kerr geometry and light-ring references;
2. frequency-domain eigenvalue/QNM predictions;
3. time-domain wave-packet / particle-source evolution;
4. the extracted prompt, ringdown, and tail signals;
5. observation-space likelihood and uncertainty.

Do not treat a discretized frequency-domain potential evolved in time as automatically physical. Symmetry constraints, boundary conditions, hyperboloidal compactification, spurious-mode detection and cross-checks against eigenvalue solutions must be explicit acceptance tests.

Related Symthaea work: [BH-TEUKOLSKY-001 #10874](https://github.com/Luminous-Dynamics/symthaea/issues/10874), with eikonal-prior/identifiability work in #10909. The external time-domain studies are reference systems; no Teukolsky evolution solver is implemented by this PR.



### New staged implementation issue — hyperboloidal time-domain perturbations

The external-method reproduction and solver-validation work is now tracked in [Symtropy BH-TEUKOLSKY-TD-001 #4052](https://github.com/Luminous-Dynamics/symtropy/issues/4052). It is staged as coordinate-contract validation, scalar-field vacuum/source tests, the declared s=-2 test problem, and only then modified-potential / beyond-Teukolsky evolution. Its acceptance gates include long-time growing-mode suppression, boundary and source-jump validation, frequency-domain cross-checks, mode-mixing diagnostics and late-time tail behavior.

The reference paper states that implementation code is included, but no source repository has been independently pinned in this programme yet. Do not invent a GitHub code URL or treat the paper's abstract as a reproducible software artifact.



### Light-ring validation hardening — normalized radial-potential residuals

The Kerr light-ring module now reports normalized residuals for both (R=0) and (R'=0) from the equatorial null radial potential, in addition to metric null and radial circularity residuals. The normalization uses the magnitudes of the corresponding analytic terms, so near-extremal cancellation is visible and can be given a separate, explicit tolerance. This is an internal analytic consistency check, not an independent QNM validation.



## New execution workstream — posterior-calibrated waveform discrepancy (2026-11-07)

A new machine-readable protocol is now in `crates/domains/symtropy-relativity/benchmarks/waveform_discrepancy_calibration_v1.json`, linked to [Symthaea BH-WAVEFORM-DISCREPANCY-001 #10944](https://github.com/Luminous-Dynamics/symthaea/issues/10944).

The research rationale is concrete:
- Hoy et al., Nature Astronomy (2025), DOI 10.1038/s41550-025-02579-7, propose accuracy-informed Bayesian model combination and report lower compute cost / more faithful recovery on their simulated comparison.
- Bachhar et al., PRD 111, 084050 (2025), DOI 10.1103/PhysRevD.111.084050, use GPR for frequency-domain amplitude/phase residuals and marginalize waveform uncertainty.
- Kumar et al., PRD 112, 104071 (2025), DOI 10.1103/nkq9-zh2y, parameterize amplitude/phase error priors and show that small phase errors can bias parameters.
- Mezzasoma et al., PRD 113, 124043 (2026), DOI 10.1103/y6lc-fr77, show NR calibration error can create false pPE deviations in simulated high-SNR signals and that their uncertainty-aware calibration protects the GR-consistent result in the studied setup.

The proposed extension is not “use GPs” as a novel claim. It is a joint, parameter-dependent discrepancy model that preserves cross-frequency, cross-mode, and source-parameter covariance; separates detector noise/calibration from NR truncation, surrogate interpolation, approximant model-form error, missing modes/domain shift and algorithmic precision; and propagates the resulting discrepancy through posterior inference.

Primary evaluation is posterior calibration (50/90/95% coverage with confidence intervals), posterior bias and sharpness, proper scoring rules, residual checks, false-positive rate for GR deviations, OOD abstention, and time-to-posterior-accuracy. The manifest defines whole-simulation train/test grouping, untouched test IDs, blind injections across SNR 10–330, adversarial model errors and active-NR acquisition baselines.

**Execution status:** manifest specification only. No discrepancy process has been fitted, no posterior injection campaign has been run, and no improved calibration result is claimed.


### Independent formula-level light-ring cross-check — 2026-11-07

A separate V8 calculation re-evaluated all 20 cases in `light_ring_reference_cases.json`: it recomputed the closed-form Kerr equatorial photon radius and signed coordinate frequency, then reconstructed (b=L/E), the null radial potential (R), (R'), (R''), and the coordinate-time radial Lyapunov exponent from the metric / geodesic first integrals.

Results against the checked-in reference grid:
- maximum relative difference in radius: 0 at the stored precision;
- maximum relative difference in signed angular frequency: 0 at the stored precision;
- maximum relative difference in impact parameter: 0 at the stored precision;
- maximum relative difference in radial Lyapunov exponent: (3.3\times10^{-11});
- maximum normalized (R) residual: (1.5\times10^{-9}), at the near-extremal prograde case (|\chi|=0.999999);
- maximum normalized (R') residual: below the displayed precision of the independent calculation.

This is a formula-level reimplementation / reference-grid cross-check, **not** execution of the Rust tests, an external numerical-relativity comparison, or a finite-(\ell) QNM validation. The near-extremal residual is an explicit conditioning signal and should remain separately tracked rather than hidden behind a global tolerance.


## New horizon-semantics workstream — 2026-11-07

A typed horizon contract is now specified in `crates/domains/symtropy-relativity/benchmarks/horizon_object_contract_v1.json`, tracked by [Symtropy BH-HORIZON-SEMANTICS-001 #4071](https://github.com/Luminous-Dynamics/symtropy/issues/4071).

The contract distinguishes stationary Kerr event/Killing horizons, retrospective event-horizon reconstructions, MOTSs, apparent horizons, marginally trapped tubes, dynamical horizons, isolated horizons and unsupported/ambiguous outcomes. It requires the relevant Cauchy slice, future extent, null-normal orientation/normalization, expansion residuals, foliation/branch lineage, signature, area/flux data and finder provenance.

Scientific motivation:
- Ashtekar & Krishnan's 2025 Living Reviews article explains why event horizons are teleological and why quasi-local horizons are more useful during numerical evolution.
- Event-horizon finders require the future spacetime development; apparent horizons are found on specified Cauchy slices.
- External reference implementations such as BHaHAHA and SpECTRE horizon tracking should be interoperated with before a bespoke surface finder is considered.

This is a semantics/schema protocol only. It does not implement an apparent-horizon finder, event-horizon reconstruction engine, dynamical-horizon flux solver, or a merger-horizon result. No such scientific claim is made.
