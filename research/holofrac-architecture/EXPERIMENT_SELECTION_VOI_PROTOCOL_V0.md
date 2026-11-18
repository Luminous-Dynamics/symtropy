# HoloFrac Budgeted Experiment-Selection and Value-of-Information Protocol v0

**Status:** exact finite-state synthetic decision benchmark  
**Implementation:** research/holofrac-retrieval/src/experiment_selection.rs  
**Executable:** research/holofrac-retrieval/src/bin/experiment_selection_sweep.rs  
**Workflow output:** experiment-selection-voi.csv  
**Related:** [#5961 — budgeted experiment requests and value-of-information evaluation](https://github.com/Luminous-Dynamics/symtropy/issues/5961)

## 1. Purpose

Test whether an experiment-selection policy chooses measurements that improve a decision, rather than merely maximizing uncertainty reduction or information gain.

The benchmark uses a finite prior over latent worlds, multiple design decisions with known utilities in each world, and measurement actions with explicit likelihood matrices and costs. All expected values, posteriors, mutual information and decision regret are calculated by exact finite sums. There is no Monte Carlo sampling in the oracle.

## 2. Synthetic counterexample

There are four equally likely world states, represented by two binary factors A and B:
- A determines which of two design alternatives is best.
- B is independent of the design decision.

The candidate decisions are 'choose_a0' and 'choose_a1'; each has utility 1 when its corresponding A state is true and 0 otherwise.

Available experiments:
- 'measure_a_70pct': noisy observation of the decision-relevant A factor, 70% correct, cost 0.02.
- 'measure_b_exact': exact observation of the decision-irrelevant B factor, cost 0.01.
- 'measure_a_90pct': more accurate observation of A, 90% correct, cost 0.15.

The prior value before testing is 0.5; perfect-information value is 1.0. The exact A=90% test improves expected decision value by 0.40 at cost 0.15, yielding net value 0.25. The exact B test provides 1.0 bit of mutual information but no improvement in expected decision value; after cost, its net value is -0.01.

This is deliberately constructed to expose the failure of uncertainty-only selection when information is irrelevant to the decision.

## 3. Compared policies

- NoTest: retain the best decision under the current prior.
- Random: seeded selection among tests within the declared cost budget.
- MaximumInformationGain: choose the affordable test with the greatest mutual information.
- ExpectedDecisionValue: choose the affordable test maximizing expected value of sample information (EVSI) minus experiment cost; choose no test when every net value is non-positive.

The benchmark keeps baseline expected value, expected posterior decision value, gross expected value improvement (EVSI), experiment cost, net decision value (EVSI minus cost), mutual information and expected decision regret as separate metrics. The raw sweep reports each quantity explicitly so an improvement in decision quality cannot be confused with an economically worthwhile measurement.

## 4. Exact calculations

For world state \(w\), prior probability \(p(w)\), test action \(e\) and observation \(y\):

\[
p(y)=\sum_w p(w)p(y\mid w,e)
\]

\[
p(w\mid y,e)=\frac{p(w)p(y\mid w,e)}{p(y)}
\]

\[
EVSI(e)=\sum_y p(y)\max_d \mathbb{E}[U(d,w)\mid y,e]
-\max_d \mathbb{E}[U(d,w)]
\]

\[
NetValue(e)=EVSI(e)-Cost(e)
\]

Mutual information is:

\[
I(W;Y\mid e)=H(Y\mid e)-\sum_w p(w)H(Y\mid w,e)
\]

Expected decision regret is reported relative to the perfect-information utility:

\[
Regret(e)=\sum_w p(w)\max_d U(d,w)
-\sum_y p(y)\max_d \mathbb{E}[U(d,w)\mid y,e]
\]

The implementation evaluates these quantities by explicit summation over every world and observation state.

## 5. Acceptance targets

- 100% agreement with independent exact enumeration on the finite corpus.
- Expected value, posterior probabilities, mutual information and regret must remain within declared numerical tolerance.
- No experiment above the budget may be selected.
- Decision-value policy must select no test when every affordable experiment has non-positive net expected value.
- The uncertainty-only policy must not be labelled decision-optimal solely because it maximizes mutual information.
- Raw per-policy/per-seed results must include selected action, information gain, baseline expected value, cost, expected posterior value, gross improvement, net decision value and regret.

## 6. Next benchmark extensions

1. Multi-stage sequential experiment selection with posterior update after each observation.
2. More latent worlds, correlated state variables and uncertain/contradictory observation models.
3. Noisy measurements, sensor calibration, delayed/corrected observations and model discrepancy.
4. Budget constraints across money, wall-clock, solver licenses, energy and risk.
5. Random, Latin hypercube, maximin, uncertainty-only, sequential A-optimal and EVSI/decision-regret baselines.
6. Pareto decision sets where no scalar utility is authorized; report changes to the non-dominated set and stakeholder-specific regret.
7. Registered solver identities, frozen input digests, applicability checks and immutable receipts through #5959.
8. Connect experiment request/approval/execution lifecycle to #5961, including predeclared stopping criteria and human authorization.

## 7. Claim ceiling

Passing establishes exact arithmetic/selection behavior on the finite synthetic fixture only. It does not establish real solver value calibration, causal truth, optimality under model misspecification, safety, or reduced real-world design cost. Real comparisons require registered high-fidelity solvers, matched budgets and independent outcome oracles.
