# Belief-state immediate-cost CVaR v0.1

**Status:** executable finite mathematical primitive. Synthetic particle weights and costs only. This is not a complete POMDP planner, calibrated belief, physical navigation result, policy guarantee, or authority API.

## Motivation and research comparator

Pariente & Indelman, **Risk-Averse Online POMDP Planning via CVaR of the Immediate Cost with Performance Guarantees** (26 September 2026; NeurIPS 2026): https://arxiv.org/abs/2609.35874

The paper distinguishes hidden-state uncertainty within the current belief from stochastic trajectory-level uncertainty. It replaces expected immediate cost under a belief with CVaR of the state-dependent immediate cost, while retaining expected cumulative return. Its paper derives finite-time policy-evaluation and sparse-sampling bounds and particle-belief approximation bounds under explicit assumptions.

This is a design comparator; its particle-belief guarantees do not automatically transfer to the current finite, non-rectangular persistent-model navigation oracle.

## Implemented primitive

The Rust API belief_immediate_cvar accepts a weighted set of belief particles:

- stable nonzero particle identity unique within the belief snapshot;
- immediate state-dependent cost for a candidate action;
- non-negative integer posterior / importance weight.

It returns both exact expected immediate cost and exact upper-tail CVaR, plus tail mass and particle count. It rejects empty beliefs, zero total mass, invalid tail mass, zero IDs and duplicate particle IDs.

### CVaR convention

The public belief API uses **tail mass** in parts per million:

- 50,000 = worst 5% of belief mass;
- 100,000 = worst 10%;
- 1,000,000 = all belief mass, which reduces to expected cost.

The lower-level path-risk API uses confidence alpha (e.g. 950,000 for the upper 5% tail). The belief API makes this conversion explicitly. This avoids silently mixing the tail-mass convention in the POMDP paper with the confidence-level convention used in the mission-path evaluator.

## Exact synthetic counterexample

Two candidate actions have the same expected immediate cost:

| Candidate | Current-belief cost distribution | Expected cost | Worst 5% CVaR |
|---|---|---:|---:|
| A | 5% mass at cost 100; 95% at cost 0 | 5 | 100 |
| B | 100% mass at cost 5 | 5 | 5 |

Expected immediate cost cannot distinguish them. Belief-state CVaR exposes the dangerous minority state in A.

These are exact synthetic particle weights and loss values. They are not empirical UWB risk rates or measurements.

## Risk semantics: do not conflate layers

Keep at least three metrics distinct:

1. **Belief-state immediate-cost CVaR:** current hidden-state uncertainty under the posterior, for a fixed candidate model.
2. **Mission-path CVaR:** accumulated loss distribution over complete stochastic mission paths, conditional on a fixed model.
3. **Persistent-model robustification:** outer maximum across complete plausible mission models whose identity remains fixed for the mission.

The current path-CVaR optimizer and this belief-CVaR primitive are complementary, not additive by default. Simply summing both risk measures can count the same hazard twice and lacks a general composition theorem.

For robust evaluation, evaluate belief-state cost distributions separately for every complete persistent mission model, then report the outer maximum. Never pool particles from different epistemic models or allow model identity to switch at each observation.

## Tests

The module tests:
- equal-mean, different-tail belief distributions;
- full tail mass reducing to the mean;
- particle-order invariance;
- duplicate / zero particle identity rejection;
- invalid tail mass, empty belief and zero total weight rejection.

## Next integration experiment

Compare four explicit policy cost formulations on identical temporal belief histories:

1. expected immediate belief cost + expected cumulative return;
2. belief-state immediate-cost CVaR + expected cumulative return;
3. static CVaR of complete mission-path return;
4. dynamic / iterated CVaR of future conditional costs.

Evaluate each separately under fixed persistent models, then apply the outer model maximum. Add event-triggered mode replacement as an additional policy layer: when a current route/speed/viewpoint mode is no longer risk-feasible, choose a different feasible mode rather than repeatedly adjusting the same plan.

Report expected mission loss, belief-CVaR, path-CVaR, false-safe / false-degraded exposure, availability, abstention, resource use and exact-oracle gap independently. Do not select a risk measure or combine levels until controlled counterexamples and sensitivity sweeps show what each one protects against.

## Claim boundary

This increment evaluates an immediate cost distribution supplied by the caller. It does not update beliefs, generate particles, establish particle approximation quality, optimize actions, estimate physical model coverage, or qualify navigation authority. All examples are synthetic.
