# SIM-REL-009 — exact risk–cost frontier for safe adaptive experimentation

**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2958  
**Date:** 2026-10-30  
**Status:** research specification; no runtime implementation or qualification result is claimed.

## 1. Research question

What is the least expensive adaptive experiment policy that identifies a finite system while keeping the probability of at least one harmful intervention throughout the campaign below a declared risk budget?

This is not ordinary information-gain maximization. An intervention can be highly discriminative and still be unacceptable because it may damage a system, destroy a sample, exceed a thermal/electrical limit, or enter an unsafe robot state. Checking whether each experiment is locally safe does not, by itself, bound the probability of at least one harmful event over a multi-step adaptive campaign.

SIM-REL-009 adds a campaign-level risk–cost reference to the existing expected-cost, minimax-cost, and stop/abstain tracks.

## 2. Scope boundaries

This first reference deliberately uses a small finite deterministic model:

- finite semantic hypotheses H with positive integer prior masses w(h);
- finite binary interventions A with positive integer costs c(a);
- a deterministic outcome y(h,a) for each safe hypothesis/action pair;
- a pre-outcome hazard indicator u(h,a) in {0,1};
- an intervention is attempted at most once on any path;
- the intervention cost is incurred when an action is attempted;
- if u(h,a)=1 under the true hypothesis, a hazard occurs and the campaign terminates;
- otherwise the safe outcome y(h,a) is observed and the policy may adapt.

A non-hazard terminal must identify exactly one semantic hypothesis. A branch that cannot be identified through remaining actions is infeasible; it must not be silently accepted as a successful terminal. The initial release does not model hazard severity, recovery, correlated hazards, or noisy observations.

The evaluator's true hypothesis and actual hazard/outcome trace remain outside the selector. The solver receives only a declared finite model, prior masses, action costs, and hazard/outcome tables; it receives no evaluator target argument.

## 3. Objective: a Pareto frontier over complete policies

For a current belief b over the surviving hypothesis set, each complete adaptive policy induces:

- C: expected remaining experiment cost;
- R: probability of at least one hazard before successful identification.

For action a, define the posterior mass of the immediate hazard event:

r_a = sum_h b(h) u(h,a).

For each safe binary outcome y, define:

p_y = sum_h b(h) [1-u(h,a)] 1[y(h,a)=y].

When p_y > 0, the safe posterior is:

b_(a,y)(h) = b(h) [1-u(h,a)] 1[y(h,a)=y] / p_y.

The branches are disjoint and satisfy:

r_a + sum_y p_y = 1.

For one selected first action and one continuation policy from each safe branch:

C = c(a) + sum_y p_y C_y

R = r_a + sum_y p_y R_y.

The immediate action cost is paid whenever the experiment is attempted, including when the hazard branch occurs. Hazard branches terminate and contribute their full probability to R. A singleton hypothesis is an identified terminal with (C,R)=(0,0).

The dynamic program returns the nondominated set of (C,R) pairs over all admissible adaptive policies. A pair is dominated when another pair is no more costly and no riskier, with at least one strict improvement. Dominance is evaluated mathematically, not with a hand-tuned floating tolerance.

For a declared campaign risk budget alpha, choose:

C*(alpha) = min { C : (C,R) is on the frontier and R <= alpha }.

If no frontier point satisfies alpha, return an explicit infeasible result. Never silently relax alpha. Publishing the whole frontier is required; a single selected operating point is not enough to understand the trade-off.

The v1 frontier enumerates deterministic, non-randomized adaptive policies. This restriction must be explicit: under an expected-risk constraint, ex-ante randomization among complete policy trees can convexify the deterministic frontier and may change the optimum for a given alpha. Report the deterministic frontier as the primary exact reference; if randomized mixtures are added, report the convexified frontier separately with its own policy and execution semantics. Do not silently mix the two policy classes.

This objective uses the Bayesian prior in the declared model. SIM-REL-007's prior-free worst-case comparator remains separate; a future robust risk frontier can optimize over a set of priors or hazard maps.

## 4. State and algorithm requirements

A DP state must identify both the surviving semantic hypotheses and the set of actions already attempted on that path. Do not memoize only a posterior entropy or a floating-point probability vector if that loses semantic state identity.

For every state:

1. Return (0,0) for an already identified singleton.
2. Enumerate every admissible unused action.
3. Partition current prior mass into immediate hazard mass and safe outcome masses.
4. Recursively obtain each safe outcome's continuation frontier.
5. Form the Cartesian product of continuation frontier choices across nonempty outcomes.
6. Combine each product using the expected-cost and campaign-risk equations above.
7. Prune only dominated pairs and apply documented deterministic tie-breaking.
8. Return an explicit infeasible result if no non-hazard path can reach exact identification.

A bounded reference should use integer prior masses and checked rational arithmetic for probabilities, expected costs, risk comparisons and dominance. Arithmetic overflow, malformed/duplicate hypotheses, invalid costs, inconsistent action tables, or state/frontier budget exhaustion must fail closed. If implementation constraints require fixed-point or floating-point approximations, label the result approximate and independently bound its error; do not call it an exact oracle.

The Pareto set may grow exponentially. State and frontier caps are mandatory, and the receipt must report both. A cap hit is not a pass and must not return a truncated frontier as if it were complete.

## 5. Independent correctness gates

### Hand-checkable golden fixture

Use four equally weighted hypotheses h0–h3 and three actions A, B, C with costs c(A)=1, c(B)=2, c(C)=1. A is unsafe only under h3; its safe outcomes are A(h0)=0, A(h1)=0, A(h2)=1. B is safe for all hypotheses and separates {h0,h1} from {h2,h3}. C is safe for all hypotheses and separates {h0,h2} from {h1,h3}.

The exact deterministic-policy frontier for this fixture must contain:
- (expected cost 2.5, campaign hazard probability 0): choose C first; after outcome 0 use A to distinguish h0/h2, and after outcome 1 use B to distinguish h1/h3.
- (expected cost 1.5, campaign hazard probability 0.25): choose A first; h3 incurs the hazard, h2 is identified by safe outcome 1, and safe outcome 0 is resolved by C.

No other non-dominated deterministic-policy pair should remain. This fixture makes the risk budget operational: alpha=0 selects the 2.5-cost safe policy; alpha=0.25 permits the 1.5-cost policy with a 25% campaign hazard probability.

- Compare the complete nondominated frontier, not just one selected action, against an independently written exhaustive policy-tree enumerator on all sealed tiny cases (4–8 hypotheses, 3–4 actions).
- Check exact branch probability normalization: immediate hazard mass plus all safe outcome masses equals one.
- Check exact agreement of expected cost and campaign hazard probability for every reference policy.
- Check monotonicity: increasing alpha cannot increase the minimum feasible expected cost; tightening alpha cannot increase the selected risk.
- At alpha=0, no action with positive posterior hazard mass may be selected.
- Every non-hazard terminal must be a singleton; every hazard and unresolved branch must remain a non-success terminal.
- Check input-order invariance, deterministic tie-breaking, duplicate rejection, malformed table rejection, action-mask semantics, and checked-arithmetic failure.
- The selector API must accept no evaluator target; negative controls should make target leakage observable.
- Test unreachable hypotheses, zero safe mass, all-hazard actions, uninformative actions, equal-risk/equal-cost ties, no-feasible-policy cases, and frontiers with multiple non-dominated trade-offs.

## 6. Benchmark programme

### Baselines

Compare under identical hypothesis support, prior, intervention tables, costs, and risk definition:

- unconstrained Bayes EIG/cost;
- maximum disagreement;
- thresholded safe-greedy EIG/cost;
- hard robust safe-only selection (an action is admissible only if every supported hypothesis marks it safe);
- SIM-REL-005 expected-cost oracle where safety constraints are disabled;
- SIM-REL-007 prior-free minimax oracle where the comparison objective matches;
- SIM-REL-008 stochastic finite-horizon policies after joint observation/hazard likelihood semantics exist;
- published safe Bayesian optimization algorithms only on matched tasks with compatible domains, feedback, assumptions, and risk definitions.

### Predeclared synthetic sweep

Start with alpha in {0, 0.01, 0.05, 0.10, 0.20}, several positive action-cost ratios (including {1,1,1} and {1,2,4}), multiple prior families, and multiple hazard-map families. Keep training/tuning cases separate from sealed family-level test worlds.

Include adversarial fixtures where:

- the most informative action is hazardous under a small but consequential posterior mass;
- a locally safe greedy action consumes most of the campaign risk budget before a critical later experiment;
- repeated safe outcomes leave a multi-hypothesis equivalence class;
- no policy can identify all non-hazard paths within the action budget;
- the nominal prior understates the mass of hazardous hypotheses;
- one action is safe for every in-support hypothesis but an omitted family would make it unsafe (a model-adequacy negative control).

### Required metrics

- expected cumulative experiment cost and worst-case realized cost;
- true probability of any campaign hazard and empirical risk-budget violation rate;
- exact identification probability without hazard;
- full nondominated frontier, selected point, and gap to the constrained exact reference;
- proportion of tasks with no feasible policy at each alpha;
- risk calibration (Brier/log score on stochastic extensions), and sensitivity to prior/hazard-model shift;
- runtime, peak memory, number of memoized states, peak frontier size, and arithmetic/cap failures.

Physical simulator families should include at least one mechanical overload/clearance system, one thermal or materials sample-damage system, and one grid/energy protection-limit system. These are not physical safety certifications. Each simulator needs an independently validated hazard map and a held-out family-level test.

## 7. Qualification targets (targets, not results)

- 100% agreement with independent brute-force frontier enumeration on all sealed tiny cases.
- Zero campaign-risk-budget violations in the exact deterministic finite harness.
- No false exact-identification success on a hazard or unresolved branch.
- For policies feasible at alpha=0, zero hazardous interventions.
- At matched identification probability and campaign risk, target at least 10% lower median experiment cost than the strongest practical safe-greedy baseline, with paired uncertainty intervals excluding zero on held-out families. This is an ambition, not an observed result.
- Publish the complete frontier and all failed/infeasible cases; do not collapse results into one alpha or one headline mean.

## 8. Relation to the existing SIM-REL tracks

- SIM-REL-003 / PR #2916: practical MDL-prior EIG selector. Existing evidence shows a small mean-cost edge over uniform EIG with an OOD-detection trade-off; this track adds a separate safety dimension, not a claim that MDL-EIG is generally better.
- SIM-REL-004 / issue #2921: open-world adequacy. A closed finite support can be confidently unsafe or wrong; posterior risk is conditional on the supplied model.
- SIM-REL-005 / PR #2928: finite deterministic Bayes expected-cost oracle.
- SIM-REL-006 / PR #2930: stop/abstain/act losses. It is a separate terminal-utility layer.
- SIM-REL-007 / PR #2941: prior-free deterministic minimax cost oracle.
- SIM-REL-008 / issue #2944: noisy and persistent observations. A later integration should define a joint likelihood P(y,z | h,a) for observation y and hazard event z.

The first implementation should remain a separate, dependency-free Rust crate and a separate receipt schema. Do not modify the existing oracles' semantics in order to force integration. Add a benchmark-only adapter after independent qualification.

## 9. External research context and fair comparison

- Bottero et al., *Information-Theoretic Safe Bayesian Optimization*, Journal of Machine Learning Research 27(145), 2026: https://www.jmlr.org/beta/papers/v27/23-1687.html. This work develops information-theoretic safe exploration for continuous Bayesian optimization and analyses high-probability safety under its model assumptions.
- Zhang, Park & Simeone, *Bayesian Optimization With Formal Safety Guarantees via Online Conformal Prediction*, IEEE Journal of Selected Topics in Signal Processing, 2024: https://doi.org/10.1109/JSTSP.2024.3422825. This gives controllable nonzero violation rates using online conformal prediction, including noisy and noiseless safety feedback.
- Wei et al., *Safe Bayesian Optimization for the Control of High-Dimensional Embodied Systems*, CoRL 2025: https://proceedings.mlr.press/v270/wei25b.html. This addresses high-dimensional embodied control with probabilistic safety constraints and cumulative violation bounds.
- *Anubis: Bayesian optimization with unknown feasibility constraints for scientific experimentation*, Digital Discovery, 2025: https://doi.org/10.1039/d5dd00018a. This is especially relevant to experimental campaigns with previously unknown feasibility constraints.

These are relevant industry/research baselines, not interchangeable algorithms. Match each task's design space, observation and safety feedback, regularity assumptions, risk metric, budget, and compute. A result on three-input Boolean hypotheses does not beat continuous safe BO.

## 10. Claim boundary

The reference is exact only for its supplied finite support, prior, deterministic observation and hazard maps, fixed costs, action-once semantics, and campaign-level risk definition. It does not guarantee real-world safety, protect against omitted hazards, model continuous dynamics, validate a physical hazard envelope, or establish SOTA superiority. Physical use requires independent hazard-model validation, open-world/discrepancy controls, conservative limits, and external safety review.
