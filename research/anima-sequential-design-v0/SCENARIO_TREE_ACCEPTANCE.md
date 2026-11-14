# Scenario-tree planning benchmark acceptance

This document specifies the next qualification ladder for the exact finite-horizon planner and its approximate successors. Targets below are proposals to preregister before running the comparison; they are not achieved results.

## Comparison systems

### Discrete experiment-design policies

Run all policies on the same frozen scenario model, candidate manifests, transition graph, costs, initial priors, outcome likelihoods, seeds and planning horizon:

1. Frozen one-shot EIG ranking.
2. Myopic sequential EIG.
3. Cost-normalized myopic EIG.
4. Robust maximin myopic EIG over the same declared ambiguity set.
5. Random feasible.
6. Fixed canonical schedule and separately designed randomized balanced factorial schedule.
7. Exact finite-horizon scenario-tree planner (current Rust implementation, only on finite tractable instances).
8. COPEx reproduction / public implementation, where its assumptions and pretrained model inputs can be matched.
9. Exhaustive sequence enumeration as an independent correctness oracle on tiny cases.
10. Clairvoyant true-model one-step policy, reported only as an upper-bound comparator, not as a feasible design.

COPEx is the method benchmark for constrained multi-step design. Its published study combines offline amortized policy and posterior-network training with online scenario-tree planning. Its public README describes active learning, location finding and CES tasks and a Python/PyTorch/SLSQP stack. A reproduction on locomotion scenarios must disclose any reimplementation, retraining, or architecture substitution; it is not a reproduction of the published results merely because the high-level algorithm is similar.

### Embodied-system comparators

Keep algorithmic design quality separate from body/controller performance. For embodied locomotion, use the version-pinned FlyGym/NeuroMechFly 2.1.1 matrix and matched PID, CPG, rule-based, hybrid, learned recurrent/RL, connectome-constrained and Symtropy controllers from LOCO-BENCH-001 (#3357). Match morphology, contact solver, timestep, observations, update rates, interventions, hardware and training/calibration budget.

## Primary metrics

- Final held-out intervention predictive log score and proper scoring-rule decomposition.
- Posterior calibration and coverage by intervention family, animal/session cluster, model family and evidence tier.
- Cost and number of experiments to a preregistered decision threshold.
- Cumulative expected information gain, final posterior entropy and true-model posterior on synthetic known truth.
- False-confident model selection, false stop, false discovery and abstention quality.
- Regret relative to an exact finite-horizon oracle on small cases; state whether regret is one-step, finite-horizon or budgeted.
- Scenario-tree node expansions, peak memory, wall-clock latency, model evaluations and cost per decision.
- Hard constraint violations, rejected action counts/reasons, budget overruns and failure-to-plan rates.

Do not substitute expected EIG for held-out predictive validity. Do not treat posterior mass over a misspecified candidate set as truth. Report the rate at which the correct answer is "none of the models are adequate."

## Known-answer and negative controls

1. Independently enumerate every legal action sequence in tiny scenarios and verify the scenario-tree first action and objective against the independently implemented exhaustive oracle.
2. Candidate and outcome order permutations must preserve the chosen action and objective within declared floating-point tolerance.
3. Impossible transitions, infeasible and safety-rejected actions, inconsistent cost units, and over-budget sequences must never enter the selected plan.
4. A horizon/tree-node limit must fail closed; no partial plan may be labelled exact.
5. A greedy trap must exist where the highest immediate-EIG action is not the best first action over the declared finite horizon.
6. Zero-information candidates must not be reported as informative.
7. A misspecified or omitted true mechanism must produce a model-inadequacy warning or explicit failure of confidence calibration, not confident selection of the least-wrong model.
8. Repeated measurements must be evaluated with conditional trajectory likelihoods and animal/session clustering; independent-step fixtures must remain separately labelled.
9. Repeated peeking tests must evaluate the declared e-process under correctly specified conditional nulls and under deliberate conditional-model misspecification.

## Proposed targets (not achieved)

- 100% match to independent exhaustive enumeration on all tractable known-answer cases.
- Zero infeasible, safety-rejected, disallowed-transition or over-budget selections.
- Zero false-confident conclusions on known non-identifiable or omitted-truth fixtures.
- On planted constrained lookahead traps, exact planning should improve expected finite-horizon information gain over myopic policies; report paired-seed effect sizes and intervals rather than assuming a fixed percentage.
- On larger problems, any approximation must report an explicit optimality gap or regret estimate whenever available, plus node/time/memory budgets.
- Under the preregistered null and correct conditional forecast, the e-process must control the chance of ever crossing the threshold at or below alpha. Under deliberate dependence-model misspecification, it must not be described as having this guarantee.
- The adaptive policy must retain held-out calibration and predictive log score; information gain alone is insufficient.

## Conditional-dependence boundary

The current exact tree reuses fixed categorical likelihood tables at every node. Its known-answer cases therefore represent conditionally independent replicate trials given the model and selected action. It must not be used to plan repeated measurements within the same animal/session or serial neural/kinematic trajectories as though they were independent. A history-dependent extension must consume conditional forecasts indexed by a frozen complete-history digest and must pass serial-dependence, carryover, cluster and forecast-calibration controls before those cases enter the qualified benchmark.

## Current implementation boundary

The current scenario-tree module is exact only for finite categorical outcome likelihoods, a finite horizon, one shared comparable cost unit, and explicit action-transition constraints. It does not yet implement continuous-valued design spaces, parameter-posterior fantasy models, stochastic branch sampling, learned policy warm-starts, admissible branch-and-bound, robust finite-horizon ambiguity optimization, or connectome/trajectory-specific conditional likelihoods.

All current tests are synthetic software qualification. No biological mechanism, animal outcome, FlyGym performance advantage, or SOTA claim follows from passing them.
