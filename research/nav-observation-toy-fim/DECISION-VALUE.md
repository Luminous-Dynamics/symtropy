# Decision-valued observation selection — prototype v0.1

**Parent:** [NAV-OBSERVATION-001 #6438](https://github.com/Luminous-Dynamics/symtropy/issues/6438)  
**Focused issue:** [NAV-DECISION-001 #6645](https://github.com/Luminous-Dynamics/symtropy/issues/6645)  
**Status:** isolated deterministic research prototype; no physical validation.

## Research shift

Fisher information and entropy reduction quantify changes in uncertainty. They do not, by themselves, tell us whether the next observation will change the decision that the navigation service must make.

This module adds an exact finite Bayesian decision benchmark. It asks whether acquiring an observation reduces expected classification loss for a declared binary task, while accounting for acquisition success/failure and hard energy, latency and bandwidth budgets. Acquisition success can depend on the latent state, so an explicit no-observation event may itself carry information. Mutual information, decision value and feasibility remain separate quantities.

The initial state is deliberately abstract: StateZero may represent a navigation solution outside a declared service envelope, while StateOne may represent a solution inside it. The benchmark does not define the service envelope or certify the decision.

## Exact one-step calculation

Let the prior state probabilities be \(p_0,p_1\), with zero-one Bayes risk

\[
R_0 = \min(p_0,p_1).
\]

For candidate action \(a\), each outcome \(o\) has a declared likelihood under both states. The risk after a successful acquisition is

\[
R_a^{\mathrm{success}} =
\sum_o \min\left(p_0 P(o\mid S_0),\;p_1 P(o\mid S_1)\right).
\]

For acquisition-success probability \(q_a\), a failure leaves the prior unchanged:

\[
\mathbb{E}[R_a] =
q_a R_a^{\mathrm{success}}+(1-q_a)R_0.
\]

The decision value is

\[
V_a=R_0-\mathbb{E}[R_a].
\]

Mutual information is calculated independently. Under state-independent acquisition, it is the success probability times the mutual information of the acquired observation. Under state-dependent acquisition, the full observation variable has three parts: acquisition success, the measured outcome conditional on success, and an explicit no-observation event conditional on failure.

Let \(q_0=P(A=1\mid S_0)\) and \(q_1=P(A=1\mid S_1)\). Then

\[
P(\varnothing\mid S_0)=1-q_0,\qquad
P(\varnothing\mid S_1)=1-q_1.
\]

For a measured outcome \(o\),

\[
P(A=1,o\mid S_s)=q_s P(o\mid S_s).
\]

The posterior and decision risk are computed over all successful outcomes plus the no-observation event. We report both \(I(S;O\mid A=1)\) and the total information in the complete acquisition event. A state-dependent dropout model is an explicit, calibrated input; the system must never infer an informative missingness model from missing data itself.

An action is feasible only when each resource dimension fits its own hard budget.

## Known-answer counterexample

The demo uses \(P(S_1)=0.9\), so the prior Bayes risk is 0.1.

- **balanced_entropy** is a symmetric 89%-accurate observation and always leaves the most probable state unchanged under this prior. It has higher expected mutual information, but zero decision value.
- **decision_relevant_alert** has \(P(alert\mid S_0)=0.70\), \(P(alert\mid S_1)=0.04\), and acquisition success 0.8. A positive alert can change the MAP decision. Its successful-acquisition risk is 0.066, and its failure-adjusted decision value is \(0.8(0.1-0.066)=0.0272\).

The two policies therefore choose different candidates from the same action set: information-gain selects **balanced_entropy**; the exact one-step decision-value oracle selects **decision_relevant_alert**. This is a constructed counterexample, not an empirical navigation result.

## State-dependent acquisition and informative missingness

The original scalar success probability remains as a backwards-compatible state-independent shorthand. The new API accepts separate success probabilities \(q_0\) and \(q_1\). An acquisition failure is not automatically assumed to be uninformative.

A known-answer control uses prior \(P(S_1)=0.6\), uninformative measurement outcomes, \(q_0=0.2\) and \(q_1=0.8\). The measurement payload carries zero information conditional on success, but success/failure carries information about the latent state. Exact marginalization gives prior risk 0.4, expected posterior risk 0.2 and decision value 0.2.

This is a modelled missing-not-at-random example. It does **not** mean that missing measurements should automatically be interpreted as evidence of danger. The state dependence must be justified from held-out acquisition data and source/failure-domain evidence. Otherwise the acquisition model stays state-independent or unknown, and the policy must not exploit a guessed missingness signal.

## Asymmetric integrity loss

The decision objective can also use separate costs for the two wrong service claims. Let StateZero be unsafe/outside the declared envelope and StateOne be safe/inside it. A false-safe claim costs \(L_{FS}\); a false-degraded claim costs \(L_{FD}\). The prior Bayes risk becomes

\[
R_0=\min(p_0L_{FS},\;p_1L_{FD}),
\]

and each observation outcome contributes

\[
\min\left(p_0P(o\mid S_0)L_{FS},\;p_1P(o\mid S_1)L_{FD}\right).
\]

The default wrapper remains symmetric zero-one loss. The explicit-loss API rejects negative, non-finite, or jointly zero losses.

For the same alert candidate, setting \(L_{FS}=10\) and \(L_{FD}=1\) changes the baseline risk to 0.9 and its failure-adjusted decision value to 0.4512 (versus 0.0272 under symmetric loss). This is a synthetic cost-sensitivity demonstration, not an empirically calibrated ratio of navigation harms. The loss ratio must be justified by the application and independently reviewed before it is used operationally.

## Implemented checks

- finite prior and acquisition probabilities;
- unique, nonempty action and outcome IDs;
- finite likelihoods in [0, 1] with normalized outcome mass under both latent states;
- exact outcome marginalization and failure-adjusted metrics;
- separate hard constraints for energy, latency and bandwidth;
- deterministic ties and order-invariant evaluation;
- negative controls for zero acquisition success and infeasible high-value candidates.

## Limits and next steps

This v0.2 model has one binary state, a finite outcome alphabet, one-step choice and an explicitly supplied acquisition-success model. It supports symmetric or asymmetric loss and state-dependent missingness, but does not infer or calibrate either model. It does not model a continuous pose posterior, multi-step planning, correlated observation likelihoods, time-varying state, stale evidence, uncertainty over the likelihood model, or physical reachability. Likelihoods and acquisition probabilities are oracle-supplied model inputs; the prototype does not establish that they are calibrated.

Next, compose with the existing generic acquisition/resource semantics rather than creating a second planner. Add finite uncertainty sets over likelihood/acquisition models, a calibration-derived ignorance penalty, downside-risk premiums, correlated observation likelihoods, sequential Bayesian updates and a frozen truth-swap noninterference test. Bind action proposal, acquisition result, posterior update, downstream decision and stop/abstain reason into the navigation transition/replay receipt.
