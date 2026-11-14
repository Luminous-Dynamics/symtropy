# Animal Causal Identifiability Benchmark V0

Status: draft synthetic contract  
Research snapshot: 2026-10-19  
Issue: [ANIMAL-CAUSAL-001 #2186](https://github.com/Luminous-Dynamics/symtropy/issues/2186)

## Purpose

Determine whether an animal simulation predicts what changes under a controlled intervention—not merely whether it reproduces observational trajectories, labels or correlations.

This benchmark complements:
- [Animal replay evaluation contract](ANIMAL_REPLAY_EVALUATION_V0.md) and its task-specific, paired receipt protocol;
- [Active sensing benchmark](ANIMAL_ACTIVE_SENSING_BENCHMARK_V0.md) and its 36-case sensor/budget oracle;
- the shared Living World Observatory and ANIMA's signal/percept boundary.

It is not a new simulation runtime, causal-inference library or qualification ladder.

## Paradigm shift: observational fit is not mechanism validity

Two candidate mechanisms can explain the same observational distribution while predicting different interventional outcomes. A simulator that exploits a proxy, memorizes a scene, leaks intervention labels or sees hidden state can look accurate while failing under even a small controlled perturbation.

Make causal evaluation a separate qualification axis:
1. Pin a structural causal model or controlled simulator mechanism and its assumptions.
2. Produce observational training data from that model; deliberately include cases where competing models are observationally hard to distinguish.
3. Freeze intervention assignments, exogenous-noise seeds and expected outcomes before model selection.
4. Evaluate each candidate's predictions under the interventions it was not tuned on, with null/placebo controls and paired independent units.
5. Do not promote synthetic structural-model success into evidence that the modeled mechanism is biologically true.

## Machine-readable oracle

The first fixture is [ANIMAL_CAUSAL_IDENTIFIABILITY_ORACLE_V1.json](ANIMAL_CAUSAL_IDENTIFIABILITY_ORACLE_V1.json), a 27-case synthetic contract corpus. It contains expected response direction, null effects, control invariants, resource/authority boundaries, split fairness and receipt integrity. The cases are expected-answer specifications, not current simulator results.

The corpus deliberately separates:
- **world intervention:** changes an actual parent or mechanism input;
- **observation intervention:** changes sensing/visibility while holding world dynamics fixed;
- **decision-cost intervention:** changes an action cost while holding percepts fixed;
- **authority attack:** tries to expose hidden state or intervention labels to the controller;
- **evaluation intervention:** changes a preregistered causal factor while keeping candidate and baseline comparison fair.

These categories must not be silently combined. Sensor ablation can change behavior through information access; it is not equivalent to changing the environment's causal mechanism.

## Required controlled cases

- **Confounding:** an observed cue/outcome association induced by a shared cause disappears under a valid intervention when there is no direct causal edge.
- **Observational equivalence:** two candidate mechanisms fit the training observations but separate under intervention. An observation-only score cannot decide between them.
- **Null and placebo controls:** interventions with no causal path remain within a preregistered equivalence margin. Null controls measure false positives and calibration, not only statistical power.
- **Dose and saturation:** dose-response direction and saturation are checked only over the region where the frozen mechanism specifies them. No unbounded extrapolation.
- **Temporal order and lag:** a cue after an outcome cannot cause that past outcome; known-lag effects are measured within a declared window.
- **Perception versus dynamics:** occlusion or sensor ablation changes evidence quality, not hidden world truth. The controller cannot be rewarded for sensing a source it could not detect.
- **Closed-loop behavior:** movement changes body state and later sensor geometry; those feedback effects and their costs must appear in the replay trace.
- **Multi-agent mechanism:** change a partner's action while holding the focal agent's initial state fixed, and report interaction terms separately from the focal agent's own dynamics.
- **Shortcut resistance:** change a non-causal correlated proxy while holding causal parents constant. Any predicted causal outcome change is a failure unless the declared SCM says otherwise.
- **Generalization:** evaluate held-out combinations of interventions/noise/occlusion without exposing the intervention labels to the controller.
- **Authority and leakage:** deny hidden intervention label queries; split by episode/animal/scene rather than correlated frames; missing or failed effects remain incomplete/failed.
- **Fair baselines:** candidate and baseline share initial units, exogenous noise/seed family, intervention schedule, sensor limits and declared compute/training budgets. An unequal budget must be explicitly labelled as a trade-off.

## Metrics

Report each intervention family independently:
- **Effect estimation:** signed average treatment/intervention effect, estimation error on known-answer synthetic cases, and uncertainty coverage.
- **Direction:** sign agreement and rank correlation over predeclared non-null effects.
- **Null behavior:** false-positive rate and equivalence-test results for null/placebo interventions.
- **Calibration:** interval coverage, proper score or calibration curve where the estimator supports it, plus missingness/failure rates.
- **Counterfactual prediction:** only where the reference outcome is defined by the frozen synthetic mechanism or a justified measured experiment.
- **Policy consequence:** task utility, decision flips and action/resource costs when the intervention is expected to affect action.
- **Robustness:** worst-case performance across held-out intervention combinations and explicit subgroup/taxon tables where sample sizes permit.

No universal “causal realism” score. A candidate cannot compensate for a false-positive null effect or hidden-state leakage by improving an unrelated task metric.

## Comparison protocol

1. Pre-register the causal diagram/structural assumptions, intervention, expected effect direction, null margin, primary metric, independent unit, uncertainty method, stop rule and holdout.
2. Generate matched exogenous-noise scenarios for candidate and baseline where that pairing is appropriate; bind the canonical independent-unit membership via the replay receipt.
3. Compare a simple no-causal-model baseline, a correlation-only predictor, a declared domain-rule model and the candidate. A clairvoyant hidden-truth policy, if included, is a diagnostic ceiling only.
4. Require exact candidate/baseline run IDs, code/config digests, evaluator/toolchain/platform, intervention schedule, split and compute/training-data budgets.
5. If a candidate matches observational data but fails predeclared interventional known answers, its **causal** qualification fails regardless of observational performance.
6. Publish null controls, failed/missing episodes, effect intervals, ablations and the full per-intervention results. Do not hide them behind one aggregate.
7. Only after synthetic qualification choose a source-specific empirical intervention with a documented protocol, exact version, permission/authority and an independent experimental unit.

## Go/no-go targets

- All 27 frozen synthetic oracle cases match their expected outcomes.
- Zero causal effect outside the preregistered equivalence margin for synthetic null/placebo cases.
- Correct direction for every known-answer non-null effect, with uncertainty reported.
- Zero controller access to arbitrary hidden state or intervention labels.
- No causal claim based solely on observational fit.
- Deterministic replay reproduces the intervention and result receipt in deterministic mode.
- Paired comparisons bind exact units and exact baseline identities and enforce declared budget policy.
- Synthetic qualification and empirical animal validation remain separate labels.

These are target acceptance gates, not achieved results. Sample-size-dependent targets and equivalence margins must be chosen in advance using the task's outcome scale and a power/sensitivity analysis.

## Next implementation steps

1. Build a compact Rust structural-causal fixture runner, with known-answer cases defined independently from implementation.
2. Keep evaluator truth, intervention labels and hidden mechanism internals in a separate test-only evaluator interface.
3. Emit controller-visible percepts only through the existing Basin/ANIMA signal path and bind action/percept provenance to the animal replay receipt.
4. Implement randomized/intervened pairs and null controls before optimizing any policy.
5. Reuse the paired-effect receipt protocol from PR #2145; do not create parallel result authority, split or evidence stores.
6. After compile/test/Clippy qualification, run the 27 synthetic cases, then adversarial negative controls.
7. Only then select a permitted empirical experiment and reproduce its baseline before making comparative claims.

## Non-claims

This draft does not claim the current Symtropy simulator has a biologically calibrated causal model, passed these 27 cases, predicts a real animal's counterfactual response, or beats state of the art. The first oracle validates only its own synthetic contract; real-biology claims require independently reviewed empirical evidence.
