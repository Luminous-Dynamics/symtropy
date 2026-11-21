# CIVDYN Attractor and Regime Qualification Contract V0

**Status:** Research specification opened 2026-11-20  
**Tracking:** Symtropy issue [#6441 — CIVDYN-ATTRACTOR-001](https://github.com/Luminous-Dynamics/symtropy/issues/6441)  
**Parent:** [#3390 — calibrated socio-ecological civilization dynamics](https://github.com/Luminous-Dynamics/symtropy/issues/3390)  
**Contract family:** Civilizational Systems Science Interchange v0.1  
**Scope:** regime identification, finite-horizon basin estimation, transitions, evidence and visualization

This document defines a qualification target. It does not claim that Symtropy currently identifies real-world civilizational attractors, that any particular society is in a named regime, or that a future trajectory can be predicted from a simulation.

---

## 1. Executive thesis

The next useful step is not a larger civilization simulation. It is a disciplined way to determine what a model's dynamics actually support.

A trajectory plot is not an attractor analysis. A cluster in a projection is not an equilibrium. A long plateau is not proof of stability. A repeated pattern under periodic external forcing is not automatically an autonomous limit cycle. Similar aggregate statistics do not establish similar transition pathways.

The target is an evidence-qualified **regime landscape**:

~~~text
declared system boundary
  + state variables and units
  + institution/mechanism profile
  + behavioral-population profile
  + external forcing and shocks
  + parameters and initial-condition ensemble
  + model, solver and data digests
        |
        v
candidate regimes
  + stability / persistence diagnostics
  + finite-horizon basin estimates
  + transition and hysteresis evidence
  + uncertainty and alternative models
  + applicability domain and counterexamples
        |
        v
reviewable visualization and reproducible receipt
~~~

This lets us explore questions such as:

- Which combinations of resource constraints, infrastructure and institutional rules produce persistent scarcity?
- Under what conditions does a community recover from a shock rather than enter a prolonged degraded regime?
- Can two arrangements deliver similar average welfare while differing sharply in dependency concentration, rights-floor breaches or recovery pathways?
- Does an emergency mechanism damp a disturbance, or does it change the institution so that later disturbances become more damaging?
- Are observed regime changes robust across plausible behavior models and structural model families?

The answers must remain conditional on the model, data, assumptions and horizon that were actually qualified.

## 2. Architectural boundary

This work composes existing authorities. It must not create a competing civilization simulator, a duplicate game-theory implementation, a second evidence ledger, or a new governance authority.

| Project / authority | Owns | This contract consumes or produces |
|---|---|---|
| Symtropy CIVDYN | Executable socio-ecological dynamics, resource and infrastructure coupling, solver/replay | Regime and transition qualification over exact model runs |
| Symtropy model adequacy | Purpose-aligned adequacy, calibration/validation splits, uncertainty and out-of-domain decisions | Adequacy status required before policy interpretation |
| Symthaea | Causal/strategic hypotheses, mechanism analysis, power/dependency observables, research orchestration | Candidate explanations and typed analysis requests; never ground truth by assertion |
| Mycelix Civilization Twin | Institutional/economic/governance contracts, authority, provenance, rights, claims and decisions | Mechanism profiles, qualified governance facts and outcome evidence |
| Sol-Atlas | Spatial-temporal projections, geography, historical context and human inspection | Views of qualified spatially situated states and transitions |
| Independent oracle | Known-answer fixtures and qualification checks separate from the candidate method | Acceptance/rejection evidence |

### 2.1 Existing programme to compose

- Symtropy [#3390](https://github.com/Luminous-Dynamics/symtropy/issues/3390): small bounded socio-ecological world, explicit stocks/flows, common initial conditions and scientific baselines.
- Symtropy [#4315](https://github.com/Luminous-Dynamics/symtropy/issues/4315): model adequacy before policy interpretation.
- Symtropy [#4392](https://github.com/Luminous-Dynamics/symtropy/issues/4392): matched adequacy, causal-policy and runtime evaluation.
- Symtropy [#4407](https://github.com/Luminous-Dynamics/symtropy/issues/4407): network topology qualification beyond degree-only summaries.
- Mycelix [#4816](https://github.com/Luminous-Dynamics/mycelix/issues/4816): Civilization Twin subsystem boundaries, causal events and multiscale state.
- Mycelix [#5699](https://github.com/Luminous-Dynamics/mycelix/issues/5699): endogenous institutional feedback, lock-in and cycle-breaking.
- Mycelix [#4824](https://github.com/Luminous-Dynamics/mycelix/issues/4824): behavioral pluralism and expectation profiles.
- Mycelix [#5512](https://github.com/Luminous-Dynamics/mycelix/issues/5512): counterexample-guided policy falsification and scenario discovery.
- Symthaea [#7190](https://github.com/Luminous-Dynamics/symthaea/issues/7190): power, agency, dependency and rights-floor stress laboratory.
- Symthaea [#3054](https://github.com/Luminous-Dynamics/symthaea/issues/3054) and [#3095](https://github.com/Luminous-Dynamics/symthaea/issues/3095): institutional game assurance and reproducible experiment orchestration.
- Sol-Atlas [#11](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/11), [#12](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/12) and [#450](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/450): historical evidence, temporal frontiers and evidence-aware scenario layers.

If an upstream authority lacks a required field, create a narrowly scoped interface task there. Do not silently copy its logic into this workstream.

## 3. Scientific object and definitions

### 3.1 Declared dynamics

For a discrete-time model, use the conceptual form:

~~~text
x[t+1] = F(x[t], u[t], e[t]; theta, m, b)
~~~

For continuous-time models:

~~~text
dx/dt = f(x, u, e; theta, m, b)
~~~

Where:

- x is the explicitly declared state vector;
- u is an intervention or endogenous control/action;
- e is exogenous forcing and shock input;
- theta is a versioned parameter profile;
- m is the institutional/mechanism profile;
- b is the behavioral-population profile;
- F or f is the exact model revision.

The state definition is not a neutral choice. Every run must state the system boundary, omitted processes, aggregation level, time unit and update schedule.

### 3.2 Regime versus attractor

A **regime** is an operationally defined pattern of system behavior over a declared observation window. Regimes may be useful even where asymptotic attractors cannot be established.

An **attractor** is a set or invariant distribution approached by trajectories under specified dynamics and conditions. Its existence, stability and basin are mathematical/model properties, not visual labels.

A **basin of attraction** is the set of initial conditions that converge to a specified attractor under the declared dynamics. In finite stochastic experiments we generally estimate a finite-horizon basin probability or volume, not an exact infinite-horizon basin.

A **transition** is a change between declared regimes or invariant sets under a specified intervention, parameter shift, shock or endogenous process. A visual crossing in a reduced projection is insufficient evidence by itself.

A **metastable regime** is a state or region with a measurable finite residence-time distribution before transition. It should not be mislabeled an asymptotically stable attractor.

### 3.3 Required classification vocabulary

The evaluator must be able to return one of the following labels, with a supporting diagnostic record:

| Class | Interpretation | Minimum evidence |
|---|---|---|
| Fixed-point attractor candidate | Trajectories approach an equilibrium | Equilibrium residual, local stability analysis where applicable, multi-initial-condition behavior |
| Periodic / limit-cycle candidate | Trajectories approach a periodic orbit | Period/orbit diagnostics, transient exclusion, stability evidence |
| Stochastic stationary regime | The distribution becomes approximately time-invariant under a stationary stochastic model | Stationarity diagnostics and distributional comparison across windows/seeds |
| Metastable regime | Long-lived state with non-negligible transition probability | Residence-time and transition estimates, horizon disclosure |
| Chaotic / aperiodic invariant candidate | Bounded aperiodic dynamics with appropriate sensitivity and invariant-set evidence | Multiple diagnostics; label only within validated method scope |
| Absorbing state | Once entered, the declared dynamics do not leave it | Formal transition/transition-kernel property or bounded exhaustive verification |
| Transient cluster / plateau | Temporary concentration or slow movement, not established as an attractor | Explicit non-attractor classification or unresolved status |
| Forced / non-autonomous tracking | Apparent cycle or regime is imposed by time-varying forcing | Forcing-aware analysis; do not call it an autonomous attractor without justified augmentation |
| Unresolved / insufficient evidence | The evidence cannot support classification | Explicit missing diagnostic, insufficient horizon, sample or model adequacy |

These labels are not interchangeable. A model can contain more than one attractor, basin, metastable region or coexisting regime. Different subsystems and aggregation levels may also have different regimes.

### 3.4 Non-autonomous and stochastic dynamics

If the governing rules, parameters or forcing vary with time, do not apply autonomous-system language without qualification. Options include:

- explicitly augmenting the state with the forcing phase/state;
- using a justified Poincaré or stroboscopic map for periodic forcing;
- evaluating conditional or time-local regimes;
- describing a forced tracking response instead of an autonomous attractor.

For stochastic dynamics, a stationary distribution, an invariant measure, a metastable state and a sample-path cluster are different objects. The evaluator must record which one it is estimating.

### 3.5 Stability and resilience are not synonyms

Local stability around an equilibrium does not imply robust recovery from large shocks. A system can be locally stable but have a small basin. A system can have a large basin but recover slowly. A resilient service may be maintained by substitute capacity even when the underlying infrastructure is not restored.

Report separately:

- local stability;
- finite-horizon persistence;
- basin size / basin probability;
- recovery time;
- service continuity;
- time spent below declared capability or rights floors;
- sensitivity to parameter, topology and behavior changes;
- transition probability under the declared forcing model.

## 4. State representation: keep the outcome vector plural

The model should not compress civilization into a universal scalar. State variables should be chosen to answer the declared research question and preserve dimensional meaning.

A first bounded basin-scale or community-scale fixture can use several independently reported families:

| Family | Example state variables | Important separation |
|---|---|---|
| Essential services | water, energy, food, housing, health or transport service coverage | Demand, capacity, delivered service and unmet service are distinct |
| Physical stocks and flows | stored water, extraction, regeneration, energy generation, storage, materials | Stocks are not flows; units must reconcile |
| Infrastructure | capacity, availability, maintenance backlog, repair time, common-cause exposure | Nominal capacity is not usable service |
| Economic capability | productive capacity, income, purchasing power, access to credit, enterprise continuity | Balances are not structural power |
| Power and dependency | control rights, bottlenecks, substitution depth, switching cost, veto/agenda concentration | Centrality is not legitimacy |
| Governance | participation, contestability, decision latency, delegated authority, emergency-power expiry | Formal authority is not effective authority |
| Distribution | group-specific access, affordability, burden, worst-decile and inequality measures | Mean outcomes can conceal subgroup harm |
| Ecological state | regeneration, depletion, pollution, habitat or ecological-pressure indicators | Proxy measures must retain their method and scope |
| Knowledge/capability | skill diffusion, maintenance knowledge, innovation and adaptation lag | Adoption is not population-wide capability |
| Information | observation coverage, missingness, verification capacity, privacy burden | High observability is not zero information bias or privacy cost |

Do not aggregate unlike units (for example, m³, kWh, kg, currency and service rates) into a generic discrepancy value. Use typed variables, explicit transformations and family-specific diagnostics.

## 5. Identification workflow

Every benchmark run follows the same auditable sequence.

### Step 1 — Freeze the experiment contract

Declare and hash:

- scenario ID and revision;
- system boundary, excluded interactions and state schema;
- model code and dependency lock / build identity;
- parameters and permissible ranges;
- initial-condition sampling design;
- institution/mechanism profile;
- behavior and expectation profile;
- network topology and identity rules, where relevant;
- forcing and shock profiles;
- random number generator, algorithm version and seed derivation;
- solver, tolerances, step size, runtime and numeric precision;
- calibration, validation, challenge and held-out splits;
- metrics and classification thresholds.

A change to any one of these creates a different experiment identity. Results must not be silently pooled across different identities.

### Step 2 — Verify model adequacy for the declared purpose

Follow the existing CIVDYN adequacy authority. Test prior reachability, calibration fit, held-out predictive adequacy, relevant strata and structural alternatives.

The adequacy result must support at least:

- adequate for a declared bounded validation domain;
- inadequate on named statistics or strata;
- not assessed because evidence or predictive draws are insufficient;
- out of domain for the declared question.

Failure to reject a discrepancy is not proof that a model is true. A model with poor adequacy must not be rescued by attractor visualization or policy optimization.

### Step 3 — Find candidates using appropriate methods

Use the simplest defensible method for the system:

- analytic equilibrium and local Jacobian analysis for small systems;
- continuation / bifurcation analysis when the equations and assumptions permit;
- Poincaré sections or orbit diagnostics for candidate periodic behavior;
- distributional convergence and stationarity checks for stochastic systems;
- finite-state transition analysis where a Markov representation is valid;
- multi-initial-condition simulation for basin estimation;
- model-family comparison when structural uncertainty matters.

A learned embedding or clustering method may help find candidate regions. It must not be the final authority for attractor classification.

### Step 4 — Test from many initial conditions

Use a declared initial-condition design spanning the supported domain. Sample across boundary regions and plausible state variations, not only near a favorable seed.

For stochastic models, repeat with independent declared seed streams and common random numbers across paired policy arms where appropriate. Actor-local or domain-separated streams should prevent one agent's extra action from shifting unrelated agents' random draws.

### Step 5 — Estimate basins and transitions

For a finite-horizon study, define a classification rule such as:

- entered a candidate regime within horizon H;
- remained within its diagnostic neighborhood for a declared dwell time;
- satisfied residual / distributional / orbit tolerances;
- did not cross a declared failure or rights floor.

Then estimate basin probability under the specified initial-condition distribution. A basin fraction is conditional on that distribution; it is not a universal property of a society.

Report boundary uncertainty and unclassified initial conditions. Do not turn unknown samples into negative or positive labels.

### Step 6 — Falsify the candidate interpretation

Attempt to disconfirm the classification with:

- longer horizons;
- alternative sampling frequency and solver tolerances;
- perturbations around the candidate basin boundary;
- changed behavior profiles;
- alternative parameter and model structures;
- correlated shocks and common-cause failures;
- exogenous forcing removal or augmentation;
- topology-preserving and topology-changing network controls;
- shuffled labels, irrelevant dimensions and projection controls;
- metric-gaming / Goodhart perturbations.

A search algorithm can propose counterexamples. Independent fixtures and held-out challenges decide whether a proposed failure is real within the benchmark contract.

### Step 7 — Publish a qualification receipt

The receipt records the experiment identity, outputs, per-metric results, uncertainty, rejected alternatives, counterexamples, limitations and exact source digests. A human-readable visualization must link back to this receipt.

## 6. Known-answer benchmark suite

Build small exact or independently analyzable systems before large synthetic civilizations. The benchmark should distinguish the ability to identify a mathematical object from the ability to model a plausible social process.

| ID | Fixture | Known answer / negative control | What it tests |
|---|---|---|---|
| AR-01 | Scalar stable equilibrium | Exact equilibrium and local stability | Equilibrium residual and stability classification |
| AR-02 | Scalar unstable equilibrium | Exact equilibrium with unstable perturbations | False stability detection |
| AR-03 | Bistable double-well system | Two attracting states and an independently known separatrix | Basin estimation and boundary uncertainty |
| AR-04 | Periodic oscillator | Known orbit and period | Limit-cycle / orbit diagnostics |
| AR-05 | Finite-state stochastic chain | Known stationary distribution | Distributional convergence and calibration |
| AR-06 | Metastable finite-state process | Known transition and residence-time law | Distinguish long-lived metastability from absorption |
| AR-07 | Absorbing-state process | Known absorbing state | Absorption and reachability |
| AR-08 | Periodically forced oscillator | Periodicity caused by external forcing | Avoid false autonomous limit-cycle claims |
| AR-09 | Slow transient plateau | No asymptotic attractor in the declared domain | False-attractor rate and abstention |
| AR-10 | Drifting-parameter system | Time-varying rules prevent stationary interpretation | Detect non-autonomy / out-of-domain conditions |
| AR-11 | Coupled resource commons | Hand-checkable stock/flow conservation and threshold | Institutional and physical dynamics |
| AR-12 | Institutional lock-in loop | Known synthetic feedback graph and controlled policy effects | Endogenous feedback and cycle-breaking |
| AR-13 | Power bottleneck | Equal balances but concentrated bridge / verifier authority | Separate material distribution from structural power |
| AR-14 | Matched marginal / different topology | Same degree sequence, different connectivity and pathways | Avoid aggregate-fit errors |
| AR-15 | Behavioral model multiverse | Same institution under several declared behavior models | Model sensitivity and behavioral pluralism |

### 6.1 Analytical controls

AR-01 through AR-07 should have independent analytic or exhaustive oracles wherever possible. Their truth must not be generated by the same candidate implementation being evaluated.

For small systems, enumerate the complete supported domain. For numerical systems, freeze tolerances and publish an independent derivation or reference implementation. A test that calls the implementation's own helper to compute both expected and actual values is not an independent oracle.

### 6.2 Institutional controls

Institutional scenarios should use matched initial conditions:

~~~text
same world + same behavior + institution A
same world + same behavior + institution B

same world + same institution + behavior A
same world + same institution + behavior B
~~~

First scenarios could include:

- shared aquifer allocation under alternative governance rules;
- a community with one critical water/energy supplier and later substitute entry;
- emergency authority with an explicit sunset, review and renewal process;
- procurement or verification bottleneck and independent oversight;
- delegation network with false-name and coalition stress;
- a maintenance backlog that causes capacity loss and longer repair times;
- an ecological resource whose regeneration responds to extraction;
- a surveillance/observability increase that changes both coordination and reporting behavior.

Synthetic fixture results must remain explicitly synthetic.

### 6.3 Negative controls

At minimum:

- one seed must not be reported as a distribution;
- a transient plateau must not be called an attractor;
- forcing-induced oscillation must not be mislabeled autonomous;
- a 2D projection must not override full-state diagnostics;
- same marginal distributions with different temporal pathways must be distinguishable;
- same degree sequence with different connectivity must be distinguishable;
- parameter drift must not be silently absorbed into a stationary model;
- unknown or unobserved state must not be converted to zero;
- a model outside its validated domain must return out-of-domain;
- a candidate classifier that is wrong but confident must fail the false-attractor control.

## 7. Metric registry and target ladder

Metrics are versioned and reported separately. Do not publish a single weighted attractor score or civilization ranking.

### 7.1 Core metrics

| Metric | Definition / interpretation | Target-setting approach |
|---|---|---|
| Attractor-class precision / recall | Correct class among independently labelled known-answer cases | Exact fixture parity first; publish per-class confusion matrix |
| False-attractor rate | Fraction of negative controls incorrectly labelled attractors | Zero on the frozen supported-domain negative controls |
| Stability classification | Correct local stability class; eigenvalue / multiplier error where defined | Match analytic oracle within predeclared numeric tolerance |
| Equilibrium residual | Distance from declared equilibrium under model equations | Tolerance declared from model scale and solver |
| Period / orbit error | Error in period and orbit representation | Predeclared relative/absolute tolerance |
| Basin-volume error | Difference between estimated and known finite-domain basin measure | Report sampling design and confidence interval |
| Basin interval coverage | Coverage of nominal basin-probability intervals over repeated fixture draws | Empirical coverage with binomial uncertainty |
| Transition threshold error | Parameter value / range at a known regime change | Report bracket width, bias and uncertainty |
| Stationary-distribution distance | Declared distribution distance against known invariant distribution | Choose metric by state type; no metric substitution without notice |
| Residence-time error | Difference in metastable dwell-time distribution | Distributional score and interval coverage |
| Predictive calibration | Calibration of finite-horizon outcomes | Proper scoring rules, calibration plots and interval coverage |
| Robustness envelope | Range of initial conditions / parameters / behaviors for which conclusions persist | Report a vector of ranges, not one pass rate |
| Abstention quality | Whether unsupported cases are rejected or marked unresolved | False-certainty rate and coverage-risk curve |
| Scaling | Runtime, memory, samples and review cost | Matched hardware, workload and accuracy/adequacy budget |
| Human interpretation | Whether users correctly read the plot, uncertainty and assumptions | Blind comprehension tasks with known answers |

For estimated probabilities, intervals must be computed from the declared sampling design. For empirical calibration, report both nominal and observed coverage and uncertainty; do not promise exact 90% or 95% coverage on a tiny test corpus.

### 7.2 Release gates

**Gate A — mathematical correctness**

- All frozen deterministic known-answer fixtures pass.
- Zero false attractor declarations on the frozen transient / forced / insufficient-horizon controls.
- Invalid, malformed or non-finite inputs fail closed.
- Deterministic runs reproduce canonical outputs for an identical experiment receipt.

**Gate B — finite-sample reliability**

- Basin and transition estimates include sampling uncertainty.
- Coverage and error targets are pre-registered before challenge evaluation.
- Seed, initial-condition distribution and sample count are explicit.
- No confidence interval is made artificially narrow by discarding unresolved samples.

**Gate C — model-family robustness**

- A policy or institutional conclusion is not called robust if it reverses across qualifying held-out model families.
- Behavior assumptions are crossed with institutional mechanisms rather than hidden in them.
- Correlated shocks and common-cause structure are retained.
- Applicability is stated per population, region, horizon, outcome and model family.

**Gate D — operational comparison**

- Runtime is measured at a matched workload and hardware profile.
- Scaling results state their precision, adequacy and memory envelope.
- A faster but inadequate simulator does not win a scientific benchmark.
- Human review cost and ability to inspect a counterexample are recorded.

### 7.3 Aspirational SOTA targets

These are goals, not current achievements:

1. **Known-answer correctness:** 100% on the frozen, supported deterministic oracle suite.
2. **Negative-control discipline:** zero false attractor claims on supported forced/transient controls.
3. **Reproducibility:** byte-identical canonical receipts for identical deterministic experiment identities, where platform/runtime contract permits.
4. **Calibration:** nominal interval coverage supported on held-out replicates with adequate sample size; report binomial intervals.
5. **Robustness:** no universal policy-advantage claim unless it survives pre-registered alternative model families, behavior profiles and challenge scenarios.
6. **Human observability:** users can trace a displayed regime, edge or basin estimate to inputs, assumptions, evidence and counterexamples.
7. **Performance:** match specialized frameworks on comparable subproblems before claiming a speed advantage; publish Pareto frontiers of runtime, memory, adequacy and review burden.

## 8. Industry and research benchmark crosswalk

We should compare the system against established tools in their strongest domains rather than build a straw-man comparison.

| Reference | Strong comparison role | What to match | What Luminous must add or prove |
|---|---|---|---|
| OpenSpiel | Formal game environments and strategic algorithms | Equilibrium / regret / exploitability known-answer tests, game coverage and runtime | Connect strategic models to evidence-bound institutional and physical state without duplicating OpenSpiel mathematics |
| DeepMind Melting Pot | Multi-agent social generalization | Held-out social dilemmas and unfamiliar-agent tests | Add institutional change, resource constraints, power/dependency and interpretable counterexample receipts |
| NetLogo | Accessible ABM and micro-to-macro exploration | Matched small/medium ABM scenarios, run reproducibility and usability | Add stronger model adequacy, typed evidence/units, power/rights outputs and independent qualification |
| Repast / Repast HPC | Open-source ABM and distributed-scale execution | Matched agent counts, workload semantics, hardware and output parity | Demonstrate comparable scaling without weakening semantic and scientific validation |
| AnyLogic | Multimethod modeling (ABM, system dynamics, discrete event) and applied tooling | Same model boundary, assumptions, scenario, calibration and outputs | Demonstrate that Rust-first components can interoperate with physical/accounting constraints and public evidence receipts |
| System dynamics reference models | Stock-flow and feedback-loop behavior | Conservation, thresholds, equilibrium and sensitivity | Compose with heterogeneous agents and explicit institutional/network mechanisms |

Useful primary references:

- [OpenSpiel project and documentation](https://github.com/google-deepmind/open_spiel) — formal game framework and algorithms.
- [Melting Pot: a multi-agent reinforcement learning benchmark for generalization](https://arxiv.org/abs/2107.06857) — social interaction and held-out partner generalization.
- [NetLogo](https://ccl.northwestern.edu/netlogo/) — agent-based modeling and emergent phenomena.
- [Repast Suite](https://repast.github.io/) and [Repast HPC](https://repast.github.io/repast_hpc.html) — open-source agent-based modeling, including high-performance computing.
- [AnyLogic modeling methods](https://www.anylogic.com/use-of-simulation/) — agent-based, system-dynamics and discrete-event simulation.
- [ODD protocol update](https://www.jasss.org/23/2/7.html) — structured description and replication of agent-based models.

### 8.0.1 Competitive landscape update (2026-11-21)

The comparison set must track the current versions, not static names:

- **OpenSpiel 2.0:** the 14 July 2026 release announcement reports more than 120 games, 19 newly added games, structured state/action/observation and trajectory interfaces, JAX/AlphaZero work, language-model examples and an MCP example. The release materially raises the bar for strategic-environment coverage and LLM interoperability. We should use its documented game definitions and exact release revision as a differential reference rather than duplicate its algorithms. Reference: [Introducing OpenSpiel 2.0 (14 July 2026)](https://www.gcbgb.ca/introducing_open_spiel_2.0/); [current OpenSpiel documentation](https://openspiel.readthedocs.io/en/latest/intro.html).
- **Social-network simulation literature:** a 25 October 2026 survey organizes 97 studies into five model families, catalogs 48 implementations, 21 benchmarks and 22 metrics, and identifies validation, multilevel representation and scalability as active challenges. This supports a benchmark design that crosses network structure, diffusion, opinion dynamics, rule-based ABM and LLM-agent simulation rather than evaluating only one family. Reference: [A survey of social network simulation in the LLM era](https://doi.org/10.1016/j.knosys.2026.116947).
- **Operational validation of LLM-agent communities:** a 2026 study evaluates a Reddit-like technology forum with 30 replications and compares activity patterns, interaction-network structure and textual convergence to an empirical reference. We should adopt the multi-run and multi-construct discipline, while extending it to typed institutional relationships, causal pathways, rights/service floors, power/dependency and evidence lineage. Reference: [Towards operational validation of LLM-agent social simulations](https://doi.org/10.1140/epjds/s13688-026-00674-x).

These references do not establish that any one benchmark is sufficient for civilizational modeling. The design consequence is that each capability family needs its own validation protocol and external reference. Dialogue similarity, aggregate fit, network-marginal fit, strategic performance and regime identification are different claims and must remain separate.

### 8.1 Fair-comparison protocol

For each external comparison:

1. Publish a common model boundary and semantic mapping.
2. Use the same initial conditions, scenario parameters, shocks and observation horizon.
3. Use common random numbers where the frameworks expose equivalent RNG semantics; otherwise document the limitation and use paired replicate design.
4. Match hardware, thread count, precision, warm-up, runtime and replication budgets.
5. Compare outputs with independent oracles, not only internal framework checks.
6. Report runtime and memory alongside scientific adequacy, calibration and output fidelity.
7. Publish adapters, source revisions, commands, raw results and excluded cases.
8. Do not infer a universal ranking from one workload or one hardware configuration.

The likely differentiation is not a claim to replace every specialist simulator. It is a composed workflow: formal strategic analysis, multi-scale physical/economic/institutional dynamics, source-qualified power and dependency representation, model-adequacy gates, geospatial context and accountable governance interfaces. That differentiation remains a hypothesis until the crosswalk is run.

## 9. Visualization: Civilizational Regime Explorer

The UI should be a projection and inspection surface over qualified model results, never the source of truth.

### 9.1 Views

**A. State-space explorer**

- Select axes and show units and value ranges.
- Show trajectories and direction of time.
- Display projection/embedding method and lost dimensions.
- Allow inspection of original state vectors.
- Overlay forcing, interventions and regime labels.
- Separate observed, inferred and scenario layers.

**B. Basin explorer**

- Show initial-condition samples and their eventual finite-horizon classifications.
- Use uncertainty shading for sparsely sampled boundaries.
- Show unresolved samples rather than silently assigning them.
- Permit alternative initial-condition distributions and report the induced basin estimate.

**C. Regime atlas**

- Display candidate regimes as nodes and qualified transitions as directed edges.
- Annotate transition conditions, evidence and uncertainty.
- Preserve multiple possible outgoing transitions.
- Display coexisting regimes across model families rather than forcing a single pathway.

**D. Power and dependency overlay**

- Link regime changes to changes in control, decision rights, substitution depth, switching costs, veto/agenda control and essential-service dependencies.
- Keep power dimensions separate rather than inventing a universal power number.
- Show where equal balances coexist with concentrated effective control.

**E. Distribution and rights-floor view**

- Report subgroup outcomes, worst-decile outcomes and rights-floor breaches separately.
- Make mean improvements visible alongside distributional harms.
- No aggregate improvement may visually hide a subgroup collapse.

**F. Model and evidence inspector**

- Select any node, trajectory, transition or metric and open its run receipt.
- Show source/claim lineage, units, temporal validity, model digest, parameter profile, limitations and counterexamples.
- Enable comparison against alternative models and assumptions.

### 9.2 Visualization invariants

- A 2D embedding does not establish an attractor.
- Visual proximity does not prove causal or institutional connection.
- A crisp basin line cannot be drawn when only a noisy finite-sample estimate exists.
- Colors must not imply confidence, truth or moral value.
- Every display carries scenario/observed/inferred labeling and time/horizon.
- Missing, unknown, withheld and measured-zero remain distinct.
- The same exact model-run receipt must produce the same exported view data.
- All interactive selections must be reproducible and exportable.

## 10. Interoperability receipt (conceptual V0)

The canonical output should be serializable without tying the contract to a specific renderer or database.

~~~text
RegimeQualificationReceiptV0 {
  schema_version,
  experiment_id,
  scenario_ref,
  model_ref,
  model_digest,
  system_boundary,
  state_schema_ref,
  unit_registry_ref,
  parameter_profile_ref,
  mechanism_profile_ref,
  behavior_profile_ref,
  forcing_profile_ref,
  initial_condition_design_ref,
  rng_profile,
  solver_profile,
  calibration_split_ref,
  validation_split_ref,
  challenge_split_ref,
  classification_method_ref,
  metric_schema_ref,
  candidates: [
    {
      candidate_id,
      class,
      supporting_diagnostics,
      contradicting_diagnostics,
      applicability_domain,
      stability_result,
      finite_horizon,
      basin_estimate,
      transition_estimates,
      unresolved_fraction,
      uncertainty,
      counterexamples
    }
  ],
  adequacy_disposition,
  claim_class,
  limitations,
  source_digests,
  output_digest
}
~~~

The conceptual type is not yet a finalized Rust API or JSON schema. Field names, numeric representations, units and serialization rules must be reviewed against the shared Interchange contract before implementation.

### 10.1 Claim classes

Every conclusion should carry a claim class:

- **Formal theorem:** proved under stated mathematical assumptions.
- **Bounded verification:** exhaustively checked within declared finite bounds.
- **Simulation result:** observed for the exact model, inputs, seed set and execution recorded.
- **Empirical observation:** sourced measurement with its own method, scope and uncertainty.
- **Observational hypothesis:** a possible explanation requiring further testing.
- **Scenario assumption:** an intentionally chosen condition, not a fact or prediction.

Never upgrade a simulation result into a theorem or empirical fact through prose or visualization.

## 11. First bounded experiment: a shared-resource basin

Start with one resource basin and a small number of communities. Do not begin with a whole planet.

### World

- A finite renewable resource stock with declared regeneration and depletion units.
- Several communities with heterogeneous essential demand.
- Infrastructure with finite capacity, maintenance backlog and repair time.
- A trade/exchange network with one controllable bridge in a positive-control case.
- Governance alternatives for resource allocation and maintenance.
- Multiple behavior profiles: optimizing, satisficing, precautionary and reciprocal, each explicitly parameterized.
- Exogenous drought and energy interruption scenarios with declared timing and magnitude.

### Matched institutional arms

- local commons rule with explicit monitoring and graduated response;
- centralized allocation;
- delegated / federated allocation;
- cooperative sharing with a defined dispute/appeal rule;
- a deliberately fragile bottleneck rule as a negative control.

These are experimental treatments, not claims that one arrangement is always best.

### Primary outcomes (separate)

- resource-stock trajectory and conservation residual;
- group-specific essential-service coverage;
- unmet demand and duration below the declared service floor;
- repair backlog and recovery time;
- dependency asymmetry and substitute depth;
- decision/verification burden;
- distributional outcomes and rights-floor breaches;
- regime and transition classification;
- sensitivity to behavior and model structure.

### Counterfactuals

- Same initial world, vary governance rule.
- Same governance rule, vary behavior model.
- Same world and rule, remove the critical bridge.
- Same world and rule, add a substitute supplier.
- Same world, apply an emergency authority and compare sunset/review with non-expiring authority.
- Same world, vary correlated shock structure rather than assuming independent failures.

The first release should establish known-answer and negative controls before any empirical claim. A later pilot can use properly licensed, source-pinned local data, for example a water/energy/service system in South Africa, only after scope, comparability and custodian permissions are qualified.

## 12. Paradigm-shift research hypotheses

The benchmark should make these falsifiable rather than turn them into doctrine.

1. **Power is not wealth:** equal balances can coexist with highly unequal control of bridges, verification, agenda-setting or essential infrastructure.
2. **Resilience is substitutability plus recovery:** nominal redundancy can fail when alternatives share a common cause or cannot provide an equivalent service.
3. **Institutional feedback is endogenous:** a response to scarcity can either expand future capability or deepen dependency and lock-in.
4. **No mechanism dominates universally:** mechanism ranking can change across behavioral populations, information regimes and network structures.
5. **Attractor claims are model-relative:** changing state aggregation, system boundary, external forcing or institution can change the identified regime landscape.
6. **Plural metrics prevent hidden trade-offs:** a regime may improve average output while harming rights floors, ecological regeneration or exit options.
7. **Adaptive institutions need contestability:** the capacity to correct decisions depends on information, appeal, independent verification and authority boundaries.
8. **Observability has costs:** more monitoring can improve coordination while increasing privacy burden or changing reporting behavior.

A hypothesis that fails is valuable: record the counterexample, revise the scope and preserve the prior result.

## 13. Threat model and governance boundaries

The same tools that expose structural dependencies can be misused to manipulate or control people. The work should default to institutional and system-level analysis rather than personal profiling.

- Do not infer private motives as facts.
- Do not create a moral ranking of people, cultures, institutions or countries.
- Do not infer corruption or wrongdoing from a structural risk indicator.
- Do not expose sensitive individual dependency or negotiation data by default.
- Preserve consent, purpose limitation, access control, correction and contestation.
- Keep expertise, reputation, contribution, identity, rights and authorization distinct.
- A model's output is not legal authority, a governance mandate or permission to act.
- Mycelix remains the authority for authorized institutional decisions; Symtropy cannot execute policy changes merely because a simulated outcome is favorable.
- Human participants and affected communities must be able to challenge scope, data, assumptions and interpretations.

## 14. Delivery sequence

| Tranche | Deliverable | Exit condition |
|---|---|---|
| A0 | This contract and issue boundary | Accepted scope; ownership and non-claims reviewed |
| A1 | Frozen analytic fixtures and independent oracles | Stable/unstable, bistable, periodic, stochastic, metastable, absorbing, forced and transient cases have known answers |
| A2 | Typed receipt and metric registry | Experiment identity binds all material revisions and units |
| A3 | Candidate detection and abstention adapter | Correct known answers; false-attractor negative controls pass |
| A4 | Finite-horizon basin / transition estimator | Error and interval coverage reported on held-out samples |
| A5 | Institutional scenario adapter | Matched policy/behavior experiments with power and rights outcomes |
| A6 | Civilizational Regime Explorer projection | Every displayed result is reversible to a qualified receipt |
| A7 | Empirical pilot | Source-pinned calibration and external validation within a named domain |

This is a dependency order, not a calendar promise. Do not wait for unrelated CI runners to do research, define fixtures or prepare independent oracles. Conversely, a queued/skipped workflow is not a pass.

## 15. Definition of done for V0

V0 is complete when:

- The classification vocabulary and non-attractor / abstention rules are unambiguous.
- The known-answer matrix and independent-oracle plan are frozen.
- The receipt binds model, data, boundary, state, units, parameters, institution, behavior, forcing, seeds, solver and metric versions.
- Every output states its finite horizon and applicability domain.
- The evaluator preserves outcome vectors and does not generate a universal civilization score.
- At least one institutional feedback scenario and one power-bottleneck counterexample are represented as explicitly synthetic fixtures.
- The external framework comparison is specified with matched workload and hardware controls.
- No result claims empirical civilization forecasting, universal mechanism superiority or SOTA performance before an independently reproducible benchmark establishes it.

---

## References and benchmark anchors

1. [OpenSpiel](https://github.com/google-deepmind/open_spiel) — formal game theory and multi-agent environment reference.
2. [Melting Pot](https://arxiv.org/abs/2107.06857) — held-out social interaction and generalization benchmark.
3. [NetLogo](https://ccl.northwestern.edu/netlogo/) — agent-based modeling and emergence.
4. [Repast Suite](https://repast.github.io/) and [Repast HPC](https://repast.github.io/repast_hpc.html) — open-source and distributed agent-based modeling.
5. [AnyLogic simulation methods](https://www.anylogic.com/use-of-simulation/) — agent-based, system-dynamics and discrete-event modeling.
6. [ODD protocol update](https://www.jasss.org/23/2/7.html) — description, rationale, design concepts and replication of agent-based models.
