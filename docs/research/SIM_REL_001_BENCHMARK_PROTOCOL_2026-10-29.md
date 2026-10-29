# SIM-REL-001 — Relational grammar benchmark protocol v1

**Status:** proposed protocol; no benchmark execution or performance result is claimed.  
**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2870  
**Companion Symthaea track:** https://github.com/Luminous-Dynamics/symthaea/issues/8919  
**Date:** 2026-10-29

## 1. Research claim and boundary

**Question:** Under matched information and compute budgets, can a compact, typed, compositional relational grammar recover hidden structure and predict unseen physical-system compositions more reliably or efficiently than an explicit equation baseline and conventional state/dependency graph?

This benchmark is motivated by the structural language of Alex Wissner-Gross's October 2026 Chain Theory compendium: finite systems, links, shared endpoints, read/distillation operations, and a construction-order coordinate. It tests a classical representational analogue. It does **not** test or validate the compendium's quantum axiom, fine-structure-constant candidate, photon mapping, claims about spacetime, gravity, consciousness, or ESP/psi.

Chain Theory itself remains one candidate model family. A graph-representation win is not evidence that the ontology is physically true.

Primary source: https://alexwg.org/publications/ChainTheoryCompendium.pdf

## 2. Research landscape and why this protocol differs

Three public research lines establish relevant but non-identical comparison points:

- **AI Feynman (2020)** focuses on symbolic regression of known equations, exploiting symmetry, separability, and compositionality. Its paper reports recovering all 100 equations in its Feynman set and 90% on a more difficult physics-based set. It is an important static equation-discovery baseline, not a benchmark for multi-step interactive experimentation. https://pmc.ncbi.nlm.nih.gov/articles/PMC7159912/
- **NewtonBench (2025 preprint)** evaluates interactive discovery across 324 tasks and 12 physics domains. It reports a sharp fall in symbolic accuracy as systems become more complex and as observational noise is introduced, and documents that tool assistance can change exploration/exploitation behavior. The new test must therefore include controlled active probing, noise, and a frozen tool/compute budget. https://arxiv.org/abs/2510.07172
- **EurekaBench (2026 preprint)** proposes 26 long-horizon tasks across neuroscience, computer science, chemistry, astrophysics, geophysics, and plasma physics, scoring 306 expert-verified scientific insights. It reports that predictive accuracy alone can overstate scientific discovery: agents can fit predictions while failing to derive broader insights. Our benchmark must score predictive transfer and rule-level insight separately. https://arxiv.org/abs/2610.00492

These papers have different tasks, protocols, models, and metrics. Their published scores are not directly comparable and must not be presented as a shared leaderboard. This protocol is intended to complement rather than reproduce them.

## 3. Narrow v1 domain: coupled spring–mass networks

Start with one small classical domain so the ground truth and invariants are independently checkable. Do not begin with a universal graph engine or a cross-domain physics claim.

For a graph of masses and springs, use an explicit reference model such as the equation:

**M x''(t) + C x'(t) + K(G, k) x(t) = f(t)**

where:
- M is a positive diagonal mass matrix;
- C is a declared damping matrix;
- K(G, k) is built from the known edge-level spring constants and connectivity;
- G is the spring connectivity graph;
- f(t) is a declared external force schedule;
- each scenario declares units, initial conditions, integrator/reference settings, output cadence, and noise model.

The generating equations and parameters are hidden from discovery agents for held-out runs but retained by the evaluator. For v1, choose stable parameter ranges and explicit numerical tolerances. Use the same numerical outputs and observation budget for each candidate model family.

Do not let a reference model with privileged hidden state count as an equal-budget discovered model. Oracle access is for evaluating correctness and for diagnosing the difficulty of each scenario only.

## 4. Candidate model families

### A. Known-equation baseline / oracle

Where equations are supplied, this establishes achievable numerical accuracy and checks the simulator. Where equations are hidden, the oracle's parameters and topology are sealed from the discovery process. Report its privileged status separately.

### B. Conventional graph plus local dynamics

Use typed nodes (masses) and typed edges (springs/dampers) with explicit local equations and conventional state/dependency semantics. This is a strong structured baseline, not a straw man. It receives the same observations, interventions, compute allowance and held-out cases.

### C. Generative relational grammar

Use typed finite entities, typed links, explicit composition/rewrite rules, shared endpoints/copy semantics where defined, a measurement/read map, and separate acyclic construction provenance. The grammar is inferred or selected using training scenarios only.

The representation may encode ordinary classical systems, but its semantics must not conflate:
- graph sharing with quantum entanglement;
- a graph edge with a physical interaction unless its semantics say so;
- a read/observation map with a quantum measurement;
- graph-construction order with physical time;
- a fitted state with a measured state.

## 5. Scenario family and leakage-resistant splits

Generate an immutable corpus containing a preregistered number of scenarios per family and difficulty. Before running any candidate, publish the corpus schema, seed commitments, split rules, and score definitions. Keep test labels/ground-truth graph and parameter values inaccessible to discovery code.

Required families:

1. **Seen primitives / unseen compositions:** train on smaller networks; test held-out combinations of the same allowed building blocks.
2. **Held-out topology:** test graph families not seen during training, including different node count / degree / cycles within declared limits.
3. **Intervention transfer:** change an edge, mass, damping level or force schedule after the model has been fit; test predictions beyond passive interpolation.
4. **Partial observability:** hide a subset of nodes or sensors; require uncertainty when alternative structures are observationally equivalent.
5. **Noise and missingness:** apply seeded sensor noise, irregular sampling and missing observations under frozen profiles.
6. **Heterogeneous grammar:** mix systems generated by incompatible rule families; a shared-grammar learner must be allowed—and expected when appropriate—to reject a shared grammar.
7. **Confounding and observational equivalence:** create pairs of systems that agree under one observation schedule but diverge under a discriminating intervention.
8. **Integrity / negative controls:** tampered manifests, duplicate observation identifiers, altered units, wrong initial conditions, and post-cutoff features must fail closed.

Split by whole graph/topology, generating-rule family, and intervention scenario. Never use random rows from the same trajectory as the sole evidence of generalization.

## 6. Active experiment selection

The discovery loop should distinguish **exploration** from **optimization of the current candidate**.

Each candidate probe records:
- exact current hypothesis/grammar frontier;
- intervention proposal and admissibility constraints;
- predicted outcomes under each live candidate model;
- expected discrimination or information gain under a named policy;
- resource cost and measurement noise assumptions;
- actual outcome, observation provenance, and resulting posterior/model change.

At least three policies should be compared:
1. uniform/random admissible intervention;
2. uncertainty-seeking / model-disagreement intervention;
3. expected-discrimination intervention, which targets where leading hypotheses make different predictions.

All policies receive the same total physical/simulated query budget. If the discovery agent uses a code tool, record each call and cap its budget. Include an ablation with tools disabled and with tools enabled under the same wall-clock/token/compute envelope where feasible. Do not treat a larger compute budget as an algorithmic gain.

## 7. Metrics (do not collapse into one score)

### Predictive fidelity
- held-out trajectory MAE and RMSE in declared physical units;
- normalized error only with the normalizer frozen on training data;
- uncertainty interval coverage and width;
- prediction quality on interventions, not only observational holdouts.

### Structural recovery
- topology precision, recall and edge F1 where topology is identifiable;
- parameter recovery error where the parameter is identifiable;
- equivalence-class score where multiple structures induce the same observed behavior;
- fraction of cases correctly marked non-identifiable / unresolved.

### Scientific discovery
- exact recovery of generating rules when uniquely identifiable;
- rule recovery under observationally equivalent models;
- successful prediction of a preregistered discriminating intervention;
- compositional transfer to held-out graph assemblies;
- invalid-rule rejection on deliberately heterogeneous families.

### Physical validity
- constraint/conservation violations;
- numerical stability and solver failures;
- units/dimensional consistency failures;
- out-of-domain detection and fail-closed rate.

### Cost and operational usefulness
- simulator calls to reach a valid model;
- wall-clock time, CPU/memory, model/grammar description length;
- candidate code/interpreter calls and associated cost;
- time/cost to add a new primitive or compose a new domain instance;
- reproducibility of model and result receipts.

Report per-family results and confidence intervals. A single aggregate score is not acceptable for qualification: better prediction cannot offset violation of a known physical invariant, and a shorter grammar cannot offset failure on held-out interventions.

## 8. Preregistered targets

These are **proposed go/no-go targets**, not current results:

- Exact same inputs and equivalent resource budgets for A/B/C comparisons; all exceptions are recorded.
- At least 30 distinct held-out topologies/compositions in the first qualification corpus, spanning three preregistered difficulty bins; deterministic seeds and case identities remain frozen after the protocol is locked.
- At least 90% of well-posed deterministic replay cases produce byte-identical canonical scenario/result receipts on repeat execution under the same pinned build and environment, unless the contract explicitly declares a non-deterministic output; such output is not silently included in the deterministic subset.
- Zero accepted results after known manifest tampering, unit mismatch, target leakage, or use of post-cutoff features.
- At least 90% empirical coverage for nominal 90% prediction intervals on adequately sized held-out groups, with interval width also reported. The calibration criterion is evaluated with uncertainty, not as a hard rule on tiny samples.
- For an algorithmic superiority claim, predefine a practical effect threshold before scoring. Initial proposal: at least 10% reduction in held-out intervention error or at least 20% fewer simulator queries to a matched quality threshold, reproduced on a second held-out topology family, **with no degradation in physical-validity or integrity gates**. These are proposed targets to stress-test, not universal physical laws.
- If the corpus cannot support a powered comparison, report uncertainty / inconclusive rather than relaxing thresholds after seeing the output.

## 9. Formal integrity properties

The harness must reject or expose:
- node/edge storage-order and identifier renaming that changes physical predictions without a declared semantic reason;
- use of ground-truth topology/parameters from test fixtures by discovery code;
- stale or altered scenario identities and observation manifests;
- unit conversion mistakes and dimensionally invalid models;
- observations after the declared decision cutoff;
- duplicate trials or post hoc exclusion without a predeclared reason;
- hidden changes to candidate budgets, solver tolerances or scoring;
- forced single-grammar output when the heterogeneous control makes a shared grammar false;
- a claim of physical support based solely on deterministic simulation.

A hash proves which bytes were scored, not that a measurement is true. A theorem or symbolic equivalence check proves only the encoded statement under its assumptions. A simulation receipt is not experimental evidence.

## 10. Integration boundaries

- **Symtropy** owns deterministic scenario execution, simulator configuration, numerical outputs, replay and scenario receipts.
- **Symthaea** owns candidate grammar generation, hypothesis tracking, active probe selection and scientific interpretation.
- **Verification layer** audits artifact identity, score implementation, invariants, provenance completeness and exact-target runs.
- **Chain Theory** is one encoded candidate grammar / hypothesis profile; it does not gain physical authority or privileged access to test data.

Reuse [Symtropy ENG-SYNTH #1172](https://github.com/Luminous-Dynamics/symtropy/issues/1172) and [SCALE-CELL-001 #1356](https://github.com/Luminous-Dynamics/symtropy/issues/1356). Reuse [Symthaea SCI-GRAMMAR-001 #8919](https://github.com/Luminous-Dynamics/symthaea/issues/8919) for shared-rule recovery and ensure the endpoint is not duplicated. This issue should remain a representation/model-family comparison on the Symtropy side, not another generic discovery or replay engine.

## 11. Execution sequence

1. Freeze v1 protocol, corpus schema, and seed/split policy.
2. Implement one reference spring-mass generator and analytic/numerical invariants.
3. Implement baseline A/B outputs under exact matched inputs.
4. Add candidate C only through an explicit interface and frozen grammar profile.
5. Add controlled active-probe policy bake-off.
6. Run synthetic null, mutation, and leakage controls.
7. Execute on an exact commit SHA and store canonical machine-readable receipts.
8. Inspect failure cases, run ablations, then expand to RC and thermal networks only if v1 provides a useful signal.
9. Require a separately operated replication before any public claim of superiority.

## 12. Claim ceiling

No benchmark has been executed in this document. No candidate architecture is currently shown to outperform existing methods. The initial target is to produce a reliable matched comparison that can establish where a relational grammar helps and where it fails. Even a clear win would support the representation as a useful engineering/discovery method, not prove Chain Theory's proposed ontology or a deeper-than-information metaphysics.
