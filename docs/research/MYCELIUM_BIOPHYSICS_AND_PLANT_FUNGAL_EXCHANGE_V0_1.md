# Mycelium Biophysics and Plant–Fungal Exchange Qualification

**Contract:** MYCO-BIO-QUAL-001  
**Version:** 0.1  
**Status:** Research/implementation specification; not a biology qualification  
**Owner:** Luminous Dynamics — Symtropy, composed with Symthaea evidence reasoning  
**Parent:** [Symtropy #5173 — Tropical Forest Ecological Qualification](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Integrated target:** [Symtropy #252 — Living World Cell](https://github.com/Luminous-Dynamics/symtropy/issues/252)  
**Tracking:** [Symtropy #5379](https://github.com/Luminous-Dynamics/symtropy/issues/5379)

## 1. Purpose

Establish a falsifiable path from the existing stylized `symtropy-mycelium` field simulation to a bounded model of hyphal-network morphology, transport, and selected plant–fungus resource exchange.

The objective is not to create a visually convincing underground web. It is to predict declared biological observations under declared conditions, preserve physical and accounting constraints, expose uncertainty, and identify experiments that distinguish competing mechanisms.

This specification does not claim that the present Mycelium V0 is a validated fungal physiological model, or that common mycorrhizal networks universally transfer resources between arbitrary plants.

## 2. Current implementation boundary

The public `symtropy-mycelium` crate currently provides a deterministic CPU-first field model:

- a shared `FieldGrid` with biomass, nutrient, toxin and other generic channels;
- nutrient and toxin diffusion/decay;
- local assimilation and heuristic growth toward a selected nutrient-rich neighboring cell;
- biomass update/decay and a thresholded connected-biomass path query;
- basic metrics and a PPM debug export.

This is a useful stylized baseline for field-mediated growth. It does **not** yet represent explicit hyphal tips/segments, branching and anastomosis events, geometry-dependent transport conductance, species-specific physiology, plant-root interfaces, isotope-labelled exchange or field-calibrated fungal dynamics.

The shared `symtropy-lifesim-core::FieldGrid` stores `f32` values but does not itself attach physical units or a conservation ledger to each channel. Its concentration sanitization/clamping is useful for bounded field storage, but clamping cannot silently serve as a mass-balance operation. Physical accounting must live in explicit typed state and flux records.

## 3. Core paradigm: graph + fields + ledgers

Use a hybrid representation rather than choosing between a graph and a grid.

```text
Plant structural authority ── typed root/interface boundary
                                  │
                                  ▼
Hyphal morphology graph ◄──► soil reaction / transport fields
       │                          │
       ▼                          ▼
Graph-constrained transport ◄──► compartment exchange
                  │
                  ▼
        element-specific ledgers
                  │
                  ▼
      canonical Symtropy state/replay
                  │
                  ▼
      evidence-bearing observations
                  │
                  ▼
 Symthaea: hypotheses, uncertainty, counterfactuals,
           experiment and measurement prioritization
```

### 3.1 Hyphal morphology graph

The initial typed graph should distinguish:

- **Tip:** an active or inactive extension front.
- **Branch point:** a segment junction created by a declared branching event.
- **Fusion point:** a hyphal anastomosis junction, created only when the selected model's fusion criterion is met.
- **Segment:** a geometrical connection with endpoints, length, optional measured radius/diameter, age/state, and parameterized transport properties.
- **Interface:** a typed association to an existing canonical root/plant structural element or another explicitly supported substrate.

Every node and segment needs stable identity, parent/lineage where meaningful, creation/removal event, source/configuration identity and replay semantics. Graph identity must not be reconstructed from a rendered mesh or image pixels.

Do not assume every fungal taxon has the same branching, fusion, septation, transport or symbiotic structures. Features unsupported for the selected taxon/profile remain unavailable, not default-true.

### 3.2 Spatial reaction and transport fields

Retain the shared field substrate for spatial conditions and local reactions. A profile may use moisture, temperature, oxygen, pH, carbon substrates, nitrogen species, phosphorus species, toxins or other variables only if the source experiment and equations support them.

Each physical quantity requires:
- quantity identity and unit;
- spatial support / coordinate frame;
- temporal support and sampling interval;
- source, transformation and uncertainty;
- valid range and model applicability envelope;
- ownership and whether it is observed, inferred, calibrated, synthetic or simulated.

A bare `f32` channel must not be mistaken for a unit-bearing physical measurement.

### 3.3 Graph-constrained transport

Represent resource movement along eligible hyphal segments and exchanges at declared boundaries. Transport properties may depend on geometry and physiology, but their constitutive law and parameters must be sourced or explicitly synthetic.

Keep local soil diffusion, hyphal transport, plant uptake, microbial mediation and losses as separate paths. A connected graph is not automatically a functioning transport conduit; transport must be independently qualified.

### 3.4 Plant–fungus interface

Compose with the existing canonical plant/root structural owner (including the #242 line and its prerequisites). Do not create a second root graph or use a view/presentation object as root identity.

The first coupled profile must identify:
- plant taxon / genotype or declared functional group;
- fungal taxon / strain;
- symbiosis type and interface anatomy;
- soil / substrate and relevant compartment arrangement;
- resource species and measured exchange direction;
- environmental conditions and calibration envelope.

Track, where evidenced, plant-derived carbon entering fungal pools and fungal-acquired nutrient entering plant pools. Keep respiration, exudation, fungal turnover, soil immobilization and plant uptake separate. Water transfer is an optional, evidence-gated process, not an assumed property of every mycorrhizal association.

Arbuscular mycorrhiza, ectomycorrhiza, endophytes and saprotrophs must not be collapsed into one generic symbiosis type.

### 3.5 Element ledgers

For each declared element (e) (initially C, N and P where source data support them), calculate:

[
R_e = Delta S_e - (I_e - O_e + T_{e,\mathrm{in}} - T_{e,\mathrm{out}})
]

where (S_e) is the declared stock boundary, (I_e/O_e) are external inputs/outputs and (T_e) are internal transfers across the selected sub-boundary. Internal transfers must cancel in the combined system budget while remaining visible in each organism/compartment budget.

A non-zero residual must be reported with units and boundary definition. It must not be hidden by clamping, normalization, a learned correction, or an unlabelled “other” term. Missing fluxes remain missing/unknown, not zero.

## 4. Scientific evidence and comparator register

These are research anchors, not claims that Luminous has reproduced their results.

| Work | Use in this programme | Limit |
|---|---|---|
| Palmer et al., *Physical Review E* 112 (2025), “Three-dimensional Modeling of Hyphal Fusion, Branching, and Nutrient Transport in Filamentous Fungi”; PNNL record published 2 February 2026. [PNNL record](https://www.pnnl.gov/publications/three-dimensional-modeling-hyphal-fusion-branching-and-nutrient-transport-filamentous), [DOI](https://doi.org/10.1103/xbjp-zqj1) | Main 3D hyphal morphology/fusion/transport comparator. The reported BMX extension uses filamentous particles, chemical diffusion, adaptive mesh refinement and GPU-oriented methods. | Check the actual code/data release, build recipe and licence before claiming reproducibility. The PNNL landing record describes code availability as TBD. |
| Yuan et al., “A Mycelium Dataset with Edge-Precise Annotation for Semantic Segmentation,” *Scientific Data* (10 December 2025). [DOI](https://doi.org/10.1038/s41597-025-06265-1) | Image morphology and boundary segmentation benchmark. The paper reports 20,176 images from four fungal species and 567 pixel-annotated samples; reference comparisons include U-Net, DeepLabv3 and SegFormer. | Publication licence is CC BY-NC-ND 4.0. Confirm the dataset's exact file-level terms before download, ingestion, training, adaptation, redistribution or commercial use. Image sequences are not automatically validated time-series trajectories. |
| Brandt et al., “Reciprocal transport of carbon and nitrogen between plants and the hyphosphere microbiome in a temperate grassland,” *New Phytologist* (first published 10 September 2026). [DOI](https://doi.org/10.1111/nph.71576) | Source-faithful isotope-tracer case: plant (^{13}\mathrm{CO}_2) pulse label and (^{15}\mathrm{N}) supplied to a hyphal compartment, with isotope recovery assessed across compartments. | Reports 16.9% recovery of initially added (^{15}\mathrm{N}) in plants in this specific experiment. This is not a global coefficient or a universal transfer rate. The study's destructive sampling and experimental design must be retained in the reproduction crosswalk. |
| Systematic mapping of experimental approaches to common mycorrhizal networks in arbuscular mycorrhiza (first published 19 December 2024). [DOI](https://doi.org/10.1002/ppp3.10618) | Design controls for proving continuity and network-mediated transfer. | Highlights limited field/ecosystem coverage and methodological confounding. A positive tracer signal alone does not establish a common network pathway. |
| CRootBox and OpenSimRoot. [CRootBox](https://plant-root-soil-interactions-modelling.github.io/CRootBox/), [OpenSimRoot report](https://root2res.eu/wp-content/uploads/2025/09/Root2Res_D6.2-Updated-version-of-OpenSimRoot_FINAL.pdf) | Root architecture and functional plant/soil comparator. | These are plant-root tools, not fungal physiology models. Compose with them as reference implementations where appropriate; do not imply they validate the hyphal model. |
| Collaborative root water uptake benchmark. [DOI](https://doi.org/10.1093/insilicoplants/diad005) | Matched problem-design precedent across DuMux, CPlantBox, R-SWMS, OpenSimRoot and SRI. | Reuse compatible benchmark scenarios and semantics; do not compare mismatched quantities. |

A 2026 review of plant regulation of arbuscular mycorrhizal symbiosis distinguishes supported regulatory checkpoints (including accommodation, investment, maintenance and interface turnover) from hypotheses that still need performance-linked evidence. Use those terms as measurable process transitions, not as evidence of unilateral “control” or organismal intention: [review](https://pmc.ncbi.nlm.nih.gov/articles/PMC13596872/).

## 5. Benchmark protocol

### T0 — data admission and rights

For every source:
- pin exact release/commit, DOI/URL, checksum and complete file inventory;
- record licence, attribution, derivative restrictions and contributor terms;
- preserve taxon/strain, treatment, replicate, dish/plot/compartment identity;
- record imaging/sensor settings, units, spatial and temporal resolution, quality flags and missingness;
- separate raw observation, derived annotation, inference, calibration, synthetic fixture and model output.

Do not treat public discoverability as permission to reuse or redistribute.

### T1 — model verification

Freeze deterministic synthetic fixtures with known graph topology and known transport truth.

Required checks:
- repeatable replay from input/config/seed digests;
- valid graph endpoints, identities, event order and connected components;
- non-negative stocks and explicit source/sink accounting;
- branch/fusion event idempotency under retries;
- no flux across disconnected edges or compartments;
- limiting cases: straight extension, no extension, zero nutrient, no branching, no fusion, uniform field, diffusion-only, isolated component;
- analytical or independently computed reference for small graph transport cases;
- time-step and spatial-resolution convergence;
- CPU/GPU and parallel/reference parity where accelerated paths are introduced.

### T2 — hyphal morphology

Measure separately:
- tip extension rate and colony-front radius versus time;
- total hyphal length, segment count, tip count and branch density;
- branch-angle distribution;
- fusion/anastomosis frequency and connected-component count;
- degree distribution, path length, cycles and spatial coverage;
- image segmentation mIoU, Boundary IoU, HD95 and ASSD where qualified masks exist.

Split by complete dish/strain/condition or experiment, not random neighboring pixels/frames. Report uncertainty and worst-stratum performance.

### T3 — transport and coupled exchange

Measure:
- isotope recovery by declared plant/fungal/soil/microbial compartment;
- time-to-detection and time-dependent enrichment;
- plant-to-fungus carbon and fungus-to-plant N/P separately;
- organism/compartment stock changes and losses;
- element-specific balance residuals in physical units;
- replicate, treatment and site variation.

### T4 — causal intervention

Use paired arms where supported by the experiment:
- hypha-permissive versus hypha-excluding mesh compartments;
- intact versus severed hyphal continuity, with sham handling control;
- live inoculum versus sterile/non-inoculated control;
- labelled versus unlabelled tracer;
- explicit plant interface versus interface-absent/non-compatible control;
- graph-mediated transport versus soil-diffusion-only model.

The experiment must support source attribution and path identification. Apparent exchange can be caused by diffusion, contamination or microbial intermediaries. Record these alternatives rather than assigning every effect to direct fungal transfer.

### T5 — generalization

- leave-strain-out and leave-condition-out morphology;
- leave-replicate/site/campaign-out exchange evaluation;
- declared environment shifts (moisture, nutrient patchiness, pH, temperature, oxygen);
- explicit out-of-domain and insufficient-evidence decisions;
- separate per-taxon, per-condition and worst-stratum results.

Never let a pooled average conceal a failed taxon, condition or transfer regime.

## 6. Baselines and competitive positioning

Use matched baselines:
1. Existing Symtropy Mycelium V0 (stylized field model).
2. Diffusion-only field transport.
3. Simple branching random walk / lattice-growth null.
4. BMX fungal model for matched 3D hyphal morphology/fusion/transport cases, when code and data are obtainable.
5. MyceliumSeg reference segmentation methods under its official split and terms.
6. CRootBox/OpenSimRoot for their respective root-side questions.
7. Measured isotope-tracer observations and experimental controls for exchange.

The intended differentiation is **composition and evidence quality**, not a claim that one solver replaces specialist tools. The opportunity is to join explicit fungal topology, typed transport, existing plant-root authority, element budgets, tracer validation and calibrated uncertainty in one reproducible causal experiment. Symtropy owns canonical physical execution; Symthaea proposes and compares hypotheses, interprets uncertainty and ranks bounded next measurements.

A comparison is fair only when species, scale, initial conditions, forcing, observations, splits, runtime budget and metrics match. Published benchmark numbers must be distinguished from reproduced numbers.

## 7. Research targets (not achieved results)

Only freeze numeric superiority targets after reproducing the comparator and its split. The tournament should aim for:
- no regression against the best reproduced BMX/reference morphology score on exactly matched cases;
- improved coupled exchange forecasts or element-budget closure over current field-only and diffusion-only controls, with independent replicate blocks;
- predictive interval coverage near its preregistered nominal level, with interval width and worst-stratum score visible;
- 100% pass on frozen synthetic conservation, disconnected-path, idempotency, provenance and invalid-unit fixtures;
- better decision value for measurement proposals than random, sensitivity-only and uncertainty-only policies under equal cost/budget.

No SOTA claim may be made from synthetic verification, morphology resemblance, segmentation alone, one species, one field study, or runtime alone.

## 8. Ownership and claim boundaries

- **Symtropy:** canonical hyphal graph, field coupling, transport, ledgers, replay and benchmark execution.
- **Plant structural owner:** canonical root/plant structure and developmental authority; this work consumes its interfaces.
- **Basin/soil/hydrology owners:** existing environmental substrate and water processes; no parallel solver.
- **Symthaea:** evidence synthesis, causal hypotheses, calibrated forecast interpretation and bounded experiment proposals; it cannot invent source truth or model parameters.
- **Sol-Atlas:** spatial/remote-sensing source identity and geospatial evidence when used.
- **Mycelix:** data rights, consent, custody and attribution when required.

Do not infer intelligence, cognition, consciousness, intention or a universal communication protocol from a graph, morphology, electrical oscillation, scalar coherence or simulated task performance.

## 9. Implementation sequence

1. Freeze T0 data and model semantics; record the exact current stylized baseline.
2. Implement the small deterministic CPU hyphal graph kernel and its synthetic known-truth fixtures.
3. Add graph-to-field transport and element ledgers with no hidden clamps.
4. Integrate one taxon-specific plant interface through the canonical root owner.
5. Reproduce one morphology comparator and one isotope-tracer case, each with a difference report and source/rights manifest.
6. Add Symthaea's hypothesis and measurement-selection loop against predeclared baselines.
7. Only then assess adaptive mesh refinement, GPU execution, multiscale fidelity and ecosystem deployment.

Performance optimization must preserve declared canonical observables. Runners are a later exact-head evidence gate, not a reason to defer source review, mathematical fixtures, data admission, and implementation design.

## 10. Claim ceiling

This document is a research/implementation contract. It does not establish biological realism, measured field transfer, common-network function in arbitrary ecosystems, empirical forecast skill, SOTA superiority, or consciousness in plants/fungi or Symthaea.