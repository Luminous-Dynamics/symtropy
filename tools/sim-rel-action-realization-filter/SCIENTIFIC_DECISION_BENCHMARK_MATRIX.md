# SIM-REL scientific decision benchmark matrix (2026)

This programme evaluates a scientific decision system as a complete closed loop:

1. What physical intervention was actually realized?
2. Which experiment should be performed for the decision at hand?
3. Which latent and model uncertainties remain after the measurement?
4. Is the conclusion justified under plausible model and apparatus shifts?
5. Can another verifier independently replay the evidence and certify the claim boundary?

The programme should compete on decision quality, execution fidelity, and evidence integrity—not information gain or task success alone.

## 1. Capability map and current external comparators

| Evaluation dimension | Strong external reference | What it evaluates | SIM-REL opportunity |
|---|---|---|---|
| Constrained sequential planning | COPEx, ICML 2026 | Amortized posterior/policy plus online multi-step scenario-tree planning under dynamic budgets and physical constraints | Exact finite-horizon decision/risk oracle; shared-policy robust worlds; replayable certificates |
| Adversarial decision stability | Harikumar et al., UAI 2026 | Sequential decision-aware design under hidden/adversarial outcome perturbations | Persistent worlds, adversarial counterexamples, policy stability, exact per-world risk constraints |
| Covariate shift and error amplification | R-IDeA, AISTATS 2026 | Representativeness, informativeness and error de-amplification under misspecification | Separate these diagnostics and measure downstream decision regret under sealed target shifts |
| Inferential/model misspecification | EGIG and model-error criterion, AISTATS 2026 | Distinguishes inferential error from data-generating model error; shows more information can harm under misspecification | Separate inferential and physical-model worlds; test whether the selected action remains justified |
| Physical lab safety | SafeLab, ICML 2026 | Trajectory-level safe success, calibrated assets, manipulation tasks and physical replay | Couple commanded/realized intervention receipts to scientific inference; never grade only final task completion |
| Embodied scientific discovery | EmbodiedLab, October 2026 | Agents operate apparatus, use instrument readings and discover physical properties across 30 experiments, five disciplines and 14 platforms | Verify actual intervention, readback and post-experiment apparatus state as first-class latent variables |
| Complete science campaigns | Autonomous Science Bench 0.1 | Budgeted design-build-test-learn campaigns with independent discovery and held-out-learning gates | Add independent evidence receipts, decision quality and model adequacy without conflating planner and full-agent performance |

Primary sources:
- COPEx (ICML 2026): https://proceedings.mlr.press/v306/guo26g.html
- Adversarial robust Bayesian decision making (UAI 2026): https://proceedings.mlr.press/v337/harikumar26a.html
- R-IDeA (AISTATS 2026): https://proceedings.mlr.press/v300/tang26d.html
- EGIG and model-error penalty (AISTATS 2026): https://proceedings.mlr.press/v300/callahan26a.html
- SafeLab (ICML 2026): https://proceedings.mlr.press/v306/bai26e.html
- EmbodiedLab (26 October 2026): https://arxiv.org/abs/2610.35385
- Autonomous Science Bench 0.1: https://www.autonomous-science-bench.ai/

The benchmarks are complementary, not interchangeable. In particular, COPEx is not just one-step EIG; SafeLab is not a scientific-discovery leaderboard; and a planner-only score is not comparable to an entire agent/model/tool/harness configuration.

## 2. Four benchmark tiers

### Tier A — Exact mathematical oracles

Purpose: make correctness falsifiable before scaling.

Families:
- Finite stochastic channels, exact Blackwell order and exact Le Cam deficiency.
- Approximate channel simulation with exact statewise TV residuals and decision-specific bounds.
- Randomized decision rules, abstention and raw joint-mass risk replay.
- Finite-horizon policy trees, shared-policy multi-world robust optimization and Pareto-frontier agreement.
- Action-realization models with dropped interventions, partial delivery, backlash and sensor/apparatus confounding.
- Sound affine/hybrid abstractions with explicit hazard intersection.

Controls:
- Exhaustive tiny domains where feasible.
- Independently implemented enumerators/replayers, not the same production routine called twice.
- Exact-rational agreement and deterministic receipts.
- Negative controls for false witnesses, impossible histories, invalid probabilities, hidden-world leakage and arithmetic/resource overflow.

Acceptance ambitions:
- 100% exact frontier, residual, witness and policy-replay agreement on supported domains.
- Zero false exact-identification claims on negative controls.
- Zero false safety/risk certification within the supplied model worlds.
- Zero under-approximation in analytically validated abstractions.
- Unsupported size must produce an explicit unsupported result, not a false proof or silent approximation.

### Tier B — Sealed synthetic model-misspecification campaigns

Purpose: stress decision quality when the data-generating process and inferential model differ.

Factorial axes:
- Inferential approximation error.
- Data-generating model-family error.
- Covariate/target-distribution shift.
- Omitted interaction and rare critical mechanism.
- Sensor calibration drift, dropout and biased readback.
- Dropped/partial interventions, action remapping and actuator backlash.
- Irreversible process change and apparatus degradation.
- Observation noise, censoring and delayed measurement.

Required controls:
- Correctly specified oracle.
- Inferential-only misspecification.
- Physical/data-generating-only misspecification.
- Both error sources simultaneously.
- Distribution shift without structural misspecification.
- OOD sealed families and rare-event controls.
- Cases where EIG wins, cases where robust decision-aware design wins, and cases where every method should abstain.

Compare, under identical experiment, hazard and compute budgets:
- Random/design-prior baseline.
- Standard EIG and expected information gain.
- Expected one-step Bayes-risk reduction.
- COPEx-style multi-step scenario-tree planning.
- R-IDeA acquisition.
- EGIG and model-error penalty.
- Sequential adversarial robust decision-aware design.
- SIM-REL exact finite oracle, robust planner and approximate scalable implementation.

### Tier C — Embodied apparatus and digital-twin campaigns

Purpose: establish that the planned intervention is what actually happened, instruments were interpreted correctly, and the apparatus remains safe.

Measure separately:
- Command success.
- Realized intervention fidelity.
- Instrument/readback fidelity.
- Scientific inference correctness.
- Final task success.
- Trajectory-level safe success.
- Post-task apparatus-state compliance.
- Recovery and safe-abort behavior.

Use SafeLab for trajectory safety and manipulation stress, and EmbodiedLab for apparatus interaction and scientific discovery. Keep the model, control policy, perception, planning and harness variants explicit. A successful goal reached through an unsafe trajectory is not a safe success. A plausible conclusion unsupported by actual instrument readings is not a scientific success.

### Tier D — Complete autonomous science campaigns

Purpose: test the entire system, not one isolated optimizer.

Use Autonomous Science Bench's separation of discovery and learning:
- Discovery gate: quality of selected/measured experiments.
- Learning gate: quality of ranking or predicting designs that were never measured.

Add independent gates for:
- Correctly interpreted realized interventions and readbacks.
- Decision loss / regret and uncertainty calibration.
- Safety and budget compliance.
- Reproducible evidence receipts and replay.
- Explicit abstention when evidence is insufficient.

Always report the complete configuration: model, agent, tools, policies, simulator/lab backend and harness. Keep public development tasks separate from sealed final holdouts.

## 3. Primary metrics

| Category | Metric | Why it matters |
|---|---|---|
| Decision quality | Expected terminal Bayes risk and regret | Optimizes the outcome that experiments are supposed to improve |
| Robustness | Worst-world risk, CVaR and decision-switch rate | Exposes fragile nominal decisions |
| Scientific honesty | False high-confidence conclusion rate; abstention precision/recall | Detects unjustified certainty |
| Model adequacy | Counterexample discovery precision/recall; omitted-family detection rate | Measures whether the agent can recognize its model is wrong |
| Experimental efficiency | Decision-risk reduction per unit experiment cost | More meaningful than bits per cost alone |
| Execution | Command-to-realization fidelity; partial/dropout detection | Prevents failed actions becoming false evidence |
| Measurement | Calibration/readback error and sensor-state attribution | Prevents biased instruments from changing scientific conclusions |
| Safety | Hazard probability, safe-success rate and apparatus-state violations | Captures trajectory risks rather than endpoint success |
| Learning | Held-out design ranking, calibrated predictive error and OOD performance | Separates discovery from generalization |
| Evidence | Replay agreement, receipt completeness and verifier disagreement | Makes claims independently auditable |
| Systems | Wall time, peak memory, compute cost and unsupported-size frequency | Measures whether exact guarantees can scale safely |

Report both the aggregate and each failure stratum. Do not hide rare but high-consequence failures in a global average.

## 4. Statistical protocol

- At least 30 paired seeds per synthetic family where practical; target 100+ for stable subgroup estimates.
- Paired seeds across every baseline, with identical observations, costs, budgets, model access and hazard limits.
- 95% paired confidence intervals and effect sizes, not only p-values.
- Sealed mechanism-family, apparatus-condition and target-distribution holdouts.
- Pre-register primary outcomes, stopping criteria and failure categories.
- Preserve all seeds, raw observations, realized-action receipts, instrument readbacks, policy traces and verifier outputs.
- Report the cases where EIG, COPEx, R-IDeA, EGIG or robust adversarial design wins.
- Distinguish algorithmic quality from model/tool/harness quality.
- For live laboratories, calibrate physical safety outcomes against expert-reviewed and instrumented replay; do not treat simulator agreement as ground truth.

## 5. Ambitious qualification targets

These are goals, not achieved results.

| Capability | Target |
|---|---:|
| Exact finite-oracle agreement on supported domains | 100% |
| Independent policy/risk/witness replay agreement | 100% |
| False exact-identification claims on negative controls | 0 |
| False risk-budget certifications in supplied worlds | 0 |
| Under-approximation in validated abstractions | 0 |
| Undetected known execution failures attributed as evidence | 0 |
| Decision-loss reduction vs. matched EIG baseline | At least 10% |
| Safe apparatus execution | No violation within qualified envelope |
| Decision confidence | Empirically calibrated on held-out families |
| Evidence provenance | Complete replayable receipt for every certified conclusion |

The 10% decision-loss target is an ambition; do not claim it from synthetic fixtures or compare unmatched tasks. A benchmark result counts only when budgets, outcomes, seeds, holdouts and baselines are matched.

## 6. Comparison rules and the route beyond current systems

1. **Do not compete on EIG's home metric alone.** Report EIG as a diagnostic and baseline, then compare downstream decisions.
2. **Do not compare one planner against an entire laboratory agent.** Use common APIs and matched harnesses, then run whole-system benchmarks separately.
3. **Separate three error sources.** Inferential misspecification, data-generating model misspecification and physical execution/measurement faults need separate causal labels.
4. **Make model-family adequacy adversarial.** A falsifier must be able to propose counterexamples; a counterexample must create a versioned world and force replanning.
5. **Keep exact oracles small and independent.** Approximate planners may scale, but their claims should be bounded against the exact oracle on overlap domains.
6. **Turn continuous physical uncertainty into certified residuals.** Derive abstraction errors from validated reachability/enclosures; never assign confidence from simulator fit alone.
7. **Attach evidence to every conclusion.** Persist model versions, apparatus calibration, action realization, raw readbacks, inference policy, loss table, risk bounds and replay receipts.
8. **Abstain when the evidence is insufficient.** A scientific agent should be rewarded for correctly refusing an unsupported claim.

The defensible competitive position is not an unsupported claim of general SOTA. It is an end-to-end scientific decision stack with exact small-domain correctness anchors, explicit model-adequacy attacks, decision-specific approximation bounds, physical execution evidence and independently replayable conclusions.