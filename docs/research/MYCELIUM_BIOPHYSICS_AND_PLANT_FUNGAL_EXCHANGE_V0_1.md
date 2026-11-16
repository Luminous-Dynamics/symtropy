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

A source-level accounting audit identified a potential non-conservative behavior in the current loop: local assimilation is credited to the local biomass pool and a further 25% of that same quantity is also credited to origin biomass, without a corresponding debit from the local biomass stock. Biomass decay is also currently applied without a typed detrital/element destination. The new `step_with_report()` diagnostic API exposes those credits/debits and observed decay in legacy model units; the ordinary `step()` path skips the report-only full-grid scans. Both paths share the same step implementation, with eight-step field-by-field parity tests. This is not a physical ledger. A frozen four-case audit fixture is `benchmarks/mycelium/mycelium-field-v0-accounting-v0-1.json`. A separate performance qualification is tracked in [MYCELIUM-FIELD-PERF-QUAL-001 #5499](https://github.com/Luminous-Dynamics/symtropy/issues/5499). See [MYCELIUM-V0-ACCOUNTING-AUDIT-001 #5419](https://github.com/Luminous-Dynamics/symtropy/issues/5419).

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
| Moseley et al., “SkelPy: an open-source GUI tool for automated fungal hyphal skeletonization,” *Applications in Plant Sciences* (2026). [Paper DOI](https://doi.org/10.1002/aps3.70049), [code repository](https://github.com/AaronMoseley/SkelPy), [downstream analysis](https://github.com/melaniemadrigal07/Post.ImageProcessing) | Direct hyphal-skeleton / time-series comparator, unlike MyceliumSeg's colony-front masks. The paper uses 180 images over 72 h from three *Botrytis cinerea* isolates and compares 15 manual traces. It reports mean normalized trace distance 0.0046 (SD 0.0009), maximum 0.0195; one 30-cell node-presence/absence assessment reports F1 0.905. On an Apple M4 MacBook Pro it skeletonized 180 images in 102 minutes using 234 MB, and processed 3,842 images in 12 h 30 min. | The node sample is small and isolate/experiment-specific; skeleton agreement is not transport or biological mechanism validation. Confirm the release's complete data availability, paper non-commercial licence and repository code licence before reuse; no root LICENSE was found in the inspected repository path. Do not train commercially or redistribute until rights are resolved. |
| Sachs et al., “mycelyso – high-throughput analysis of Streptomyces mycelium live cell imaging data,” *BMC Bioinformatics* 20, 452 (2019). [DOI](https://doi.org/10.1186/s12859-019-3004-1), [source](https://github.com/modsim/mycelyso), [demo data](https://doi.org/10.5281/zenodo.376281) | Strongest high-throughput hypha-level time-series baseline found so far. Fully automated phase-contrast batch pipeline segments, skeletonizes and tracks individual Streptomyces hyphae; exports graph/track data, intermediates and parameters in HDF5/GraphML. Metrics include per-hypha growth, whole-network elongation, branching, tip counts and hyphal growth unit. Reported 1.4 GiB / 136 time points processed in 52.5 ± 1.2 s (~2.5 frames/s) on quad-core i7-4790 / 32 GB; a screening processed ~680 GiB and >600 stacks. | Designed for Streptomyces microfluidic cultivation, not AMF. Original pipeline stops tracking at 20% chamber coverage to limit dense-network errors and visually inspects a subset; this censoring must be disclosed and tested. BSD-2-Clause code license, confirmed against the upstream LICENSE.rst at pinned revision `8af386c6e7f02f1c83ca8136487c9f3c05605fc8`. The package metadata identifies version 1.0.1.dev2; demo archive DOI 10.5281/zenodo.376281 still needs record-level Rights/checksum admission. Benchmark full automation, topology identity and dense-network failure, not just throughput. |
| Guo et al., “TipQuant: a robust algorithm for quantitative analysis of spatiotemporally dynamic activities in tip-growing cells,” *New Phytologist* 252 (2026), 926–935. [DOI](https://doi.org/10.1111/nph.71474), [open paper](https://pmc.ncbi.nlm.nih.gov/articles/PMC13583362/) | Live apex detection by maximum contour expansion plus selectable plasma-membrane and apical-cytoplasm ROIs, kymographs and fluorescence distributions. Demonstrated on *Fusarium graminearum* hyphae and *Arabidopsis thaliana* pollen tubes; tested with Leica/Nikon/Zeiss confocal data. Valuable comparator for time-localized tip and molecular dynamics, not complete graph extraction. | Explicit limitations: >90° turns/U-turns reduce tip detection robustness; nongrowing cells are unsupported; multiple tips in one video require cropping; confocal only, not epifluorescence. Python 3.11+ and Poetry. Article CC BY 4.0. Upstream source is pinned to `julianlegouic/TipQuant@7495eaefa481d07fd6f657e3660a3c7c2dddb1f5` (TipQuant 1.0); the inspected repository has no root LICENSE file, so code reuse rights remain unresolved and must not be inferred from the article license. The paper's raw-video dataset is Dryad DOI 10.5061/dryad.dr7sqvbd2 (203.05 MB, published 10 July 2026); Dryad datasets are CC0 under Dryad's policy, but validate dataset README/version at ingest. |
| HyphaTracker, “An ImageJ toolbox for time-resolved analysis of spore germination in filamentous fungi,” *Scientific Reports* 8, 605 (2018). [DOI](https://doi.org/10.1038/s41598-017-19103-1), [paper and supplement](https://pmc.ncbi.nlm.nih.gov/articles/PMC5766585/) | Early germination / germ-tube time series baseline for *Fusarium fujikuroi*: drift correction, stack reduction, binarization, germling ROIs, identity, area, circularity and timing. Tested against synthetic germlings across SNR levels and 184 bright-field images spanning >15 h. | The published analysis omitted crossing hyphae and image-boundary objects; authors state this is only valid when crossings are infrequent and the field is sparse. It is a germination baseline, not dense mycelial graph reconstruction. |
| AnaMorf, “Automated analysis of filamentous microbial morphology with AnaMorf,” *Biotechnology Progress* (2015). [DOI](https://doi.org/10.1002/btpr.2087) | Legacy ImageJ plugin for automated morphology quantification of filamentous microbes; useful as a process-engineering / morphology feature baseline. | Requires exact source version, data and license admission; older ImageJ ecosystem compatibility needs reproduction. Do not assume it provides persistent graph identity or 3D topology. |
| Sten et al., “Three-dimensional graph reconstruction of filamentous structures from z-stack images,” *Ecological Informatics* 90 (December 2025), 103317. [DOI](https://doi.org/10.1016/j.ecoinf.2025.103317), [Zenodo v2 code/data](https://doi.org/10.5281/zenodo.15780623) | Direct 3D Z-stack graph reconstruction comparator: per-plane skeletons are compressed to homeomorphic graphs, matched across focal planes, depth-estimated by shape-from-focus, and graph overlaps/crossings resolved. Reports printed-prototype geometry RMSE below the 0.5 mm filament radius for most cases and overlap-detection F1 0.91–0.92 on six annotated *Rhizophagus irregularis* images. | Zenodo v2 includes an MIT-style software license, but data directories have distinct source/copyright notes; audit each separately. Authors explicitly state some branch/crossing cases need manual adjudication and document a missed crossing in `o.jpg` that must be added by hand. Score automated-only and human-assisted modes separately. It is not proof of true anastomosis. |
| SNT (Simple Neurite Tracer successor), ImageJ/Fiji framework. [Documentation](https://imagej.net/plugins/snt/), [Nature Methods paper](https://doi.org/10.1038/s41592-021-01105-7), [source](https://github.com/morphonets/SNT) | Cross-domain 3D morphology baseline, not fungal-specific. Supports tracing, proof-editing, visualization, quantification and modeling of neuronal morphology; its documentation explicitly notes use on filamentous structures beyond neurons, modern multidimensional microscopy, automated/semi-automated tracing, scripting and experimental TB-scale support. The precursor Simple Neurite Tracer supports semi-automated 3D tube/filament tracing. | GPL-3.0; Fiji/ImageJ and the current SNT release must be pinned separately from the archived legacy Fiji/SNT repository. Benchmark it as a transfer baseline on the same z-stacks; do not treat neuronal tuning or tracing accuracy as fungal validation. Compare both automated and human-assisted workflow, labor and output graph semantics. |\n| Oyarte Galvez et al., “A travelling-wave strategy for plant–fungal trade,” *Nature* 639, 172–180 (2025). [DOI](https://doi.org/10.1038/s41586-025-08614-x), [MIT analysis code](https://github.com/Cocopyth/AMFTravellingWave), [Figshare source data](https://doi.org/10.6084/m9.figshare.27889143) | High-value graph + measured-flow benchmark for three AMF profiles (*Rhizophagus irregularis* A5/C2 and *R. aggregatum*) with *Daucus carota* root-organ culture. The study tracked >500,000 nodes, ~100,000 cytoplasmic-flow trajectories and >1,600 flow videos. It reports translating tip/density waves, approximately stable root-directed efficiency with rising global efficiency, edge-radius/betweenness association and bidirectional/antiparallel flow. | In-vitro, 2D-constrained plate networks; not a 3D soil reconstruction or a universal fungal rule. Flow was recorded in selected ROIs; max-speed outliers were manually screened. Code is stated as MIT by the article, but confirm the exact repository licence file; Figshare data terms must be checked at file level. Graph efficiency, flow velocity, volumetric flux and actual nutrient delivery must remain separate observables. |
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

### T10 — 3D Z-stack graph reconstruction and overlap uncertainty

Use the frozen Sten et al. release [Zenodo v2 DOI 10.5281/zenodo.15780623](https://doi.org/10.5281/zenodo.15780623) and paper [DOI 10.1016/j.ecoinf.2025.103317](https://doi.org/10.1016/j.ecoinf.2025.103317). The source-specific admission record is `benchmarks/mycelium/sten-3d-zstack-source-admission-v0-1.json`; reproduction is tracked in [MYCO-3D-ZSTACK-REPRO-001 #5468](https://github.com/Luminous-Dynamics/symtropy/issues/5468).

Reproduce the stages separately: per-plane skeleton detection, sparse homeomorphic graph extraction, cross-plane node matching, shape-from-focus depth estimation, overlap/crossing detection and topology correction. Score printed 3D prototype geometry separately from biological overlap labels. Preserve separate data rights for Z-stacks, annotations, 2D source images, prototype imagery and third-party functions.

The paper explicitly relies on manual intervention for some crossing cases, including a documented false negative in `o.jpg`. Report automated-only, automated-with-abstention, human-assisted and independently adjudicated results separately. Ambiguous projection overlap is not evidence of hyphal fusion.

The code branch now has a conservative `crossing_qualification` module. It compares explicit closed depth-uncertainty intervals: disjoint intervals are classified as depth-separated; overlapping or missing intervals require review. The API exposes a set-valued topology result: disjoint depths allow only separate filaments, while ambiguous/missing depth retains both separate-filament and shared-3D-junction hypotheses. It is not a probability distribution and never authorizes an automatic graph join. Eleven tests cover separated, overlapping, touching, missing, invalid and same-track evidence plus hypothesis-set semantics. The synthetic 12-case tournament is tracked in [MYCO-3D-KNOWN-TRUTH-001 #5494](https://github.com/Luminous-Dynamics/symtropy/issues/5494), with `benchmarks/mycelium/myco-3d-known-truth-v0-1.json` and `benchmarks/mycelium/myco-3d-known-truth-fixtures-v0-1.json`.

The active-graph criticality implementation is also checked against an independent remove-an-edge/remove-node connected-component oracle for every simple undirected graph with one through five vertices (1,099 graphs in total). This is more valuable than a few hand-picked fixtures because it checks all small topologies, including disconnected graphs, cycles and articulation cases.

For a cross-domain benchmark, include [SNT](https://imagej.net/plugins/snt/) as a 3D filament-tracing comparator. It supports multidimensional microscopy, semi/automated tracing, proof-editing and scripting; it is GPL-3.0 and is not a fungal-specific accuracy reference. SkelPy adds an important throughput baseline: 180 images in 102 minutes / 234 MB on Apple M4, and 3,842 images in 12.5 hours. Match hardware, resolution and input complexity before claiming a speed advantage.

A separate 2.6 TB bright-field fungal Z-stack/time-lapse dataset [Zenodo DOI 10.5281/zenodo.23135747](https://doi.org/10.5281/zenodo.23135747) is embargoed until 8 October 2027. Record it as a future target only; do not ingest or redistribute files before release and rights review.

### T11 — observation-tool tournament: hypha tracking, tip growth and molecular dynamics

The matched tool landscape and source-admission matrix are recorded in `benchmarks/mycelium/myco-observation-tool-tournament-v0-1.json`; execution is tracked in [MYCO-OBS-TOOL-TOURNAMENT-001 #5504](https://github.com/Luminous-Dynamics/symtropy/issues/5504).

Use each tool only on the task it actually measures:
- **mycelyso:** hypha-level segmentation/skeletonization/tracking and growth kinetics at high throughput; include its 20% chamber-coverage stop condition and manual QC burden in the comparison.
- **TipQuant:** growing-apex and membrane/cytoplasm molecular dynamics; stratify sharp turns, U-turns, nongrowing periods and multiple tips.
- **HyphaTracker:** early conidial germination and germ-tube growth; preserve its sparse-field/crossing omission rule.
- **AnaMorf/SParticle:** morphology and aggregate-level complementary baselines; admit exact code and source data before comparison.
- **SNT/Fiji:** cross-domain 3D trace/proof-editing baseline.

Do not produce one pooled score across segmentation, skeletonization, graph tracking, tip-growth and molecular fluorescence. Report per-task metrics, data-censoring, human review, speed, memory and worst-stratum performance. The Symtropy target is a composable evidence chain linking frame/ROI to 3D hypotheses, persistent graph identity and measured dynamics, not a claim to replace each specialized tool at its own task.

### T12 — source-to-graph evidence receipt and replay

The code branch adds `ObservationEvidenceReceipt` in `crates/domains/symtropy-mycelium/src/observation_evidence.rs`. It binds each observation to source-file SHA-256, frame/slice/timestamp, pixel ROI, physical pixel/voxel calibration, processor name/revision, parameter digest, observation subject, optional graph node/segment target, optional applied graph-event ID, spatial uncertainty and pseudonymous human-review status. Depth estimates require explicit Z calibration; graph targets and graph-event references must resolve against the canonical graph; zero digests are rejected.

The implementation is tracked in [MYCO-EVIDENCE-RECEIPT-001 #5509](https://github.com/Luminous-Dynamics/symtropy/issues/5509). Sixteen tests are authored for 2D/3D calibration, invalid time/ROI/spacing, human-review and independent-adjudication fields, uncertainty, graph-target/event identity, zero digests and canonical byte determinism. The CFN/CMN evidence classifier adds 14 tests for criterion receipts, claim levels, missing evidence and the separation of continuity from cytoplasmic flow. `canonical_bytes()` now defines a versioned encoding: fixed magic/schema prefix, explicit field order and enum/option tags, little-endian integers/f64, length-prefixed UTF-8 strings, sorted independent-reviewer keys and normalized negative zero. The module does not calculate cryptographic hashes; a separately admitted SHA-256 implementation must hash these bytes. A receipt proves traceability and declared calibration, not measurement correctness or biological validation.

### T13 — multiplex hyphal highway: filament, liquid film and bacterial dispersal

The new source is Moratto et al. 2026, [microLife DOI 10.1093/femsml/uqag035](https://doi.org/10.1093/femsml/uqag035), with source data at [Zenodo DOI 10.5281/zenodo.20071448](https://doi.org/10.5281/zenodo.20071448). The benchmark is [MYCO-HYPHAL-HIGHWAY-001 #5562](https://github.com/Luminous-Dynamics/symtropy/issues/5562), with evaluation contract `benchmarks/mycelium/hyphal-highway-benchmark-v0-1.json`.

The new code path adds `HyphalHighwayNetwork`: every observed bacterial route hop must have an active source-supported hyphal segment, a source-supported connected liquid bridge, two separately source-linked endpoint liquid patches and a source-linked bacterial trajectory. One receipt cannot be reused to stand for two distinct liquid patches. Filament identity distinguishes true fungus, oomycete and mixed routes; `Pythium ultimum` remains an oomycete, not a true fungus. The classification does not infer plant nutrient delivery.

This track measures outlet arrival, CFU recovery, time to arrival, liquid-patch connectivity, bacterial tracks and false highway attribution. Preserve the source result that outlet CFUs were detected in 4 of 8 co-inoculated devices; do not turn a small-sample stochastic outcome into a deterministic connectivity law.

### T14 — host-independent AMF flow and cargo-specific motion

Klein et al. 2026, [Fungal Biology DOI 10.1016/j.funbio.2026.101775](https://doi.org/10.1016/j.funbio.2026.101775), studied two `Rhizophagus irregularis` strains without a plant host. The reproduction contract is [AMF-AUTONOMOUS-FLOW-QUAL-001 #5565](https://github.com/Luminous-Dynamics/symtropy/issues/5565), with `benchmarks/mycelium/amf-autonomous-flow-benchmark-v0-1.json`.

The paper reports ~5,000 cytoplasmic-flow videos / ~25,000 trajectories, myristic-acid-associated increases in network length/area and mean flow speed, 10–30 µm/s rare flow bursts, and ~3 µm/s fluorescent-lipid motion unaffected by myristic acid. Raw data are available upon request rather than in a publicly downloadable raw archive.

The Rust flow record now distinguishes `BulkCytoplasm`, `FluorescentLipidCargo` and `UnspecifiedParticle`; trajectory-particle velocity is never promoted to a cross-section mean. Keep local velocity, bulk volumetric flow, cargo speed, solute mass rate and plant transfer as separate observables. Host-free internal movement is not evidence of plant-to-plant transfer.

### T15 — global AM fungal hyphal density and biomass: measurement-chain reproduction

Stewart et al. 2026, [Science DOI 10.1126/science.adu4373](https://doi.org/10.1126/science.adu4373), combine 322 studies / >16,000 soil cores across nine biomes with robotic imaging of >300,000 hyphae to calibrate biomass from network-length measures. The authors estimate 1.10 × 10^17 ± 0.13 × 10^17 km of living topsoil hyphae and ~300 ± 60 megatons of biomass. Data and code are listed on [Dryad DOI 10.5061/dryad.p2ngf1w1f](https://doi.org/10.5061/dryad.p2ngf1w1f).

The reproduction and generalization track is [AMF-GLOBAL-BIOMASS-QUAL-001 #5566](https://github.com/Luminous-Dynamics/symtropy/issues/5566), with `benchmarks/mycelium/amf-global-biomass-benchmark-v0-1.json`. Treat the result as a statistical measurement chain: field-core harmonization → spatial density model → two-species image-length calibration → global biomass extrapolation. Do not present the global total as direct underground observation. Required splits are study-, biome- and spatial-blocked; random core splits within a study leak protocol/site information.

### T16 — hyphal SIP-Raman / SRS metabolomics and measurement-induced damage

Pucetaite et al. 2026, [Fungal Biology DOI 10.1016/j.funbio.2026.101749](https://doi.org/10.1016/j.funbio.2026.101749), use deuterated-glucose SIP-Raman and SRS on `Psilocybe cf. subviscida` in microfluidic soil chips. Cu stress reduced glucose metabolic activity and growth; cytochrome-c spectral signal was also reduced. Repeated spontaneous-Raman exposure damaged hyphae, especially thin exploratory and Cu-stressed hyphae; SRS provided faster chemical mapping at lower exposure and localized labelled glucose near hyphal inner edges. The source says further chip changes are needed for optical transparency and sterility.

The work is tracked in [MYCO-HYPHAL-SPECTRAL-QUAL-001 #5568](https://github.com/Luminous-Dynamics/symtropy/issues/5568), with `benchmarks/mycelium/hyphal-spectral-benchmark-v0-1.json`. PR #5388 adds typed source subjects and `HyphalRamanObservation` for SIP-Raman, spontaneous Raman, SRS maps, isotope label, stressor, bands, acquisition settings and prior exposures to the same ROI. Nominal incident laser energy is not absorbed dose; spectral intensity is not metabolic rate without an independently receipted calibration; cytochrome-c signal is not a direct viability assay.

### T17 — global AM fungal hyphal density and biomass

Stewart et al. 2026, Science 392(6803), 1171–1176, [DOI 10.1126/science.adu4373](https://doi.org/10.1126/science.adu4373), combine 322 studies and more than 16,000 soil cores across nine biomes with a Random Forest density model at 1 km² and robotic hyphal-radius imaging. The public [Dryad release 10.5061/dryad.p2ngf1w1f](https://doi.org/10.5061/dryad.p2ngf1w1f) is 2.16 GB and contains source tables, mean/SD rasters, sampling-intensity rasters, raw hyphal-width TIFFs and Python/Google Earth Engine + R code. The paper abstract reports >300,000 imaged hyphae while the Dryad README reports >280,000 across five AMF species; exact counts must be reconciled from the source inventory.

The source register is benchmarks/mycelium/amf-global-density-source-admission-v0-1.json; the spatial model tournament is benchmarks/mycelium/amf-global-density-tournament-v0-1.json; reproduction is tracked in [AMF-GLOBAL-DENSITY-QUAL-001 #5599](https://github.com/Luminous-Dynamics/symtropy/issues/5599).

Required split policy is leave-study-out, spatial block, leave-biome-out, leave-method-out and deep-soil holdout. Random row splits are diagnostic only. Report prediction-interval coverage and width by biome and sampling-intensity stratum, residual spatial autocorrelation, out-of-domain detection and active-sampling value. Preserve the distinction between observed soil-core density, predicted raster density, effective-radius calibration, geometric hyphal volume fraction, dry mass, carbon stock, turnover and nutrient flux. A modelled density raster is not a direct underground observation, and higher hyphal abundance is not proof of plant benefit.

The public Rust spatial-evidence module represents field measurements and raster predictions as distinct types. It retains study/record identity, source-file SHA-256, optional coordinates and depth, density units, method, replicate count, raster model/source digests, prediction uncertainty and sampling intensity. Missing geolocation/depth remains missing. It derives a geometric hyphal-volume fraction from length density and effective radius, but deliberately does not return dry biomass or carbon without separate tissue-density and carbon-fraction calibration. An optional image/ROI receipt must resolve and validate when referenced. The field-observation catalog rejects duplicate sample IDs and duplicate source-row keys within a study, while retaining non-georeferenced rows outside the spatial-training iterator. It assigns complete studies to deterministic, approximately balanced round-robin folds; this prevents within-study leakage but does not replace geographic block or leave-biome-out validation. It also derives a delta-method, first-order geometric volume-fraction standard deviation only when both length-density SE and radius SD are supplied; missing uncertainty is never treated as zero, and the formula assumes independent input errors. Twenty-one tests cover unit/range validation, missingness, receipt linkage, duplicate identities, grouped-study folds, raster uncertainty and geometric volume/uncertainty conversion.

### T18 — CMN effect-size reproduction and context dependence

Lehmann et al. 2025, Functional Ecology 39(10), 2758–2771, [DOI 10.1111/1365-2435.70125](https://doi.org/10.1111/1365-2435.70125), screened 833 articles and extracted 31 eligible AM network studies; only seven tested CMN with hyphal continuity. The plant-unit network analysis found neutral effects for 11 of 15 response variables, while growth-unit robustness analyses did not reproduce the plant-unit significant effects; CMN-HC effects were neutral. The authors describe generally small, inconsistent and context-dependent effects. The underlying data are at [Figshare 10.6084/m9.figshare.29416427](https://doi.org/10.6084/m9.figshare.29416427).

The reproduction contract is benchmarks/mycelium/amf-cmn-meta-reproduction-v0-1.json; work is tracked in [AMF-CMN-META-REPRO-001 #5600](https://github.com/Luminous-Dynamics/symtropy/issues/5600). Reproduce the four effects (network, CMN-HC, mycorrhiza and root) independently, keep plant-unit and growth-unit levels distinct, reproduce natural-log response ratios and sampling variances, and reproduce the multi-level mixed-effects structure. Audit the two variance imputations, n=3 fallback for missing sample size and zero-control root-colonization transform.

The paradigm shift is to benchmark a model on its ability to recover small, neutral, negative and context-specific outcomes—not to make all network interactions appear beneficial. Infrastructure density and functional plant response must remain separate target variables. A neutral or negative effect prediction is a valid model success when supported by the data; no positive-result target is permitted.

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
10. PNAS 2026 C:P morphology/ML-derived exchange estimates and host-genotype-dependent exploration/exploitation Pareto fronts; reproduce the exact release before treating its estimates as direct flux.
11. Root-anatomy energetic theory (Cao et al. 2025) as a separate equation-level comparator, not a field-calibrated transport solver.
12. Field hyphosphere C:N time-course (Brandt et al. 2026), preserving its 36-hyphobox randomized design and reduced 3-day 15N n=3.

The intended differentiation is **composition, process-driven multiscale fidelity and evidence quality**, not a claim that one solver replaces specialist tools. The opportunity is to join explicit fungal topology, typed transport, a calibrated colony-scale surrogate, existing plant-root authority, element budgets, multi-microbe interaction hypotheses, tracer validation and calibrated uncertainty in one reproducible causal experiment. Symtropy owns canonical physical execution; Symthaea proposes and compares hypotheses, interprets uncertainty and ranks bounded next measurements.

A comparison is fair only when species, scale, initial conditions, forcing, observations, splits, runtime budget and metrics match. Published benchmark numbers must be distinguished from reproduced numbers.

## 7. Research targets (not achieved results)

Freeze numeric superiority targets only after reproducing each comparator's exact release, hardware, data split and evaluation metric. Until then, these are proposed research targets, not achieved results or committed acceptance thresholds:

| Competition axis | Published comparison point | Proposed Symtropy target after matched reproduction |
|---|---|---|
| 2D colony-edge segmentation | MyceliumSeg: 20,176 images and 567 manually pixel-labelled masks; boundary metrics are distinct from internal hyphal graph truth | Reproduce exact split and published metrics; do not use its colony masks to score branch/junction topology |
| 2D hyphal skeletonization | SkelPy: 180 images / 72 h / three *Botrytis cinerea* isolates; mean normalized trace distance 0.0046, SD 0.0009; 3,842-image run in 12 h 30 min using 234 MB on Apple M4 | Match or improve trace error; target ≥2× throughput on the same hardware and image batch only if topology/trace error is non-inferior |
| 3D printed filament geometry | Sten et al.: RMSE below 0.5 mm filament radius for most printed prototypes | Target median RMSE ≤0.25 filament radii and 95th-percentile RMSE ≤0.5 radii across every admitted prototype; report every case, not “most” |
| Biological overlap / crossing | Sten et al.: F1 0.91–0.92 on six manually annotated *R. irregularis* images, with at least one documented manual false-negative correction | First reproduce the exact six-image result and the assisted-workflow ledger; then target automated F1 ≥0.95 on a larger independently adjudicated set, with 95% CIs and zero false fusion on depth-separated controls |
| 3D topology / abstention | No directly comparable broad fungal 3D topology score is established by these small image sets | Zero false fusion on all known-truth depth-separated / near-touch fixtures; on independent biological data target false-fusion rate <1% while preserving ≥95% true-junction recall, with abstention/coverage shown |
| Global AMF hyphal infrastructure | Stewart et al. Science 2026: 322 studies, >16,000 soil cores, 9 biomes; 1 km² Random Forest density maps and biomass conversion from >300,000 imaged hyphae | Reproduce the complete spatial density/uncertainty/geometry chain; beat RF only on leave-study-out and spatial-blocked tests with calibrated intervals |
| CMN functional effects | Lehmann et al. Functional Ecology 2025: 833 records screened, 31 eligible AMF network studies, 7 CMN-HC; mostly neutral/inconsistent small effects | Reproduce lnRR and multilevel models; improve leave-study-out prediction without biasing toward positive effects; neutral or context-specific outcomes are valid |
| Flow / resource dynamics | Nature 2025 AMF study: time-resolved graph data and measured cytoplasmic-flow trajectories, but only three AMF profiles in a 2D-constrained in-vitro system | Beat topology-only, uniform-flow and shuffled-flow baselines on leave-replicate-out and leave-strain-out predictive scores; freeze a numeric margin only after source data reproduction |
| Hyphal-highway dispersal | Moratto et al. 2026: Pythium–Pseudomonas HHD; only 4/8 co-inoculated devices yielded outlet CFUs | Zero confirmed highway classification without two patch receipts, a separate connected-film bridge receipt, active filament support and a bacterial trajectory; beat hypha-only/liquid-only arrival baselines |
| Host-free AMF flow vs cargo | Klein et al. 2026: ~25,000 flow trajectories; bulk flow speed responds to myristic acid but fluorescent-lipid speed (~3 µm/s) does not | Separate tracer classes with zero silent pooling; reproduce treatment effects by strain/time and predict held-out plates with calibrated intervals |
| Global AMF density / biomass | Stewart et al. 2026: 322 studies, >16,000 soil cores, 9 biomes and >300,000 imaged hyphae; global totals are model extrapolations | Reproduce the full measurement chain, then beat its held-out study/biome baseline while decomposing uncertainty and flagging unsupported biomes |
| Hyphal spectral metabolism | Pucetaite et al. 2026: SIP-Raman/SRS detects isotope uptake and Cu response; repeated spontaneous Raman can damage hyphae | Reproduce labelled-band/growth contrasts, model exposure history and photodamage, and achieve zero cell-death claims from cytochrome-c signal alone |
| C:P exchange / Pareto | Bisot et al. PNAS 2026: morphology/ML-derived C/P exchange estimates from about 100 million hyphal-shape measurements; host genotype shifts the exploration/exploitation front | Reproduce the exact front and source tables; on held-out host genotypes and fungal strains improve front prediction and calibrated flux estimates versus host-blind / constant-ratio baselines |
| Root anatomy / energetic efficiency | Cao et al. Nature Communications 2025: theory couples root radius, ToS geometry, arbuscule position, nutrient return and active-uptake energy | Reproduce equations 1–10 with dimensional checks and limiting cases before extending; quantify energy per nutrient unit by arbuscule layer |
| Field hyphosphere C:N | Brandt et al. New Phytologist 2026: 36 randomized hyphoboxes, 13C pulse labelling and 15N hyphal-compartment supply; 16.9% 15N recovery in this particular experiment | Reproduce the complete treatment/timepoint hierarchy; preserve the reduced 3-day n=3 and separate fungal/bacterial/abiotic pathway evidence |
| CMN-HC evidence qualification | Systematic mapping of 123 studies found 9 meeting all five criteria; continuity testing itself is not proof of continuous cytoplasmic flow | Zero unsupported CMN-HC classifications; require source-linked PASS for each criterion and separate plant-to-plant flow evidence |
| Multiscale simulation | BMX is the mechanistic high-detail comparison; sBARE is a preprint surrogate until reproduced | Target ≥10× lower compute at matched preregistered observable error and uncertainty coverage; retain BMX as the high-fidelity reference rather than replacing it by assertion |
| Uncertainty / evidence | Competitors often report point accuracy or manual agreement; this does not automatically give calibrated uncertainty | Nominal 90% and 95% depth/topology intervals should achieve empirical coverage within ±3 percentage points on held-out scene families; report selective risk vs coverage |
| Deterministic graph correctness | Current Symtropy code has explicit transactional topology and passive-transport primitives | 100% pass on frozen known-truth topology, conservation, replay, invalid-unit and cross-platform determinism cases |

The field-revolution strategy is not to produce a slightly better skeletonization button. It is to join **image evidence → uncertain 3D hypotheses → event-sourced topology → independently measured flow → resource/element accounting → controlled intervention → calibrated model update** without collapsing these into a single opaque confidence score.

The current code branch adds a conservative depth-interval gate and a source-to-graph observation receipt. The latest shared-network evidence layer records CFN/CMN/CMN-HC criteria independently and keeps plant-to-plant flow as a separate claim requiring its own evidence receipt. The graph criticality routine is checked against an independent removal oracle across 1,099 small graphs. The 12-scene analytic known-truth fixture file is benchmarks/mycelium/myco-3d-known-truth-fixtures-v0-1.json; it is synthetic graph geometry, not microscopy imagery. A new C:P/N exchange and CMN evidence benchmark is specified in benchmarks/mycelium/myco-exchange-tradeoff-v0-1.json; all external datasets remain not downloaded and rights-admission checks remain open.

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

### T13 — CFN / CMN / CMN-HC evidence qualification

Use Lehmann & Rillig's five-criterion framework (DOI 10.1002/ppp3.10618) as an explicit evidence ladder, not as a decorative label. The framework's 123-study map found only 9 studies meeting all five criteria. Record each criterion independently with a source-linked observation receipt:
1. at least two plant units;
2. all linked plants AM-colonized;
3. root systems separated to suppress the root-root pathway;
4. a hyphal-linking treatment;
5. hyphal continuity tested.

Four criteria (1–4) are the framework's threshold for CMN effect attribution; all five support a CMN-HC evidence level. An unreported criterion is not a pass. The fifth criterion does not alone establish continuous cytoplasmic flow.

The public Rust module `network_evidence.rs` encodes the five criteria as PASS / FAIL / UNREPORTED / NOT_APPLICABLE, requires source-linked receipt IDs for PASS/FAIL, and returns separate network-claim and flow-observation levels. Its receipt resolver requires every referenced observation ID to map to one unique, valid ObservationEvidenceReceipt; duplicate, missing or invalid receipts fail closed. A flow claim is deliberately scoped to **a cytoplasmic-flow stream observed on a segment**. It does not claim plant-to-plant transport. That stronger claim requires separately evidenced source and recipient plant-interface identities, a verified continuous path through the canonical graph, and compatible flow observations along that path. Nineteen tests cover criterion completeness/order, CFN vs AMF classification, root/hyphal control failures, missing evidence, unique receipt resolution, flow-receipt subject typing and conservative claim boundaries. Implementation is on [PR #5388](https://github.com/Luminous-Dynamics/symtropy/pull/5388); the benchmark contract is `benchmarks/mycelium/myco-exchange-tradeoff-v0-1.json`.

### T14 — host-conditioned C:P/N exchange and root-anatomy trade-offs

Add [MYCO-EXCHANGE-TRADEOFF-001 #5521](https://github.com/Luminous-Dynamics/symtropy/issues/5521) as a new benchmark lane. It joins three complementary sources without pooling their evidential basis:
- **PNAS 2026 C:P exchange:** Bisot et al. (DOI 10.1073/pnas.2512182123) used robotics, machine learning and about 100 million hyphal-shape measurements to estimate reciprocal C/P fluxes. The reported C:P ratio was comparatively stable across tested fungal strains within host context, but strongly host-genotype-dependent; host swaps shifted the exploration/exploitation Pareto front. The [PC_2025 replication repository](https://github.com/Cocopyth/PC_2025) describes coarse-grained data tables, a simulator and ML training materials. Exact revisions and code/data rights remain to be admitted. Its flux estimates must remain labeled morphology/ML-derived unless independently tracer-grounded.
- **Root anatomy theory:** Cao et al. (DOI 10.1038/s41467-025-64553-1) derive equations linking root radius, stele/ToS geometry, arbuscule position, nutrient delivery and energetic cost. Reproduce equations 1–10 as a distinct theoretical comparator; do not present it as a fitted or field-validated root-fungal solver.
- **Field hyphosphere C:N:** Brandt et al. (DOI 10.1111/nph.71576) used 36 hyphoboxes, three label treatments, three destructive timepoints and four nominal replicates. The 3-day 15N replicate count fell to n=3 after a damaged box was replaced. The paper reports 16.9% recovery of added 15N in plants in that experiment, and 13C transfer to fungal then bacterial biomarkers. Other raw amplicon data are in SRA PRJNA1415267 and data in BExIS 32469 (DOI 10.71615/bexis.32469). Preserve the missing replicate and distinguish isotope recovery from source attribution and actual flux.

The central new benchmark is an exchange surface conditioned on host genotype, root anatomy, fungal strain, network morphology, nutrient availability and microbial context. Report Pareto front shape and uncertainty, flux source/evidence class, and worst-stratum predictions. The target is not one universal C:P coefficient: it is to predict when host genotype or fungal phenotype shifts the feasible exploration/exploitation surface, and to identify what measurement would distinguish fungal transport from soil diffusion, root leakage, bacterial mediation or abiotic tracer movement.

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

## 11. New research extensions — 16 November 2026

### T19 — cross-modal experiment clock alignment

A plant–fungus experiment often combines microscopy, flow video, Raman/SRS, isotope imaging, electrical sensing and possibly PET/fluorescent-particle measurements. Their timestamps must not be treated as one clock simply because they are stored in the same dataset.

The implementation in PR #5388 adds `ExperimentTimeAligner` and a source-receipted `ClockCalibration`. Each non-reference clock needs an explicit synchronization observation; the transform is valid only over its declared source-time interval; anchor, rate and timestamp uncertainty are propagated using conservative absolute bounds. Temporal order is asserted only when two bounded intervals are disjoint. Overlapping or touching intervals return `IndeterminateOverlap`. Calibration-chain composition is intentionally unsupported in v0.1.

The benchmark is [MYCO-TEMPORAL-ALIGN-QUAL-001 #5631](https://github.com/Luminous-Dynamics/symtropy/issues/5631), with 14 synthetic known-truth cases in `benchmarks/mycelium/temporal-alignment-v0-1.json`. Its primary safety metric is false precedence, not timestamp MAE alone. A temporal sequence is not causality; causal inference still requires interventions and mechanism-specific evidence.

### T20 — overlapping plant–fungus interface checkpoints

The 2026 checkpoint framework distinguishes invitation, admission, accommodation, investment, maintenance, and termination/renewal. These are not six globally ordered states. Multiple interfaces within one root system can be at different checkpoints simultaneously, and fungal regulatory inputs must remain represented alongside plant regulation.

The new `symbiosis_checkpoints` module registers a plant/fungus interface identity, links each checkpoint observation to a typed source receipt, allows overlapping/repeated checkpoint events, and keeps chronological display order separate from temporal precedence. A separate performance-evaluation evidence chain requires isotope/tracer resource contribution, host nutrient response, subsequent fate of that same interface, and matched control. It returns only sequence-consistent, contradicted, or temporally indeterminate status; even a complete sequence explicitly does **not** establish causality, sanction, partner choice or intention.

Tracked in [MYCO-SYMBIOSIS-CHECKPOINT-QUAL-001 #5638](https://github.com/Luminous-Dynamics/symtropy/issues/5638), with the six-checkpoint evaluation contract in `benchmarks/mycelium/symbiosis-checkpoints-v0-1.json`. Local arbuscule termination must never be conflated with removal of a fungal individual/genotype. Gene expression, colonization abundance or degeneration alone are not resource-flux measurements.

### T21 — global AMF infrastructure measurement chain

The 2026 global AMF hyphal-density study provides a large empirical infrastructure benchmark: 322 studies, more than 16,000 soil cores across nine biomes, machine-learning spatial predictions, and robotic imaging used to calibrate geometry-to-biomass conversion. Its reported global length and biomass totals are model-based extrapolations, not a direct global census.

A new end-to-end reproduction contract is [AMF-GLOBAL-INFRASTRUCTURE-REPRO-001 #5639](https://github.com/Luminous-Dynamics/symtropy/issues/5639), with `benchmarks/mycelium/amf-global-infrastructure-v0-1.json`. The work is deliberately decomposed into study/core admission, unit harmonization, study- and spatial-blocked density modelling, leave-biome-out transfer, uncertainty calibration, effective-radius geometry, dry-biomass conversion, and active sampling. Hyphal length density, geometric volume fraction, dry biomass, carbon stock, nutrient flux and plant benefit remain separate outputs.

### Current code boundary and validation

The current PR adds the temporal alignment and checkpoint modules to the existing graph, transport, BMX reference, source-receipt, spatial-field, spectroscopy and hyphal-highway code. It also adds an explicit `cargo test --locked -p symtropy-mycelium --lib` step to CI; this corrects an earlier test-coverage gap where the workflow checked the workspace but did not run this crate's unit tests.

The first explicit Mycelium test run compiled the crate and executed 177 tests; 176 passed and one timeline-order expectation failed. The implementation's deterministic center-time display order placed the 10.0-second item before the 10.5-second item, while the test expected the reverse. The correction now asserts that display order and independently checks that the overlapping uncertainty intervals remain `IndeterminateOverlap`. This correction awaits its exact-head rerun. No green current-head CI, external source reproduction, data-rights admission, biological validation or SOTA superiority is claimed.

## 11. Claim ceiling

This document is a research/implementation contract. It does not establish biological realism, measured field transfer, common-network function in arbitrary ecosystems, empirical forecast skill, SOTA superiority, or consciousness in plants/fungi or Symthaea.