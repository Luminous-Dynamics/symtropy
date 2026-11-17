---
title: Muon Collider Cooling-to-Reconstruction Benchmark Alignment
version: 0.1.0
status: proposed
scope: interoperability with the public MuonColliderSoft detector benchmark stack and independent system-level qualification
---

# Muon Collider Cooling-to-Reconstruction Benchmark Alignment

**Version:** 0.1.0  
**Status:** Proposed benchmark contract  
**Related work:** #5669 (beam-state evidence handoffs), #5690 (physics-agent validation), #5783 (BDSIM 6D cooling reproduction), #5736 (WG4 native input audit)

## 1. Strategic decision

Symtropy should not build a competing detector simulation or reconstruction stack. The public MuonColliderSoft ecosystem already provides a staged workflow for generation, Geant4 detector simulation, digitisation, reconstruction and performance analysis, with geometry-specific configurations and a maintained Key4hep/Spack software stack.

The opportunity is to connect upstream accelerator and cooling evidence to that ecosystem, then add a stricter qualification layer:
- the exact input beam distribution and its ancestry must be known;
- each stage must preserve coordinate, unit, weight and population semantics;
- detector metrics must be compared against admitted reference values, not just plotted;
- statistical, numerical and model uncertainties must remain distinct;
- a successful job must not be confused with scientific validity;
- the final decision must use reconstructed signal and background, not just local emittance or tracking scores.

This makes Symtropy an evidence and qualification layer around specialist software, rather than a replacement for BDSIM, Geant4, DD4hep, ACTS, k4Reco or the MuonColliderSoft stack.

## 2. Pinned public industry baseline

The following repository snapshots were inspected on 17 November 2026 and are pinned here for reproducible comparison.

### MuonColliderSoft benchmark tools

Repository: https://github.com/MuonColliderSoft/mucoll-benchmarks  
Branch: main  
Commit: f1d1cddc412fa037dd21449af90c97ce4d16f293

The repository separates:
- generation of signal or particle-gun inputs;
- Geant4 detector simulation, typically producing SimHits;
- geometry-specific digitisation and reconstruction configuration;
- analysis into standard EDM4hep/ROOT objects and histograms;
- plotting and purpose-specific workflows.

Its workflows include release validation, full BIB production and MDI optimisation.

The current reconstruction-metric sidecars report:
- track efficiency, fake rate, track quality and track resolutions;
- seed matching, multiplicity and resolutions;
- tracker-hit occupancy;
- photon matching, PFO multiplicity, response and resolution;
- optional empty-track diagnostics.

The aggregator binds metric fragments to the RECO input file size and SHA-256, stack version, benchmark repository revision and dirty state, producer/aggregator checksums, GitHub workflow identifiers, and production metadata where available.

**Important limitation in the inspected benchmark documentation:** the sidecars are currently report-only. The benchmark README explicitly says that no pass/fail physics tolerances have yet been applied; initial samples are intended to establish metric definitions and statistical stability before references are frozen. This is a precise opportunity for Symtropy to contribute, but it must not invent acceptance limits before the reference samples and uncertainty model are admitted.

### MuonColliderSoft software stack

Repository: https://github.com/MuonColliderSoft/mucoll-spack  
Branch: main  
Commit: 553debf49dd25628054e95f3a24d1d067d5ef1ed

The current stack recipe declares a 3.2 version line and nightly builds. The latest tagged release documented in the release notes is v3.1 (13 August 2026). The 3.1 release notes describe the native Key4hep migration, removal of the legacy Marlin/ILCSoft/LCIO reconstruction chain, and restoration of the ACTS/GNN tracking pipeline.

The stack's simulation/reconstruction dependencies include DD4hep/Geant4, k4geo, k4reco, k4GaudiPandora, k4ActsTracking, k4simGeant4, calorimeter/tracker reconstruction and digitisation packages. Its ML variant includes PyTorch and ONNX tooling. The release validation pipeline runs SIM, DIGI and RECO as separate jobs, carries a production record through job artifacts, and then produces per-particle metrics.

This is a mature domain-specific baseline. Symtropy should integrate with its file formats, receipts and workflows rather than fork those capabilities.

## 3. Evidence contract across the system

The full chain should preserve stage evidence:

    target/capture
      → cooling
      → acceleration
      → collider interaction
      → beam-induced background transport
      → detector simulation
      → digitisation
      → reconstruction
      → physics selection
      → independently qualified useful event yield

At every handoff, record:
- immutable source distribution and SHA-256;
- particle species, coordinate frame, reference plane and units;
- particle IDs/ancestry where available, statistical weights and resampling semantics;
- particle count, represented population and loss ledger;
- full 6D covariance and distribution/tail summaries where available;
- solver release, repository commit, configuration, geometry, physics list, materials and toolchain;
- random seed policy, simulation budget and numerical settings;
- uncertainty class and validity domain;
- stage technical status, scientific comparison status, evidence integrity and qualification decision.

The existing beam-state schema in #5669 should be the handoff envelope. EDM4hep and MuonColliderSoft metric sidecars remain the native detector data plane; Symtropy should add a linked evidence envelope rather than redesigning their event formats.

## 4. Benchmark layers and metrics

### A. Accelerator and cooling admission

For BDSIM D4.1 and IPAC'26 reproduction, freeze the full-channel configuration, input distribution, reference momentum, decay switch, field maps, absorber materials, RF settings and output planes. Keep the 80 m and 100 m studies separate.

Report:
- input and output 6D covariance / eigen-emittances;
- transverse and longitudinal emittance under the source paper's definitions;
- transmission and loss partition;
- momentum and amplitude correlations;
- quantiles, tail exceedance and acceptance losses;
- uncertainty and particle statistics.

Do not compare a native x-xp angle covariance with a normalized canonical emittance as though they were the same quantity.

### B. Detector software validation

Reuse the MuonColliderSoft metrics and geometry configurations:
- tracking efficiency and fake rate;
- track parameter residuals and resolution;
- seed efficiency, multiplicity and resolution;
- tracker-hit occupancy and high-occupancy tails;
- photon matching, PFO multiplicity, response and resolution;
- event reconstruction efficiency and selection efficiency for each declared physics channel.

The existing fast workflow is useful for integration smoke tests, while the split high-statistics workflow is the appropriate basis for physics performance comparisons. A 10-event smoke test cannot establish stable rare-tail rates or physics acceptance.

### C. BIB and rare-tail qualification

Mean histogram agreement is not enough. Compare:
- occupancy exceedance probabilities at predeclared thresholds;
- spatial/time correlations and bunch-crossing structure;
- detector subsystem dependence;
- high-percentile hit and energy-deposit distributions;
- fake-track and fake-PFO rates under matched background;
- signal efficiency at fixed background rejection.

A surrogate is only qualified over its tested domain. It must abstain or request full simulation outside that domain, particularly around rare-tail and reconstruction-acceptance boundaries.

### D. End-to-end physics objective

For each named physics channel, expose the factorized yield:

    N_reconstructed_signal =
      integrated_luminosity
      × cross_section
      × branching_fraction
      × geometric_acceptance
      × reconstruction_efficiency
      × selection_efficiency

Carry uncertainties and source definitions for every factor. Report expected background and systematic uncertainty alongside signal. A larger number of generated events, transported muons, tracks or reconstructed objects is not automatically a better collider.

The system objective is a constrained Pareto set across:
- reconstructed signal yield by physics channel;
- integrated luminosity and availability;
- BIB occupancy, fake rate and tail risk;
- wall-plug energy and peak/average power;
- target, absorber, RF and magnet lifetime;
- cost, maintenance and facility footprint.

Radiation, machine protection, minimum beam quality and detector safety are hard gates, not objectives that can be traded away.

## 5. Fair state-of-the-art comparison

All comparisons must freeze:
- beam input distribution and its full provenance;
- geometry, material database, magnetic/electric field maps and physics lists;
- solver and stack commits, container digest and build/toolchain;
- random seeds, event counts, particle weights and resampling;
- detector geometry, digitisation and reconstruction configuration;
- metric definitions, selection cuts, histogram ranges and tolerances;
- available high-fidelity compute budget.

Compare conventional expert-tuned designs, genetic/multi-objective optimisation, Bayesian optimisation, differentiable/adjoint models, uncertainty-aware active learning and Symthaea-assisted decision selection under the same evaluation budget.

Report separately:
- Pareto-frontier changes and feasibility;
- high-fidelity evaluations and CPU/GPU wall time;
- held-out metric agreement and uncertainty coverage;
- rare-tail error and false-accept rate;
- source/provenance completeness;
- independent replay success;
- human review effort and expected decision value.

A benchmark pass/fail gate should be added only after admitted reference distributions, repeat statistics and uncertainties establish defensible tolerance intervals. Never set a convenient percentage merely to make a new optimizer win.

## 6. What Symtropy should add

1. A provenance adapter for the MuonColliderSoft production record and benchmark metric sidecars.
2. A versioned qualification manifest tying cooling handoff IDs to detector SIM/DIGI/RECO outputs.
3. Reference-set admission and validation of metric definitions, event counts, selections, input hashes and reconstruction configuration.
4. Statistical adjudication: confidence/credible intervals, bootstrap or replicate variability where appropriate, calibration and tail coverage.
5. Negative controls for stale geometry, wrong input distribution, altered physics list, missing metric fragment, changed selection cuts, decay-off/on confusion and underpowered statistics.
6. Machine-readable decision receipts that distinguish technical execution, scientific agreement, evidence integrity and qualification.
7. Symthaea integration only after the oracle and reference set are fixed; the agent must be scored on blind tasks and cannot self-authorize its own physics result.

## 7. First benchmark ladder

| Tier | Deliverable | Claim ceiling |
|---|---|---|
| 0 | Pinned source, input, container and geometry identities | Source admission |
| 1 | Deterministic generation/SIM/DIGI/RECO smoke replay | Software execution |
| 2 | Reproduced detector metric sidecars with event and config checks | Metric reproduction |
| 3 | Independent comparison against admitted reference distributions | Numerical/scientific comparison |
| 4 | Held-out BIB and signal cases with calibrated uncertainty/tail metrics | Domain-bounded model qualification |
| 5 | Cooling-to-reconstruction propagation of compatible beam states | Integrated simulation qualification |
| 6 | Physics-channel useful-yield improvement at equal high-fidelity cost | Decision-quality engineering evidence |
| 7 | Independent facility/experimental correlation | Experimental qualification |

The new Rust beam-handoff and physics-agent validators are designed to provide the evidence/qualification layer for tiers 0–4. The BDSIM reproduction issue #5783 owns the cooling-side tiers; the end-to-end contract #5669 owns the common interfaces.

## 8. Claim ceiling

This benchmark alignment reuses and qualifies public workflows. It does not claim that Symtropy has run the MuonColliderSoft stack, reproduced any detector metric, improved tracking, reduced BIB or demonstrated better physics reach. Those claims require source-pinned executions, matched input/geometry, high-statistics references and independent adjudication.
