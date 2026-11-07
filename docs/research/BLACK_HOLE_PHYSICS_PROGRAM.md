# Extreme Spacetime Physics: Black-Hole Research Program

**Status:** research charter; implementation work is tracked in the four linked repository issues.  
**Last researched:** 2026-11-06.  
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

