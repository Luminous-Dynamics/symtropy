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
