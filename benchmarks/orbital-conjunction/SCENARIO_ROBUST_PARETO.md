# Scenario-robust Pareto selection after mission-risk feasibility

Status: research implementation; not an operational conjunction-assessment service.

This note defines the intended contract for the scenario-robust Pareto frontier implemented by `rare-event-kernel::scenario_robust_pareto`. It distinguishes the mathematical comparison performed by the module from the validity of the physical and statistical evidence supplied to it.

## Decision ordering

The decision order is deliberately lexicographic:

1. **Validate the mission-risk evidence.** The upstream `ScenarioMissionRiskGate` verifies the complete declared scenario-by-portfolio risk-check matrix and recomputes each exact finite-support probability-mass comparison.
2. **Exclude every portfolio that violates a required scenario.** A portfolio is eligible only when it satisfies the hard mission-risk budget in every admitted epistemic scenario. There is no risk/fuel weighted score and no way for a cheap or high-performing plan to buy its way back into the feasible set.
3. **Freeze objective semantics and error allocation.** Each objective has a stable identity, unit, scale, direction and model-configuration identity. A separate family-wise error budget is allocated over every scenario × eligible portfolio × objective cell before the intervals are examined.
4. **Construct worst-scenario intervals.** Epistemic scenarios are alternatives, not probability weights. They are not averaged.
5. **Prove only conservative dominance.** Alternatives not proven dominated remain on the frontier; the evaluator does not invent operator preferences or choose a winner.

## Interval contract

Let (s in mathcal S) index admitted epistemic scenarios, (a in mathcal A_{mathrm{risk}}) index portfolios that passed the hard mission-risk gate, and (j in mathcal J) index the frozen objectives.

For an objective minimized by the operator, each evidence cell supplies a confidence interval ([L_{s,a,j}, U_{s,a,j}]). Its robust envelope is

[
left[
max_{sinmathcal S} L_{s,a,j},
max_{sinmathcal S} U_{s,a,j}
ight].
]

For a maximized objective, the robust envelope is

[
left[
min_{sinmathcal S} L_{s,a,j},
min_{sinmathcal S} U_{s,a,j}
ight].
]

The module allocates

[
alpha_{mathrm{cell}}
=
rac{alpha_{mathrm{objective family}}}
{|mathcal S|,|mathcal A_{mathrm{risk}}|,|mathcal J|}
]

to each cell. A union bound gives family-wise coverage **provided that each supplied cell interval actually has coverage at its declared alpha**. The implementation checks the allocation and evidence bindings; it does not prove that an arbitrary interval producer is statistically valid. The next qualification step must independently validate those producers.

For a minimized objective, portfolio (a) is no worse than (b) only when (U_{a,j}le L_{b,j}). For a maximized objective, it is no worse only when (L_{a,j}ge U_{b,j}). A dominance witness requires no-worse separation on every objective and strict separation on at least one. Touching intervals, overlapping intervals, or missing evidence do not establish strict dominance.

A Pareto frontier is a *set of non-dominated alternatives*, not a recommendation. The absence of a dominance proof is not proof of equal performance.

## Evidence bindings

Each objective specification freezes:

- objective identity;
- physical or operational unit identity;
- scale and tick interpretation identity;
- minimize/maximize direction;
- objective model/configuration digest.

Each scenario × portfolio × objective evidence row must match the exact frozen objective unit, scale and model/configuration digest. It must also bind to the hard-risk cell's mission manifest, risk envelope, post-action horizon, risk policy, target measure and density-space identities.

Canonical digests establish the identity of supplied content. They do **not** authenticate the issuer, prove source-system integrity, establish that the object catalogue is complete, or prove that the underlying physical model is accurate.

## Required adversarial controls

The test corpus should continue to include:

- an attractive low-fuel alternative that fails the mission-risk gate in one scenario;
- a portfolio that fails in several scenarios, with every exclusion reason retained;
- an empty feasible set (the result must remain an empty frontier);
- missing, duplicated, substituted and reordered scenario/portfolio/objective cells;
- changed objective unit, scale or model-configuration identity;
- changed hard-risk manifest, risk-envelope, horizon, policy, target-measure or density-space identity;
- intervals that overlap or only touch, which must not be called strictly dominant;
- alternatives that trade fuel against downtime or performance and must remain incomparable;
- objective-family alpha exhaustion, zero/underflowed allocations and non-finite endpoints;
- order-invariance and digest changes when any evidence field changes.

The current synthetic tests exercise the finite evaluator's structural and arithmetic controls. They are not orbital measurements or a physical validation campaign.

## Benchmark programme and fair incumbent comparison

Do not claim novelty from mission-level or multi-encounter probability alone. NASA CARA literature has discussed total probability for finite conjunction assessment since 2015; CARA's current methods include analytical 2-D and 3-D collision probability, covariance-realism assessment, single-covariance maximum probability, collision-consequence analysis and Monte Carlo comparisons. Its 2-D-Nc and 3-D-Nc methods also address single- and multi-encounter interactions.

The benchmark programme is therefore layered:

| Layer | External reference | What Symtropy must demonstrate |
| --- | --- | --- |
| Event screening | NOAA TraCSS Conjunction Assessment Verification dataset, using both its spherical and SFSH answer keys | Exact screening agreement on the released cases; separately report misses, false positives, duplicate handling and usable ephemeris-window handling |
| Single-encounter probability | NASA CARA 2-D Pc and 3-D Pc | Matched-input numerical accuracy, uncertainty coverage, and explicit identification of each method's assumptions and applicability |
| Extended / repeated encounters | NASA CARA 2-D-Nc / 3-D-Nc and high-fidelity OD-epoch Monte Carlo | Correct time-dependent / joint encounter handling, not merely a product or sum of independent event probabilities |
| Rare-event estimation | Crude Monte Carlo plus published importance-sampling / cross-entropy baselines | Unbiasedness or bounded bias, confidence-interval coverage, effective sample size, mode coverage, and expensive propagation count at matched error |
| Covariance and physical-model realism | NASA CARA covariance-realism methods plus independently held-out residual histories | Calibration and coverage under correlated, biased, non-Gaussian and time-varying uncertainty |
| Portfolio decision | Exhaustive finite-support oracle, then independent high-fidelity trajectory replay | Zero incorrect selection on exhaustive cases; zero false-safe decisions; risk constraint respected in every declared scenario |
| Multi-objective decision | Independent Pareto oracle over objective vectors and intervals | Exact frontier recovery on finite oracles; no false dominance; no unit/model mixing; explicit incomparable alternatives |
| Operational workflow | ISO 23705:2026 process and post-action screening requirements | Evaluate new conjunctions introduced by an avoidance maneuver and preserve costs, schedules, mission impacts, coordination and approval boundaries |

The TraCSS dataset is an event-screening diagnostic corpus. Its publisher explicitly says it is not comprehensive and is not intended for live operations or formal certification. Its answer keys are screening outputs, not universal ground truth for collision probability. Do not score Pc against the TraCSS screening answer key.

For numerical comparisons, freeze the same orbit-determination epoch, state and covariance inputs, object hard-body radii, force models, time span, event inventory, and numerical tolerances. Record software versions and input digests. Use independent truth generation where possible. Any use of common random numbers must share the same latent trajectory realization across all candidate portfolios and retain its complete proposal-density provenance.

### Primary metrics

Report at least:

- false-safe decision count and rate;
- mission-risk constraint violations by scenario;
- interval coverage and width, including simultaneous family-wise coverage;
- false-dominance and false-exclusion counts;
- Pareto-frontier precision/recall against an exhaustive finite oracle;
- selected-portfolio regret against the oracle;
- collision-risk reduction, propellant/delta-v, schedule loss and coordination burden as separate dimensions;
- independent high-fidelity propagations, effective sample size, wall time and compute cost.

A speed claim is admissible only at matched physical model, matched error, matched simultaneous coverage and matched decision-selection accuracy. A faster estimate that misses a rare-event mode is not an improvement.

## Current implementation boundaries

The research module currently implements finite-support hard-risk gating followed by worst-scenario interval envelopes and conservative Pareto dominance. It does not yet implement or qualify:

- the statistical construction of each objective confidence interval;
- continuous orbital-distribution coverage or adaptive importance-sampling coverage;
- independent physical propagation or real-world covariance calibration;
- authenticity of source receipts or completeness of the operational object inventory;
- operator preference elicitation, maneuver authorization or spacecraft command execution.

Those are separate gates, not assumptions to silently inherit.

## Related public work

- [PR #7670 — Scenario-robust Pareto frontier](https://github.com/Luminous-Dynamics/symtropy/pull/7670)
- [Issue #7643 — Conservative Pareto frontier after hard mission-risk feasibility](https://github.com/Luminous-Dynamics/symtropy/issues/7643)
- [Issue #7605 — NASA CARA Pc and covariance-realism benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/7605)
- [Issue #7579 — TraCSS screening-verification dataset](https://github.com/Luminous-Dynamics/symtropy/issues/7579)
- [Issue #7641 — ISO 23705 post-action horizon and policy bindings](https://github.com/Luminous-Dynamics/symtropy/issues/7641)
- [Issue #4217 — Astrodynamics interoperability benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/4217)

## Public reference material

- [NASA CARA — conjunction assessment and current handbook](https://www.nasa.gov/cara/)
- [NASA CARA — publicly available software](https://www.nasa.gov/cara/publicly-available-cara-software/)
- [NASA NTRS — Total Probability of Collision for Finite Conjunction Assessment (2015)](https://ntrs.nasa.gov/citations/20150018410)
- [NASA Software Catalog — 3D-Nc expected collision rates](https://software.nasa.gov/software/GSC-18593-1)
- [NOAA Office of Space Commerce — TraCSS verification dataset and limitations](https://space.commerce.gov/dataset-for-conjunction-assessment-verification/)
- [ISO 23705:2026 — published standard and scope](https://www.iso.org/standard/87556.html)
- [Sztamfater-Garcia et al. (2025) — approximate dynamics with importance sampling](https://doi.org/10.1016/j.asr.2024.12.074)
