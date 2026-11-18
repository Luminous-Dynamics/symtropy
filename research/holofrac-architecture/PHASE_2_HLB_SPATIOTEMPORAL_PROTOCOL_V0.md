# HoloFrac Phase 2: HLB Spatiotemporal World Model Protocol v0

**Status:** research protocol draft; no qualified capability or SOTA claim  
**Date:** 2026-11-17  
**Parent:** [HoloFrac architecture PR #5844](https://github.com/Luminous-Dynamics/symtropy/pull/5844)  
**Symthaea owner:** [#12519 — multiscale structural grammars](https://github.com/Luminous-Dynamics/symthaea/issues/12519)

## 1. Purpose and thesis

Phase 1 established executable research slices for typed spatial retrieval, factorized memory, continuous fields, shared multi-channel fields, geometric scaling diagnostics and exact dependency closure.

Phase 2 asks whether Symthaea's Holographic Liquid Brain (HLB) can add a measurable capability that static spatial representations cannot provide:

**A persistent, multiscale, uncertainty-aware spatial world model that updates from observations, predicts how spaces and systems evolve, retrieves structurally relevant history, and selects the next experiment whose result is most likely to change a design decision.**

HLB should be used more, but it should not become the source of truth for everything. Its HDC/VSA, liquid temporal dynamics (LTC/CfC), predictive processing and active-inference mechanisms should operate as a cognitive/predictive layer over exact, versioned state. Conventional geometry, exact graph algorithms, deterministic constraints and physics solvers remain authoritative for their respective claims.

This is an experimentally falsifiable extension, not a declaration that the current HLB already provides a qualified building world model.

## 2. Architecture: five cooperating planes

### Plane A — Exact, source-authoritative world state

Symtropy owns exact entities, directed typed relations, geometry, coordinate frames, dimensions, units, constraints, versions, immutable observations and solver inputs/outputs.

A spatial observation should be bound to a canonical record containing, at minimum:

- exact subject/entity ID and field/channel ID;
- value, unit, coordinate frame and spatial support;
- valid/effective time and observed/recorded time as distinct values;
- source event ID, source revision and canonical payload digest;
- sensor, calibration and configuration identity where applicable;
- uncertainty/quality metadata and missingness state;
- provenance and rights/purpose metadata where applicable.

Do not let a predicted value overwrite an observation. Corrections append a new version and preserve the prior record. Late-arriving observations must not be exposed to a decision that occurred before their admission time.

The authoritative state is exact and inspectable; a hypervector is never its integrity digest, revision ID or authorization token.

### Plane B — HDC/VSA associative spatial memory

Use HDC for:

- partial-context and noisy-cue retrieval;
- role-aware relationship and cross-scale analogy;
- compact semantic indexing of typed observations and event context;
- recall of prior design cases, constraints, interventions and outcomes;
- candidate dependency suggestions for later exact verification.

Use explicit role, relation, scale, channel, coordinate-frame and time-context identities. Preserve edge direction. Keep a typed decoder/cleanup path and measure false suggestions, not only top-k recall.

The current Symtropy encoder is bipolar element-wise multiplicative binding plus majority bundling (MAP-like). Do not relabel it HRR/FHRR, BSC XOR, TPR or matrix GHRR. Each representation family requires a separately specified implementation and equal-budget comparison.

Exact graph/postings lookup remains the baseline for exact identifiers. HDC earns its place on incomplete, noisy, aliased, semantic or held-out cross-scale queries—not by being compared against an intentionally weak exact lookup.

### Plane C — Liquid temporal state and predictive dynamics

Investigate LTC/CfC as the temporal state-transition layer over spatial observations and HDC-retrieved context.

The input should include observed state, elapsed time, action/intervention context and explicit missingness. The model should support irregular sampling intervals and multiple time scales without silently treating missing observations as zero or healthy state.

Candidate state families:

- fast: occupancy transitions, access events, equipment state and local environmental transients;
- medium: daily thermal/light/moisture/airflow and occupancy cycles;
- slow: material ageing, maintenance condition, seasonal/site context and long-lived structural changes.

This is a research decomposition, not an assumption that one recurrent state can faithfully represent all these processes. Compare one shared temporal state with separated per-channel/per-timescale states and a hybrid.

A forecast must bind to:
- exact source snapshot and observation cutoff;
- model/encoder/configuration digest;
- target entities, channels, spatial support and prediction horizon;
- predicted value/distribution or interval, uncertainty and calibration domain;
- training-data lineage and held-out-family identity.

Forecasts remain estimates. They do not become sensor measurements or solver results.

### Plane D — Causal, counterfactual and experiment-selection layer

Symthaea may use predicted state, retrieved analogies and declared causal relations to propose:
- candidate design mutations;
- counterfactual interventions;
- likely affected physical domains;
- the next high-value simulation or measurement;
- explanations of expected trade-offs and uncertainty.

Do not allow correlation or HDC similarity to create an authoritative causal edge. Candidate causal/dependency relationships must be represented as proposals with provenance and confidence, then admitted only through the exact owner-controlled interface.

Select experiments by expected decision value, not by novelty or raw prediction uncertainty alone. A useful policy asks: which affordable experiment is likely to distinguish between currently competitive decisions, reduce a consequential uncertainty, or expose a hard-constraint failure?

### Plane E — Exact execution and evidence qualification

Symtropy owns geometry evaluation, physical simulation adapters, solver receipts, evidence dependency closure and currentness output.

Each solver receipt should bind the exact input snapshot, geometry/configuration digest, solver/build digest, material and boundary assumptions, numerical settings, convergence/residual data, uncertainty, output artifacts, and declared applicability domain.

The exact dependency graph decides which evidence becomes stale. HLB may propose missing dependency edges; it cannot retain or invalidate evidence by similarity. Unknown dependency coverage remains unknown and must fail closed.

## 3. Core experimental hypotheses

- H1 — Temporal value: HDC + CfC/LTC improves spatial-state forecasting, change detection or anomaly lead time over static HDC and strong temporal baselines on held-out building/topology families.
- H2 — Cross-scale transfer: role- and scale-aware HLB retrieval improves recovery of useful context and relation combinations for held-out modules, rooms, interfaces and spatial levels without increasing unsupported completions.
- H3 — Multi-channel value: a shared HLB representation reduces the combined cost of maintaining several interacting fields, or improves cross-channel prediction, at a declared and acceptable interference cost.
- H4 — Experiment efficiency: uncertainty- and decision-value-directed selection reduces high-fidelity solver calls while preserving feasible Pareto quality and constraint reliability under matched budgets.
- H5 — Evidence correctness: exact change-impact closure remains in full agreement with an independent oracle, with zero false-current evidence classifications under source, solver, model and dependency mutations.
- H6 — Functional scale coupling: recursive/multiscale grammars produce measurable cross-scale functional benefits, not merely visually recursive geometry or a high box-counting fit.

Each hypothesis is independent. A failure in one must not be presented as evidence for another.

## 4. Benchmark tracks and mandatory baselines

### Track T1 — Dynamic spatial state estimation and forecasting

Start with deterministic synthetic fixtures that expose known ground truth: smooth periodic fields, localised moving sources, discontinuous regime changes, anisotropic propagation, coupled channels, missing sensors, irregular sampling, sensor noise, delayed/late arrivals and source correction events.

Then add measured building observations only when licensing, units, calibration, coordinate frames, temporal coverage and provenance are available. Never present analytic fixtures as measured building data.

Compare:
- persistence and seasonal/periodic baselines;
- linear autoregression and VAR where appropriate;
- Kalman/state-space filters where their assumptions fit;
- RBF/kernel or Gaussian-process regression for spatial interpolation/prediction;
- LSTM/GRU and a coordinate-based neural field where task-appropriate;
- LTC and CfC without HDC;
- static HDC retrieval without temporal dynamics;
- HDC + LTC/CfC;
- separated per-channel/per-timescale models and a shared multichannel model.

Report MAE/RMSE and skill against persistence, probabilistic NLL/CRPS where predictive distributions are emitted, 50/80/90/95% interval coverage and width, calibration error, change-point precision/recall, detection delay, false alarms, missing-data robustness, latency, peak RSS and energy where measurable.

Split by complete project, sensor deployment and temporal episode. No random point-level splits across adjacent samples from the same field.

### Track T2 — Partial-context and cross-scale retrieval

Retain the exact source-graph oracle and exact role-postings as primary baselines. Add graph traversal and a learned graph/ANN baseline only where the same query semantics and source information are available.

Compare MAP, HRR, FHRR, BSC, TPR and a separately implemented non-commutative GHRR variant when available. Match effective dimension, stored bytes, precision, codebook/cleanup cost, query set and hardware. Report full encode → index → retrieve → cleanup → verify costs.

Test partial identifiers, aliases, corrupted labels, reversed edges, missing relations, multi-hop context, unseen relation combinations, cross-scale analogies and unanswerable queries. Split by complete project and topology family.

Metrics: precision/recall/F1, MRR, exact verified coverage, pre-verification false-suggestion rate, false verified acceptance (target zero), abstention risk/coverage, calibration by scale/family, p50/p95 latency, build/update cost, allocator-inclusive memory and energy.

### Track T3 — Continuous and shared multi-channel fields

Compare HLB/HDC field representations against nearest-neighbour, bilinear/trilinear, RBF, Gaussian-process and declared neural implicit-field baselines on identical samples, queries, dimensions, precision and held-out regions.

Vary field smoothness, frequency, discontinuity, anisotropy, sparsity, sensor noise, coordinate corruption, field dimension, sample density and channel coupling.

Report each channel separately and jointly. Measure cross-channel interference, channel confusion, reconstruction and forecast errors, calibration, memory including codebook/index metadata, allocator-inclusive peak RSS, build/update/query cost and energy.

Do not demand that HDC beat a scalar grid on raw storage where a grid is the appropriate representation. The useful claim is a measurable accuracy/latency/semantic-retrieval trade-off under a declared budget.

### Track T4 — Compositional grammar generalization

Use project-held-out and family-held-out splits. Hold out combinations of:
- module types and connector/interface families;
- room functions and access/circulation patterns;
- topology families and assembly sequences;
- site/climate/operating regimes;
- structural and service relationships;
- component, room, storey, building and site scales.

Compare hand-authored parametric grammars, exact graph rewrite/search, GA/NSGA baselines, topology optimization where comparable, graph neural methods and the HLB-proposed grammar.

Report valid-composition rate, hard-constraint satisfaction, graph/geometry validity, held-out relation F1, graph edit distance, diversity among feasible candidates, time to first feasible candidate and human edit/override rate. Report each domain separately; do not collapse safety, accessibility, privacy, comfort and cultural/occupant preferences into one hidden scalar objective.

The Modified Swiss Dwellings dataset is a useful first public building-complex benchmark (5,372 plans, 18.9K+ units, 165.3K+ areas, with geometry and access-graph modalities). Preserve its license and published split where usable; add project/family leakage checks. MSD does not supply structural or environmental ground truth.

### Track T5 — Active experiment selection and multi-objective optimization

Use RBFMOpt, TPE, HypE, NSGA-II, NSGA-III, random search and Latin-hypercube sampling as the mandatory comparison set where their problem assumptions apply. Add a decision-aware Bayesian/multi-objective acquisition baseline when implementation and compute permit.

Treat adaptive sparse Polynomial Chaos Expansion (PCE) as a surrogate/uncertainty-quantification method and sequential A-optimal sampling as a sampling design—not as if PCE were itself an optimizer. Compare the complete optimizer + surrogate + acquisition + solver workflow.

A relevant November 2026 Energy and Buildings study applied adaptive sparse PCE with sequential A-optimal sampling to four Irish residential archetypes, a 14-dimensional uncertainty space, and reported held-out R² >= 0.99 for EUI and >= 0.93 for annual heating demand using 480–560 training simulations per stage. Reproduce its stated protocol and archetype scope before claiming to exceed it; do not generalize its results to other buildings or climates.

Use identical design variables, hard constraints, objectives, solver fidelity, initialization access, failure handling, and total expensive-solver budgets. Freeze seeds and report repeated runs. Suggested budgets: 25 / 50 / 100 / 250 / 500 solver calls, with a separate wall-clock/compute budget.

Metrics: feasible hypervolume, hypervolume attainment curve, IGD+, feasibility rate, time/calls to first feasible point, evaluation cost, uncertainty calibration, dominated-call fraction, robustness over seeds and worst-case constraint violation. Use bootstrap confidence intervals and non-inferiority margins declared before candidate results are seen.

A prospective goal is at least 20% fewer high-fidelity calls at non-inferior feasible hypervolume (suggested initial margin: no more than 2% relative HV loss); this is a target, not a result. If feasibility or safety constraints degrade, the policy fails regardless of hypervolume.

### Track T6 — Evidence-impact and replay correctness

Extend the current exact DAG from synthetic artifacts to real versioned source snapshots, geometry/solver input manifests and Symtropy solver receipts.

Required independent oracle tests:
- one local parameter mutation invalidates exactly dependent receipts;
- shared site/weather/material/model changes invalidate all dependent projects;
- unrelated changes retain only evidence whose full dependency set is known and unchanged;
- solver/build/configuration changes invalidate the claims dependent on them;
- missing dependency inventory yields conservative unknown/stale output;
- delayed/corrected source events preserve historical decision replay while changing current applicability;
- evidence from a different source digest cannot be rebound to a new candidate.

Targets: 100% independent-oracle agreement on the declared finite corpus; zero false-current classifications; zero unjustified unrelated invalidations where dependency completeness is established; deterministic replay for exact source cutoffs. A passing DAG unit test does not prove the upstream dependency inventory is complete.

Reuse existing Mycelix temporal/provenance/currentness components where applicable. Do not create a second generic event store, signature system, identity registry or evidence lease service.

### Track T7 — Geometric and functional scaling

Retain known-dimension positive controls and non-fractal negative controls. Extend from point-set box counting to a declared family of structural/environmental observables: load-path distribution, branch/member hierarchy, service connectivity, shading, daylight, ventilation, drainage and maintenance reach.

For each observable, predeclare the scale interval, estimator, sampling scheme and matched non-fractal control. Report full curves, estimator residuals, window sensitivity, grid/orientation sensitivity, finite-size effects and uncertainty. A recursive grammar is not itself a fractal, and geometric scaling does not establish functional performance.

## 5. Representation and model ablations

At minimum, ablate:
1. exact graph/geometry only;
2. exact graph + static HDC;
3. exact graph + temporal model without HDC;
4. exact graph + HDC + CfC/LTC;
5. shared versus separated temporal/channel states;
6. HLB with and without retrieved historical context;
7. HLB with and without the active experiment selector;
8. HLB predictions with and without calibrated uncertainty;
9. exact dependency closure with and without HLB-suggested candidate edges (suggestions never become authority automatically).

A model complexity increase is justified only when it improves an intended endpoint under matched data, compute and memory. Report negative results.

## 6. Prospective acceptance targets

These targets should be frozen before the corresponding held-out evaluation:

| Capability | Prospective target |
|---|---|
| Exact graph/postings agreement | 100% on finite oracle corpus |
| Evidence currentness | Zero false-current classifications |
| HLB retrieval | At least +5 percentage points F1 on a predeclared noisy/partial/held-out task at matched memory, without increased verified false acceptance |
| Retrieval efficiency | 2x lower p50 latency only at non-inferior F1 and matched memory/compute; not a target for exact key lookup |
| Temporal prediction | Statistically better than persistence and strongest task-appropriate baseline on held-out project families, with calibration reported |
| Probabilistic forecast | Nominal interval coverage within 5 percentage points for predeclared intervals, with width and sharpness reported |
| Active simulation selection | At least 20% fewer high-fidelity calls at the frozen non-inferiority margin for feasible Pareto quality |
| Grammar transfer | Improvement over exact/parametric grammar baseline on held-out combinations, while hard-constraint violations remain zero in accepted candidates |
| Shared field memory | Demonstrated accuracy/latency/memory benefit under matched budgets, or a documented negative result |
| Geometric scaling | Stable, predeclared estimates on known controls and no false scale claims on matched null controls |

All targets are research goals, not achieved metrics or production acceptance criteria.

## 7. Industry and research positioning

Autodesk Forma's 2026 public roadmap describes connected site-to-building workflows, experimental generative building-layout exploration and room-level daylight analysis. nTop describes parametric design exploration with structural, thermal and CFD simulation integrated into iteration. HoloFrac therefore cannot differentiate merely by saying it generates layouts, analyzes daylight or explores many variants.

buildingSMART IDS 1.0 is an important interoperability contract for IFC information requirements and automated checking of schema-level information. Its first version explicitly does not cover geometry checks, calculated/dynamic values or requirements that need external data. HoloFrac should integrate IDS for the portion it can express and use exact geometry/solver/process-specific verifiers for the rest; do not represent IDS as a complete building-engineering assurance standard.

The defensible differentiation hypothesis is:
- persistent cross-scale semantic and temporal context;
- candidate generation conditioned on explicit intent, functional grammar and constraints;
- experiment selection based on expected decision value;
- exact source-bound mutation impact;
- independently replayable solver evidence and applicability;
- interoperable data contracts without surrendering authoritative source state.

This is an architecture and research position, not evidence that current competitors lack all these capabilities. Compare actual workflows and public APIs in a matched task study.

## 8. Data and evaluation controls

- Publish dataset version, source URL, license, rights/purpose restrictions, filtering manifest and cryptographic content digest.
- Split by project/building, topology family, site/climate and temporal episode, not random neighboring cells.
- Keep all variants descended from one base design in the same partition.
- Freeze input manifests, seeds, toolchain, hardware, solver versions and benchmark protocol before final evaluation.
- Store raw per-query/per-run outputs, failures, abstentions and outliers; do not keep only aggregated scores.
- Report CPU/GPU, peak RSS, energy if measured, p50/p95/p99, and total solver/model-call cost.
- Count failed and timed-out runs; never treat missing benchmark output as a pass.
- Separate synthetic proof fixtures, public-dataset results, measured field results and real-project validation.
- Record model predictions separately from observations and qualified solver outputs.
- Use independent reviewers/oracles for evidence correctness and high-consequence constraint checks.

## 9. Ownership boundary and sequencing

**Symthaea**
- HLB state encoding, context retrieval, temporal prediction, calibrated uncertainty, structural grammar proposal, counterfactual reasoning and experiment selection.
- HLB may suggest missing relations/dependencies, never certify them.

**Symtropy**
- Exact source state, geometry/coordinate/units authority, physical simulation, solver receipts, exact dependency closure, evidence invalidation and replay.

**Mycelix**
- Reuse existing temporal event, authenticated provenance, policy and evidence-custody facilities when a cross-domain trust or bitemporal service is needed. Do not duplicate generic infrastructure inside HoloFrac.

**Humans / qualified professionals**
- Retain authority over requirements, occupant priorities, design review, engineering acceptance and any regulated approval.

Suggested sequence:
1. Freeze the source-observation and prediction receipt schema.
2. Build a synthetic irregular-time multichannel world-model fixture with exact ground truth and strong baselines.
3. Compare static HDC, CfC/LTC-only and HDC+CfC/LTC under matched budgets.
4. Add held-out project/topology grammar tasks and calibrated abstention.
5. Connect active experiment selection to Symtropy solver receipts.
6. Connect exact impact closure to versioned receipts and historical/current replay.
7. Only then evaluate measured building data and real design workflows.

## 10. Claim ceiling

A successful Phase 2 result can establish benchmark-specific predictive, retrieval, selection or evidence-closure improvements for the declared datasets, split, models, hardware, solver and applicability domain.

It cannot by itself establish structural safety, code compliance, occupant health/comfort, real-world energy savings, certified fractality, causal truth, engineering approval or production readiness. Those require their own independent evidence and domain-specific validation.


## 11. Sensor-network co-design and virtual sensing

HoloFrac should optimize the observation system as well as the built environment. A spatial world model is only as useful as its observability, sensor placement, calibration lineage and uncertainty under missing/delayed observations.

A recent research reference is Yu, Ding and Zhou, *Data-driven discovery of sensor relationships for virtual sensing in predictive building operations*, Energy and Buildings, 15 October 2026, DOI: 10.1016/j.enbuild.2026.117836. The study uses two years of Harvard HouseZero operational data, discovers sensor redundancy with multi-target relationships and evaluates generalization from Year 1 to Year 2. This is a relevant reference to reproduce or compare against; it is not evidence that HoloFrac has achieved virtual sensing.

Symtropy issue #5974 tracks a deterministic synthetic event fixture and sensor placement / adaptive-observation benchmark:
https://github.com/Luminous-Dynamics/symtropy/issues/5974

The current synthetic fixture lives at:
- \`research/holofrac-retrieval/src/temporal.rs\`
- \`research/holofrac-retrieval/src/bin/temporal_fixture_sweep.rs\`
- \`research/holofrac-architecture/TEMPORAL_FIXTURE_PROTOCOL_V0.md\`

It is explicitly synthetic. Its typed observation event separates valid/effective time from recorded/available time; missing readings are \`None\`, never zero; corrections are new events with exact parent IDs and source revisions; and evaluator-only truth is excluded from decision snapshots. Its stable digest is a non-security regression identifier only.

### New T8 hypotheses

- T8.1 — Sensor-placement value: at a matched sensor count and cost, HLB-assisted / decision-aware placement improves held-out field reconstruction or decision quality over regular-grid, random and maximin placement.
- T8.2 — Virtual-sensing value: multi-channel models can reconstruct selected missing sensors from a sparse network with calibrated uncertainty and acceptable cross-year / cross-deployment degradation.
- T8.3 — Adaptive-observation value: selecting a new measurement by expected decision value reduces sensor/solver cost at non-inferior decision quality compared with uncertainty-only, sequential A-optimal and fixed schedules.
- T8.4 — Observability robustness: sensor failure, calibration drift, correlated common-mode loss and delayed delivery produce explicit degradation/unknown states rather than overconfident virtual observations.

### T8 baselines and metrics

Compare regular-grid, random and maximin placement; nearest-neighbour, bilinear/trilinear, RBF and Gaussian-process virtual sensing; linear/VAR and Kalman/state-space models where applicable; LSTM/GRU and CfC/LTC; static HDC, temporal-only and HDC+temporal models; uncertainty-only, sequential A-optimal and decision-value measurement selectors.

Report per-channel / per-horizon MAE and RMSE, NLL/CRPS, interval coverage and width, calibration, virtual-sensor error, sensor count/cost, spatial coverage, redundancy, observability rank/condition metrics only when mathematically defined, anomaly detection delay, false alarms, bandwidth/energy, inference/selection cost and p50/p95/p99 latency.

Split by complete temporal episodes / years, spatial regions and sensor deployments. Report in-year interpolation separately from cross-year and cross-deployment generalization. Never randomly split neighbouring samples from the same episode across train and test.

### T8 authority boundary

An inferred virtual-sensor value must remain typed as inferred and retain its model/source lineage and uncertainty. It must not be relabelled as an observed measurement, counted as an independent physical source, or used to preserve current engineering evidence as if an actual sensor observation existed. A measurement request is a proposal; Symtropy applies budgets and exact request admission.



## 12. Sensor-network co-design and virtual sensing

Treat the observation network as part of the spatial design problem. HoloFrac should investigate not only how to infer fields from a fixed sensor deployment, but how to choose sensor locations, identify redundant/complementary observations, estimate virtual sensors with calibrated uncertainty, and request new measurements when they are likely to change a decision.

A relevant comparison is Yu, Ding and Zhou, *Data-driven discovery of sensor relationships for virtual sensing in predictive building operations*, Energy and Buildings (15 October 2026), DOI 10.1016/j.enbuild.2026.117836. The work uses two years of Harvard HouseZero operational data and evaluates Year-1 to Year-2 transfer. This is a reproduction target, not evidence of HoloFrac capability.

Symtropy issue #5974 tracks this programme:
https://github.com/Luminous-Dynamics/symtropy/issues/5974

Current synthetic executable slices on the HoloFrac implementation branch:
- bitemporal observations: \`research/holofrac-retrieval/src/temporal.rs\`;
- observation sweep: \`src/bin/temporal_fixture_sweep.rs\`;
- sensor placement baselines: \`src/sensor_placement.rs\`;
- placement sweep: \`src/bin/sensor_placement_sweep.rs\`;
- protocols: \`TEMPORAL_FIXTURE_PROTOCOL_V0.md\` and \`SENSOR_PLACEMENT_PROTOCOL_V0.md\`.

The initial placement benchmark compares regular-grid, seeded-random and greedy-maximin selection on the same synthetic six-channel fields, using nearest-sensor virtual sensing. It isolates geometric coverage only: it is noise-free, does not model real calibration, and does not establish sensor-placement optimality.

### T8 hypotheses

- T8.1 — At matched sensor count and cost, information-aware placement improves held-out field reconstruction or decision quality over regular-grid, random and maximin layouts.
- T8.2 — Multichannel virtual sensing can reconstruct selected missing sensors with calibrated uncertainty and acceptable cross-year / cross-deployment degradation.
- T8.3 — Expected-decision-value measurement selection reduces observation/solver cost at non-inferior decision quality compared with uncertainty-only, sequential A-optimal and fixed schedules.
- T8.4 — Sensor failures, drift, delayed events and correlated common-mode losses produce explicit uncertainty/unknown states rather than overconfident virtual observations.

Mandatory metrics include per-channel/per-horizon MAE/RMSE, NLL/CRPS, interval coverage and width, spatial coverage, nearest-sensor distance, calibration, virtual-sensor error, false alarms/detection delay, sensor count/cost, energy/bandwidth, peak RSS and p50/p95/p99 latency.

Split by complete temporal episode/year, spatial region and sensor deployment. Report in-year interpolation separately from held-out-year and held-out-deployment generalization. Never randomly split neighboring samples from one episode.

**Authority boundary:** a virtual-sensor estimate remains typed as inferred and carries model/source lineage and uncertainty. It cannot be relabelled as an observation, counted as an independent physical source, or used to preserve engineering evidence as if an actual sensor measurement existed. HLB may propose placements and requests; Symtropy applies exact validation, budget policy and solver/measurement execution.


## 13. Adaptive circular lifecycle and component reuse

Extend HoloFrac from single-state design/operation to a sequence of versioned lifecycle states: design, assembly, operation, maintenance, adaptation, repair, replacement, selective deconstruction, recovery and second-life assembly.

A relevant June 2026 reference is Al-Obaidy et al., *BCEF: Early design framework for circular and low-impact buildings*, DOI 10.1016/j.rineng.2026.110824. Its six separate indicators are global-warming potential, land-use footprint, reused content, disassembly potential, functional adaptation and product-service systems. A further 2026 operational-digital-twin review reports that only a minority of reviewed deployments reached prescriptive, control-driving maturity and that quantitative outcomes were sparsely reported. These references motivate a testable lifecycle and outcome loop; they do not prove a HoloFrac advantage.

Symtropy issue #5985 owns the executable lifecycle fixture and transition/mass-balance qualification:
https://github.com/Luminous-Dynamics/symtropy/issues/5985

### T9 hypotheses

- T9.1 — Functional adaptability: versioned spatial grammars can produce future-use configurations that satisfy declared hard constraints with less demolition/replacement than matched static designs.
- T9.2 — Component recoverability: explicit reversible interfaces, access envelopes and disassembly sequences improve verified recovery/reuse outcomes over geometry-only modularity.
- T9.3 — Lifecycle evidence: exact component/condition/source lineage allows evidence to be selectively retained, invalidated or made unknown across repair, replacement and second-life assembly.
- T9.4 — Reuse matching: HLB retrieval can improve candidate recall for recovered components while exact dimensional, condition, material and applicability checks prevent unsupported matches.
- T9.5 — Whole-life decision quality: multi-stage design alternatives can improve declared functional retention and/or whole-life impact versus matched static and conventional design-for-disassembly baselines.

### T9 benchmark requirements

Use separate indicators, never one hidden circularity score: whole-life GWP, land-use footprint, reused content, disassembly potential, functional adaptation and product/service model. Preserve declared lifecycle boundaries, functional unit, EPD source/geography/version, service period, replacements and scenario assumptions.

Required adversarial controls include missing/invalid passports, changed component IDs, non-reversible or inaccessible connections, tool/lifting/sequence constraints, damaged recovery, incompatible dimensions/material/condition, partial dependency inventory, stale solver receipt, adaptation outside the qualified use envelope, and second-life components with lost or uncertain provenance.

Targets on the finite fixture:
- 100% component identity/lineage preservation;
- 100% mass-balance closure within declared tolerance;
- zero unsupported recovery/reuse credits;
- zero invalid transitions admitted;
- zero false-current evidence classifications;
- independent-oracle agreement for transition validity and impact closure.

Do not claim real whole-life carbon, circularity, building safety or reuse permission from synthetic data. Real case studies require product-specific passport/EPD/condition evidence, qualified disassembly records and a frozen lifecycle boundary.


## 14. Lifecycle evidence applicability and source-bound currentness

A lifecycle digital twin must not merely retain a history of component events. It must know which evidence was valid for which exact component/design state, and which receipts remain applicable after repair, functional change, interface modification, material substitution or reuse.

A relevant 3 November 2026 material-passport study argues that product-centric passport fields alone do not bridge end-of-use to next-use; it proposes process, organizational and information requirements for continuity across maintenance, planning for next use, disassembly, reprocessing and reinstallation. A 15 November 2026 review of regenerative digital twins explicitly frames lifecycle twins around material flows, embodied carbon and end-of-life reuse potential. These references reinforce the need to model evidence/process continuity, rather than just store a static product passport:
- Contextualizing a process information approach in material passports (3 November 2026), DOI 10.1080/09613218.2026.2743409.
- Regenerative twins as catalysts for circular, net-zero readiness in built environments (15 November 2026), DOI 10.1016/j.enbuild.2026.118243.

Symtropy #5959 remains the source-manifest and solver-receipt integration track. The HoloFrac executable extension in src/lifecycle_impact.rs composes the existing exact dependency closure with four component mutations: condition, connection/interface, material passport and functional use. It includes a frozen independent oracle for seven current evidence receipts and an incomplete-inventory negative control.

### T10 hypotheses

- T10.1 — Exact lifecycle invalidation: a component/assembly mutation invalidates every and only current receipts whose declared source dependencies transitively include that mutation.
- T10.2 — Historical/current distinction: a receipt remains queryable as historical for its original state but cannot be represented as current for a later state without an explicit applicability relation.
- T10.3 — Completeness fail-closed: missing, stale, contradictory or partial dependency inventories produce unknown/conservative invalidation, never a positive current result.
- T10.4 — Reuse transfer: when a recovered component enters a second-life assembly, material/passport identity may carry forward, but load, use, interface, condition and solver evidence must be re-qualified for the new context.

### T10 benchmark and baselines

Compare exact dependency-DAG closure, full-project rebuild/revalidation, manual project-level invalidation and owner-authored rule-based applicability. HDC/HLB may propose candidate dependency edges, but the exact source owner or verified adapter must authorize each edge before it enters the authoritative manifest.

Use matched source inventories and frozen receipts. Test local condition change, interface change, material substitution, use-class adaptation, common-cause weather/site/model changes, stale solver/build digest, omitted source artifact, incomplete manifest, incorrect receipt input digest and a late-arriving correction. Independent oracle output must be authored separately from the production adjacency/closure implementation.

Report:
- exact stale/current/historical/unknown receipt identifiers;
- false-current classifications (target zero);
- unnecessary invalidations where complete inventory proves independence (target zero);
- incomplete-inventory conservative invalidation (target 100%);
- exact agreement with independent oracle (target 100% on frozen finite corpus);
- evidence revalidation avoided relative to full rebuild;
- graph build/update cost, closure p50/p95/p99 and peak RSS;
- all missing/contradictory/rejected states.

The finite lifecycle fixture's present current-evidence graph is synthetic and explicitly complete by construction. It is not evidence that a real building inventory is complete. Source-manifest integration and adapter-level dependency enumeration remain required before any real project claim.


## 15. External benchmark and standards qualification map

The companion repository map is `research/holofrac-architecture/EXTERNAL_BENCHMARK_MAP_V0.md` on the HoloFrac implementation branch and records current industry capabilities, benchmark scope limits and a six-layer comparison matrix.

Symtropy issue #6007 establishes an external qualification suite:
https://github.com/Luminous-Dynamics/symtropy/issues/6007

The benchmark system must be layered. A result in one layer does not qualify the others.

### Exchange and information requirements

Use buildingSMART IDS 1.0 for machine-readable IFC information requirements: entity applicability, attributes, classifications, property sets, materials, values and supported relationships. IDS is useful for exchange/data completeness, but its first version does not perform geometry/clash checks, calculated/dynamic-value checks or checks that require external data. Pair IDS with exact geometry/physics oracles and source/receipt checks.

### Geometry and building simulation

Use ASHRAE Standard 140 for applicable first-principles building-performance simulation software. The 2023 edition adds weather-data interpretation tests to the existing 2020 cases. Do not claim ASHRAE 140 qualification for a learned HDC/CfC predictor merely because it forecasts energy or temperature: the standard's test methods are primarily for first-principles simulation software, and its user manual identifies data-driven models as out of scope for some methods.

### Operational building control

Use BOPTEST as the external benchmark for building-control strategies and fault detection. Its Modelica building emulators, standardized sensor/actuator API and KPI reporting provide a stronger matched test than a private synthetic control environment. Compare schedules/rule-based control (including ASHRAE Guideline 36 sequences where applicable), MPC, robust/stochastic MPC where supported, published learned/RL controllers and HLB-assisted policies using identical test cases, weather, occupancy, actuators, constraints, evaluation horizon and cost/comfort metrics.

The 15 October 2026 scoping review of operational building twins filtered 2,301 records and retained 37 deployments with real-time connectivity, physical instrumentation and operational use. It reported 48.6% descriptive systems, 24.3% prescriptive systems and quantitative outcomes in only 27% of the corpus; prescriptive studies reported quantified outcomes roughly four times as often as descriptive ones. Treat this as a maturity/evidence benchmark: HoloFrac should not call a simulation-only artifact an operational twin or claim real outcomes without measured deployment evidence.

### Circular design and disassembly

Use ISO 20887:2020 as the design-for-disassembly and adaptability principles/guidance reference. ISO reviewed and confirmed it in June 2025; it remains current. It does not set universal numeric performance levels, so HoloFrac must declare measurable thresholds for disassembly sequence feasibility, access, damage, recovery, reuse compatibility and retained function. Use BCEF's separate circularity indicators—GWP, land use, reused content, disassembly potential, functional adaptation and product-service systems—rather than a single unconstrained scalar.

### Qualification matrix

Map every HoloFrac claim to one of:
1. Exchange: IFC round-trip, IDS checks, stable IDs, units/frames and unmapped semantics.
2. Geometry/physics: analytic controls, numerical convergence, geometry invariants and independent solver tests.
3. Forecast: held-out time/space accuracy, probabilistic calibration, missing/delayed/corrupted observations and no knowledge-time leakage.
4. Control/decision: BOPTEST matched controllers, cost/comfort trade-offs, calibrated value of information, safe failure and occupant override.
5. Lifecycle: ISO 20887 principles, BCEF indicators, exact component lineage, disassembly/recovery evidence and material mass balance.
6. Evidence: exact source manifests, solver-input/result receipts, zero false-current status, selective invalidation and independent oracle agreement.

All external comparisons must freeze versions, datasets/test cases, weather, occupancy, geometry, functional units, system boundaries, fidelity, resource budgets and seeds. Split by complete buildings/projects, climate zones, component families and temporal episodes. Publish per-case raw outputs, failed/infeasible/timeouts and toolchain/source manifests.

Prospective targets in #6007 are research goals, not achieved results: zero false-current evidence and admitted hard-infeasible control actions; non-inferior comfort/fault safety plus at least 10% lower energy/cost on a predeclared BOPTEST case; at least 20% fewer high-fidelity evaluations than the strongest matched experiment-selection baseline at non-inferior Pareto quality; and 100% identity/mass/oracle correctness on finite lifecycle fixtures.


## 16. Consent-first pluralistic decisions and occupant agency

Symtropy issue #6011 adds a consent-first, multi-stakeholder decision benchmark:
https://github.com/Luminous-Dynamics/symtropy/issues/6011

Recent research suggests that occupant-centred digital twins and controls remain early-stage. A September 2026 systematic review of 40 studies found only 15% integrated real-time occupant feedback. A July 2026 review of 82 non-intrusive occupant-monitoring studies highlights privacy, implementation cost and scarce labelled data. A 3,504-office-worker survey in Spain and the Netherlands found automation preferences differ by service: HVAC automation was more accepted, while windows and personal devices were preferred to remain manual. A 120-participant shared-space digital-twin study reports value in visualizing preferences, including those of quieter participants. These studies motivate agency/consent benchmarks; they do not prove HoloFrac has solved group decision-making.

The current executable synthetic slice is in src/pluralistic.rs with CLI src/bin/pluralistic_decision_sweep.rs on the HoloFrac implementation branch. It models explicit preference weights and hard vetoes, bitemporal preference/consent revision events, local-only / anonymous-aggregate / named-aggregate scopes, not-granted / granted / withdrawn status, and separate Pareto, mean, maximin and leximin comparisons. Anonymous profiles have no identity in shared output. Withdrawn or local-only profiles cannot affect future group aggregation. A late-recorded withdrawal does not rewrite earlier decision knowledge.

### T12 hypotheses

- T12.1 — Preference transparency: explicit participant-supplied preferences and outcome vectors let affected parties inspect which trade-offs each design makes.
- T12.2 — Minority preservation: maximin/leximin, Pareto and explicit hard vetoes can expose losses hidden by mean-score optimization.
- T12.3 — Consent correctness: only profiles with recorded consent and an authorized scope participate; non-response is not consent; recorded revocation excludes future group use.
- T12.4 — Automation agency: participants can choose control mode per service (e.g., automatic HVAC vs manual windows/personal devices) and retain a reachable override.
- T12.5 — Privacy minimization: anonymous aggregation can retain decision value without exposing identity in shared reports, while local-only profiles remain outside group aggregation.

### T12 benchmark corpus and baselines

Use 5–12 synthetic stakeholders per scenario and criteria covering thermal comfort, air quality, daylight, acoustics, accessibility, privacy, energy/cost, adaptability and maintenance. Compare Pareto filtering, explicit weighted-sum policies, maximin, leximin, Nash social welfare, voting/approval/quorum and human-authored choice. Report assumptions, sensitivity and failure cases for each; do not label a method universally fair.

Required adversarial cases include a minority veto, conflicting vetoes, local-only participant, no consent, late-recorded withdrawal, stale preference revision, non-response, unequal participation, strategic report, accessibility floor, privacy floor, automation preference by service and connectivity loss.

Targets on the finite synthetic corpus:
- 100% independent Pareto oracle agreement;
- zero hard-infeasible or veto-violating alternatives admitted;
- zero use of withdrawn or out-of-scope profiles after the revocation is recorded;
- zero identities in anonymous output;
- 100% historical snapshot agreement at frozen knowledge cutoffs;
- per-stakeholder outcomes and unmet preferences always visible; group averages alone are insufficient.

The current implementation hardens the anonymous output boundary: raw preference/consent events transfer through a validating constructor into a private ledger; raw snapshot fields, cohort counts and private leximin vectors are not exposed through public fields, Debug, Clone or equality output. Anonymous stable IDs, per-person satisfaction rows and numeric veto details are not emitted. The synthetic shared-aggregation policy requires at least five anonymous participants; one through four fail closed. This is identity-redacted minimum-cohort aggregation, not differential privacy, and aggregate mean/minimum, rankings and criterion-only anonymous veto signals remain possible query side channels. Repeated-query reconstruction and membership inference are tracked separately in #6034: https://github.com/Luminous-Dynamics/symtropy/issues/6034.

No camera, biometrics, psychological-state inference or demographic proxy should be a default input. Preference data is supplied explicitly, revocable and limited to the participant-authorized scope. Real deployments require informed consent, data minimization, accessible participation, appropriate ethics review and human control.


## 17. Decision-value experiment selection and budgeted learning

Symtropy issue #5961 owns budgeted experiment requests and independent execution/receipt validation:
https://github.com/Luminous-Dynamics/symtropy/issues/5961

The current HoloFrac implementation branch adds a finite exact value-of-information fixture:
- src/experiment_selection.rs
- src/bin/experiment_selection_sweep.rs
- EXPERIMENT_SELECTION_VOI_PROTOCOL_V0.md

It demonstrates a key distinction. One synthetic measurement gives 1 bit of information about a latent variable that is irrelevant to the current design decision; another gives less information but changes the optimal decision and reduces expected regret. The finite oracle computes posterior values, mutual information, EVSI minus experiment cost and regret by exact enumeration, not Monte Carlo.

### T13 hypotheses

- T13.1 — Decision relevance: expected-decision-value selection chooses tests that improve the declared decision, not merely those with the greatest entropy reduction.
- T13.2 — Budget efficiency: a sequential policy achieves non-inferior feasible Pareto quality with fewer high-fidelity solver calls than random, space-filling, uncertainty-only and A-optimal baselines.
- T13.3 — Calibration: predicted value of information is calibrated against realized decision improvement; the selector learns when to request no test.
- T13.4 — Evidence integration: every executed measurement/solver call is bound to an admitted source snapshot, exact input digest, solver identity/version/fidelity, budget reservation and immutable result receipt.
- T13.5 — Pluralistic value: where no scalar social utility is authorized, the selector evaluates changes in the Pareto set and stakeholder-specific regret rather than inventing a hidden group objective.

### T13 baselines and metrics

Compare no-test, seeded-random, Latin hypercube/maximin, maximum information gain, sequential A-optimal, expected value of sample information (EVSI), decision-regret and appropriate Bayesian optimization baselines.

Freeze:
- source and candidate digests;
- observation and solver likelihood/fidelity model;
- decision alternatives and utility/constraint contracts;
- test action costs and total budget;
- initial priors/training data;
- stopping criteria, tie-breaks and fallback/no-test behavior.

Report selected actions, mutual information, expected posterior value, EVSI, net expected value after cost, realized decision regret, feasible Pareto hypervolume/IGD+, high-fidelity call count, cost, wall-clock, solver failures, abstentions and receipt completeness.

Prospective target from #5961: at least 20% fewer high-fidelity solver evaluations than the strongest matched baseline at non-inferior feasible Pareto quality, with a predeclared 2% relative hypervolume non-inferiority margin. The target must be frozen before evaluation and measured across held-out problem families, not selected after observing results.

The synthetic fixture proves only exact finite arithmetic and policy behavior for its declared likelihoods. It does not establish calibrated real-world solver value, causal truth, or real engineering-cost savings. The integrated path must wait for #5959 source/solver receipts and #5961 request admission/execution before making such claims.


## 18. Cross-scenario climate robustness, Pareto persistence and regret

Symtropy issue #6025 owns the external research and scenario-robustness programme:
https://github.com/Luminous-Dynamics/symtropy/issues/6025

The first executable finite-scenario slice is now on the HoloFrac implementation branch:
- research/holofrac-retrieval/src/climate_robustness.rs
- research/holofrac-retrieval/src/bin/climate_robustness_sweep.rs
- research/holofrac-architecture/CLIMATE_ROBUSTNESS_PROTOCOL_V0.md

It computes scenario-specific Pareto frontiers, their union and intersection, mean pairwise Jaccard persistence, worst-case objective vectors, weighted feasibility coverage, per-criterion weighted regret, maximum regret and a robust Pareto frontier restricted to candidates feasible in every declared scenario.

The first synthetic scenarios are present-TMY, future-hot and compound heat/grid stress. They use explicitly declared **analytical decision weights**, not probabilities. The fixture deliberately includes a present-day specialist that is infeasible under later scenarios; it must not be called guaranteed-feasible or appear on the robust frontier. It also tests a valid no-robust-candidate outcome: every scenario has at least one feasible candidate, but no candidate passes all scenarios, so the robust frontier is empty and HLB/Symtropy must not relax constraints to force a winner.

### T14 hypotheses

- T14.1 — Pareto persistence: candidate rankings/frontiers change materially across future-climate, microclimate, occupancy and supply scenarios; report the changes rather than optimize against one TMY.
- T14.2 — Feasibility robustness: candidates called guaranteed-feasible pass every declared hard scenario constraint.
- T14.3 — Regret: a balanced cross-scenario design can reduce maximum regret relative to a present-day-only specialist under a matched solver budget.
- T14.4 — Uncertainty-directed sampling: Symthaea proposes the next climate/occupancy/compound stress experiment by expected decision value; Symtropy validates and executes it under a budget and exact solver receipt.
- T14.5 — Out-of-domain honesty: scenario coverage, surrogate error and missing future conditions are explicit unknowns, not extrapolated certainty.

### T14 baselines and metrics

Compare present-TMY-only optimization, independent optimization per scenario, union/intersection of scenario Pareto sets, robust maximin, minimax regret and weighted utility only when probability weights are independently justified. At higher fidelity, include matched NSGA-II/III, MOPSO and surrogate-assisted methods under equal variables, constraints, solver fidelity and evaluation budgets.

Report scenario-specific feasible frontiers, union/intersection, pairwise Jaccard, robust worst-case frontier, weighted feasibility coverage, guaranteed-feasibility flag, worst-case objective vectors, weighted/max regret, feasibility-margin erosion, solver calls, wall-clock, surrogate error and all infeasible/unknown cases.

The finite fixture targets 100% independent-oracle agreement for scenario/robust frontiers and regret, and zero candidates labelled guaranteed-feasible if any scenario fails a hard constraint. Real climate studies require source-versioned future weather, local microclimate, occupancy, tariff/grid/water assumptions, calibrated first-principles simulation and held-out scenarios.

SSP narratives are not probability distributions by default. The initial analytical decision weights are decision priorities only; do not describe weighted outcomes or feasibility coverage as expected values or probabilities.

The initial synthetic fixture does not prove climate resilience, future-climate likelihood, energy savings, comfort, net-zero status, safety, code compliance or adaptation certification.
