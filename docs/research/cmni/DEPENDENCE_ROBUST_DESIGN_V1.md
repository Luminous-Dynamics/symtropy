# CMNI minimax experiment design under partially identified source dependence v1

**Status:** exploratory Rust ranking oracle authored; exact-head CI and external source review pending.  
**Implementation:** `research/cmni/reference-kernel/src/robust_dependence_design.rs`.  
**Parent capability:** `dependence_envelope.rs` computes scalar two-observation posterior/EIG extrema over an interval of source correlations.

## 1. Research question

When two experiments share calibration or reference uncertainty, the cross-source correlation can be unknown. Setting it to zero because the source IDs differ can produce a misleading experiment ranking. A plug-in point estimate can also hide decision sensitivity to uncertainty in that correlation.

The first robust-design question is therefore:

> Does the preferred experiment remain preferred for every source correlation consistent with the declared evidence?

The current reference kernel supports a narrow, exactly solvable model: two observations, one scalar parameter, two shared scalar uncertainty sources, known source marginal standard deviations, and an admissible correlation interval.

## 2. New implementation

The new `rank_two_source_dependence_designs` API accepts a finite set of candidate designs and one campaign-scope receipt reference. Candidate models must share:
- prior parameter variance;
- both shared-source marginal standard deviations;
- the same admissible correlation interval.

Candidate-specific observation sensitivities, source Jacobians and independent observation noise may differ.

For each candidate it calculates:
- nominal EIG at rho = 0, when zero is inside the admissible interval;
- minimum and maximum EIG over the entire declared correlation interval;
- posterior parameter variance bounds;
- the dependence values attaining the worst and best information;
- a nominal independence rank and a minimax robust rank;
- whether the two orderings reverse.

The robust ranking maximizes the worst-case EIG:
[
d^* \in \arg\max_d \; \min_{\rho\in[\rho_{\min},\rho_{\max}]} I_d(\theta;Y).
]

This is a decision criterion over the declared dependence set, not a probability distribution over rho. It is not equivalent to averaging EIG over a uniform correlation prior.

The scope receipt must cover the complete physical source definitions and correlation interval across the candidate family. The kernel only checks that a non-empty receipt reference is supplied; it cannot authenticate the receipt or prove the physical interval is correct.

## 3. Synthetic rank-reversal benchmark

Machine-readable fixture: `research/cmni/fixtures/nominal-vs-minimax-source-correlation-rank-reversal-v1.json`.

Two candidate designs share:
- prior variance 1;
- two shared-source marginal standard deviations of 1;
- independent observation noise variances 1;
- admissible source correlation rho in [-0.75, 0.75].

The independence baseline prefers `nominal-winner`:
- nominal EIG: 0.4489707966 nats;
- worst-case EIG: 0.3404385440 nats.

The robust design prefers `robust-winner`:
- nominal EIG: 0.3589198966 nats;
- worst-case EIG: 0.3589198966 nats.

The choice reverses. Selecting the nominal winner loses 0.0184813526 nats of worst-case EIG in this analytical fixture. The candidate's maximum EIG is not the decision objective; the minimum over the full admitted dependence range is.

This does not prove that the second design is better for any physical experiment. It shows that an independence-based ranking can reverse under a plausible, explicitly bounded dependence parameter.

## 4. Why this is a distinct robust-design layer

Current related methods address important adjacent problems:

- **COPEx / constrained sequential Bayesian design:** Guo et al. combine amortized policies and posterior networks with multi-step scenario-tree planning under dynamic budget and physical constraints. CMNI should reproduce COPEx on its published tasks, then compare whether dependence-aware likelihood qualification changes decisions on correlated-source tasks. https://proceedings.mlr.press/v306/guo26g.html
- **Maximin robust Bayesian experimental design:** Abdulsamad et al. formulate a max-min game against adversarial model misspecification subject to information-theoretic constraints, using Sibson alpha-mutual information and PAC-Bayes high-probability lower bounds. CMNI's current interval is narrower: it represents a physically interpretable covariance-correlation ambiguity set. It should be compared, not conflated, with their more general distributional adversary. https://arxiv.org/abs/2603.14094
- **Generalized Bayesian OED:** Barlas, Sloman and Kaski use Gibbs/generalized Bayesian inference and Gibbs EIG to improve robustness to outliers and incorrect outcome-noise assumptions. That concerns likelihood/noise robustness; CMNI should combine such misspecification tests with source-lineage and dependence uncertainty. https://arxiv.org/abs/2511.07671
- **Adaptive multi-fidelity EIG:** Coons and Huan use approximate control variates for high-fidelity EIG estimation, reporting unbiasedness under their assumptions and one-to-two orders of magnitude variance reduction on their examples. CMNI's value-of-computation controller should eventually optimize compute after source-dependence uncertainty is represented. https://doi.org/10.1137/25M1731812

A separate 2026 position paper argues that causal discovery benchmarks often hide untestable identifiability and consistency assumptions. This is directly relevant: every CMNI benchmark needs explicit assumption strata and failure controls, not just a headline score. https://arxiv.org/abs/2611.03166

## 5. Benchmark plan

| Benchmark | Baselines | Measurements |
|---|---|---|
| Analytic envelope | exact stationary-point extrema; independent dense grid | interval containment, worst/best correlation, arithmetic error |
| Nominal vs robust design | rho=0 plug-in, interval minimax, oracle knowing true rho | rank-reversal rate, worst-case EIG regret |
| Dependence interval width | narrow, medium, full [-1,1] | conservatism versus uncertainty width |
| Misspecified interval | true rho inside/outside the declared interval | coverage of worst-case envelope, unsafe-choice rate |
| Common-cause ablation | full joint covariance, split-independent model, unknown state | posterior undercoverage, attribution error, abstention |
| Multi-source extension | two-source exact oracle; PSD-constrained multi-source reference | worst-case regret, solver gap, feasibility |
| Sequential campaign | myopic EVSI, exact scenario tree, robust lookahead | total utility, resource violations, source qualification |

Targets (not achieved):
- zero envelope violations over dense and independently implemented correlation sweeps;
- zero strict-qualified choices when the true dependence lies outside the admitted interval or its evidence receipt is absent;
- report robust regret at the true rho against the oracle that knows rho;
- report worst-case regret against the minimax oracle;
- median regret <=1% and 95th percentile <=5% on tractable, frozen candidate suites;
- no false statement of formal certification while outward-rounded arithmetic is absent.

## 6. Limitations and next implementation

The present module remains a two-observation, one-parameter, two-source f64 prototype. It does not provide:
- outward-rounded or formally certified numerical enclosures;
- multiple parameters or more than two observations/sources;
- correlated source hyperpriors or a posterior over rho;
- uncertainty in source marginal variances or Jacobians;
- non-Gaussian, nonlinear or history-dependent source models;
- physical evidence receipt authentication.

The next step is a multi-source PSD-constrained dependence set, with a small-instance semidefinite or vertex-enumeration oracle, followed by interval arithmetic and integration with the portfolio/QoI/campaign planner. Until then, this ranking remains exploratory and cannot independently certify a physical experiment choice.


## 7. Acquire evidence about dependence before choosing a physics design

A source-dependence interval need not be static. Sometimes the most useful next experiment is a calibration that narrows the interval enough to change the subsequent physics-design decision.

The existing finite-state EVSI policy can represent this directly:
- latent states represent plausible dependence regimes;
- actions represent physical experiment designs;
- state-dependent action utilities represent design value under each dependence regime;
- the information option represents a calibration whose outcome likelihood depends on the regime;
- EVSI measures the expected improvement in the best downstream design choice;
- ENBS subtracts the calibration cost in explicit utility-equivalent units.

The fixture `research/cmni/fixtures/dependence-calibration-before-design-choice-v1.json` gives a small exact example:
- two equally likely dependence regimes;
- design A utilities 0.1 and 0.8 under the two regimes;
- design B utility 0.5 under both regimes;
- current choice is B, with utility 0.5;
- a qualified calibration has 90% accuracy, raises expected post-calibration utility to 0.615, and has EVSI 0.115;
- calibration cost is 0.03, giving net benefit of sampling 0.085;
- EVPI is 0.15, correctly bounding EVSI.

The policy therefore recommends calibrating the dependence before committing to the physical design. This is synthetic finite-state decision analysis, not a real source-correlation measurement.

### Research implication

The robust-design and EVSI modules should eventually be coupled into an adaptive ambiguity-set controller:
1. start with a defensible set of possible source-dependence structures;
2. compute minimax physical-design utility over that set;
3. enumerate possible calibration or documentation actions;
4. estimate their expected value in reducing decision uncertainty;
5. acquire the evidence when net value is positive and its provenance qualifies;
6. update the dependence set only using valid calibration evidence;
7. re-rank the physical designs and preserve the original uncertainty history.

A critical safeguard is that the calibration likelihood itself can be misspecified or share a common cause with the original measurements. Its own source dependencies must be modeled and qualified; otherwise it cannot legitimately narrow the ambiguity set.
