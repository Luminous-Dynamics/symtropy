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
| Constrained sequential planning | Constrained Bayesian Experimental Design via Online Planning, ICML 2026 | Amortized posterior/policy plus online multi-step scenario-tree planning under dynamic budgets and physical constraints | Exact finite-horizon decision/risk oracle; shared-policy robust worlds; replayable certificates |
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

The benchmarks are complementary, not interchangeable. In particular, the ICML scenario-tree planner is not just one-step EIG; SafeLab is not a scientific-discovery leaderboard; and a planner-only score is not comparable to an entire agent/model/tool/harness configuration.

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
- constrained multi-step scenario-tree planning.
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
- Report the cases where EIG, constrained scenario-tree planning, R-IDeA, EGIG or robust adversarial design wins.
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

## 7. Benchmark saturation, scoring audits and adaptive challenge generation

A current industry lesson changes the programme's benchmark strategy:

- **AutoSciBench (October 2026)** adaptively refines task recipes and reports that generated tasks reduce solver accuracy by 22.4 / 25.5 percentage points versus human-curated benchmarks in computational biology/materials science. This is evidence for dynamic challenge generation, not evidence that any one generator is automatically unbiased.
- **OpenScience (1 October 2026 update)** explicitly reports issues in its evaluation setup and imperfect comparisons on some benchmarks, plus weaker performance on engineering than physical-science tasks. A public leaderboard therefore needs a harness audit and matched reruns before general SOTA conclusions.
- **DOE (8 October 2026)** selected national-lab autonomous-science projects emphasizing open interfaces, reusable robot skills, digital twins, benchmark tasks, datasets, safety and provenance. This supports portable challenge manifests and evidence receipts.

The new [Adaptive Scientific Benchmark Protocol](ADAPTIVE_SCIENTIFIC_BENCHMARK_PROTOCOL.md) specifies task recipes, hidden parameter generation, independent verifiers, sealed family splits, anti-saturation rules, matched baselines and separate scoring gates. SIM-REL-034/035 provide the split firewall and replayable ledger needed to keep adaptive benchmark refinement from consuming its own calibration/holdout data.

Primary sources:
- AutoSciBench: https://arxiv.org/abs/2610.05140
- OpenScience benchmark update: https://www.openscience.sh/benchmark
- DOE autonomous science selections: https://www.energy.gov/science/articles/doe-announces-four-national-laboratory-led-selections-advance-robotics-and

A benchmark pass means only that a system performed under a declared task recipe, model, tool and harness. It is not proof of open-world scientific capability.


## 8. SIM-REL implementation map (1 November 2026)

| Increment | Purpose | Current evidence boundary |
|---|---|---|
| SIM-REL-034 / PR #3236 | Append-only admission, split reservations, invalidation and re-plan ledger | Open draft; latest recorded qualification was `action_required` with no completed jobs. Not qualified. |
| SIM-REL-035 / PR #3245 | Offline transcript reconstruction | Open draft; no exact-head PR check run was returned in the live status check. Not qualified. The verifier shares the producer's transition and physical/policy replay code. |
| SIM-REL-036 / issue #3246 | Exact binary-channel challenge compiler | Implementation branch exists. It generates deterministic tasks with public/sealed artifacts and exact references; it does not yet adapt recipe selection or generate physical apparatus challenges. |
| SIM-REL-037 / PR #3265 | Independent logical transition oracle | Open draft. The focused module tests passed on its first exact-head run, but the full crate run exposed two pre-existing test defects in the stacked branch; fixes were pushed and rerun evidence is pending. Not qualified. |
| SIM-REL-038 / issue #3266 | Development-only adaptive recipe selection | Research issue and branch created; implementation and qualification remain pending. |
| SIM-REL-039 / issue #3267 | Exact finite physical challenge compiler | Research issue created; implementation and qualification remain pending. |

SIM-REL-037 checks transition ordering, revision/policy lifecycle, split/receipt integrity and snapshot consistency in a separate implementation. It is not an independent physics or policy replay engine. The baseline policy metrics are retained as a distinct immutable replay receipt, but this receipt is not cryptographically authenticated by the transition auditor.

The intended assurance composition is therefore:

1. **Physical/intervention oracle:** independently validate apparatus state, action realization and measurement semantics.
2. **Policy/risk replay:** recompute decision loss and risk from exact public histories and the supplied model family.
3. **Logical transition oracle:** independently validate admission, invalidation, re-planning, sealing, split use and transcript state.
4. **Provenance/split authority:** authenticate calibration, development and sealed-holdout manifests and artifact identities.
5. **Open-world adequacy evaluation:** use sealed physical/mechanism-family holdouts and explicit limits; no finite test family proves universal model adequacy.

A result qualifies only for the layers that actually ran and passed on the exact source, lockfile, input manifest and workflow checkout identity.


## 9. 2026 scientific-agent benchmark comparison and lessons (2 November 2026)

| Benchmark | Scale / protocol | Published signal | What Symtropy should learn |
|---|---|---|---|
| FIRE-Bench | 30 rediscovery tasks from 2024–2025 ICLR/ICML/NeurIPS papers; three independent trials; atomic claim-level P/R/F1 | Best listed F1 46.7; high run variance; 73.6% of classified errors attributed to flawed planning | Separate planning, implementation, execution and evidence reasoning; measure every run and preserve claim-level evidence |
| Autonomous Science Bench 0.1 | Six real-lab pilot tasks across biology, chemistry and materials; three trials per task; final discovery and unmeasured-design learning gates | GPT-6 Astra/Codex 72.2%, Opus 5.5/Claude Code 66.7%; authors caution the small gaps are not meaningful at this pilot size | Score design quality and learning on unmeasured designs, cost and decision improvement—not success alone |
| EmbodiedLab | 30 simulated quantitative experiments, five disciplines, 14 apparatus platforms; instance parameters resampled; instrument readings and apparatus state are part of grading | Strongest native-control configuration below 50% in the paper abstract; apparatus interaction is a major failure source | Require actual readbacks, state transitions and restoration; do not grade only final numerical answers |
| AgentIdeaBench | 33 matched models, 40 subfields, five disciplines; active retrieval compared with static literature | Active exploration improves feasibility/clarity/specificity; measured originality remains flat | Keep originality and grounding separate; freeze corpus date and verify prior art independently |
| SciAgentArena | Approximately 200 interactive scientific tasks with stepwise verification | Good performance on well-specified analysis; weak genuine novelty and sustained autonomous exploration | Use long-horizon, agent-agnostic tasks and independent stepwise oracles |
| OpenScience benchmark | Public scientific-agent benchmark claims | Its 1 Oct 2026 update acknowledges evaluation problems and imperfect cross-agent comparisons, with engineering reruns pending | Do not inherit public leaderboard claims without auditing the harness and rerunning matched tasks |

Primary sources:
- FIRE-Bench, PMLR 306 (2026): https://proceedings.mlr.press/v306/wang26d.html and https://firebench.github.io/
- Autonomous Science Bench 0.1 (1 Oct 2026): https://www.autonomous-science-bench.ai/
- EmbodiedLab (26 Oct 2026): https://arxiv.org/abs/2610.35385
- AgentIdeaBench (7 Sep 2026): https://arxiv.org/abs/2609.07611
- SciAgentArena (10 Jun 2026): https://arxiv.org/abs/2606.12736
- OpenScience benchmark update (1 Oct 2026): https://www.openscience.sh/benchmark

These results use different task pools, harnesses, domains, budgets, trial counts and endpoints. They are not a common leaderboard and their headline percentages must not be directly ranked against each other.

### 9.1 Paradigm shift: discovery claims need a three-axis proof

A scientific agent should not receive one "discovery score" for producing a plausible answer. We should assess three independent properties:

1. **Truth / effect:** did the claimed relationship or outcome survive a preregistered experiment, independent measurement and appropriate statistical analysis?
2. **Novelty:** was the claim absent from a dated and versioned prior-art corpus under a disclosed search protocol, with expert adjudication of the closest prior work?
3. **Reproducibility / transfer:** did a different run, operator, apparatus, dataset or laboratory reproduce the effect under a declared independence profile?

A claim can be true but already known; novel-sounding but false; reproduced but non-generalizing; or useful despite being non-novel. These outcomes must remain distinct. "No prior art found" is not a proof of novelty, and a signature/transparency receipt is not proof of scientific truth.

### 9.2 Internal campaign targets

For a serious multi-domain comparison:
- At least 100 distinct task clusters across five scientific domains (minimum 20 per domain).
- Five independent repetitions per task cluster by default; ten for high-variance tasks or where a declared reliability bound requires more.
- Same task instances, seeds, agent panel, model/prompt versions, harness, tools and resource budgets for competing arms.
- Sealed task-family holdouts excluded from development, prompt tuning, adaptive recipe generation and critic calibration.
- Report pass@5, pass^5, run-level safe-success rate, any-hazard, any-abstention, completion, evidence quality and cost separately.
- Compute uncertainty by resampling independent task clusters, never the repetitions within one cluster.
- Include random, expert/lab workflow and strongest accessible agent baselines under the same interface and budgets.
- Publish all adverse strata, invalid exclusions, task-level outcomes and reproducible evidence artifacts.

Ambitious internal target (not a current result): at least +10 percentage points against the strongest matched baseline on the preregistered primary cluster-level endpoint, with a positive paired cluster-level 95% interval, no increased cost, and zero false safety certifications in the negative-control suite. A high pass@5 without strong pass^5 is not operational reliability.

## 10. Live research increments (2 November 2026)

| Increment | Research contribution | Latest recorded evidence | Status |
|---|---|---|---|
| SIM-REL-041 / PR #3276 | Matched benchmark admission, shared tasks/configs/harness/tools/resource budgets, family-level paired deltas | Exact head `f23bac7a07e375ab462fab08d1725a823a914dd1`; 6 focused / 166 full tests; Clippy, Rust 1.96 fmt, exact-head and clean-tree passed | Qualified branch; draft PR |
| SIM-REL-042 / PR #3291 | Fixed-sample exact paired sign test, bounded mean-loss interval and Holm family-wise correction | Exact head `9b6ee78884fb8503e3f1c8c71e22745ba07dad0f`; 7 focused / 173 full tests; all qualification gates passed | Qualified branch; draft PR |
| SIM-REL-043 / PR #3292 | Scientific evidence envelope, content-addressed artifact graph and pluggable COSE/SCITT verification providers | Exact head `c58582fb84cb7386598b677b673fbff8262841a0`; 5 focused / 171 full tests; all qualification gates passed | Qualified structural verifier; draft PR; production cryptography/SCITT wire encoding pending |
| SIM-REL-044 / PR #3295 | Exact thermal challenge + causal command/actuator/transition/sensor/decision consistency bridge | Exact head `0653344db8c4fa0ec5d0413b3e94560aeed6ce65`; 8 focused / 185 full tests; all qualification gates passed | Qualified finite-model bridge; draft PR; hardware/metrology validation pending |
| SIM-REL-045 / PR #3299 | Exact-rational anytime-valid paired sign e-process; historical threshold crossings; fixed Bonferroni family allocation | Exact head `57233e3020fd4ef637f64c6d71f858df89aa4f7e`; 7 focused / 192 full tests; all qualification gates passed | Qualified sign-process implementation; draft PR; conditional sign-symmetry assumptions apply |
| SIM-REL-046 / issue #3300 | Task-clustered repeated-run reliability, pass@k vs pass^k, run success, hazards and abstention | Exact formatted head `ef569b2067618dfdde05cf6d713fbfba3ff7f3d0`; 5 focused / 197 full tests; Clippy, Rust 1.96 fmt, exact-head and clean-tree passed. Workflow: https://github.com/Luminous-Dynamics/symtropy/actions/runs/41404077635 | Qualified branch; draft PR pending |

All PRs remain drafts and no merge is implied. Qualification applies only to the exact SHA and assurance layer listed. The counts above are branch-workflow evidence, not claims of superiority on the external benchmarks.
