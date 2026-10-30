# SIM-REL-003 — MDL-prior expected information gain per cost

**Status:** executable prototype; awaiting exact-head Rust qualification.  
**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2906  
**Predecessor:** https://github.com/Luminous-Dynamics/symtropy/pull/2904  
**Date:** 2026-10-30

## 1. Research question

Does a predeclared grammar-derived prior combined with expected information gain per unit experiment cost reduce the total cost of identifying a rule, compared with maximum-disagreement, fixed-order and seeded-random probing, while preserving target retention and misspecification detection?

This prototype changes the experiment selector, not the hypothesis language or truth-table simulator. It isolates the effect of prior-weighted, cost-aware selection.

## 2. Hypothesis grammar and prior

The bounded grammar remains three binary inputs, constants 0/1, NOT/AND/OR/XOR, syntax-tree depth at most two, and eight possible intervention assignments. The grammar enumerates 109 unique semantic truth tables at depth two.

A probabilistic context-free grammar is fixed before examining task outputs. At each expression node:
- with probability 1/2 choose to terminate, then choose one of five terminals uniformly;
- with probability 1/2 choose one of four operators uniformly, then recursively encode its child expression(s).

This yields terminal code length log2(10) bits and operator-node code length 3 bits plus the sum of child code lengths. The expected number of children per generated node is 0.875, so the recursive prior is subcritical.

For a semantic function h, define L(h) as the minimum derivation code length among generated expressions with the same complete truth table. The prior weight is proportional to 2^(-L(h)); weights are normalized over the finite depth-two semantic candidate set.

This is an explicit shortest-derivation MDL prior, **not** a sum over all syntactic derivations, and not a claim of calibrated posterior probabilities. The shortest program approximation intentionally avoids letting a function collect extra probability simply because the enumerator generated many aliases. Prior sensitivity to the grammar and coding scheme is a required future experiment.

## 3. Selection objective

After noiseless observations D, the surviving semantic functions are those consistent with every observed input/output pair. Their MDL weights are renormalized to obtain p(h | D).

For each available binary intervention a, the deterministic simulator induces p(y | D,a). The expected posterior entropy reduction is:

EIG(a) = H(H | D) - sum_y p(y | D,a) H(H | D,y,a)

Because the current observation model is deterministic conditional on h, EIG(a) equals the binary outcome entropy H(Y_a | D). The MDL selector chooses the action maximizing EIG(a) divided by its positive integer fixture cost.

The selector receives only surviving candidates, available actions, grammar prior weights and costs. The hidden target is not a selector argument. The runner uses the evaluator-only target to simulate the environmental response after an action has been chosen.

This formula is exact for the declared deterministic observation model. Under noisy observations, EIG must include H(Y | H,D,a), use likelihood updates instead of hard elimination, and be evaluated with a proper noise model. That is not implemented in v0.

## 4. Falsification

When a provisional singleton remains and query budget is available, the MDL policy chooses an unqueried action maximizing the prior-weighted probability that a grammar hypothesis disagrees with the provisional candidate, divided by action cost. This is an explicit adversarial falsification objective, not the ordinary posterior EIG of a point-mass posterior.

A contradictory result reports misspecification. A successful falsification query does not establish the full truth; exact identification still requires all eight assignments.

## 5. Baselines and metrics

Four policies use the same grammar, target, query budget and task-specific action costs:
1. SIM-REL-002 maximum-minority / maximum-disagreement;
2. MDL-prior expected information gain per cost;
3. fixed ascending intervention;
4. seeded-random intervention (32 runs per task).

Report separately:
- intervention count: mean, median, nearest-rank p90;
- total experiment cost: mean, median, nearest-rank p90;
- exact/provisional/ambiguous/misspecified outcomes;
- in-grammar target retention;
- out-of-grammar detection and provisional-unique escape rates;
- falsification action and contradiction outcomes;
- semantic class code lengths, representatives and alias counts;
- exact Git SHA and SHA-256 digests for both fixture files.

Do not collapse integrity, target retention or false-identification failures into a compensable aggregate score.

## 6. Pilot fixture and leakage warning

V0 reuses the 18 hand-authored public SIM-REL-002 functions and adds varied synthetic action costs. It has six train, six validation and six nominally held-out cases; **the held-out labels and targets are public and are not an independent confirmatory set**. The only new variable is action cost. This pilot is useful for checking that the cost-aware objective behaves differently under different cost schedules, not for claiming generalization or SOTA performance.

Qualification requires at least 100 independently generated worlds across three difficulty bands, complete generating-rule/composition family splits, sealed held-out targets, positive/negative controls, observational-equivalence controls, alias-multiplicity controls, noisy observations, non-uniform priors and prior-sensitivity sweeps.

## 7. Qualification targets (proposed, not results)

- 100% retention of in-grammar targets for all policies under the deterministic, well-posed simulator.
- Zero false exact identifications on observational-equivalence controls.
- Explicit reporting of out-of-grammar detection and provisional-unique escape separately.
- At least 15% lower median cost than the strongest baseline at equivalent target retention, identification, abstention and falsification quality, with paired uncertainty interval excluding zero and replication on an independently sealed rule family.
- If the result is underpowered, report inconclusive; do not change thresholds after examining the holdout.

## 8. Field positioning

This is not a direct head-to-head against symbolic-regression systems because the task is adaptive intervention selection over a finite grammar, not equation recovery from continuous-valued measurements.

Relevant methodological baselines:
- [Desmond (2026), Exhaustive symbolic regression and MDL](https://doi.org/10.1098/rsta.2024.0584): MDL provides an information-theoretic accuracy/complexity trade-off and discusses exhaustive search as a counterpoint to stochastic symbolic regression.
- [Yu et al. (ICLR 2025), SR4MDL](https://proceedings.iclr.cc/paper_files/paper/2025/hash/a402493de088886740b5939f666a6e56-Abstract-Conference.html): uses an MDLformer-guided search objective for formula recovery and reports evaluation on 133 problems plus 122 unseen black-box problems. Its reported 43.92% improvement is paper-specific and not comparable to this Boolean pilot.
- [Soltani et al. (2026), comparison of SR model-selection criteria](https://arxiv.org/abs/2605.11233): compares AIC/AICc/BIC/MDL/bootstrap on noisy synthetic datasets and reports that no single criterion dominates every regime.
- [EurekaBench (2026)](https://arxiv.org/abs/2610.00492): emphasizes whether discovered mechanisms support expert-verified scientific insights, beyond predictive accuracy alone.
- [Automated Scientific Discovery survey (2025)](https://link.springer.com/article/10.1007/s10994-025-06955-2): surveys equation discovery, experiment selection and autonomous discovery environments, including expected information gain as an evaluation dimension.
- [SRBench 2025](https://doi.org/10.1145/3712255.3734309): emphasizes reproducible comparisons across diverse symbolic-regression tasks.

The long-term comparison is a shared experiment-design benchmark against uniform max-split, MDL priors, Bayesian information gain, fixed/random baselines, and later established symbolic-regression or scientific-agent systems on exactly matched action spaces and costs.

## 9. Boundaries and next work

V0 does not implement sensor noise, continuous-valued measurements, latent parameters, model averaging across uncertain parameter estimates, calibration, experiment setup overhead, or real-world cost models. It does not establish physical discovery, a universal relational grammar, an industry-leading result or any Chain Theory claim.

Next work after exact-head qualification:
1. prior temperature / grammar coding sensitivity sweep;
2. noisy likelihood and calibrated posterior prediction;
3. action setup overhead and multi-objective cost (time, energy, money, safety);
4. at least 100 sealed worlds and paired statistical analysis;
5. typed spring–mass and graph-rewrite transfer with strong conventional baselines.
