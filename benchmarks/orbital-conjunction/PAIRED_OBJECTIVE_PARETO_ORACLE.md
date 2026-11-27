# Paired-objective Pareto oracle

Status: research implementation and exact finite-support reference oracle. This is not a statistical confidence procedure, physical orbit-validation campaign, or operational conjunction-assessment service.

## Motivation

The marginal-envelope frontier in [#7670](https://github.com/Luminous-Dynamics/symtropy/pull/7670) compares a portfolio's worst-scenario objective interval with another portfolio's worst-scenario interval. This is conservative, but it can lose useful dependence information. The worst outcome for portfolio A and the worst outcome for portfolio B may occur on different realizations of a common physical disturbance.

For a fixed epistemic scenario \(s\), shared latent trajectory realization \(U\), candidate portfolios \(a,b\), and objective \(j\), define the direct paired contrast

\[
D_{s,a,b,j}(U)=f_{s,a,j}(U)-f_{s,b,j}(U).
\]

The new oracle evaluates both portfolio consequences on the same scenario/latent-atom row and then calculates the exact finite-support population mean and variance of this difference. It does not independently sample each portfolio or pair.

## Decision contract

The campaign freezes:

- the completed upstream hard mission-risk gate;
- a scenario-specific physical-world/force-model manifest binding, separate from the objective-function configuration digest;
- its eligible and ineligible portfolio partition;
- the epistemic scenario universe and each scenario's physical-world/force-model manifest digest, which must match every upstream hard-risk check for that scenario;
- the complete finite latent-atom universe inside each scenario;
- objective identity, unit, scale, objective-function model/configuration digest, and minimize/maximize direction;
- per-portfolio/per-objective integer lower and upper bounds.

The evaluator independently recomputes exact finite-support risk-budget classifications from the upstream gate's rational masses and rejects a forged or inconsistent eligibility partition. Only portfolios admitted by that gate enter the contrast matrix. Ineligible portfolio IDs remain in the report and cannot be reintroduced by objective performance.

Every scenario/latent-atom row must contain exactly one directly evaluated value for every eligible portfolio × objective cell. Atom identity is unique within a scenario. All outcomes must satisfy their frozen arm bounds. The implementation canonicalizes row and objective order, and its result digest binds the exact supplied values.

For each unordered portfolio pair, scenario and objective, the exact paired-moment kernel returns:

- the exact mean of left minus right;
- the population variance of the paired difference;
- the variance of the difference under independent pairing of the same two marginal finite distributions;
- the cross-portfolio covariance;
- whether this shared coupling reduces, preserves, or increases variance.

The contrast's declared bounds are derived from the frozen per-arm bounds:

\[
D_{a,b}\in[\ell_a-u_b,\;u_a-\ell_b].
\]

These are checked against every outcome through the individual arm bound checks.

## Robust dominance rule

Epistemic scenarios are alternatives, not probability weights. Their objective contrasts are not averaged.

For a minimized objective, portfolio A is robustly no worse than B only if

\[
\mathbb E_s[D_{s,A,B,j}]\leq 0
\quad \text{for every admitted scenario }s.
\]

For a maximized objective, it is robustly no worse only if

\[
\mathbb E_s[D_{s,A,B,j}]\geq 0
\quad \text{for every admitted scenario }s.
\]

A dominance witness requires no-worse relations for every objective in every scenario and strict improvement for at least one scenario/objective. Exact ties do not create strict dominance. If no proof exists, both alternatives remain on the frontier.

Because this version exhaustively enumerates a declared equally weighted finite support, the means and variances are exact population quantities for that support. They are **not** confidence intervals for an unknown continuous orbital distribution. They do not justify a probabilistic claim outside the frozen support.

## Why paired contrasts can be more informative

Consider two epistemic scenarios and one minimized objective:

| Scenario | A outcomes | B outcomes | Exact mean A − B |
| --- | --- | --- | ---: |
| S1 | 1, 2, 3, 4 | 2, 3, 4, 5 | -1 |
| S2 | 101, 102, 103, 104 | 103, 104, 105, 106 | -2 |

The worst marginal value for A is 104, while the best marginal value for B is 2. Comparing those marginal worst-case envelopes cannot establish A better than B. Yet the shared-atom contrast is -1 in every S1 atom and -2 in every S2 atom. The exact contrast-first oracle proves the robust finite-support dominance.

This example demonstrates a legitimate effect of common-mode cancellation. It is not evidence that common random numbers always reduce variance. The paired-moment outputs explicitly classify variance reduction, no change and variance inflation; negative cross-arm covariance can make pairing worse.

## Statistical extension boundary

The next stage is not to pretend the exact finite oracle is a confidence-sequence estimator. It is to use this finite oracle as the known-answer layer for an inference adapter.

For a stochastic/trajectory-sampled campaign, the planned simultaneous family is

\[
|\mathcal S|\;|\mathcal J|\;\frac{K(K-1)}{2},
\]

with baseline comparisons added if the baseline is an arm. One time-uniform bounded-contrast confidence sequence per cell with a preregistered family-wise allocation is the conservative reference. Because cells share rows and are dependent, the proof must rely on valid per-cell time-uniform coverage and a union bound—not on a false independence assumption.

For importance-sampled trajectory rows, the follow-on must retain the target measure, full defensive-mixture density, support coverage, exact likelihood-ratio provenance, seed schedule, latent atom identity, portfolio universe and model identity. It must reject component-only mixture densities and any proposal that opens a mode introduced by one portfolio but assigns it zero support.

Adaptive allocation, post-selection, optional stopping and model-scenario selection each require their own coverage argument. The exact finite oracle does not provide those guarantees.

## Benchmark and incumbent comparison

The right comparison is against both the incumbent Symtropy marginal-envelope frontier and established research—not against a weak, unpaired baseline.

| Reference | What it establishes | Symtropy test |
| --- | --- | --- |
| Symtropy #7670 marginal-envelope frontier | Conservative worst-scenario marginal objective comparison | Find exhaustive cases where marginal intervals abstain but paired contrasts prove dominance; verify zero false dominance |
| Symtropy #7578 exact multi-arm shared-atom moments | Exact all-pairs means, variances and covariance on one finite support | Require exact agreement of each objective contrast with an independent single-pair recomputation |
| NASA CARA 2D-Pc / 3D-Pc and Monte Carlo | Established single-encounter probability and simulation references | Use the same propagated states, covariances, hard-body radii, object pair, epoch and force model; compare Pc methods independently of the Pareto oracle |
| NASA CARA 2D-Nc / 3D-Nc and from-OD-epoch brute-force Monte Carlo | Repeated/multiple-encounter methods and high-fidelity propagation | Evaluate every portfolio's post-action event sequence and mission union; do not assume independent encounters |
| NOAA TraCSS verification dataset | Two screening answer keys for spherical and SFSH screening configurations | Validate screening detection/reproduction only; do not use its answer key as Pc ground truth |
| Constrained Pareto-set identification (Kone, Kaufmann & Richert, ICML 2025) | State-of-the-art constrained Pareto identification under bandit feedback | Compare future sampled/inference layer under the paper's stated feedback and constraint assumptions; state where shared trajectory rows and epistemic model scenarios differ |
| Fixed-budget Pareto set identification (Kone, Kaufmann & Richert, AISTATS 2024) | Finite-budget Pareto identification baseline | Compare frontier recovery at matched trajectory-call budget |
| Common-random-number multiple comparisons (Yang & Nelson, 1991) | Established use of CRN and control variates for simultaneous performance differences | Treat CRN itself as prior art; focus on domain-specific trajectory, risk-gate and evidence qualification |

### Required finite-oracle metrics

- exact agreement with all exhaustive pairwise objective means and moments;
- exact Pareto-frontier precision/recall against an independently written oracle;
- zero false dominance on exhaustive controls;
- zero acceptance of missing, duplicated or substituted scenario/atom/objective/portfolio cells;
- zero acceptance of objective unit/model/bound substitution;
- exact order invariance under row and arm permutations;
- exact risk-gate eligibility agreement;
- explicit empty-frontier behavior when no portfolio is eligible;
- p50/p95/p99 time and memory as portfolio/scenario/objective/atom cardinality grows.

### Required inference metrics (future work)

- simultaneous coverage across all scenario × objective × pair cells at every stopping time;
- false-dominance probability and false-frontier-exclusion probability;
- false unique-best selection probability;
- frontier size and unresolved/abstention rate;
- high-fidelity propagation count needed to certify a decision;
- paired-to-independent variance ratio by objective/scenario;
- performance stratified by positive, zero and negative covariance;
- independent challenger agreement and exact replay rate.

A speed claim is valid only at matched physical-model fidelity, matched error, matched simultaneous coverage and matched decision accuracy. A narrow interval from an incomplete or misspecified trajectory universe is not an improvement.

## Prior art and standards context

Common random numbers in multiple-comparison procedures are established prior art: Yang and Nelson (1991) studied simultaneous differences in expected performance using common random numbers and control variates. Glasserman and Yao (1992) analyzed when common random numbers help and when they may not.

Pareto-set identification is also an established research area. Kone, Kaufmann and Richert (AISTATS 2024) studied fixed-budget Pareto-set identification. Their ICML 2025 work studies constrained Pareto-set identification under bandit feedback and gives sample-complexity results. Chen et al. (AISTATS 2025) study multi-objective best-arm identification. Symtropy should not claim generic Pareto identification, common-random-number comparisons or constrained bandit optimization as new.

The domain-specific research opportunity is the intersection of:
- directly evaluated orbital post-action trajectory portfolios;
- dependence-preserving shared exogenous trajectory realizations;
- epistemic physical/model scenarios that must not be averaged;
- a separate hard mission-risk gate;
- simultaneous pair/objective inference and abstention;
- complete, independently challengeable evidence for the entire decision.

NASA CARA's public software covers analytical 2D and 3D collision probability, Monte Carlo probability comparisons, collision consequence, maximum probability with a single covariance, and covariance-realism assessment. NASA's publications also discuss finite total probability, repeating conjunctions, cross-correlation and high-fidelity brute-force Monte Carlo. These are reference baselines, not straw men.

NOAA's TraCSS verification dataset provides separate spherical and SFSH screening answer keys. NOAA explicitly says the data are not comprehensive and are not intended for live operations or formal certification. The dataset is a screening diagnostic, not collision-probability ground truth.

ISO 23705:2026 specifies workflow and technical requirements for identifying, evaluating and avoiding collisions among orbiting objects, including data requirements, probability/consequence methods and collision-avoidance maneuver guidance. It should guide operational workflow alignment, but passing a repository oracle does not establish standard compliance.

## Public references

- [NASA CARA — publicly available software](https://www.nasa.gov/cara/publicly-available-cara-software/)
- [NASA CARA — technical publications](https://www.nasa.gov/cara/cara-publications/)
- [NASA CARA — conjunction assessment workflow](https://www.nasa.gov/cara/)
- [NOAA TraCSS — verification dataset and limitations](https://space.commerce.gov/dataset-for-conjunction-assessment-verification/)
- [ISO 23705:2026 — scope and workflow](https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3A23705%3Aed-1%3Av1%3Aen)
- [Kone, Kaufmann & Richert (2025) — constrained Pareto set identification](https://proceedings.mlr.press/v267/kone25a.html)
- [Kone, Kaufmann & Richert (2024) — fixed-budget Pareto set identification](https://proceedings.mlr.press/v238/kone24a.html)
- [Chen et al. (2025) — multi-objective best-arm identification](https://proceedings.mlr.press/v258/chen25b.html)
- [Yang & Nelson (1991) — common random numbers in multiple comparisons](https://doi.org/10.1287/opre.39.4.583)
- [Glasserman & Yao (1992) — when common random numbers help](https://doi.org/10.1287/mnsc.38.6.884)

## Related Symtropy work

- [#7578 — exact all-pairs comparisons on shared finite atoms](https://github.com/Luminous-Dynamics/symtropy/pull/7578)
- [#7670 — scenario-robust marginal-envelope Pareto reference](https://github.com/Luminous-Dynamics/symtropy/pull/7670)
- [#7581 — simultaneous anytime-valid selection across paired portfolios](https://github.com/Luminous-Dynamics/symtropy/issues/7581)
- [#7637 — hard mission-risk feasibility across scenarios](https://github.com/Luminous-Dynamics/symtropy/pull/7637)
