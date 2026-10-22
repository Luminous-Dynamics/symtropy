# Animal Active-Sensing Benchmark V0

Status: draft benchmark/implementation contract  
Research snapshot: 2026-10-19  
Tracks: [ANIMAL-VOI-002 #2171](https://github.com/Luminous-Dynamics/symtropy/issues/2171), [ANIMAL-REPLAY-001 #2149](https://github.com/Luminous-Dynamics/symtropy/issues/2149)

## Purpose

Measure whether a simulated animal can choose *when, where, and how to sense* under limited energy, time, receptor capacity and uncertain observations—and whether that policy improves behavior over simple sensing baselines without receiving hidden world truth.

This specializes the shared Living World Observatory [PR #245](https://github.com/Luminous-Dynamics/symtropy/pull/245); it is not a parallel qualification ladder. Reuse the Basin signal vocabulary and non-omniscient fauna perception oracle [PR #246](https://github.com/Luminous-Dynamics/symtropy/pull/246), ANIMA's epistemic-state boundaries [PR #1190](https://github.com/Luminous-Dynamics/symtropy/pull/1190), and the evidence/paired-comparison receipt path [PR #2145](https://github.com/Luminous-Dynamics/symtropy/pull/2145). Symthaea's existing expected-value-of-sample-information work ([#8098](https://github.com/Luminous-Dynamics/symthaea/pull/8098), [#8114](https://github.com/Luminous-Dynamics/symthaea/pull/8114), [#8116](https://github.com/Luminous-Dynamics/symthaea/pull/8116)) is the preferred decision-theoretic substrate; do not duplicate an EVSI engine merely to create this benchmark.

## Paradigm shift

Move from **animals consume a complete sensor feed** to **animals manage their own observability**.

The environment owns hidden canonical truth and emitted/propagated signals. The agent receives only allowed percepts, their uncertainty and their provenance. An agent may decide to resample, reorient a receptor, move to improve sensing geometry, revisit a location, switch modality, wait for a cue, or abstain. Each sensing action pays explicit time/energy/attention cost and can return no signal, a noisy signal, an ambiguous signal or a stale signal.

The evaluator can inspect world truth to grade the episode. The controller API must not provide arbitrary hidden-state queries, exact source coordinates unless sensed, future events, or evaluator labels. A clairvoyant policy is allowed only as a separately labelled diagnostic ceiling; it cannot appear as a fair competing method or evidence of animal capability.

## Evaluation hierarchy

Use the shared Observatory Q-levels:

- Q0 static schema/type and authority contract.
- Q1 exact deterministic replay and frozen oracle.
- Q2 invariance and differential comparison.
- Q3 controlled ecological/sensorimotor intervention.
- Q4 presentation causal legibility.
- Q5 human review.

Source rights, split independence, hidden-state isolation and budget matching are prerequisites, not extra Q-levels. An active-sensing oracle pass is not empirical animal validation.

## Benchmark world and agent boundary

Use a small deterministic headless scene with versioned geometry, signal emitters, propagation/attenuation, occluders, ambient noise, multiple receptor modalities, movement affordances and explicit action costs. Begin with synthetic signal kinds already represented in Basin; do not invent a second sensor ontology.

The controller input consists only of:
- its current body-relative state/allowed action affordances;
- previously received percepts and their provenance;
- a fallible belief/memory state and its age/uncertainty;
- currently available sensor/action choices and declared costs;
- its homeostatic resource budget.

It must not receive:
- the evaluator's complete source-emission table or hidden canonical environment state;
- future emissions, target labels or the expected outcome for the current fixture;
- free sensing, free movement or uncharged modality changes;
- a test-set seed/condition label that directly reveals the correct action.

The evaluator is separate, read-only with respect to the controller, and binds scene/scenario inputs, controller code/config, seed, baseline identities, and observations into replay receipts.

## Policies to compare

Use the same initial scenes, allowed percept history, intervention assignments, sensor-quality profile and episode budget.

1. **Passive/static:** receive the default sensor schedule or only an initial observation.
2. **Periodic:** resample on a fixed interval, independent of uncertainty.
3. **Reactive/event-triggered:** resample when a documented signal/uncertainty threshold fires.
4. **Information-seeking:** choose the available observation/action with the best declared expected utility or expected value of sample information (EVSI), net of sensor/time/energy cost.
5. **Resource-aware EVSI:** value information by whether it is expected to change the decision enough to justify its cost, with a finite homeostatic budget and explicit stopping/abstention.
6. **Clairvoyant diagnostic ceiling:** reads hidden truth only in the evaluation harness. Publish separately; never count it as a compliant baseline win or as a candidate animal agent.

Baselines must identify exact code/config/run/toolchain/platform plus training-data and compute budgets. In a matched-budget task those budgets must match; an unequal-budget frontier experiment must declare that policy up front and cannot be reported as equal-resource SOTA.

## Frozen synthetic oracle

Freeze **at least 30 independently specified cases** before model tuning. The current 36-case machine-readable corpus is [ANIMAL_ACTIVE_SENSING_ORACLE_V1.json](ANIMAL_ACTIVE_SENSING_ORACLE_V1.json). Each case has an expected class of outcome and explicit percept/action/budget invariants authored independently of the implementation. All initial fixtures are synthetic contract cases, not measured animal observations or performance claims.

Required hostile conditions:

1. No emission: no percept, no information-gain credit and no cue-caused action.
2. Wrong signal kind for the active receptor: no compatible percept.
3. Signal present but fully occluded or attenuated below the declared threshold: no percept.
4. Source outside receptor field/orientation: no percept until action changes geometry.
5. Ambient noise masks a cue that was detectable in the paired quiet control.
6. Two emitters are observationally ambiguous: preserve multiple hypotheses or abstain.
7. A second observation disambiguates the source: reduce the correct uncertainty without retroactively claiming the first observation was certain.
8. A delayed percept keeps its original emission/observation times; it must not be silently treated as current.
9. Contradictory observations: belief update records conflict rather than deleting the earlier evidence.
10. A previously reliable cue is falsified: stale belief must be revised.
11. Sensor battery is exhausted: no zero-cost observation remains available.
12. Receptor sampling has a cooldown: repeated requests cannot bypass it.
13. Reorientation has explicit time and/or energy cost.
14. Moving for a better view costs energy/time and changes the physical body state.
15. A high-cost observation predicts no decision change: EVSI can rationally reject it.
16. A cheap observation has modest information but large decision value: a rational controller can prefer it.
17. A cue has high raw information but does not affect the current objective: raw entropy reduction must not be mistaken for utility.
18. No compatible signal exists for any available action: abstention remains valid and no hidden cue is fabricated.
19. A cue is deliberately misleading: the controller retains fallible confidence and can recover on disconfirmation.
20. A sensory modality is ablated: report effects on accuracy, uncertainty and resource use.
21. Receptor sensitivity is degraded: signal detection cannot improve unless another declared causal factor changes.
22. Higher source strength under an otherwise fixed path cannot reduce received signal strength.
23. A low-cost modality becomes unavailable mid-episode: the controller must replan with current affordances.
24. The same scene and seed replay exactly in deterministic mode.
25. Rendering on/off leaves canonical signals, sensing cost and task outcome unchanged.
26. Reordering an input fixture cannot change its canonical digest.
27. A different scene seed tests robustness without exposing the seed to the controller.
28. An unseen emitter configuration tests compositional generalization without changing measurement semantics.
29. A different noise/occlusion regime is held out from policy selection.
30. A controller attempts to request hidden truth: reject or deny the request and record the violation.
31. A controller invents a percept with no source-emission provenance: reject.
32. A missing or failed result is not treated as zero cost or zero task error.
33. The baseline has more sensing budget than the candidate under matched-budget policy: reject the comparison setup.
34. The source/observation receipt has unknown or revoked use permissions: block qualification.
35. A declared independent unit or paired unit-manifest digest is missing or malformed: comparison incomplete/rejected.
36. The same scene run with passive sensing and EVSI shows an apparently better score because EVSI received extra free samples: the fairness gate must reject the result.

These are contract cases, not claims that any current Symtropy controller has passed them.

## Metrics

Report separate, interpretable dimensions rather than one “animal intelligence” scalar.

| Dimension | Metrics | Required interpretation |
|---|---|---|
| Task outcome | Success rate, target/error distance, correct decision rate | Primary task observable and claim ceiling declared in advance |
| Sensing quality | Detection/false-percept rates, source association, response latency | Count no-signal and incompatible-receptor negative controls explicitly |
| Belief quality | Calibration/error, coverage or proper score where mathematically applicable | A hidden truth available only to the evaluator is not an agent input |
| Decision value | Paired utility change, VOI/EVSI prediction error when a known synthetic distribution exists | High entropy reduction is not automatically useful information |
| Resource cost | Sensor activations, movement distance, time, energy units, missed opportunities | All actions and repeated samples charged consistently |
| Robustness | Worst-stratum performance, abstentions, failure counts, held-out noise/occlusion/scene regimes | Do not hide degraded conditions in a pooled mean |
| Generalization | Leave-one-scene/source-pattern/regime-out results | A result on a familiar seed does not establish transfer |
| Runtime | p50/p95 decision latency, memory, simulation steps/sec | Same platform/config for matched-runtime claims |

Use independent episodes/animals/scenes as the statistical unit, not every correlated tick or emitted frame. Report sample counts, missingness, failed/abstained episodes and intervals per scenario family.

## Paired comparison protocol

For model-versus-baseline claims:
- Freeze start states, scene/seed family, interventions, sensor and energy budgets, primary outcome, objective direction, practical-effect threshold, non-inferiority margin, confidence level, uncertainty method and stop rule before opening the holdout set.
- Bind paired-unit membership to a canonical manifest digest; both policies must act on the declared matched units, not just share a broad dataset name.
- Use an independent unit such as an episode/animal/scene and estimate candidate-minus-baseline effects at that level.
- Require the paired mean difference to agree with the candidate and baseline aggregate means when both exist. If the evidence is missing/inconsistent, do not claim superiority.
- Classify outcomes as outperforms-by-margin, non-inferior, regresses-beyond-margin, uncertain, not-assessed or incomplete.
- Keep the clairvoyant oracle out of candidate/baseline wins. It is useful for diagnosing achievable headroom, not proving the learned controller is good.
- Preserve separate quality-vs-resource points. Declare a budget trade-off before comparison; never conceal extra training data, free samples, extra inference or a larger compute budget.

## Initial go/no-go targets

These are desired future gates, not achieved results.

- 100% agreement with the synthetic known-answer oracle.
- Zero false percepts for no-emission, incompatible-modality and below-threshold/occluded controls.
- Zero information-gain or task reward credit from an observation that did not occur.
- Zero controller paths that can query arbitrary hidden canonical state.
- Deterministic replay equality for the canonical sensor/event/decision receipt in deterministic mode.
- Matched-budget comparison requires identical declared training-data scope and compute budgets. Trade-off policy must be explicitly declared and reported as a trade-off, not a matched-budget win.
- Report paired intervals and uncertainty; an overlapping interval is uncertain, and a missing interval is not an implicit pass.
- Separate the synthetic oracle status from empirical reference-task qualification and the shared Observatory Q-levels.

Do not set a universal success-rate target until the task, episode difficulty, stochasticity and independent sample size are preregistered. Then select a meaningful target and non-inferiority margin with sufficient power.

## Implementation sequence

1. Add a compact synthetic scene+signal fixture and at least 30 independent expected-outcome cases.
2. Implement the fixture oracle and hidden-state-access negative control in Rust, reusing Basin signal and existing ANIMA/Symthaea EVSI types.
3. Record action costs and percept provenance into a canonical replay trace; bind to PR #2145's result receipt, baseline identities, matched-budget policy and paired-unit digest.
4. Run static/passive/reactive baselines first; compare EVSI/resource-aware policies only on the same observations, initial states, and resource limits.
5. Verify deterministic replay, rendering invariance, no-signal/hidden-state negative controls, and fair baseline setup before optimizing policy performance.
6. After synthetic qualification, review one precisely pinned, legally usable sensorimotor task from NeuroMechFly v2/FlyGym. The project's v2 API is a breaking rewrite, so release/API/task/measurement protocols must be explicitly pinned; its results are Drosophila-task-specific.
7. Reproduce the external reference baseline before attempting superiority or cross-taxon generalization claims.

## Non-claims

This document does not claim Symtropy currently has a qualified active-sensing controller, achieves a biological EVSI policy, simulates animal cognition, or beats any published baseline. Synthetic known answers establish evaluator behavior only. Empirical/causal claims require a rights-reviewed pinned reference task, held-out comparisons, calibrated uncertainty and appropriate scientific review.
