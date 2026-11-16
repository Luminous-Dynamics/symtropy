---
title: Muon Collider Beam-State Handoff and End-to-End Qualification Contract
version: 0.1.0
status: proposed
scope: interoperable beam-state handoffs, uncertainty, loss accounting, and end-to-end benchmarks
---

# Muon Collider Beam-State Handoff and End-to-End Qualification Contract

**Status:** Proposed research contract, version 0.1.0  
**Scope:** Production/capture through cooling, acceleration, collider ring, machine–detector interface, and detector reconstruction  
**Related work:** Symtropy issue #5669; demonstrator roadmap #5636; acceleration survival #5606; RCS–FFA comparison #5608

## 1. Purpose

A muon collider is not qualified by independently excellent subsystem scores. A target can produce more pions while delivering fewer muons inside the cooling acceptance. A cooling channel can reduce a quoted emittance while losing useful population in the tails. An acceleration stage can increase energy while degrading longitudinal acceptance. A lattice can improve luminosity while worsening detector backgrounds. A detector can reconstruct more generated events while increasing fake or incorrectly associated objects.

The contract defines what must cross each stage boundary so that these trade-offs remain visible and auditable.

The central research quantity is **useful, independently qualified reconstructed signal yield under explicit physical and resource constraints**. That quantity is evaluated by physics channel and by operating scenario; it is not a single opaque score.

The November 5, 2026 National Lab Accelerator Study Group report states that subsystem requirements are strongly coupled, that the complete production-to-collision sequence has not been demonstrated as an integrated system, and that an end-to-end production → cooling → acceleration → detector framework, code coupling, prototype-data validation and AI-assisted workflows are priorities. This contract turns that integration need into a concrete data and qualification boundary.

## 2. Non-goals

This contract does not:
- replace FLUKA, Geant4, WarpX/ImpactX, BMAD, MAD-X, XSuite, RF-Track, BDSIM, GUINEA-PIG or other specialist solvers;
- define a new general-purpose particle tracker, PIC solver, radiation-transport code or detector reconstruction engine;
- assume that every solver uses the same internal coordinates or data format;
- convert a simulated result into an experimental result;
- infer missing beam distributions from RMS values;
- claim physical qualification merely because a file validates against a schema.

The intent is an **evidence envelope plus adapter contract** around specialist tools.

## 3. System boundary

The canonical research chain is:

    proton driver
      → target and pion production
      → capture and initial muon distribution
      → 6D cooling and bunch preparation
      → pre-acceleration and recirculating linacs
      → RCS / FFA / plasma / hybrid acceleration
      → collider ring and beam–beam interaction
      → machine–detector interface and decay-background transport
      → detector simulation, digitization and reconstruction
      → physics-channel acceptance and useful event yield

Not every experiment exercises every stage. Each omitted stage must be marked as outside the tested boundary; it must not be silently treated as lossless or perfect.

Every handoff is a typed record for one particle species, bunch or explicitly declared group. Opposite-sign beams, secondary species and separate bunches may use separate records linked by parent identifiers.

## 4. The handoff has two planes

### 4.1 Data plane

The data plane contains the actual particle distribution or a declared reduced representation.

A full-distribution handoff must retain an immutable asset in an appropriate binary or native solver format. The contract does not require large macro-particle arrays to be embedded in JSON. It requires the asset's format, version, byte length, digest, sampling semantics and coordinate mapping to be declared.

A summary-only handoff is valid as a summary, but cannot pass a downstream gate that requires tails, correlations, particle ancestry or full phase-space propagation.

### 4.2 Evidence plane

The evidence envelope contains:
- schema and handoff identity;
- stage and run identity, parent handoffs and random seeds;
- particle species and charge convention;
- reference plane, coordinate frame and units;
- immutable distribution-asset URI and digest, where applicable;
- weighted population and loss ledger;
- distribution summaries, tail metrics and effective sample size;
- separate uncertainty components;
- solver, configuration, toolchain and reference provenance;
- qualification status, validity domain and claim ceiling;
- downstream acceptance requirements and decision.

An envelope is not a substitute for the asset it describes.

## 5. Six-dimensional coordinate contract

The canonical interchange basis is the ordered local phase-space vector:

    (x, px, y, py, z, pz)

where:
- x, y and z are positions in metres;
- px, py and pz are momenta in GeV/c;
- z is the local longitudinal coordinate along the declared reference trajectory;
- the local coordinate frame is right-handed;
- positive z follows the declared beam direction.

This basis is an interchange convention, not a claim that every native solver uses these canonical variables. Each adapter must publish its explicit, testable transformation from native variables, including any sign convention, reference momentum, reference particle, time origin, longitudinal coordinate convention and units.

The envelope must distinguish z, ct, time, relative momentum deviation, relative energy deviation and RF phase. They are not interchangeable labels. A conversion between them must cite its equation, assumptions and reference quantities.

The reference plane and local frame must be updated at each physical boundary. In curved lattices, the adapter must state how the local frame is transported and how the coordinate transformation is applied. A silent frame reset is invalid.

The following are separate definitions and must never be collapsed into an unqualified field named “emittance”:
- geometric versus normalized emittance;
- RMS versus 95% or other percentile emittance;
- projected versus eigen-emittance;
- transverse versus longitudinal versus 6D emittance;
- coordinate and momentum conventions used to calculate covariance;
- rms bunch length versus full bunch extent;
- rms energy spread versus full or percentile energy spread.

Every reported value carries its definition and units.

## 6. Population and loss accounting

A stage reports weighted physical population, not merely the number of macro-particles in a file.

For each particle species, the ledger records:

    computed_residual = input_weight + generated_weight
                        - surviving_weight - sum(weighted_losses)

The declared balance residual must equal that computed residual and its
absolute value must be within the declared relative tolerance. The residual
is an independently checked discrepancy, not an extra balancing term.
For `particle_production`, `input_weighted_count` is zero and
`generated_weighted_count` is the output-species population created before
losses. The input/source species and its population are recorded separately
in `source_accounting`; proton population must not be added to muon
population as if they were the same conserved species.

The residual is computed and compared with a declared numerical tolerance. The validator must reject unexplained positive residuals, double-counted losses, hidden reweighting and inconsistent parent/child bunch accounting.

Loss causes are explicit, and may include:
- muon decay;
- aperture or dynamic-aperture loss;
- RF-bucket or momentum-acceptance loss;
- material interaction, absorption or scattering out of acceptance;
- deliberate collimation;
- detector selection or reconstruction rejection;
- other declared loss;
- unknown.

“Unknown” is an admissible result; silently relabeling it is not.

Particle production changes species. Proton losses, pion production, muon generation and captured muons therefore require separate species ledgers plus an explicit production relation or yield definition. Do not apply a same-species conservation equation across a proton-to-pion-to-muon transformation.

Bunch splitting, merging and resampling are permitted when their transformations are described. The record must retain input/output lineage, weights, the number of represented physical particles, and any dependence introduced by resampling. Reweighting must never be used to manufacture physical yield.

## 7. Distribution fidelity, correlations and tails

The full joint 6D distribution is the preferred handoff. A list of marginal RMS values cannot reconstruct it.

The evidence summary should include, where supported:
- weighted population and transmission;
- mean vector and full covariance matrix, with coordinates and units;
- declared geometric/normalized and projected/eigen emittances;
- momentum, energy and bunch-time distributions;
- selected high percentiles and tail exceedance probabilities;
- transverse and longitudinal–transverse correlations;
- macro-particle count, effective sample size and sampling uncertainty;
- sampling method, seed set and parent/ancestry relationship.

Tail thresholds must be tied to a physical decision: aperture, RF acceptance, detector occupancy, heat deposition, radiation dose or another declared constraint. An arbitrary tail score is not a substitute for the underlying distribution.

Azimuthal cloning or other synthetic expansion must not be reported as new independent events. If resampling or augmentation is used for a surrogate or training set, the dependence structure, ancestry, weights, transformation and effective sample size must remain available.

## 8. Uncertainty contract

Keep at least these uncertainty classes separate:

1. **Monte Carlo/statistical:** finite macro-particle or event sample, including weighted-sample effective size.
2. **Numerical:** timestep, mesh, solver tolerance, convergence and discretization effects.
3. **Model-form:** missing or approximate physics, such as neglected collective effects or simplified material response.
4. **Input/parameter:** uncertain magnet, RF, target, material, geometry, alignment or calibration inputs.
5. **Measurement:** instrument resolution, calibration, sampling and experimental uncertainty.

Do not combine these into one standard deviation unless the dependence model and combination rule are declared. Preserve correlations and common-cause dependencies when they affect downstream risk. A distribution with non-Gaussian tails must not be summarized as Gaussian merely for convenience.

A value may be quantified, bounded, or explicitly unquantified. Missing uncertainty is not zero uncertainty.

## 9. Qualification state machine

Each stage result has one of four acceptance states:

- **qualified:** all declared acceptance requirements passed within the stated validity domain;
- **conditional:** some requirements pass, but named conditions or exclusions remain;
- **rejected:** a required condition failed;
- **not evaluated:** evidence is absent or insufficient.

The validation tier is a separate axis:

1. **schema-only:** envelope and asset references conform;
2. **analytical:** closed-form or independently calculated reference checks pass;
3. **code-to-code:** matched inputs have been compared across independent implementations;
4. **experiment-correlated:** comparison against identified measurements, with a declared calibration/validation split;
5. **independently replicated:** a separate team, facility or method reproduced the stated result.

A higher tier is not implied by a lower tier. A well-formed receipt may have a qualification state of “not evaluated”.

Every result carries:
- validity domain and excluded regimes;
- exact input and configuration digests;
- solver name, version, commit and build environment;
- random seed(s), physics list and material database where relevant;
- convergence or tolerance criteria;
- reference datasets and comparison method;
- claim ceiling.

## 10. Stage acceptance contracts

Every stage publishes four things.

**Input requirements:** species, intensity, bunch structure, 6D distribution fidelity, reference frame, timing, energy and uncertainty bounds it can accept.

**Output guarantees:** the state actually produced, population ledger, distribution summaries and uncertainty components.

**Failure modes:** the causes of loss, invalidity, non-convergence or unsupported physics.

**Downstream acceptance:** explicit thresholds and the receiving stage's accept/reject/conditional decision.

A receiving stage must validate units, frames, particle identity, reference quantities, weights, digests and required distribution fidelity before using the input. If any required property is unknown or incompatible, the handoff is blocked or marked conditional. No silent default may promote it to qualified.

## 11. End-to-end objective: useful physics, not subsystem vanity metrics

The system objective is a constrained vector rather than a weighted score chosen for convenience.

It includes:
- delivered and integrated luminosity;
- reconstructed signal yield by physics channel;
- beam transmission and 6D quality at every boundary;
- background occupancy, rare tails, fake rates and reconstruction efficiency;
- wall-plug energy, peak/average power and operating availability;
- target, absorber, RF and magnet lifetime and maintenance burden;
- facility footprint and lifecycle resources;
- radiation, machine protection and other hard safety constraints.

For a named physics channel, expose the complete factorization:

    N_signal = integrated_luminosity
               × cross_section
               × branching_fraction
               × geometric_acceptance
               × reconstruction_efficiency
               × selection_efficiency

Report the uncertainty and provenance of every factor. Report background and systematic uncertainty alongside signal. Generated events, transported muons or a larger optimizer score do not establish useful reconstructed yield.

The optimization result is a constrained Pareto set. Machine protection, radiation, and minimum beam-quality requirements are hard gates, not costs that an optimizer can trade away. Resource objectives can include useful reconstructed signal per wall-plug MWh, per operating day, per lifecycle cost envelope and per facility footprint, but these ratios must be reported alongside their numerators and denominators.

## 12. Benchmark protocol

Use a qualification ladder so that each claim is tied to the strongest evidence actually available.

### A. Contract and accounting tests

Test:
- missing or ambiguous units and coordinate conventions;
- reference-frame and reference-plane mismatches;
- duplicated or missing particle identifiers;
- invalid parent/child relationships;
- weighted population balance and loss partitioning;
- hidden reweighting and synthetic sample inflation;
- absent or mismatched asset digests;
- summary-only inputs presented as full distributions;
- unquantified uncertainty incorrectly promoted to zero;
- unqualified upstream states passed to a qualified downstream stage.

Every negative control must fail for the intended reason. Tests must not weaken a gate to make a fixture pass.

### B. Analytical references

Start with exact decay-hazard integration, coordinate transforms, unit conversions, weighted population accounting, deterministic manifest generation and known limiting cases. The existing PR #3962 RCS survival oracle is a natural starting fixture, but it remains a decay-arithmetic reference rather than beam-dynamics validation.

### C. Matched code-to-code comparisons

Use established beam tracking, PIC, beam–matter, radiation and detector tools. Freeze:
- initial distribution and reference plane;
- particle weights and sampling;
- physics lists, materials and geometries;
- solver tolerances, mesh and timestep;
- machine configuration and random-seed policy;
- output observables and acceptance definitions.

Compare more than scalar means: distribution distances, covariance, quantiles, tail exceedance, loss partitions and downstream quantities.

### D. Held-out experimental correlation

Use measurements from relevant facilities and material programmes. Partition by experiment, run, setting or measurement campaign—not randomly split records from the same run. Preserve instrument calibration, resolution and selection. Use separate calibration and blind validation datasets.

### E. Coupled end-to-end chain

Only after individual adapters pass should the workflow propagate uncertainty from target/capture through cooling and acceleration to detector-level observables. No unqualified stage may be skipped while retaining an “end-to-end qualified” label.

## 13. Optimizer benchmark: how to compare with the state of the art

Compare the following on an identical design space and evaluation budget:
- conventional expert-designed baselines;
- genetic/multi-objective evolutionary optimization;
- Bayesian optimization;
- differentiable/adjoint optimization where the physics is differentiable;
- uncertainty-aware active learning and multi-fidelity optimization;
- hybrid strategies that use surrogates for screening and full specialist simulations for adjudication.

Equalize physical model fidelity, solver tolerances, particle statistics, constraints, replicates, seed policy and total high-fidelity simulation budget. Report wall-clock and accelerator/CPU/GPU cost, not only optimizer calls.

Evaluate:
- held-out objective improvement;
- feasible Pareto-frontier coverage and hypervolume using a frozen reference point;
- probability of violating hard constraints;
- robustness to input uncertainty and model discrepancy;
- tail and distributional accuracy;
- reproducibility across seeds;
- downstream reconstructed signal/background;
- expected decision improvement per unit of simulation or experiment cost.

A surrogate must be evaluated on blind high-fidelity points, including points near acceptance boundaries and rare-event regimes. It must emit uncertainty and be allowed to abstain. Any optimizer result is a candidate for specialist re-evaluation, not a physical result.

Symthaea's role is to propose the next simulation or experiment based on expected decision impact and unresolved uncertainty. Symtropy's role is to preserve the state, provenance, replay receipts and qualification gates. Neither can self-authorize a physical claim.

## 14. Published benchmark anchors and reproduction targets

The July 2026 Fermilab rectilinear 6D cooling study reports:
- a solution with a 38% shorter total channel at comparable final emittance;
- an alternative with up to 22% lower final emittance while remaining 20% shorter.

Reproduce the underlying trade-off and its definitions before comparing a new optimizer. These are not gains attributable to Symtropy.

The February 2026 plasma-wakefield paper reports 300 MeV to 40 GeV acceleration driven by a 23 GeV electron beam, with 18.85% final energy spread and 8.86 mm mrad normalized emittance. The handoff contract requires those beam-quality quantities to travel downstream with the energy gain. It does not treat the gradient or energy gain as proof of collider acceptance.

The November 2026 NLASG report supplies a 10 TeV design-study reference and emphasizes that the stages are not yet demonstrated as an integrated system. It is a reference case, not an experimentally closed collider chain.

The RCS1 tracking work is another important anchor: RF distribution and energy–field mismatch affect transverse emittance growth. A handoff that preserves only RMS beam values can hide the tails and correlations needed to diagnose these effects.

## 15. Competitive benchmark scorecard

The following are **reproduction gates and proposed comparison criteria**, not claims that Symtropy has achieved these results. A result is comparable only when the reference distribution, physics model, geometry, solver tolerances, particle statistics, acceptance definitions and evaluation budget match.

| Workstream | Public reference to reproduce | Minimum reproduction gate | Improvement criterion |
|---|---|---|---|
| Target and capture | SNS, J-PARC, Mu2e, RaDIATE and published target/capture studies | Reproduce the measured/simulation-linked pion and muon yield, accepted 6D distribution, stress/thermal and irradiation observables for a named configuration | Increase muons delivered inside the downstream cooling acceptance per incident proton while meeting target lifetime, thermal-shock, radiation and maintainability constraints |
| 6D cooling | July 2026 Fermilab rectilinear-channel poster: one solution 38% shorter at comparable final emittance; a separate alternative 22% lower final emittance at 20% shorter. AI4MuC also reports a factor-of-two reduction in final transverse emittance from a conventional IMCC redesign | Reproduce each published trade-off as its own case, using the source definitions and initial distributions; report full 6D distribution, transmission, tail loss and material sensitivity | Improve the matched Pareto frontier in channel length, 6D cooling, accepted population and robustness—not merely a quoted transverse RMS emittance |
| RCS1 and high-energy acceleration | Soubirou–Chancé, arXiv:2611.02865: 8-arc RCS1 >500% horizontal emittance growth versus 13% for 32 arcs in the stated RF-distribution comparison; their 32-arc energy–field mismatch table reports 6.8%, 0.4%, 1.0%, 1.5% horizontal growth in RCS1–RCS4 | Reproduce the RF-distribution and energy–field mismatch cases separately, including the 17-turn RCS1 trajectory, beta beating, transverse/longitudinal emittance, transmission and jitter sensitivity | Keep cumulative transverse emittance growth within the declared 10% chain budget under the full matched lattice and tolerance set, while preserving survival, RF acceptance and power constraints |
| Plasma muon acceleration | Wang et al., PRAB 29, 021302 (2026): 23 GeV electron driver; 300 MeV → 40 GeV; 18.85% energy spread; 8.86 mm mrad normalized emittance | Independently reproduce the source PIC configuration, then separately test finite energy spread/emittance injection; retain source-data availability and finite-emittance losses in the receipt | Demonstrate increased downstream-accepted muons per driver joule or lower decay loss at equal downstream beam-quality and capture constraints; high gradient alone is not a win |
| Beam-induced background (BIB) | FLUKA/Geant4 full transport and the AI4MuC report's identified spectra/correlation/tail problem | Match reference geometry, material, lattice and physics list; compare distributions and event-level dependencies on blinded configurations | Reduce high-fidelity simulation cost at fixed accuracy in spectra, lattice asymmetry, correlations, occupancy, rare-tail exceedance and reconstructed physics observables |
| Detector/reconstruction | AI4MuC recommended efficiency, fake rates, resolution, timing rejection and robustness to BIB variations | Benchmark on held-out signal channels and BIB conditions; report signal efficiency and background/fake rates jointly | Improve physics-channel significance or useful reconstructed yield without eroding prompt, displaced or other unconventional-signal acceptance |
| Whole chain | NLASG 10 TeV centre-of-mass design study, clearly labelled as a design reference rather than an integrated demonstration | Propagate a compatible distribution and its uncertainty through every admitted stage, with explicit untested boundaries | Improve the constrained Pareto frontier for reconstructed signal yield, integrated luminosity, wall-plug energy, availability, lifecycle burden and footprint, subject to hard safety gates |

### Comparison rules that prevent false wins

1. **Freeze the baseline first.** Preserve source distributions, versioned lattice/geometry, physics lists, material data, solver commit, tolerances and output definitions.
2. **Match evaluation effort.** Equalize high-fidelity particle statistics, replicates, random-seed policy, solver fidelity and compute budget. Report CPU/GPU hours and wall time.
3. **Use blind holdouts.** Hold out entire lattice settings, target/material conditions, absorber configurations or detector-background scenarios. Randomly splitting particles from one generated run is not independent validation.
4. **Score distributions, not only moments.** Compare transmission, covariance, emittance definitions, quantiles, correlations, tail exceedance and losses. For BIB, evaluate lattice-induced asymmetry and event-level correlations, not just one-dimensional spectra.
5. **Treat uncertainty as an output.** Report calibration, confidence intervals, coverage and failure/abstention rates. A surrogate that is accurate on average but underestimates rare tails is not qualified for detector or machine-protection decisions.
6. **Require downstream survival of the improvement.** An optimizer's proposed gain remains provisional until a trusted specialist simulation checks it and the benefit survives the next stage's acceptance conditions.
7. **Publish losses and non-wins.** Report where the new method loses, fails, or abstains. No clipping negative skill or hiding infeasible Pareto points.

### What would constitute a meaningful result beyond current practice

A credible first research result would not be “Symtropy beats the world's accelerator codes.” It would be a public, reproducible demonstration that a cross-code workflow can:
- reproduce a published cooling or RCS benchmark;
- preserve the full input/output distribution and population ledger across a solver boundary;
- identify a design change that improves a downstream-accepted objective under a fixed high-fidelity compute budget;
- carry calibrated uncertainty and tail-risk into the next-stage decision; and
- reproduce the decision independently from frozen inputs and receipts.

A stronger result would show the same method across two different domains—for example, cooling-to-acceleration and lattice-to-BIB—without changing the meaning of the evidence contract. That would demonstrate reusable research infrastructure rather than a one-off optimizer.

## 16. Initial implementation sequence

1. Freeze the envelope schema and a field dictionary, including units and coordinate conversion rules.
2. Add valid synthetic fixtures and negative controls. Label these schema-only.
3. Implement a small validator for envelope, asset digest, coordinate compatibility and weighted population accounting.
4. Add one adapter for an existing beam-dynamics dataset and one adapter for a detector/background dataset.
5. Replay the analytical RCS decay case and verify that the new handoff records preserve the original claim ceiling.
6. Add matched code-to-code fixtures with frozen input distributions and independent solver receipts.
7. Add held-out experiment correlation and an end-to-end benchmark manifest.
8. Only then benchmark optimizer and active-learning strategies against matched conventional baselines.

## 17. Acceptance criteria and claim ceiling

The first implementation is accepted only when:
- schema and fixture validation pass, including negative controls;
- all coordinate conversions are explicit and unit-tested;
- weighted population accounting closes within a declared tolerance or reports an explicit residual;
- losses, generation, splitting, merging and reweighting are visible;
- summary-only data cannot pass full-distribution or tail-dependent requirements;
- uncertainty classes remain distinguishable;
- exact inputs, adapter versions and output manifests are content-addressed;
- software conformance is reported separately from code agreement and experimental correlation.

This contract can establish reproducible interfaces, accounting, provenance and qualification boundaries. It cannot establish collider feasibility, real beam transmission, cooling performance, accelerator stability, detector readiness or safe operation without qualified specialist simulation and experimental evidence.

## 18. Public source register

The benchmark anchors above are traceable to these public sources:

1. National Lab Accelerator Study Group, [Report of the National Lab Accelerator Study Group for a Muon Collider](https://arxiv.org/abs/2611.06680), 5 November 2026; Fermilab report FERMILAB-PUB-26-0741-TD.
2. AI4MuC Workshop, [AI Acceleration of Muon Collider R&D](https://arxiv.org/abs/2610.14303), 9 October 2026. Particularly relevant are its challenge map, multi-objective metrics, stochastic cooling/tail requirements, BIB limitations, and call for public benchmarks with trusted validation samples.
3. Lisa Soubirou and Antoine Chancé, [Impact of Rapid Acceleration on Beam Dynamics in the Rapid-Cycling Synchrotrons of a Muon Collider](https://arxiv.org/abs/2611.02865), 2 November 2026. Its RCS1 RF-distribution and 32-section energy–field mismatch results provide matched tracking cases.
4. Wang et al., [Muon injection and acceleration via ultraintense electron beam-driven plasma wakefield](https://doi.org/10.1103/wq5g-mtjq), Physical Review Accelerators and Beams 29, 021302 (2026).
5. Aubrey Zhang et al., [Optimizations of Performance Tradeoffs in a Rectilinear 6D Ionization Cooling Channel for a Muon Collider](https://indico.fnal.gov/event/73552/contributions/344908/), Fermilab 59th Annual Users and Affiliates Meeting, July 2026. This is a conference poster/abstract, not an independently replicated facility demonstration.

Publication type and validation status are part of the evidence: a design report, conference poster, analytical screen, PIC result, code-to-code reproduction, and measured experiment are different evidence classes.
