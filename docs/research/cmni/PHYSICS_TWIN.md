# CMNI physical twin: heterogeneous metal-hydride D–D fusion

**Status:** solver-neutral research design, v0  
**Tracking:** https://github.com/Luminous-Dynamics/symtropy/issues/6453  
**Companion:** https://github.com/Luminous-Dynamics/symthaea/issues/13023

## 1. Purpose and boundary

Define a replayable multi-physics digital twin for published accelerator-driven D–D fusion experiments in metal foils/hydrides. The target problem couples implantation, depth-dependent stopping, material state, deuterium transport, defects/traps, effective screening models, and detector observations.

This does **not** establish room-temperature cold fusion or useful energy production. The 2026 study explicitly says the reported solid-state reaction rates are not presently relevant to energy production. It reports beam-driven D–D fusion, not spontaneous electrochemical fusion.

Symtropy's role is orchestration, state coupling, conservation, uncertainty propagation, and deterministic replay. It is not a replacement for validated nuclear, electronic-structure, ion-stopping, or materials-science solvers.

## 2. Evidence and access constraints

- 2025 *Nature* study: https://doi.org/10.1038/s41586-025-09042-7
- 2026 *Nature Communications* study: https://doi.org/10.1038/s41467-026-74421-1
- 2026 processed dataset: https://zenodo.org/records/20435774 — embargoed until **2026-12-01** as checked 2026-11-20.
- The 2026 experiment reports proton measurements down to 0.25 keV and neutron measurements over the higher-energy range. Each observation must carry channel-specific energy coverage and availability; do not fabricate cross-channel validation at low energy.

## 3. Coupled model graph

`material state → ion transport/implantation → local D/defect state → reaction yield → detector observation`

Each edge is a versioned, typed adapter with explicit units, coordinate/frame conventions, validity domain, uncertainty, provenance, and solver identity.

### A. Material state

Represent, at minimum:

- Host material and composition (Pd, Ti, hydride phase, isotope fractions).
- Geometry, foil thickness, surface coatings and roughness metadata.
- Spatial/depth-resolved deuterium concentration (n_D(z,t)).
- Defect/trap state (d(z,t)), occupancy and uncertainty.
- Phase, strain, grain/surface characterization where measured.
- Preparation, loading/unloading, annealing, irradiation, and sample history.

A uniform concentration or defect field is an explicit baseline assumption, not the default truth.

### B. Implantation, stopping and transport

Use validated, versioned external tools/data for ion stopping and implantation (for example, SRIM/TRIM-class tools where their domain is appropriate), plus suitable transport/defect solvers.

A reduced depth-resolved state may evolve as:

[
partial_t n_D = partial_z(D_D,partial_z n_D) + S_{implant} - L_{desorb} - L_{beam} + R_{trap}
]

This is a model contract, not an assertion that one diffusion equation captures every electrochemical or irradiation process. Boundary conditions, coefficients, units and calibration must be supplied by the selected solver and source data.

Beam energy evolves with depth through a stopping model:

[
rac{dE}{dz}=-S_{stop}(E,z,t)
]

The input energy distribution must be convolved through the model; a nominal beam energy alone is insufficient where energy spread is material.

### C. Reaction-yield layer

Keep the published homogeneous ALR-style screening model as a reference baseline, and keep the bare-nucleus reference as a separate baseline. A representative cross-section form used in the 2026 paper is:

[
sigma(E,U_e)=rac{S(E)}{sqrt{E(E+U_e)}},
exp!left[-sqrt{rac{E_G}{E+U_e}}ight]
]

The published thick-target yield is a stopping-weighted energy integral. For a depth-resolved implementation, express it equivalently through the spatial trajectory:

[
Y(E_0,t)=int_0^L n_D(z,t),
leftlanglesigma!left(E(z),U_e;	hetaight)ightangle_{p(U_emid z,t)}
,dz
]

The local (U_e) distribution is initially a phenomenological representation of heterogeneous environments. It must not be mislabelled as a directly measured screening field or unique microscopic mechanism. Alternative implementations may integrate in energy space, but must demonstrate dimensional equivalence and reproduce the homogeneous limit.

Required model variants:

1. Bare-nucleus cross-section and source reference.
2. Homogeneous constant-(U_e) ALR baseline.
3. Depth-dependent heterogeneous effective-environment model.
4. Defect/trap- and occupancy-coupled model.
5. Nuisance-aware model including energy spread, stopping uncertainty, target density, and run-level drift.

### D. Observation and detector layer

Keep physical reaction yield separate from measured counts. Model each channel independently:

- Branching fraction and emitted particle spectrum.
- Detector geometry and solid angle.
- Entrance windows/absorbers and target self-absorption.
- Energy resolution, calibration, threshold and selection procedure.
- Efficiency, live time, dead time, background and uncertainty.
- Channel availability by energy and run.

The 2026 paper uses proton and neutron channels over different energy ranges; do not fill missing channels with synthetic “observations.” Synthetic detector data must be labelled as simulation.

## 4. Solver interface and evidence receipt

Each adapter input/output must record:

- Schema version and SI unit identifiers.
- Experiment/run/sample IDs.
- Source artifact URI and SHA-256.
- Solver name, version, executable/container digest and settings.
- Input state digest and output artifact digest.
- Boundary/initial conditions and validity limits.
- Statistical and systematic uncertainty, including covariance where available.
- Warnings, rejected rows, convergence state and numerical tolerances.
- Seed and deterministic-replay metadata where randomness exists.

Unknown inputs stay unknown. No silent defaulting of material occupancy, stopping power, detector efficiency, or uncertainty.

A replay either reproduces the output within a preregistered numerical tolerance or emits a failed receipt that identifies the mismatch. It must never silently accept a changed solver, hidden data file, or missing calibration.

## 5. Numerical and physical qualification

### Manufactured-solution tests

- Diffusion equation: uniform field with zero-flux boundaries remains uniform.
- Diffusion: known analytic transient solution converges under mesh/time refinement.
- Implantation: integrated implanted dose matches the imposed source after explicitly accounted losses.
- Stopping: energy is monotone non-increasing with depth and the total deposited/escaped energy is accounted for.
- Yield: homogeneous-field numerical integral converges to the reference thick-target integral.
- Heterogeneous yield: collapses to homogeneous ALR when the distribution becomes a delta function.
- Detector: sampled count means match efficiency × rate × live time; Poisson variance and injected backgrounds are recovered statistically.

### Conservation and dimensional tests

- Check dimensional consistency at every adapter boundary.
- Track incident-beam energy, deposited energy, escaped energy, electrochemical input, thermal outputs and reaction-product energy as distinct ledgers.
- Reject double-counting of energy transferred between coupled solvers.
- Preserve isotope, charge, and particle bookkeeping.
- Distinguish reaction energy from electrical/chemical input and from measured calorimetric heat.
- Numerical conservation tolerances must be declared per solver and tested under refinement.

## 6. Benchmark suite

| Benchmark | Required comparison / pass evidence |
|---|---|
| Homogeneous reference | Reproduce published ALR reference behavior in its stated validity range |
| Relative-yield reconstruction | Implement the published normalization/reference-energy definition with explicit count and covariance propagation |
| Heterogeneous model | Improve preregistered held-out predictive score, not only training fit |
| Material transfer | Leave-one-material and leave-one-campaign-out tests |
| Transport | Manufactured analytical solutions and mesh/time convergence |
| Stopping/implantation | Cross-check external solver outputs and dose/energy balance |
| Detector observation | Channel-specific calibration, background recovery, and count-statistics tests |
| Fault injection | Recover known calibration, energy-spread, current, stopping, and background faults |
| Replay | Exact input/output digests and deterministic re-execution |
| Cost | Runtime, memory, numerical error and information gain per compute/experiment budget |

Report predictive log score, 50/90/95% interval coverage, residual structure, parameter identifiability, sensitivity to priors/nuisance assumptions, conservation residuals, convergence order, runtime and replay fidelity.

Do not claim a SOTA improvement unless the baseline implementation, source inputs, holdout partition, metrics, compute budget, and uncertainty are all recorded and matched. The intended differentiator is the integrated qualification/replay system, not a claim that Symtropy is a more accurate nuclear solver than specialist codes.

## 7. Paradigm shift: a counterfactual material twin

The primary research object should be a family of counterfactual target states, not a single best-fit target:

- Same host/geometry, alternative defect profiles.
- Same defect profile, alternative D occupancy.
- Same occupancy, alternative depth distribution.
- Same depth state, alternative surface/coating/phase.
- Same target state, alternative beam-energy distributions.
- Same underlying physical yield, alternative detector/background configurations.

Symtropy should identify which counterfactual changes produce distinguishable observations and which parameters are non-identifiable from the available channels. This turns a curve-fitting exercise into experiment design.

## 8. Initial deliverable sequence

1. Add typed schemas, units, and immutable evidence manifests.
2. Implement a pure reference yield integral and independent numerical tests.
3. Add synthetic depth profiles and heterogeneous-to-homogeneous limiting tests.
4. Add solver-neutral adapters for stopping/implantation and material profiles.
5. Add the detector observation model with distinct proton/neutron coverage.
6. Add conservation, uncertainty, fault-injection, and replay receipts.
7. After 2026-12-01, ingest the released Zenodo files, freeze hashes, then attempt source reconstruction.

No laboratory control loop is in scope. Ion-beam and deuterium/electrochemical experiments belong to qualified institutional laboratories operating under their own radiation, chemical, electrical, vacuum, and pressure safety controls.

## 9. References

- Chen et al. (2025): https://doi.org/10.1038/s41586-025-09042-7
- Karahadian et al. (2026): https://doi.org/10.1038/s41467-026-74421-1
- Zenodo data record (embargoed until 2026-12-01): https://zenodo.org/records/20435774


## 10. Beam species and per-deuteron energy distribution

The 2026 source paper reports a typical beam mixture of approximately 85% D⁺ and 15% D₂⁺ and computes deuteron flux as (I/ecdot(0.85+2cdot0.15)/A). This is important: **ion-current fractions are not deuteron-flux fractions**.

For singly charged ions accelerated through (V) kV:
- D⁺: (E_{lab}approx V) keV per deuteron, so (E_{cm}approx V/2) keV for D on stationary D.
- D₂⁺: total molecular kinetic energy (approx V) keV. Under prompt symmetric dissociation with fragment velocity inherited from the molecular ion, each D fragment carries (approx V/2) keV, so (E_{cm}approx V/4) keV.

Under the conditional 85/15 ion-current mixture:
[
f_{D^+} = rac{0.85}{0.85+2(0.15)}approx0.7391,qquad
f_{D_2^+ightarrow2D}=rac{2(0.15)}{0.85+2(0.15)}approx0.2609.
]

This is a **prompt-dissociation limiting model**, not a claim about actual fragmentation in the experiment. The model must retain alternatives for molecular survival and dissociation depth. Weight component yields by deuteron flux, not by ion-current percentage.

The published Methods report an approximately 5% energy spread based on a retarding-potential analyzer. The accessible methods do not establish that the quoted spread is a species-resolved per-deuteron energy distribution. Therefore:
- retain D⁺ and D₂⁺ components separately;
- preserve the measured source fraction and its uncertainty, per operating point if available;
- convolve each species with its own measured energy distribution;
- use suitable species-resolved beam diagnostics or documented external source characterization;
- propagate uncertainty in molecular fraction, fragmentation, and energy into the reaction-yield prediction.

The Rust reference kernel includes a first-order prompt-dissociation component mapper. It uses the same S-factor/stopping functions for both fragment populations as a synthetic sensitivity scaffold; it does not yet model mass-dependent molecular stopping, dissociation depth, or experimental fragmentation data. Never represent it as a validated species-resolved beam transport model.


## 11. Arbitrary beam-energy distribution support

The reference kernel now accepts a discrete set of `BeamEnergyComponent` values with center-of-mass energy and **deuteron-flux weight**. This is designed for source-provided histograms or quadrature nodes. It deliberately does not assume the reported ~5% energy spread is Gaussian, nor infer a distribution from a single spread statistic.

A measured source histogram must specify:
- species identity and the basis of the reported fraction (ion current vs deuteron flux);
- bin edges/centers and calibrated energy coordinate;
- per-bin weights and covariance/uncertainty;
- energy analyzer calibration, response and mass resolution;
- whether values are incident ion energy, per-nucleus energy, or center-of-mass energy;
- the source artifact hash and processing revision.

Only after the species-resolved distributions are measured should the beam forward model convolve yields over the observed distribution. A retarding-potential analyzer result is not automatically a species-resolved per-nucleus spectrum.

The synthetic test fixture for the prompt-dissociation 85/15 kinematic limit is `research/cmni/fixtures/d2plus-prompt-dissociation-85-15-synthetic.json`. It is not an empirical beam record.


## 12. Source-specific EJ-309 observation model

The 2025 Thunderbird study describes one EJ-309 liquid-scintillator neutron detector and a pulse-shape discrimination chain. The initial detector adapter should represent:
- Cylindrical EJ-309 geometry: 12.7 × 12.7 cm.
- Hamamatsu R1250 PMT; CAEN DT5730S, 14-bit / 500 MHz, DPP-PSD digitizer.
- Light-output calibration using 60Co, 137Cs and 152Eu.
- PSD feature: tail/total charge-integral ratio.
- Per-light-output slice gamma fit: 5-keVee slices with fitted `mu_gamma` and `sigma_gamma`.
- Neutron gate: lower PSD boundary `mu_gamma + 5*sigma_gamma`; upper boundary another 0.2 PSD units higher; light-output threshold 50 keVee.
- Pileup contribution reported below 2%.
- Magnetic field at detector front reduced from 40 G to below 0.3 G by shielding.
- Response simulation: MCNP6.2 + DRiFT and Geant4, 2.45-MeV D-D neutron source, detector resolution function, GRAVEL unfolding with Geant4 response matrix.

The source-reported >99.9999% gamma rejection is not a universal guarantee. Model per-slice gamma distributions, gate acceptance, threshold, pileup, gain drift, calibration and background. The SI reports response agreement mainly over 200–1000 keVee; do not extrapolate it below the measured/calibrated range.

The reported 2025 apparatus describes one EJ-309 neutron detector. Do not present its response model as an independent second detector or multi-detector replication. Future independent validation should use a separately calibrated second detector or different neutron-detection physics.

Tracking: https://github.com/Luminous-Dynamics/symtropy/issues/6486


## 13. Spatial beam footprint and spatiotemporal target state

A depth-only state is an intermediate model, not the final physical representation. The 2026 Methods report an approximately 1 cm² spot, voltage-dependent focusing above 5 kV, and implantation depths from about 4.4 nm at 0.25 keV to 67.5 nm at 6.5 keV. The foils are 250 µm thick. The beam-induced region is therefore an extremely shallow fraction of the total membrane.

The target-state field should evolve over space and time:
c_D(x,y,z,t), d_defect(x,y,z,t), local phase/strain and temperature where measured or calibrated.

The incident beam should be represented by a species- and energy-resolved intensity map J_s(x,y,E,t), normalized so that its spatial/energy/species integral matches the Faraday-cup charge and independent beam diagnostics.

Required coupling:
1. Measured 2-D beam footprint / profile (Gaussian only as an explicit synthetic baseline).
2. Voltage-dependent spot size and current-density distribution; do not equate total beam current with uniform local fluence.
3. Species-resolved D+ / D2+ energy components and mass-resolved uncertainty.
4. Depth-dependent implantation and damage from a validated specialist solver.
5. D diffusion, replenishment from the bulk/electrochemical boundary, beam-induced depletion, defect trapping/detrapping and surface loss.
6. Reaction-yield integration over x,y,z with the local per-deuteron energy and target state.
7. Detector forward model using the product distribution and true detector geometry.

The isolated Rust prototype now includes a current-normalized axisymmetric Gaussian footprint as a synthetic baseline. Its integral is normalized to the supplied D arrival rate. It does not infer a Gaussian sigma from the paper's approximate 1 cm² spot area; a measured map should take precedence.

Tracking:
- https://github.com/Luminous-Dynamics/symtropy/issues/6518 (space-time target model)
- https://github.com/Luminous-Dynamics/symtropy/issues/6483 (species-resolved beam)
