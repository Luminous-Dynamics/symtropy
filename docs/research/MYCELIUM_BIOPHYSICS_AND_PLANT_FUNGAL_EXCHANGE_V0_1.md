# Mycelium Biophysics and Plant–Fungal Exchange Qualification

**Contract:** MYCO-BIO-QUAL-001  
**Version:** 0.1  
**Status:** Research/implementation specification; not a biology qualification  
**Owner:** Luminous Dynamics — Symtropy, composed with Symthaea evidence reasoning  
**Parent:** [Symtropy #5173 — Tropical Forest Ecological Qualification](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Integrated target:** [Symtropy #252 — Living World Cell](https://github.com/Luminous-Dynamics/symtropy/issues/252)  
**Tracking:** [Symtropy #5379](https://github.com/Luminous-Dynamics/symtropy/issues/5379)  
**Multiscale qualification:** [Symtropy #5413](https://github.com/Luminous-Dynamics/symtropy/issues/5413)

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

A source-level accounting audit identified a potential non-conservative behavior in the current loop: local assimilation is credited to the local biomass pool and a further 25% of that same quantity is also credited to origin biomass, without a corresponding debit from the local biomass stock. Biomass decay is also currently applied without a typed detrital/element destination. The new `step_with_report()` diagnostic API reports those credits/debits and observed decay in legacy model units while preserving `step()` behavior; it is not a physical ledger. A frozen four-case audit fixture is `benchmarks/mycelium/mycelium-field-v0-accounting-v0-1.json`. See [MYCELIUM-V0-ACCOUNTING-AUDIT-001 #5419](https://github.com/Luminous-Dynamics/symtropy/issues/5419).

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
- **Fusion point:** a hyphal anastomosis junction, created only when the selected model's fusion criterion is met. Distinguish tip-to-tip connection from tip-to-interior-segment fusion; the latter splits the target segment at the attachment point, as in the published BMX topology operation.
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

Transport must be decomposed into distinct regimes: external-medium diffusion, membrane exchange, passive inter-segment diffusion, selective/active transport, growth-induced advection, and microbial/extracellular transformation. BMX's published small-colony model uses passive concentration-gradient exchange between connected segments; the article states that larger-colony advection is a future requirement. The new passive transport primitive is only the numerical small-colony building block. The complete regime taxonomy, dimensional mapping and benchmark are tracked in [MYCO-TRANSPORT-REGIME-001 #5420](https://github.com/Luminous-Dynamics/symtropy/issues/5420).

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

### 3.6 Multiscale colony dynamics and process-driven fidelity

Explicit hyphal graphs are the high-detail reference, not necessarily the cheapest representation for every location and question. The model should support a hierarchy:

`explicit segments / tips / junctions -> colony-front and density surrogate -> patch / soil process state`

A reduced representation is admissible only for an explicitly declared observable-preservation profile. At minimum, the profile must state whether it preserves:
- colony-front position and width distribution;
- front advance mean and variance;
- radial/interior biomass or network-density profile;
- tip, branch, fusion and connected-component statistics;
- species-resolved transport or only aggregate fluxes;
- C/N/P pool and flux closure;
- intervention response and the causal paths represented.

Refinement must be **process-driven**, not camera-distance-driven. Promote to graph detail before an enabled process consumes missing topology, segment-level transport, root-interface geometry, or causal history. Use residuals and uncertainty to detect reduced-model inadequacy; do not infer that a smooth or visually plausible reduced output is adequate.

Demotion is allowed only when future-bearing history and the observables required by all enabled processes are preserved or explicitly marked unsupported. A coarse state must disclose lost information; it must never fabricate individual hyphae when materialized again. Any graph reconstruction is a new proposal/realization with explicit uncertainty, not recovery of the previous biological truth.

The current candidate is the recently published sBARE traveling-wave surrogate coupled to BMX. Its 5–6 November 2026 bioRxiv preprint is a challenger hypothesis, not a peer-reviewed result or reproduced baseline. The dedicated qualification and matched-cost tournament is [MYCO-MULTISCALE-QUAL-001 #5413](https://github.com/Luminous-Dynamics/symtropy/issues/5413).

## 4. Scientific evidence and comparator register

These are research anchors, not claims that Luminous has reproduced their results.

| Work | Use in this programme | Limit |
|---|---|---|
| Palmer et al., *Physical Review E* 112 (2025), “Three-dimensional Modeling of Hyphal Fusion, Branching, and Nutrient Transport in Filamentous Fungi”; [DOI](https://doi.org/10.1103/xbjp-zqj1), [PNNL record](https://www.pnnl.gov/publications/three-dimensional-modeling-hyphal-fusion-branching-and-nutrient-transport-filamentous), [Zenodo release 10.5281/zenodo.13851660](https://doi.org/10.5281/zenodo.13851660), [GitHub](https://github.com/PNNL-CompBio/BoltzmannMFX) | Main 3D discrete hyphal morphology / branching / fusion / transport comparator. BMX models connected rod-like hyphae, chemical reaction and diffusion, segment-to-segment transport and mechanical interactions. The published article identifies a BSD-2-Clause release on Zenodo. | Pin the exact Zenodo artifact, source revision, build recipe, input deck, solver settings, hardware and seed. Do not assume the moving GitHub default branch equals the paper's deposited release. A reproduced BMX case is not itself biological validation. |
| Yuan et al., “A Mycelium Dataset with Edge-Precise Annotation for Semantic Segmentation,” *Scientific Data* (10 December 2025). [DOI](https://doi.org/10.1038/s41597-025-06265-1), [benchmark code](https://github.com/yuanqianguang/MyceliumSeg-benchmark), [Zenodo record](https://zenodo.org/records/14243151) | Colony-footprint / outer-edge segmentation benchmark. The paper reports 20,176 images from four species and 567 pixel-annotated examples. Published baseline results include SegFormer IoU 95.22% / Boundary IoU 28.60% / HD95 75.95 px / ASSD 15.28 px; DeepLabv3 reports HD95 63.53 px, the best of the three on that metric. | The ground-truth mask outlines the colony growing around a sclerotium; internal hypha-level skeleton and network connectivity are not annotated. Therefore these masks cannot validate branch counts, individual hyphal segments, anastomosis or transport paths. The paper is CC BY-NC-ND 4.0; the Zenodo record's Rights/License field is blank, so resolve exact file-level terms and commercial/derivative permission before use. Image sequences are not automatically validated time-series trajectories. |
| Moseley et al., “SkelPy: an open-source GUI tool for automated fungal hyphal skeletonization,” *Applications in Plant Sciences* (2026). [Paper DOI](https://doi.org/10.1002/aps3.70049), [code repository](https://github.com/AaronMoseley/SkelPy), [downstream analysis](https://github.com/melaniemadrigal07/Post.ImageProcessing) | Direct hyphal-skeleton / time-series comparator, unlike MyceliumSeg's colony-front masks. The paper uses 180 images over 72 h from three *Botrytis cinerea* isolates and compares 15 manual traces. It reports mean normalized trace distance 0.0046 (SD 0.0009), maximum 0.0195; one 30-cell node-presence/absence assessment reports F1 0.905. | The node sample is small and isolate/experiment-specific; skeleton agreement is not transport or biological mechanism validation. Confirm the release's complete data availability, paper non-commercial licence and repository code licence before reuse; no root LICENSE was found in the inspected repository path. Do not train commercially or redistribute until rights are resolved. |
| Oyarte Galvez et al., “A travelling-wave strategy for plant–fungal trade,” *Nature* 639, 172–180 (2025). [DOI](https://doi.org/10.1038/s41586-025-08614-x), [MIT analysis code](https://github.com/Cocopyth/AMFTravellingWave), [Figshare source data](https://doi.org/10.6084/m9.figshare.27889143) | High-value graph + measured-flow benchmark for three AMF profiles (*Rhizophagus irregularis* A5/C2 and *R. aggregatum*) with *Daucus carota* root-organ culture. The study tracked >500,000 nodes, ~100,000 cytoplasmic-flow trajectories and >1,600 flow videos. It reports translating tip/density waves, approximately stable root-directed efficiency with rising global efficiency, edge-radius/betweenness association and bidirectional/antiparallel flow. | In-vitro, 2D-constrained plate networks; not a 3D soil reconstruction or a universal fungal rule. Flow was recorded in selected ROIs; max-speed outliers were manually screened. Code is stated as MIT by the article, but confirm the exact repository licence file; Figshare data terms must be checked at file level. Graph efficiency, flow velocity, volumetric flux and actual nutrient delivery must remain separate observables. |
| Yan & Marschmann, “Multi-scale biophysical modeling of fungal colony growth,” bioRxiv preprint v1 (posted 5–6 November 2026). [DOI](https://doi.org/10.64898/2026.11.05.764668) | sBARE is a stochastic traveling-wave surrogate proposed alongside high-fidelity BMX mechanics. Its reported analyses motivate separating front drift/variance from interior network density and testing whether reduced dynamics preserve colony-scale observables. | Preprint, not peer reviewed. The paper-specific state variables, equations, fitting data, source code and validation domain must be independently mapped and reproduced. Treat all reported results as preprint claims, not Symtropy results or accepted biological laws. |
| Waschk et al., “Evidence for resource transfer via common endophyte networks,” *Scientific Reports* (8 June 2026). [DOI](https://doi.org/10.1038/s41598-026-56653-9), [analysis/data repository](https://gitlab.com/fungalecology/cen) | Adds a distinct plant–endophyte network experiment: *Arabidopsis thaliana* with *Trichoderma viride*, *Mucor hiemalis* and *Fusarium temperatum* in triple-compartment Petri dishes. The four arms are +E/+I/+R, +E/+I/-R, -E/+I/+R and -E/-I/+R. | The paper includes a no-receiver arm, an isotope-only non-inoculated receiver control and a natural-abundance control, but lacks a donorless arm and the full isotope-control factorial for the no-receiver arm. F. temperatum receiver-plant 15N enrichment cannot be separated from fungal exudate/necromass followed by root uptake because receiver-soil 15N also increased. Plant D2O enrichment in the isotope-only control indicates an abiotic route such as evaporation/condensation; fungal-treatment plant values did not differ from that isotope control. 13C and 15N moved independently. Preserve those limits; do not claim direct hyphal transport or a universal coefficient. Verify separate GitLab data/script rights before ingestion. |
| 2026 review of AMF hyphosphere nitrogen cycling. [DOI](https://doi.org/10.1093/femsec/fiag050) | Identifies plausible hyphosphere pathways involving mineralization, nitrification, denitrification, N retention, fungal uptake and interactions with other microbes. It motivates a typed microbiome/process layer instead of attributing all N movement directly to fungi. | Review synthesis is not a single calibrated process law. Add only individually sourced processes and measurements; keep microbial mediation, abiotic transformation and fungal internal transport as separate mechanisms. |
| Brandt et al., “Reciprocal transport of carbon and nitrogen between plants and the hyphosphere microbiome in a temperate grassland,” *New Phytologist* (first published 10 September 2026). [DOI](https://doi.org/10.1111/nph.71576) | Source-faithful isotope-tracer case: plant (^{13}\mathrm{CO}_2) pulse label and (^{15}\mathrm{N}) supplied to a hyphal compartment, with isotope recovery assessed across compartments. | Reports 16.9% recovery of initially added (^{15}\mathrm{N}) in plants in this specific experiment. This is not a global coefficient or a universal transfer rate. The study's destructive sampling and experimental design must be retained in the reproduction crosswalk. |
| Systematic mapping of experimental approaches to common mycorrhizal networks in arbuscular mycorrhiza (first published 19 December 2024). [DOI](https://doi.org/10.1002/ppp3.10618) | Design controls for proving continuity and network-mediated transfer. | Highlights limited field/ecosystem coverage and methodological confounding. A positive tracer signal alone does not establish a common network pathway. |
| CRootBox and OpenSimRoot. [CRootBox](https://plant-root-soil-interactions-modelling.github.io/CRootBox/), [OpenSimRoot report](https://root2res.eu/wp-content/uploads/2025/09/Root2Res_D6.2-Updated-version-of-OpenSimRoot_FINAL.pdf) | Root architecture and functional plant/soil comparator. | These are plant-root tools, not fungal physiology models. Compose with them as reference implementations where appropriate; do not imply they validate the hyphal model. |
| OpenAlea.HydroRoot (2026). [Paper](https://doi.org/10.1093/insilicoplants/diag025), [source](https://github.com/openalea/hydroroot) | Current root-side comparator for multiscale root graphs, hydraulic conductance, solute transport and dynamic root-growth coupling. | It is a root hydraulics / solute framework, not a fungal-network model. Use it to define compatible plant-root exchange boundaries; do not duplicate root authority or assume its hydraulic parameters identify fungal conductance. |
| Collaborative root water uptake benchmark. [DOI](https://doi.org/10.1093/insilicoplants/diad005) | Matched problem-design precedent across DuMux, CPlantBox, R-SWMS, OpenSimRoot and SRI. | Reuse compatible benchmark scenarios and semantics; do not compare mismatched quantities. |

MyceliumSeg is deliberately only the colony-front imaging track. The masks trace a colony outline around a sclerotium; they do not provide individual hyphal skeletons or internal graph topology. A separate microscopy dataset with annotated hyphal traces, branch/fusion junctions, or a source-faithful manually traced subset is required before evaluating graph topology from images. Do not infer a transport-capable network from colony segmentation or visually continuous pixels.

The Brandt et al. isotope study is a community-level field experiment, not a single-strain fungal physiology calibration. It used 36 hyphoboxes in three labelling treatments × three destructive sampling times × four replicates, with a 50 μm mesh and a 4 mm air gap intended to restrict solution movement between compartments. The experiment sampled days 1, 3 and 7; one damaged dual-labelled box reduced the 3-day (^{15}N) sample to three replicates. The 16.9% recovery result is therefore specific to its mixed grassland plant/hyphosphere-microbiome system, labels, compartment geometry and sampling schedule. Preserve those details in any reproduction adapter; do not fit a universal “mycorrhizal transfer coefficient” from it.

The CEN study is a separate evidence profile, not a substitute for the Brandt study. Its four actual arms are +E/+I/+R (fungus, isotope, donor and receiver plants), +E/+I/-R (fungus and isotope, donor but no receiver), -E/+I/+R (no inoculated fungus, isotope and receiver) and -E/-I/+R (natural abundance). Its limitations are more specific than simply “missing controls”: there is no donorless arm and no full isotope-control factorial for the no-receiver arm (the authors give -E/-I/-R and -E/+I/-R as examples). For F. temperatum, amino-acid-derived 15N is elevated in both receiver plant and receiver soil, so direct hyphal delivery cannot be separated from fungal exudate/necromass followed by root uptake. For D2O, receiver-plant isotope controls are already elevated over natural abundance and fungal treatments do not differ from that isotope control, leaving evaporation/condensation or another abiotic pathway plausible. 13C and 15N moved independently. These are not grounds to dismiss the result; they define the mechanism-identification boundary and tell us exactly which follow-up arms to add. The machine-readable audit is `benchmarks/mycelium/cen-study-data-admission-v0-1.json`, and the reproduction work is tracked in [CEN-EXCHANGE-REPRO-001 #5416](https://github.com/Luminous-Dynamics/symtropy/issues/5416).

A 2026 review of plant regulation of arbuscular mycorrhizal symbiosis distinguishes supported regulatory checkpoints (including accommodation, investment, maintenance and interface turnover) from hypotheses that still need performance-linked evidence. Use those terms as measurable process transitions, not as evidence of unilateral “control” or organismal intention: [review](https://pmc.ncbi.nlm.nih.gov/articles/PMC13596872/).

## 5. Benchmark protocol

The initial machine-readable synthetic suite is `benchmarks/mycelium/mycelium-known-truth-cases-v0-1.json`. It contains six cases: linear extension, branch insertion/segment splitting, tip-to-tip anastomosis, tip-to-interior-segment anastomosis, rejection of hyphal flux without a connected path, and exact element-ledger closure. The fixture is deliberately synthetic and exists to define runner semantics before any biological calibration.

The multiscale graph-to-wave tournament is specified in `benchmarks/mycelium/multiscale-colony-benchmark-v0-1.json`: seven task families, six baseline/model families, ten observable/compute tracks, and explicit promotion/demotion and receipt contracts. It is a benchmark protocol, not an executed result. The CEN source-admission register is `benchmarks/mycelium/cen-study-data-admission-v0-1.json`; it explicitly records that source bytes are not downloaded, the data/script licences are unverified, and the study's control gaps must remain visible before any direct-transfer claim.

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

### T6 — multiscale reduction and adaptive fidelity

Compare explicit graph mechanics, source-faithful sBARE where reproducible, field-only Mycelium V0, fixed coarse dynamics and a residual-/process-triggered hybrid under the same initial conditions and evaluation observables.

Required metrics:
- front position, front width, mean front speed and front-advance variance;
- drift versus effective-diffusion identifiability and parameter uncertainty;
- radial/interior network-density profile;
- branch/fusion/topology error where explicit graph truth is available;
- transport/element-budget residuals when resource exchange is enabled;
- prediction-interval calibration, out-of-domain detection and worst-regime error;
- compute/error Pareto curves: wall time, memory, energy when measured, graph detail ratio and number of high-fidelity steps.

Controls must include fixed high fidelity, fixed coarse fidelity, random refinement and presentation-distance refinement. Promotion/demotion must preserve process sufficiency and future-bearing history; camera distance is not a model-validity signal. The detailed protocol belongs to [MYCO-MULTISCALE-QUAL-001 #5413](https://github.com/Luminous-Dynamics/symtropy/issues/5413).

### T7 — hyphal skeleton and topology observation

Use SkelPy as a direct skeletonization comparator where its complete image/data rights and acquisition metadata are available. The machine-readable admission record is `benchmarks/mycelium/skelpy-source-admission-v0-1.json`; the source-specific reproduction is tracked in [SKELPY-HYPHAL-REPRO-001 #5421](https://github.com/Luminous-Dynamics/symtropy/issues/5421). It records that the published 180-image series is a fixed-focal-plane 2D projection, that only 15 images from one isolate were manually traced, and that node-validation sample sizes are small. Separate publication, code and image-data rights must be admitted. Separate:
- trace geometry distance against manual traces;
- node detection precision/recall/F1;
- graph connectivity and branch-degree agreement;
- topology agreement under threshold / preprocessing sensitivity;
- time-series continuity across repeated images.

A small manual-trace subset must not be represented as dense ground truth for the full image corpus. Colony-outline masks from MyceliumSeg remain a separate segmentation task and cannot validate internal hyphal topology.

### T8 — dynamic AMF graph and cytoplasmic-flow co-evolution

Use the source-admitted Nature/Figshare dataset where its code/data versions and file-level rights are verified. The source register is `benchmarks/mycelium/amf-travelling-wave-source-admission-v0-1.json`; the machine-readable six-track evaluation contract is `benchmarks/mycelium/amf-graph-flow-benchmark-v0-1.json`; the reproduction and modelling work is tracked in [AMF-GRAPH-FLOW-QUAL-001 #5450](https://github.com/Luminous-Dynamics/symtropy/issues/5450).

Reproduce graph evolution and flow separately, then measure their coupling:
- node/edge identity persistence, branching, anastomosis, spore/BAS structures and front movement;
- radial growing-tip and filament-density profiles, wave speed and saturation density;
- normalized root-directed and global graph efficiency versus construction cost, using the source's MST/DT reference normalization;
- ROI-linked flow direction, bidirectional/antiparallel streams, velocity distributions and temporal stability;
- relationships between edge radius, betweenness centrality, distance to root and measured flow;
- held-out prediction of later network states and flow using early-time observations.

Controls must include topology-only shortest-path, uniform-radius/uniform-flow, shuffled flow-to-edge assignment, degree-preserving rewiring and leave-strain-out evaluation. Do not infer measured flow from graph centrality alone. The Rust observation adapter `CytoplasmicFlowObservation` preserves individual signed stream/ROI/time/segment identities, including antiparallel streams. Volumetric flow is returned only when cross-section area is available; solute mass rate also requires an admitted concentration. The source is limited to three AMF profiles in a 2D-constrained in-vitro system.

This benchmark tests whether structure and flow co-evolve to balance root delivery, exploration, robustness and construction cost. It does not imply that fungi optimize a human-designed objective; report the measured Pareto structure descriptively.

### T9 — structural network resilience

The separate qualification is [MYCO-TOPO-RESILIENCE-QUAL-001 #5461](https://github.com/Luminous-Dynamics/symtropy/issues/5461), with known-truth cases in `benchmarks/mycelium/topology-resilience-v0-1.json`.

Measure active-segment bridges and articulation nodes; compare random edge deletion with betweenness-, bridge- and articulation-targeted removals, plus degree/radius/age-matched controls. Report root-reachable fraction, component count, largest-component ratio, normalized root/global efficiency, and edge/node survival curves. The graph kernel's iterative `active_graph_criticality()` uses iterative Tarjan-style traversal, excludes inactive segments and handles parallel segments as distinct edges. The current ordered-map/set representation adds logarithmic lookup overhead; large-graph scaling must be measured before claiming a complexity/runtime target.

This is structural fragility only. It must not be described as measured flow disruption, organismal resilience or nutrient-delivery loss without source-linked flow/tracer observations. The AMF source is observational and does not include randomized damage/re-growth experiments; simulated repair remains a hypothesis until experimentally measured.

## 6. Baselines and competitive positioning

Use matched baselines:
1. Existing Symtropy Mycelium V0 (stylized field model; preserve as a legacy profile until [#5419](https://github.com/Luminous-Dynamics/symtropy/issues/5419) resolves its stock/flux accounting).
2. Diffusion-only field transport.
3. Simple branching random walk / lattice-growth null.
4. BMX fungal model pinned to the published Zenodo release for matched 3D hyphal morphology/fusion/transport cases.
5. sBARE source-faithful reproduction, if its code/data and equations are sufficiently available; otherwise NOT_RUN.
6. MyceliumSeg for colony-front segmentation under its official split and terms.
7. SkelPy for hyphal skeletonization / manual-trace comparison, subject to full data and code-rights admission.
8. CRootBox/OpenSimRoot/HydroRoot for their respective root-side questions.
9. Measured isotope-tracer observations and experimental controls, separately for mycorrhizal and endophyte systems.

The intended differentiation is **composition, process-driven multiscale fidelity and evidence quality**, not a claim that one solver replaces specialist tools. The opportunity is to join explicit fungal topology, typed transport, a calibrated colony-scale surrogate, existing plant-root authority, element budgets, multi-microbe interaction hypotheses, tracer validation and calibrated uncertainty in one reproducible causal experiment. Symtropy owns canonical physical execution; Symthaea proposes and compares hypotheses, interprets uncertainty and ranks bounded next measurements.

A comparison is fair only when species, scale, initial conditions, forcing, observations, splits, runtime budget and metrics match. Published benchmark numbers must be distinguished from reproduced numbers.

## 7. Research targets (not achieved results)

Only freeze numeric superiority targets after reproducing the comparator and its split. The tournament should aim for:
- no regression against the best reproduced BMX/reference morphology score on exactly matched cases;
- improved coupled exchange forecasts or element-budget closure over current field-only and diffusion-only controls, with independent replicate blocks;
- predictive interval coverage near its preregistered nominal level, with interval width and worst-stratum score visible;
- 100% pass on frozen synthetic conservation, disconnected-path, idempotency, provenance and invalid-unit fixtures;
- better decision value for measurement proposals than random, sensitivity-only and uncertainty-only policies under equal cost/budget.

No SOTA claim may be made from synthetic verification, morphology resemblance, segmentation alone, one species, one field study, or runtime alone.

## 8. Symthaea reasoning integration

Compose with existing Symthaea research and experiment-selection owners rather than building a second hypothesis engine:

- [OLI-EXPERIMENT-001 #11446](https://github.com/Luminous-Dynamics/symthaea/issues/11446): substrate-agnostic property hypotheses and robust experiment selection.
- [OLI-OOD-ATTACK-001 #11451](https://github.com/Luminous-Dynamics/symthaea/issues/11451): adversarial confidence-surface search for false life/agency/intelligence claims.
- [FEP open-world model adequacy #11181](https://github.com/Luminous-Dynamics/symthaea/issues/11181): residual/OTHER mechanisms and adequacy-gated conclusions.
- [SCI-EXP-001 #6226](https://github.com/Luminous-Dynamics/symthaea/issues/6226): experimental-unit, replication, blocking and inference-validity semantics.
- [RQ-043 #2579](https://github.com/Luminous-Dynamics/symthaea/issues/2579): epistemic action selection and value-of-information qualification.

The biological adapter should expose source-bound observation and simulation receipts, explicit rival explanations (e.g. hyphal transport, soil diffusion, microbial mediation, contamination, no transfer), predicted discriminating outcomes and costs. Symthaea compares those hypotheses and proposes the next measurement under the existing budget/adequacy contracts. It does not access evaluator-only truth, invent biological priors or parameters, authorize wet-lab work, or treat a high information score as proof.

The Symthaea benchmark should separately measure proper predictive score/calibration, model-inadequacy detection, false causal attribution under adversarial controls, abstention/coverage, and realized decision value at equal experiment budgets. A benchmark pass in hypothesis selection does not qualify Symtropy's physical model.

## 9. Ownership and claim boundaries

- **Symtropy:** canonical hyphal graph, field coupling, transport, ledgers, replay and benchmark execution. The initial passive transport primitive, if accepted by review and CI, only settles caller-supplied conductance on active graph edges; it is not a biological conductance model.
- **Plant structural owner:** canonical root/plant structure and developmental authority; this work consumes its interfaces.
- **Basin/soil/hydrology owners:** existing environmental substrate and water processes; no parallel solver.
- **Symthaea:** evidence synthesis, causal hypotheses, calibrated forecast interpretation and bounded experiment proposals; it cannot invent source truth or model parameters.
- **Sol-Atlas:** spatial/remote-sensing source identity and geospatial evidence when used.
- **Mycelix:** data rights, consent, custody and attribution when required.

Do not infer intelligence, cognition, consciousness, intention or a universal communication protocol from a graph, morphology, electrical oscillation, scalar coherence or simulated task performance.

## 10. Implementation sequence

1. Freeze T0 data and model semantics; record the exact current stylized baseline.
2. Implement the small deterministic CPU hyphal graph kernel and its synthetic known-truth fixtures.
3. Add graph-to-field transport and element ledgers with no hidden clamps.
4. Integrate one taxon-specific plant interface through the canonical root owner.
5. Admit and reproduce the BMX Zenodo release on a minimal matched case; map sBARE's preprint equations and artifacts, marking unavailable pieces NOT_RUN.
6. Add a SkelPy/manual-trace topology observation adapter and keep it separate from MyceliumSeg colony segmentation.
7. Reproduce one isotope-tracer case and separately admit the CEN study with explicit abiotic/microbial confounder controls.
8. Add Symthaea's open-world hypotheses, uncertainty and measurement-selection loop against predeclared baselines.
9. Only then assess adaptive mesh refinement, GPU execution, multiscale fidelity and ecosystem deployment.

Performance optimization must preserve declared canonical observables. Runners are a later exact-head evidence gate, not a reason to defer source review, mathematical fixtures, data admission, and implementation design.

## 11. Claim ceiling

This document is a research/implementation contract. It does not establish biological realism, measured field transfer, common-network function in arbitrary ecosystems, empirical forecast skill, SOTA superiority, or consciousness in plants/fungi or Symthaea.