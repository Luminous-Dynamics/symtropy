# Contingent two-step active sensing — prototype v0.1

**Issue:** [NAV-SEQUENTIAL-VOI-001 #6689](https://github.com/Luminous-Dynamics/symtropy/issues/6689)  
**Status:** exact finite synthetic oracle; no physical validation.

## Research shift

A sensor observation changes the belief and therefore changes the value of the next sensor. A one-step policy scores each sensor under the initial belief and can miss this interaction.

The horizon-two oracle evaluates every candidate first action, enumerates its successful measurement outcomes and explicit no-observation event, updates the posterior for each event, then chooses the best feasible *different* second action on each branch. The second-action choice is contingent on the first observation.

A greedy comparator first selects the best one-step action under the initial belief, then optimizes the follow-up per branch. Both policies use identical likelihoods, candidate actions, asymmetric loss and resource budgets.

## Probability model

For latent states S0 and S1 with prior p0,p1, acquisition success probabilities q0,q1 and measurement outcome o:

[
P(o,mathrm{success})=p_0q_0P(omid S_0)+p_1q_1P(omid S_1)
]

[
P(mathrm{no observation})=p_0(1-q_0)+p_1(1-q_1)
]

The posterior is updated from the corresponding joint state probabilities. Therefore, a no-observation event is informative only when the declared q0 and q1 differ. If the acquisition model is unknown, this signal must not be exploited.

The v0 oracle assumes successive observations are conditionally independent given the same static binary latent state. It does not model common-mode sensor errors, temporal state evolution, shared calibration drift or cross-sensor likelihood dependence. Those are necessary next extensions.

## Implemented comparisons

- Exact contingent horizon-two plan: globally optimizes the first action and each branch's second action.
- Myopic-first horizon-two plan: chooses first action using one-step expected Bayes risk, then optimizes each second action.
- Hard energy, latency and bandwidth constraints at each path.
- No action can be reused.
- Asymmetric false-safe / false-degraded loss through the shared decision-value API.
- No positive expected value returns no plan rather than an arbitrary action.

## Known-answer counterexample

With prior (P(S_1)=0.9), the symmetric 89%-accurate balanced observation has zero one-step decision value, and the standalone adverse alert has positive one-step value. Yet the balanced observation creates a branch close to the decision boundary where a second 80%-accurate measurement becomes valuable. The horizon-two oracle may therefore choose the balanced first action while a myopic first-step policy chooses the adverse alert.

The demo and tests use constructed probabilities. This is a mathematical planning example, not a measured navigation effect.

## Limits

Finite binary state, horizon two, conditional independence, finite action set, known likelihood/acquisition models and one-step terminal decision. Not a continuous pose planner, physical sensor selector, model-uncertainty guarantee or safety qualification.
