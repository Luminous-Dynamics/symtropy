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

