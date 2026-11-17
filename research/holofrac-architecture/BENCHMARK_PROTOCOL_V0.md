# HoloFrac Architecture Benchmark Protocol v0

**Status:** pre-registration draft; freeze before candidate ranking  
**Date:** 2026-11-17  
**Purpose:** benchmark multiscale architectural representations and their connection to performance-driven design without confusing associative retrieval, geometric generation and physical qualification.

## 1. Hypotheses

H1. A typed multiscale spatial graph plus an HDC/VSA associative index can improve partial-context retrieval and analogy over exact graph/metadata baselines at an acceptable memory and latency cost.

H2. Role-aware, direction-preserving encodings improve retrieval of directed spatial and service relations over untyped or commutative-only binding.

H3. A shared multiscale grammar can transfer useful design patterns between components, rooms, buildings and structural assemblies without violating held-out hard constraints.

H4. Active, uncertainty-aware selection of expensive simulations can reduce evaluation cost while preserving or improving feasible Pareto quality.

H5. Co-designing module/interface vocabulary, assembly and service access can improve lifecycle-level outcomes relative to geometry-only optimization under matched problem definitions.

These are hypotheses, not achieved results. H1-H5 are independently falsifiable; a failure of one does not validate another.

## 2. Benchmark tasks

### H0 — Encoder and source-integrity contract

Input: versioned typed graph plus exact source snapshot.

Checks:
- same graph, schema, seed and encoder configuration produce byte-identical deterministic outputs where promised;
- encoder fingerprint changes when any semantic encoding parameter changes;
- dimensions, units, coordinate frames and relation types are validated;
- exact graph digest differs when authoritative source state differs, even when an invariant HDC representation is intentionally identical;
- vector similarity is never used as an integrity or authorization primitive.

Report: deterministic replay rate, schema rejection rate, invalid-input handling, serialization size, encoding time and exact source/vector linkage.

### H1 — Directed relation retrieval

Queries include containment, adjacency, access, support, supply/drain, shading, ventilation and maintenance relationships. Include reversed-edge negative controls (for example, room A supplies room B versus room B supplies room A).

Baselines:
- exact typed-edge lookup and a source-keyed inverted postings index;
- canonical key/value and adjacency-list lookup;
- graph embedding / ANN retrieval where the same partial-query semantics can be implemented;
- HRR/FHRR/MAP/BSC/TPR;
- GHRR after independently specified non-commutative implementation and its flattened inference treatment;
- untyped, direction-ablated and random-vector HDC as negative controls.

The exact postings index is the primary baseline whenever query fields are exact source identifiers. HDC should not be expected to beat it on fully specified or exact-key queries. The HDC research question is whether it adds robust recall for noisy cues, aliases, cross-scale analogies, incomplete semantic descriptions or generalization beyond observed identifiers.

Metrics: top-1/top-k accuracy, MRR, relation precision/recall/F1, reversed-edge false match rate, abstention, query/build/update latency, full index memory including postings/codebook metadata, and energy where measurable. Report separate exact-oracle, exact-postings, slot-HDC and holistic-HDC results.

### H2 — Partial graph completion and context retrieval

Mask 10%, 25%, 50% and 75% of selected non-critical nodes/relations using a published, deterministic masking schedule. Ask the system to retrieve candidate missing relations or relevant parent/neighbor context.

Keep ground truth hidden from the retrieval step. Use distinct query regimes: exact identifiers partially masked; aliases/synonyms; noisy or corrupted labels; natural-language descriptions mapped through a declared parser; and truly novel/unanswerable contexts. Report exact-edge recovery, graph edit distance, calibration, abstention quality and false completion rate. Do not reward plausible but unsupported invention.

For each regime compare the source-graph oracle, exact role-postings intersection, adjacency-list traversal, HDC role-slot ranking and holistic partial-bundle ranking. If using learned text or graph embeddings, include their training corpus, leakage controls and ANN index cost. A hybrid may use approximate retrieval for recall and exact source verification for accepted facts; report both pre-verification suggestions and verified outcomes.

Split by complete building/project, not randomly by adjacent rooms or repeated variants from one source model. Hold out terminology, topology families, relation combinations and at least one complete project family.

### H3 — Cross-scale and held-out-family transfer

Train/index on selected spatial levels and test on held-out combinations of:
- building type;
- room/function family;
- topology family;
- connector/interface family;
- spatial scale;
- climate/operating regime;
- service-network layout.

Report performance by held-out family and scale. Include an exact graph oracle and simple structural/metadata baselines.

### H4 — Floor-plan understanding and generation

Use Modified Swiss Dwellings (MSD) as the first building-complex-scale public dataset. Preserve its released train/validation/test split where available; otherwise create a project-level split and document the exact version and filtering. The dataset reports 5,372 plans, 18.9K+ units and 165.3K+ areas, with geometry and access-graph modalities.

Tasks:
- functional-graph-conditioned completion;
- room/zone relation retrieval;
- circulation and access graph validity;
- constrained generation of candidate layouts.

Metrics:
- hard constraint satisfaction and invalid-layout rate;
- connectivity and accessible-route coverage;
- room programme/area error;
- boundary and overlap validity;
- graph relation F1 and graph edit distance;
- diversity among feasible solutions;
- daylight/thermal/egress results only when evaluated by declared physical or rules-based tools.

Do not use raster similarity alone as architectural quality. Do not claim that matching a dataset's historical design proves code compliance or occupant suitability.

### H5 — Fractal and multiscale geometry qualification

For each candidate, report the exact geometric observable and scale range. Candidate observables include box-counting dimension, mass-radius scaling, surface/volume scaling, branch-length distributions and scale-dependent connectivity.

Required controls:
- Euclidean grids and conventional trusses;
- non-fractal branching graphs;
- random/geometric null structures with matched size/density;
- recursive structures with deliberately broken scale similarity.

Report fit interval, residuals, sensitivity to resolution, confidence intervals, finite-size limitations and estimator choice. A recursively generated hierarchy is classified as recursive/hierarchical unless statistical scaling evidence supports a fractal descriptor.

### H6 — Structural/interface integration

Reuse FFMFE protocol v1 tasks B0-B5; do not silently redefine their geometry, supports, loads, mass boundary or solver level. Add the HDC layer only as a representation/retrieval treatment. It must not change the exact structure being evaluated.

Extend the later phases to:
- connector and joint identity retrieval;
- interface-family generalization;
- assembly-order and tool-access queries;
- inspection and service-access queries;
- damage/repair history retrieval.

Structural outputs retain the FFMFE evidence ceiling. HDC does not upgrade an L0-L3 result.

### H7 — Building performance loop

Freeze a small courtyard-dwelling/pavilion reference project with:
- climate and site assumptions;
- spatial programme, occupancy and schedules;
- envelope, glazing, shading and material definitions;
- HVAC/ventilation and water system boundaries;
- access, egress, privacy and maintenance requirements.

Evaluate declared candidate sets for:
- annual and peak energy;
- overheating/thermal comfort;
- daylight availability and glare;
- ventilation/indoor-air metrics;
- water demand, collection and reuse;
- embodied and operational carbon under a stated boundary;
- construction/maintenance quantities and cost assumptions.

Use ASHRAE Standard 140 test cases when evaluating the building-simulation engine's applicable capabilities. This is engine testing, not whole-project validation. Use independent solvers where formulations overlap and measured data for later calibration. BOPTEST is a useful operational-control benchmark, not a substitute for a design-generation benchmark.

### H8 — Continuous spatial fields and reconstruction

The typed-graph track (H0-H3) evaluates discrete entities and relations. It does not establish that a VSA can represent or reconstruct continuous spatial fields. Evaluate this as a separate pipeline, using identical field samples and query locations across:

- HRR and FHRR;
- MAP/BSC/TPR where their continuous encoders are well-defined;
- GHRR only as a separately specified spatial-field composition treatment;
- exact grid/voxel storage and nearest-neighbour, bilinear/trilinear interpolation;
- RBF or kernel interpolation;
- a declared neural implicit-field baseline where training data and compute permit.

Use 2D and 3D fields derived from analytic functions, public spatial data, and later measured building fields (for example temperature, illuminance, airflow, occupancy density or moisture). Freeze the function family, spatial bounds, units, sampling density and held-out coordinates. Include smooth, discontinuous, high-frequency, sparse, anisotropic and boundary-heavy fields; do not let a single smooth sinusoid determine the conclusion.

A VSA pipeline must be measured end to end, not as a binding-only microbenchmark. Record separate costs for coordinate encoding, value encoding, binding, bundling/storage, inversion/unbinding, similarity, cleanup and regression/decoding. Compare at matched field samples, held-out query points, precision, actual bytes and compute budget. Include the cleanup mechanism as a first-class ablation: resonator, modern Hopfield, codebook/nearest-neighbour cleanup, or no cleanup when applicable. Report both the best pipeline and every component configuration used to obtain it.

Metrics:
- held-out MAE, RMSE, R² and normalized error;
- field-specific conservation/invariant error where a physical law applies;
- spatial gradient and boundary-condition error, not only pointwise error;
- reconstruction error versus sample density, dimension, noise and number of bundled samples;
- query p50/p95 latency, throughput, build time, actual retained memory and energy;
- degradation outside the sampled coordinate set and under distribution shift.

Exact grid interpolation is a correctness/reference arm, not an assumed competitor that a lossy representation must beat at every point. HDC earns a role when it offers a better memory/latency/generalization trade-off or enables compositional field retrieval under the same explicit error budget. It must never be used as the authoritative solver field.

## 3. Representation and optimizer baselines

### Representation baselines

For discrete graph retrieval:
1. Exact typed graph traversal and source-keyed role postings (authoritative exact-answer reference).
2. Canonical serialized lookup / relational database query.
3. HRR.
4. FHRR.
5. MAP.
6. BSC.
7. TPR.
8. GHRR, only after implementation and independent test fixtures.
9. Graph embedding or graph-neural baseline where training data and compute permit.

For continuous spatial fields (H8), add exact grid/voxel interpolation, nearest-neighbour, RBF/kernel interpolation, and HyperSpace-style modular HRR/FHRR pipelines with explicit inversion, cleanup and regression choices.

Use identical graph snapshots or field samples, queries, held-out splits, dimension budgets where meaningful and hardware. Report every stage's cost and each representation's actual memory footprint; equal nominal dimensions do not imply equal memory or compute. The April 2026 HyperSpace study specifically finds that similarity/cleanup and regression can dominate spatial-pipeline runtime, and that HRR/FHRR isolated operator complexity does not predict their end-to-end latency. Its results make stage-level profiling and decoder/cleanup ablation mandatory, not optional: https://arxiv.org/abs/2604.15113.

### Generation and optimization baselines

- human-authored reference candidate;
- parametric rule/grammar enumeration;
- constrained graph rewriting/search;
- NSGA-II/III or another declared evolutionary method;
- Bayesian/model-based optimization;
- surrogate-assisted active learning;
- process-aware topology optimization where the problem admits it;
- HDC-assisted candidate retrieval/initialization, ablated from the same optimizer.

HDC is not presumed to be an optimizer. First establish whether it helps retrieve relevant design memory, compose constraints or warm-start search. Compare model-based optimization against evolutionary and surrogate methods; do not assume NSGA-II is the strongest baseline.

## 4. Fair comparison contract

Freeze before runs:
- source dataset and checksum;
- complete project/building split;
- graph schema, relation vocabulary and encoder schema;
- geometry, units, coordinate frames and model versions;
- climate/weather, occupancy, schedules and operating assumptions;
- material and manufacturing assumptions;
- supports, load cases and mass-accounting boundary for structural tests;
- solver formulation, mesh/element resolution, convergence and numerical tolerances;
- search budget, simulation budget, hardware, seeds and stopping rules;
- objective definitions, constraint priority and normalization;
- uncertainty distributions and missing-data treatment.

Do not select the benchmark after inspecting candidate performance. Publish every failed, infeasible and out-of-domain candidate. No candidate-specific relaxation is permitted.

## 5. Primary metrics

### HDC and graph reasoning
- top-1/top-k, MRR, relation precision/recall/F1;
- partial-graph edge recovery and graph edit distance;
- reversed-edge false-match rate;
- calibration and abstention risk;
- latency, throughput, peak memory and energy per query;
- update/re-encoding cost and scale-transfer degradation.

### Continuous spatial-field reconstruction
- held-out MAE/RMSE/R² and normalized field error;
- gradient, boundary and applicable conservation/invariant errors;
- error-versus-sample-density, vector-dimension and noise curves;
- per-stage and end-to-end latency/throughput;
- decoder/cleanup compute, vector and codebook memory, and actual process RSS.

### Geometry and architectural feasibility
- invalid geometry and topology rate;
- hard spatial/program/access/egress constraint satisfaction;
- usable-area and circulation efficiency, with definitions;
- feasible solution diversity and Pareto spread;
- manufacturability and assembly feasibility;
- occupant task success, wayfinding and preference/agency measures where user studies exist.

### Physical performance
- structural displacement/compliance and applicable stress/stability metrics;
- EUI, peak loads, overheating hours, daylight and glare;
- ventilation/indoor-air metrics;
- water and carbon measures;
- lifecycle cost, maintenance access and repair time.

### Optimization and evidence quality
- feasible hypervolume and IGD+ with a frozen reference/normalization;
- high-fidelity simulation count and wall-clock time;
- surrogate calibration and out-of-domain detection;
- solver/constraint failure rate;
- independent solver discrepancy;
- evidence completeness and exact replay rate.

Do not collapse these into one composite score without preregistered stakeholder weights and sensitivity analysis.

## 6. Internal improvement targets (not results)

After reproducing every baseline, target one or more of the following on held-out tasks:
- at least +5 percentage points in relation-retrieval F1 at the same memory budget; or equal F1 within 1 percentage point with at least 2x lower median query latency;
- at least 20% fewer high-fidelity simulations than the strongest matched model-based/surrogate baseline at non-inferior feasible hypervolume, with calibrated uncertainty and no hard-constraint regression;
- no reduction in hard spatial/access/egress feasibility to obtain energy, mass or cost improvements;
- 100% of ranked candidates linked to exact input snapshots, solver configurations, failed-run records and replayable receipts.

These are go/no-go aspirations for the research programme, not claims about current performance. Report confidence intervals across seeds and tasks; do not claim a win from one fixture.

## 7. Evidence and release ladder

| Level | Evidence | Permitted claim |
|---|---|---|
| E0 | schema/type and deterministic unit fixtures | representation contract works on fixtures |
| E1 | held-out synthetic graph benchmarks | measured retrieval/generalization on those graphs |
| E2 | public MSD and frozen architectural testbed | measured plan/task results within the tested scope |
| E3 | independent simulation agreement and engine tests | qualified model results under declared assumptions |
| E4 | physical coupons, prototypes, metrology and independent tests | evidence limited to tested configurations/material/process |
| E5 | monitored occupied building and post-occupancy evaluation | measured operational and human outcomes within the deployment context |

A pass at one level does not imply a pass at a higher level. Engineering code compliance, structural approval and occupancy permission remain separate professional processes.

## 8. Reproducibility receipt

Every run must store:
- Git repository, exact commit SHA and dirty-tree state;
- Rust/compiler/tool versions, target, CPU/GPU and features;
- dataset versions, licenses, source IDs and checksums;
- graph and encoder schema versions plus encoder fingerprint;
- configuration, units, seed and split manifest;
- solver versions, settings and boundary conditions;
- raw query/candidate outputs, including failures;
- metrics with confidence intervals;
- exact hashes for input/output artifacts;
- known limitations and maximum claim level.

Use cryptographic digests for artifact integrity. Non-cryptographic HDC fingerprints or regression hashes are not signatures and must not be represented as tamper evidence.

## 9. External references and competitive baselines

### Strong optimization baselines

Do not use NSGA-II as the sole or presumed strongest comparator for expensive building-performance optimization.

- Zorn et al. (1 June 2025), *Optimizing an expensive multi-objective building performance problem*, compared RBFMOpt and TPE against NSGA-II, NSGA-III, MHACO, MOEA/D and related baselines. RBFMOpt and TPE had stronger robustness and achieved hypervolume in that case; RBFMOpt reached useful solutions in fewer than 100 expensive evaluations. Reproduce the accessible benchmark and compare on the same candidate/evaluation budget before generalizing: https://doi.org/10.1016/j.enbuild.2025.115562
- Talami, Costa & Stouffs (online 20 October 2026), *A fast, interpretable and uncertainty-informed design-support framework for multi-objective building performance optimization*, combines physics simulation, RBF surrogate-assisted search, uncertainty quantification and bipartite-modularity clustering. Its Nottingham office case reports 93.8% fewer simulations than exhaustive search with its own performance/uncertainty settings. This is a strong integration comparator, not a universal performance guarantee: https://doi.org/10.1016/j.enbuild.2026.118417
- Da Silva et al. (issue published March 2026), *Multi-objective optimization algorithms for building performance assessment – A benchmark*, compares seven optimizers across nine thermal/energy/daylight problems. RBFMOpt and HypE performed strongly across the tested problem set; RBFMOpt tended to require fewer simulations. Include HypE and RBFMOpt among the strong reference arms rather than treating NSGA-II as the sole SOTA comparator: https://doi.org/10.1177/14780771241296263
- Mandatory search arms for HoloFrac's later design loop: RBFMOpt; TPE; NSGA-II/III; MOEA/D where appropriate; surrogate-assisted NSGA-III; stage-adaptive RBF + uncertainty + graph clustering; and high-fidelity exhaustive evaluation on tractable fixtures. Tune each under the same development budget and keep the held-out evaluation set untouched.
- A 2026 stage-adaptive study's LLM-generated stakeholder personas are simulated decision agents, not evidence of real resident participation. Measure human agency separately with actual consented participants; reuse the Sol-Atlas resident preference/decision receipts rather than inventing a scalar preference model.

### Industry capability baselines (checked 17 November 2026)

The product comparison owner is Sol-Atlas [HIGHVELD-KEEP-INDUSTRY-001 #617](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/617); HoloFrac should provide a measured component for that matched comparison, not create a competing feature-count survey.

- **Autodesk Forma Building Design:** the 4 November 2026 update documents Indoor Daylight (public beta), parking layout with accessible/EV markings and aisle dimensions, furniture in plan/3D, DXF import and DWG export. Compare on a matched schematic building task and verify exact edition/region/access at test time: https://blogs.autodesk.com/forma/2026/11/04/whats-new-in-forma-building-design-indoor-daylight-parking-furniture-and-autocad-workflows/
- **Autodesk Forma connected AEC cloud:** its 15 September 2026 product strategy connects Civil 3D, Forma and Revit, expands shared project context, site and subsurface workflows, and describes AI-assisted drawing search/change analysis and multifamily layout exploration against unit-mix goals. Treat the connected Forma/Revit/Civil3D toolchain—not a single isolated feature—as the serious workflow baseline: https://adsknews.autodesk.com/en/news/autodesk-forma-ai-aec-connected-workflows-2026/
- **TestFit Generative Design:** its documented 5.2 update adds zoning-data ingestion and unit/parking mix goals to its solve workflow. Compare actual zoning-data availability and candidate outputs for the same jurisdiction/site: https://www.testfit.io/blog/testfit-5-2-better-data-brings-better-generative-design
- **nTop:** nTop 5.23 expanded simulation to CFD on implicit bodies without body-fitted volume meshes, alongside structural and other multiphysics workflows. HoloFrac must not claim to replace its implicit geometry or CFD engines: https://support.ntop.com/hc/en-us/articles/39936980909715-nTop-CFD-and-Simulation-Updates
- **Autodesk Fusion Generative Design:** a relevant manufacturing-constrained engineering baseline for components, not a full architectural workflow: https://help.autodesk.com/view/fusion360/ENU/?contextId=GD-OVERVIEW

### What HoloFrac should attempt to outperform

The proposed advantage is not raw CAD, daylight, CFD, FEA, zoning, drawing search, generic project-change highlighting or floor-plan generation: Autodesk Forma is actively investing in connected project context, AI drawing search/change analysis and goal-based multifamily layouts. HoloFrac must demonstrate something more specific in matched tasks: machine-checkable source lineage, exact dependency closure, independent evidence invalidation/replay, and measurable reductions in stale analyses or engineering rework across heterogeneous toolchains. Whether commercial platforms already satisfy parts of this requirement must be assessed empirically, not assumed.

The testable hypothesis is a reduction in cross-tool information loss and rework through:

1. typed multiscale spatial memory with source IDs and relation direction preserved;
2. intent-to-design lineage and hard/soft requirement traceability;
3. fast retrieval of affected rooms, services, structural interfaces and analyses after a change;
4. reliable evidence-currentness/invalidation, with zero false-current decisions on seeded changes;
5. uncertainty-aware selection of the next high-fidelity simulation;
6. multiple feasible, human-steerable alternatives without forcing incomparable preferences into a single score.

Primary comparative measures should include time to first feasible candidate, design/task requirement coverage, time to update after a change, affected-analysis recall/precision, stale-analysis detection, hard-constraint false-PASS, IFC/IDS and geometry round-trip loss, solver calls, human interventions, compute energy and total workflow cost. Keep each measure separate and disclose unavailable product capability as unassessed/not-comparable rather than absent.

### Research and standards references

- Performance-driven generative design review (153 studies; fragmented metrics, interoperability, validation and reproducibility): https://doi.org/10.3390/buildings15244556
- Fast building simulation review (765 studies; calls for application-specific end-to-end latency and uncertainty-aware validation): https://doi.org/10.1016/j.enbuild.2026.118152
- Snyder et al. (16 April 2026), *HyperSpace: A Generalized Framework for Spatial Encoding in Hyperdimensional Representations*: modular continuous VSA pipeline and HRR/FHRR end-to-end comparisons, including similarity/cleanup/regression and memory trade-offs: https://arxiv.org/abs/2604.15113
- MSD floor-plan dataset and paper: https://arxiv.org/abs/2407.10121 and https://archilyse.standfest.science/modified-swiss-dwellings
- IFC 4.3 / ISO 16739 and buildingSMART IDS: https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/ and https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/
- ASHRAE Standard 140 resources: https://data.ashrae.org/standard140/index.html
- BOPTEST building-operations benchmark: https://ibpsa.github.io/project1-boptest/
- Yeung, Zou & Imani (2024), *Generalized Holographic Reduced Representations*: non-commutative matrix binding, compositional decoding and capacity baselines: https://arxiv.org/abs/2405.09689
- Chung et al. (arXiv version 12 November 2026; ISLPED '26), *Vector-Space Projection and Unified Complex-Valued Acceleration for Scaling Matrix-Based Brain-Inspired Representations*: GHRR flattening equivalence and an ASIC/software comparison. The paper's flattened effective dimension is D*m^2; its RTX 3060 Ti sequence experiment, RTX 4090 PyTorch comparison and 28 nm ASIC results must be reported as distinct experimental settings: https://arxiv.org/abs/2611.16363 and https://doi.org/10.1145/3816440.3818598
- Naamrudi & Aghamolaei (1 November 2026), *A BIM-based adaptive surrogate framework for efficient uncertainty quantification in building energy simulation*: Sparse Polynomial Chaos Expansion with sequential sampling, multi-output/multi-timescale sensitivity analysis, and archetype-specific uncertainty rankings. Include adaptive PCE as a comparison arm alongside RBF surrogates and TPE; measure calibration and simulation calls as well as point prediction error: https://doi.org/10.1016/j.enbuild.2026.118098
- Chung et al., DATE 2026 GHRR accelerator paper: https://doi.org/10.23919/DATE69613.2026.11539261

### Required GHRR comparison contract

The November 2026 flattened-GHRR paper derives an exact inference similarity representation in effective complex dimension D*m^2; its synthetic RTX 3060 Ti sequence experiment, software comparisons and 28 nm ASIC results are different experimental settings. The hardware paper reports D=8K, m=2, P=8 for one configuration and additional large-D sweeps. We must not describe the flattened representation as O(D) storage or compare ASIC efficiency directly with laptop CPU/GPU software.

A nominal hyperdimension D is not a fair comparison between FHRR and matrix-valued GHRR. For every GHRR run record:
- matrix block count D and matrix width m;
- effective flattened dimension D*m^2;
- complex precision, component layout and actual encoded bytes;
- arithmetic operations, memory traffic, binding/unbinding work and normalization;
- whether the result is matrix-native, software-flattened or hardware-accelerated.

Compare at least matrix-native GHRR, flattened GHRR and current FHRR/bipolar baselines under both matched effective-dimension and matched-byte budgets. Report the trade-off rather than selecting whichever normalization favours one method. Test sequence order, reversed edges, nested trees, graph depth, decoding accuracy, capacity under superposition, noise/corruption tolerance, bind/unbind error, query latency, throughput, peak memory and energy. Do not import ASIC throughput or energy figures as expected software performance.

## 10. First implementation sequence

1. Build a typed spatial-graph fixture, exact graph oracle, exact role-postings baseline and directed/reversed negative controls in Rust.
2. Add HDC adapters using existing symtropy-hdc-core; persist encoder specification, exact source IDs and codebook/index statistics.
3. Publish H0-H2 raw results comparing exact postings, slot-HDC and holistic-HDC under exact, masked, noisy and held-out query regimes.
4. Add separate HRR/FHRR/MAP/BSC/TPR adapters; add GHRR only as an independently specified, dimension/byte-matched research implementation.
5. Publish held-out manifests and raw artifacts before connecting architectural generation.
5. Connect a minimal IFC 4.3/IDS exchange fixture and MSD task adapter.
6. Connect Symtropy physical evaluation and FFMFE structural/assembly tests without allowing representation changes to mutate authoritative physics inputs.
7. Add building-performance simulation, uncertainty, measured calibration and occupant evaluation only when independently reproducible inputs are available.

The first deliverable is a tested retrieval benchmark—not an autonomous building designer.
