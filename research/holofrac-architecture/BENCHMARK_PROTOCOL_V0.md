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
- exact typed-edge lookup;
- canonical key/value or adjacency-list lookup;
- HRR/FHRR/MAP/BSC/TPR;
- GHRR after independently specified implementation;
- untyped or direction-ablated HDC as negative controls.

Metrics: top-1/top-k accuracy, mean reciprocal rank, relation precision/recall/F1, false match rate under reversed edges, query latency, index memory and update cost.

### H2 — Partial graph completion and context retrieval

Mask 10%, 25%, 50% and 75% of selected non-critical nodes/relations using a published, deterministic masking schedule. Ask the system to retrieve candidate missing relations or relevant parent/neighbor context.

Keep ground truth hidden from the retrieval step. Report exact-edge recovery, graph edit distance, calibration, abstention quality and false completion rate. Do not reward plausible but unsupported invention.

Split by complete building/project, not randomly by adjacent rooms or repeated variants from one source model.

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

## 3. Representation and optimizer baselines

### Representation baselines

1. Exact typed graph traversal (authoritative correctness reference).
2. Canonical serialized lookup / relational database query.
3. HRR.
4. FHRR.
5. MAP.
6. BSC.
7. TPR.
8. GHRR, only after implementation and independent test fixtures.
9. Graph embedding or graph-neural baseline where training data and compute permit.

Use identical graph snapshots, queries, held-out splits, dimension budgets where meaningful and hardware. Report each representation's actual memory footprint; equal dimensions do not imply equal memory or compute.

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

- Performance-driven generative design review (153 studies; documents interoperability, scale, validation and reproducibility gaps): https://doi.org/10.3390/buildings15244556
- 2025 comparison of model-based optimization and metaheuristics on an expensive building-performance problem; reports stronger robustness and hypervolume for model-based optimization in that case: https://doi.org/10.1016/j.enbuild.2025.115562
- 20 Oct 2026 stage-adaptive, uncertainty-informed building design optimization paper; reports 93.8% fewer simulations than exhaustive evaluation in its study. Treat as a task-specific comparator, not a universal target: https://doi.org/10.1016/j.enbuild.2026.118417
- 1 Nov 2026 review of fast building simulation (765 studies); emphasizes application-specific end-to-end latency and uncertainty-aware validation: https://doi.org/10.1016/j.enbuild.2026.118152
- MSD floor-plan dataset and paper: https://arxiv.org/abs/2407.10121 and https://archilyse.standfest.science/modified-swiss-dwellings
- IFC 4.3 / ISO 16739 and buildingSMART IDS: https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/ and https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/
- ASHRAE Standard 140 resources: https://data.ashrae.org/standard140/index.html
- BOPTEST building-operations benchmark: https://ibpsa.github.io/project1-boptest/
- Autodesk Fusion Generative Design: https://help.autodesk.com/view/fusion360/ENU/?contextId=GD-OVERVIEW
- nTop simulation capabilities: https://www.ntop.com/software/capabilities/simulation/
- GHRR DATE 2026 paper: https://doi.org/10.23919/DATE69613.2026.11539261

## 10. First implementation sequence

1. Build a typed spatial-graph fixture and an exact graph oracle in Rust; include directed-edge and reversed-edge negative controls.
2. Add an HDC adapter using existing symtropy-hdc-core without changing its semantics; persist encoder fingerprint and exact source IDs.
3. Add separate HRR/FHRR/MAP/BSC/TPR adapters; add GHRR only as an independently specified research implementation.
4. Publish H0-H3 raw results and hold-out manifests before integrating generation.
5. Connect a minimal IFC 4.3/IDS exchange fixture and MSD task adapter.
6. Connect Symtropy physical evaluation and FFMFE structural/assembly tests without allowing representation changes to mutate authoritative physics inputs.
7. Add building-performance simulation, uncertainty, measured calibration and occupant evaluation only when independently reproducible inputs are available.

The first deliverable is a tested retrieval benchmark—not an autonomous building designer.
