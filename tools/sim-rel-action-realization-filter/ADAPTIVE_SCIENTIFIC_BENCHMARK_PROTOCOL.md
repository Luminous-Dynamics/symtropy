# Adaptive scientific decision benchmark generation protocol

Status: benchmark-generation design and exact-oracle requirements. This document does not claim a completed benchmark generator or a measured leaderboard result.

## 1. Why static scientific-agent benchmarks are insufficient

Scientific-agent evaluation has two coupled failure modes:

1. Task saturation: systems learn the benchmark's fixed task surface and scores stop discriminating new capabilities.
2. Harness/scoring error: ostensibly matched systems may be compared using different tool access, judging rules, physical abilities or scoring assumptions.

AutoSciBench (October 2026) formalizes recipe-based automatic benchmark generation and iterative refinement. It reports reduced agent accuracy relative to human-curated tasks by 22.4 and 25.5 percentage points in computational biology and materials science, respectively, suggesting dynamically generated tasks can close shortcut paths.

OpenScience's 1 October 2026 update explicitly reports that some of its evaluation setup had issues and that some benchmark comparisons were not perfectly matched; it also observed weaker performance on engineering tasks than physical-science tasks. This is a concrete warning not to turn a leaderboard score into a general SOTA claim without a harness audit.

DOE's 8 October 2026 autonomous scientific discovery selections emphasize open interfaces, reusable robot skills, digital twins, benchmark tasks, datasets, safety practices and provenance records. That points toward reusable challenge manifests and reproducible evaluation infrastructure rather than isolated benchmark scripts.

References:
- AutoSciBench: https://arxiv.org/abs/2610.05140
- OpenScience benchmark and its 1 October update: https://www.openscience.sh/benchmark
- Autonomous Science Bench 0.1: https://www.autonomous-science-bench.ai/
- DOE autonomous scientific discovery selections, 8 October 2026: https://www.energy.gov/science/articles/doe-announces-four-national-laboratory-led-selections-advance-robotics-and

## 2. Scientific decision challenge recipe

Every generated challenge should declare:

- Scientific domain and apparatus family.
- Hidden mechanism family and difficulty stratum.
- Observation/action interface and public observation vocabulary.
- Commanded-to-realized intervention model.
- Sensor/readback model and calibration state.
- Terminal decision set, abstention action and complete loss table.
- Hazard conditions and irreversible transition families.
- Experiment-cost and compute budgets.
- Ground-truth model family and allowed inference-family class.
- Exact-oracle availability and unsupported-size boundary.
- Development/calibration/sealed partition group.
- Reference solver and deterministic verifier version.

A recipe is a task-construction specification, not a test row. Concrete tasks are instantiated with hidden parameter draws and apparatus states.

## 3. Challenge families

The minimum generator should cover orthogonal, composable dimensions:

1. **Mechanism uncertainty:** competing mechanisms, rare critical regimes, latent interactions and hidden thresholds.
2. **Model misspecification:** inferential family mismatch, data-generating-family mismatch, omitted mechanisms, and covariate/target shift.
3. **Action realization:** dropped/partial delivery, backlash, action remapping, delayed actuation and irreversible process change.
4. **Measurement integrity:** sensor inversion, drift, dropout, correlated errors, censored readback and apparatus-state confounding.
5. **Decision consequence:** symmetric 0-1 loss, asymmetric losses, catastrophic rare-state loss, abstention and escalation.
6. **Sequential structure:** complementarity, non-myopic value, adaptive stopping, finite horizon and budget exhaustion.
7. **Safety:** trajectory hazard, forbidden intermediate states, apparatus recovery, safe abort and post-run state obligations.

The challenge generator must include explicit controls where ordinary EIG wins, decision-aware design wins, robust planning wins, or every method should abstain. A benchmark that only rewards the proposed algorithm is not a benchmark.

## 4. Generator pipeline

1. Select a registered recipe family and difficulty target.
2. Draw hidden mechanism, apparatus and sensor parameters from a versioned deterministic generator.
3. Instantiate a finite exact reference problem wherever possible.
4. Produce the reference truth, feasible policy set, exact Bayes/robust decision, hazard outcomes and held-out design ranking.
5. Validate the generated task using a verifier that is independent of the generator path.
6. Run negative controls and invariants: probability mass, physical/schema constraints, known causal relationships, action semantics, and exact-oracle replay.
7. Assign the split group before the task is exposed.
8. Store raw parameters and oracle outputs in an immutable manifest; expose only the task interface and public observations to the agent.
9. Evaluate all baselines under identical model access, tools, budgets, observations and judge/harness versions.
10. Preserve task artifacts, trajectories, raw readbacks, realized-action receipts, final decisions and verifier outputs.

Generation must fail closed if the task has no known solution, if the independent verifier disagrees, or if a task accidentally leaks the hidden mechanism through public metadata.

## 5. Anti-saturation and anti-leakage rules

- Split by mechanism family, apparatus family and generator lineage, not random task rows.
- Reserve sealed family groups before adaptive challenge refinement.
- The development falsifier may refine only Development groups.
- Calibration groups are never used to expand the development challenge family.
- Sealed holdouts are consumed once, including failed replay/criteria results.
- The agent must not receive recipe IDs, hidden parameter seeds, oracle labels or sealed manifest metadata.
- Track exact policy/tool/model/harness versions at evaluation time.
- A benchmark refinement may create new challenge groups but must not silently rewrite prior sealed manifests.
- Publish used, rejected, failed-validation and unconsumed task counts.
- Treat benchmark-generator output as a candidate challenge until an independent verifier accepts it.

SIM-REL-034 governs split and ledger transitions. SIM-REL-035 makes its event transcript portable and replay-verifiable by replaying the established ledger transition and policy/physics functions; that verifier still has common-mode risk with the producer. SIM-REL-037 adds a separately coded logical transition auditor, but it does not independently establish the physical perturbation or policy metrics. The benchmark generator must use these controls; it must not implement its own weaker holdout bookkeeping.

The initial SIM-REL-036 implementation generates deterministic exact binary-channel tasks with separate public-task and sealed-oracle artifacts. It is a task compiler, not yet an adaptive challenge-selection policy and not yet a physical apparatus generator.

SIM-REL-038 is the planned development-only recipe-selection controller. It must consume Development outcomes only, track difficulty and mechanism coverage by stratum, use confidence-aware updates and freeze its selection policy before calibration/holdout evaluation. Calibration is diagnostic; sealed holdouts must never update the recipe selector.

## 6. Scoring: separate the scientific outcomes

Never collapse the complete scientific workflow into a single pass/fail or one scalar unless the component metrics remain available.

Measure separately:

- Discovery quality on measured experiments.
- Held-out learning / ranking of unmeasured designs.
- Expected terminal decision loss and regret.
- Worst-world/CVaR loss and decision-switch rate.
- Calibration and selective risk under abstention.
- False confident conclusion rate.
- Model-family counterexample detection and refinement.
- Command/realization fidelity.
- Measurement/readback fidelity.
- Trajectory-level safe success and apparatus post-state compliance.
- Evidence receipt completeness and independent replay agreement.
- Experiment cost, wall time, memory, compute cost and unsupported task rate.

For all scientific claims include the exact task, model, agent, tool, harness and verifier version.

## 7. Matched industry comparison

Required baseline tracks:

- Random/design-prior.
- EIG and standard Bayesian experimental design.
- Expected one-step Bayes-risk reduction.
- COPEx-style constrained sequential scenario-tree planning.
- R-IDeA (representativeness, informativeness, de-amplification).
- EGIG / explicit model-error penalties.
- Sequential adversarial robust decision-aware design.
- SIM-REL exact oracle on tractable cases.
- SIM-REL approximate/robust planner on scale cases.
- Whole-system agent baselines on embodied/complete campaigns.

Separate algorithmic comparisons from complete agent/model/tool/harness comparisons. A planner must not be claimed to beat a whole laboratory agent from unmatched evaluation settings.

Use at least 30 paired seeds per synthetic family where practical, target 100+, 95% paired confidence intervals, pre-registered outcomes, sealed family holdouts and full per-stratum reporting.

## 8. Qualification targets

These are ambitions, not measured results:

- 100% exact truth/oracle agreement on declared supported task domains.
- 100% deterministic verifier replay agreement.
- Zero hidden-answer leakage detected in generated tasks.
- Zero sealed-holdout reuse or calibration-to-development leakage.
- Zero false exact-identification claims on negative controls.
- Zero known intervention failures attributed as successful evidence.
- Zero risk-budget false certifications within supplied worlds.
- Zero abstraction under-approximation in validated analytical cases.
- ≥10% decision-loss improvement over matched EIG as an eventual empirical target; do not claim it before adequately powered head-to-head trials.
- Every task has an independently accepted manifest and reproducible generation receipt.

## 9. Evidence boundary

Adaptive benchmark generation can improve task coverage and reduce shortcuts. It cannot prove the generated task family represents the open physical world, and generator/verifier common-mode errors remain possible. Use an independent verifier and retain external domain review for physical plausibility. Report coverage only against the declared recipe and generator family, never as open-world scientific adequacy.
