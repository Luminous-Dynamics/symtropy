# Social Dynamics Laboratory — proposal v0.1

**Status:** research proposal and benchmark contract; not yet a validated social simulator.  
**Home:** Symtropy research docs.  
**Related systems:** Symthaea (bounded cognition), Mycelix (trust, evidence, commitments, governance), Sol-Atlas (space and service capacity).

## Thesis

Build a social-systems laboratory that predicts and explains *changes caused by interventions*, rather than optimizing for agents that merely produce plausible stories.

The research gap is not another agent with a personality prompt. It is operational validity: a model should only be trusted for the specific mechanism and outcome it has been tested to predict. A 2025 review of LLM-based agent-based models found validation practices often weakly connected to the proposed mechanisms; this project therefore treats empirical validation, calibration, sensitivity analysis, and falsification as first-class deliverables. See the reading list below.

## Paradigm shifts

1. **From synthetic people to causal mechanisms.** Each behavioral rule is an explicit, ablatable hypothesis. Compare it with simpler alternatives under the same seeds, scenarios, and evidence.
2. **From fixed graphs to co-evolving relationships.** Interaction changes trust, tie persistence, information exposure, and future interaction opportunities. Social structure is state, not just input data.
3. **From disembodied agents to resource-bounded actors.** Time, capacity, distance, service availability, and material scarcity constrain decisions. A social preference cannot manufacture water or travel time.
4. **From one forecast to an uncertainty-bearing ensemble.** Report predictive distributions, calibration, sensitivity, and non-identifiability. Several mechanisms may explain the same observed pattern.
5. **From realism by demonstration to falsifiable, held-out prediction.** Use temporal holdouts, external validation where ethically available, negative controls, and independent implementations of canonical baseline models.
6. **From opaque optimization to legible intervention comparison.** Record what intervention changed, which assumptions caused the predicted result, who benefits, who bears costs, and where the result becomes uncertain.
7. **From central authority to plural evidence.** Mycelix-inspired evidence and commitment records may allow participants to inspect, challenge, correct, and revoke claims without making one opaque reputation number the truth about a person.

## Division of responsibilities

- **Symtropy:** event-driven world, resource constraints, repeatable scenario runs, time and space, intervention application, and system-level metrics.
- **Symthaea:** bounded decision policies, belief/confidence update mechanisms, attention, and optional cognitive-state representations. High-cost or generative policies are optional comparators, not the default.
- **Mycelix:** evidence provenance, consent, commitments, dispute outcomes, and governance-rule representations. Do not treat social reputation as an intrinsic human value or a universal ranking.
- **Sol-Atlas:** spatial and infrastructure layers such as travel cost, housing, water, electricity, service access, and capacity. The public repository is currently named sol-atlas-leptos.
  
These are proposed interfaces, not a claim that this complete pipeline already exists.

## Minimal shared model

### Entities

- **Agent/household:** needs, resource endowment, available time/capacity, current policy state, uncertainty, and chosen actions. Keep sensitive identity attributes out unless scientifically necessary and ethically approved.
- **Directed tie:** interaction opportunity, observed evidence, confidence, tie persistence, and consent/visibility constraints. Trust should be an estimate about a defined interaction, not a universal scalar for a person.
- **Institution:** rule set, authority boundaries, allocation protocol, appeals/dispute process, and enforcement cost.
- **Resource/service node:** amount, capacity, spatial location, reliability, access cost, and dependencies.
- **Event:** interaction, evidence observation, resource shock, action, rule change, dispute, movement, or service recovery.

### Event order

Each tick must use a versioned and documented order: (1) observe available signals, (2) compute bounded decisions from prior state, (3) resolve actions against shared capacity, (4) emit evidence/events, (5) update ties and institutional state, (6) record outputs. If model semantics require another order, that is a versioned model change, not a silent refactor.

### Minimal dynamics

The first model family should be intentionally simple and inspectable:

- Trust estimate updates are bounded and evidence-weighted, with missing and contested observations represented explicitly.
- Cooperation is a decision affected by expected benefit, cost, reciprocity, resource constraints, and confidence in the applicable commitment.
- Ties can strengthen, weaken, form, or expire according to explicit transition rules.
- Institutions change feasible actions and resource allocation; they are not merely narrative labels.
- Shocks affect resource and service capacity, then propagate through dependency and interaction networks.

No single equation is mandated as ground truth. The benchmark must compare competing mechanisms and include null models. A model must not infer a psychological trait merely because an outcome correlates with it.

## First falsifiable question

**Question:** Under what conditions does noisy, evidence-based trust updating improve cooperation and recovery after a shared-service shock, compared with (a) no explicit trust mechanism, (b) static trust, and (c) a simple reciprocity heuristic?

**Hypothesis, not result:** evidence-based trust updating can improve coordination when observations are informative and verification cost is lower than the expected loss from failed commitments. Benefits may vanish or reverse when observations are noisy, verification is expensive, or adversaries can cheaply tamper with signals. The current prototype models signal tampering only; institutional capture and a formal appeals/dispute process are later, separately testable extensions.

### Initial scenario: community water-service shock

Use synthetic agents first. Each household has a water need, time budget, contribution capacity, and access to a shared water-service network. A shock reduces capacity for a defined interval. Households can contribute repair work or resources under alternative coordination and governance rules.

Compare interventions while holding initial topology, resources, policy parameters, exogenous random channels, and seed list constant. The current --sweep mode varies observation noise (0.00/0.20), observation cost (0.00/0.35), signal-tampering rate (0.00/0.20), and shock capacity (0.25/0.65) one factor at a time around the baseline. The network stays a degree-four ring lattice; network-density and interaction effects are deferred to a future implementation. Do not describe the signal-flip parameter as a complete institutional-capture model.

Do not begin with a city-wide “digital twin” claim. Start with this small scenario and prove that model behavior, event logs, and metrics are reproducible.

## Benchmark suites

1. **Canonical dynamics and invariants:** deterministic replay; bounded state; conservation of resources; action feasibility; stable behavior under no-op interventions.
2. **Network co-evolution:** degree distribution, reciprocity, clustering, tie survival, mixing, and community structure over time.
3. **Cooperation and public goods:** contribution distributions, free-riding, recovery after shocks, subgroup disparities, and outcomes under changes to information/commitment rules.
4. **Information/opinion dynamics:** adoption and reversal rates, cascade sizes, polarization measures, uncertainty calibration, and exposure-response relations.
5. **Infrastructure-coupled resilience:** unmet need, access inequality, time-to-recovery, failure cascades, and resource throughput.
6. **Performance and scale:** throughput, peak memory, runtime distribution across seeds, and instrumentation overhead at 100, 1,000, 10,000, and (only after prior tiers pass) 100,000 agents.

Use canonical models such as voter dynamics, bounded-confidence opinion updates, simple contagion, preferential attachment/configuration-model graph baselines, and public-goods games as *reference models*. First reproduce expected results; only then claim a new mechanism improves on them.

## Proposed go/no-go gates

These are target gates for this project's first benchmark, not established industry standards or results already achieved.

- **Reproducibility:** same binary, scenario, configuration, seed, and event order produce identical event/state digests on the same target platform.
- **Invariant suite:** 100% pass for feasibility, bounded values, declared resource conservation, event ordering, and schema validation.
- **Baseline coverage:** at least one named null/simple baseline for every behavioral claim; all methods receive identical data access and evaluation windows.
- **Predictive value:** on a preregistered held-out task, seek at least a 10% relative reduction in the primary proper scoring loss versus the strongest eligible simple baseline, with a paired 95% bootstrap interval excluding no improvement. If sample size cannot support this, report the uncertainty and do not claim superiority.
- **Calibration:** for nominal 90% predictive intervals, target empirical coverage of 85–95% on held-out cases, while also reporting interval width. Wide intervals alone must not count as success.
- **Mechanistic value:** remove the proposed coupling in an ablation. The full model must improve the predeclared outcome, or the mechanism is not justified by that experiment.
- **Robustness:** report seed variance, parameter sensitivity, subgroup metrics where lawful/ethical, negative controls, and all failed scenario families.
- **Performance:** establish measured baselines on a published reference machine before setting a speedup claim. Optimize only without changing model semantics or weakening validity checks.

## Reproducibility receipt

Every run should record:

- model version and source commit SHA;
- scenario ID/version and configuration digest;
- data provenance, licence, time period, transformations, and content digest;
- seed list and RNG/version details;
- hardware, OS, compiler/runtime version, build features, and thread count;
- start/end status, warnings, metrics, output digest, and failures;
- model assumptions, exclusions, known validity boundary, and intended decision context.

The receipt is evidence of what ran; it is not evidence that the model is valid.

## Safety and governance

- Synthetic populations first; aggregate or consented observations for later calibration.
- No individual-level consequential scoring, covert citizen surveillance, or automatic eligibility/policing decisions.
- Do not turn inferred trust into a persistent universal reputation score.
- Maintain opt-out, contestability, provenance, retention limits, and role-based access when real data are introduced.
- Treat results as conditional projections under stated assumptions, not deterministic predictions about actual people.
- Report distributional effects and uncertainty instead of collapsing flourishing, fairness, resilience, and efficiency into one unexamined scalar.

## Reading list

- Larooij & Törnberg (2025), “Validation is the central challenge for generative social simulation: a critical review of LLMs in agent-based modeling,” *Artificial Intelligence Review*: https://doi.org/10.1007/s10462-025-11412-6
- Grimm et al. / replication work using ODD and NetLogo (2025), reporting successful replication of 15 of 18 reviewed models: https://doi.org/10.1016/j.ecolmodel.2024.110967
- NetLogo benchmarking methodology: seeded models, repeated runs, warm-up handling, and stable result estimates: https://github.com/netlogo/netlogo/wiki/Benchmarking
- Mesa framework capabilities and model analysis: https://mesa.readthedocs.io/stable/
- Repast reference, including separation of model/data/visualization and spatial/network projections: https://repast.github.io/docs/RepastReference/RepastReference.html

## Next implementation stages

Before trying to claim real-world social validity, follow the rights-aware external benchmark plan in [EXTERNAL_VALIDATION_PLAN.md](EXTERNAL_VALIDATION_PLAN.md). It identifies human public-goods candidate data, a separate LLM-agent dataset, license gates, grouped holdout splits, and empirical scoring rules.

1. Keep this protocol and primary outcomes fixed before looking at any result that could tune model assumptions.
2. The initial dependency-free Rust reference scaffold now lives at research/social-dynamics-reference. Run its tests and CSV experiment on a Rust-equipped host; revise any discovered model or implementation defects openly.
3. Reproduce at least two canonical models and add negative controls before adding cognitive complexity.
4. Treat commitments/evidence as an intervention arm; measure whether it beats simpler policies and whether the effect survives ablations.
5. Calibrate only on permitted data, lock a temporal holdout, and report uncertainty before claiming external predictive value.
6. Only after validity gates pass, connect run artifacts to Symtropy, Mycelix, Symthaea, and Sol-Atlas adapters.

**Current status:** proposal only. No simulation results, speedup, or out-of-sample improvement are claimed by this document.
