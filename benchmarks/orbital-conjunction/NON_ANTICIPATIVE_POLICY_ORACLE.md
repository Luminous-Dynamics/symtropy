# Non-anticipative orbital policy oracle

Status: finite one-decision-epoch verification prototype. This is not a general POMDP solver, belief-space planner, statistically calibrated chance-constrained controller, flight software or maneuver authorization system.

## Motivation

A static portfolio answers: which fixed manoeuvre is feasible and Pareto-efficient across the declared scenarios?

A belief-space policy asks a stronger question: should the system wait for more tracking information, and how should its action change after the information is actually observed?

The policy must be a function of observable history. The decision-schedule digest binds the availability indices to the declared absolute-time/time-scale semantics; an integer epoch alone is not a physical timestamp:

\[
\pi(h_t)\rightarrow a_t.
\]

It must not branch on a latent scenario ID, hidden world ID, future measurement or information that would not be available to the operator at the declared decision time. Two worlds with the same observable history must receive the same action under the same policy.

## Comparator: belief-space MCTS

Kim, Eddy and Kochenderfer (2026), *Chance-Constrained Belief-Space Maneuver Planning for Autonomous Collision Avoidance Under Uncertainty*:

- [arXiv:2609.13428](https://arxiv.org/abs/2609.13428)

The paper uses Gaussian belief states and chance-constrained belief-space MCTS to choose between waiting for additional tracking and manoeuvring before TCA. It studies eight historical NASA CARA conjunctions across 96 measurement-quality/cadence configurations (three measurement-quality conditions and four cadences). It reports zero observed terminal chance-constraint violations for its chance-constrained planners in those tested configurations; the no-manoeuvre resolution fraction varies substantially with tracking quality and cadence.

This is a strong comparator, not proof of universal safety. The paper's experiment is scoped to a manoeuvrable primary / non-manoeuvrable secondary and cases for which 2D encounter-plane Pc is appropriate. Its chance constraint is evaluated from a finite number of MCTS rollouts. Reproduction must preserve its object/epoch/dynamics/measurement assumptions and compare on matched conditions.

## v0.1 scope

The current Rust verifier covers one frozen decision epoch after a declared observation window. The caller supplies:

1. A validated hard mission-risk gate.
2. A validated shared-exogenous trajectory receipt report containing one row per epistemic scenario × latent world and one trajectory per eligible portfolio.
3. A complete measurement inventory for every scenario/world row, including measurements arriving after the decision epoch.
4. A frozen policy implementation identity and a finite table mapping canonical observable-history digests to eligible portfolio IDs.

The verifier freezes both a logical decision-epoch index and a nonzero decision-schedule digest. That digest must identify the mapping from logical epochs to absolute instants, time scale and delivery cut-offs. The verifier checks the trajectory-receipt report digest, revalidates the hard-risk gate, reconstructs the exact scenario/world/eligible-portfolio trajectory key universe, and rejects omissions or substitutions. Each selected policy action is linked to the exact propagated trajectory digest for that scenario/world/portfolio.

This makes the policy map non-anticipative over the supplied finite observation corpus: policy keys contain only sensor/channel identity, sample sequence, availability epoch, measurement-value digest and declared quality digest. Hidden scenario/world IDs, source receipt digests and future observations are excluded from the policy key. Those identities remain in the audit report.

The verifier requires exactly one policy branch for every observable history in the corpus, no unused branch and no ineligible action. If two hidden worlds induce the same observable history, the map has exactly one action for that history.

## Hard-risk and policy boundaries

The upstream hard-risk gate defines the eligible action universe before policy evaluation. A policy cannot select a portfolio that failed the gate in any required epistemic scenario.

**Static action eligibility does not imply policy-conditioned mission-risk feasibility.** An observation-conditioned selector can choose different individually eligible portfolios in different latent worlds and combine their disjoint contact sets into a larger union than either static portfolio.

The follow-on in [#7734](https://github.com/Luminous-Dynamics/symtropy/issues/7734) is now implemented on the policy-risk branch: shared world receipts carry exact target mass ticks; portfolio trajectory receipts carry a definite pathwise mission-union contact classification; the receipt validator re-derives each eligible static action's exact mission union; and the policy report recomputes the selected union after applying the non-anticipative action mapping. Its per-scenario exact mass and unchanged budget are reported separately. A structurally valid policy can therefore be explicitly marked mission-risk infeasible rather than inheriting the static gate's pass.

This prototype does **not** yet prove that:
- the measurement inventory is complete relative to the physical tracking network;
- the measurement values or quality models are calibrated;
- the policy code digest corresponds to executable code that ran;
- the policy was committed before held-out outcomes were observed;
- the action's execution noise is statistically calibrated;
- the chance-constraint or mission-risk confidence interval has valid coverage across repeated policy selection;
- a future observation will actually arrive on schedule;
- a manoeuvre is authorized by a real operator.

An observation inventory digest is a content identity for supplied evidence. It is not an issuer signature or completeness proof. A validated trajectory receipt is a structural cross-link, not evidence that the declared propagator executed honestly.

A missing or out-of-profile observation must eventually map to a separate abstention / emergency response policy. This v0.1 verifier does not manufacture a fallback action.

## Causal coupling

Reuse the #7721/#7724/#7726 distinction:

- Common exogenous pre-action state/covariance, environment, target measure, density space, frame, time and epoch.
- Observation stream as generated by the declared measurement model and actual action-dependent sensor geometry.
- Portfolio-specific intervention and manoeuvre execution noise.
- Portfolio-specific post-action state, trajectory, encounter inventory and hard-risk binding.

Do not force observations to be identical after trajectories have diverged when the sensor geometry makes them different. Do not force action-conditioned actuation noise to be common across portfolios. Reuse a disturbance draw only when it represents a physically meaningful shared random variable under the declared causal model.

## Adversarial known-answer corpus

Required controls include:
- Same observable history across different hidden scenarios/worlds yields one action.
- Different observable histories may map to different eligible actions.
- A future measurement arriving after the decision cannot change the current policy key or action.
- Different source receipt IDs do not create an observable-history branch by themselves.
- A policy attempting to add a branch for a hidden world with no distinct observable history is rejected.
- Duplicate policy branches for the same observable history are rejected.
- Missing world traces, missing policy branches, duplicate measurement sequence IDs and substituted inventory digests fail closed.
- An action outside the hard-risk-eligible portfolio set is rejected.
- Selected portfolio must have an exact scenario/world trajectory receipt.
- Input row permutation preserves the policy result digest.

The finite oracle should be cross-checked against an independently written policy-table implementation. A branch-count match alone is not enough: compare the exact mapping from each independently derived observable history class to the selected action.

## Benchmark plan

Reproduce the Kim et al. eight-CARA-case corpus and its 96 tracking conditions, preserving the published model scope. Compare:
- static robust manoeuvre portfolio;
- deterministic wait-threshold rule;
- finite exact policy-table oracle;
- belief-space MCTS with the published finite-rollout chance constraint;
- clairvoyant per-world action as an information-relaxation lower bound only.

Then extend the corpus to:
- multiple manoeuvre portfolios and different action execution noise;
- repeated encounters and hard mission-union risk;
- epistemic atmosphere / covariance / physical-model scenarios;
- action-dependent observation geometry;
- multiple measurement-source delays and out-of-profile observations;
- fuel, downtime, sensing cost and coordination cost as distinct objectives;
- finite multi-stage policy trees, where non-anticipativity is checked at every observable history.

Measure:
- false-feasible / terminal chance-constraint violation rate;
- mission-union violation rate;
- safe no-manoeuvre resolution fraction (never optimize this alone);
- delta-v / fuel, mission downtime and observation cost separately;
- worst-case regret and clairvoyant gap;
- Pareto frontier precision/recall;
- policy-tree size and runtime;
- observation coverage, abstention and out-of-profile rates;
- non-anticipativity violation detection;
- simultaneous/time-uniform coverage under adaptive observation and stopping.

Do not claim a speedup or a safety improvement unless dynamics, sensor model, hard-risk definition, statistical coverage, rollout/propagation budget and decision threshold are matched.

## Next steps after the finite verifier

1. Add a canonical observation-source completeness receipt and held-out policy commitment mechanism.
2. Extend from one decision epoch to finite multi-stage policy trees with history-dependent non-anticipativity.
3. Add a belief update receipt and calibration contract; do not trust a posterior label without its prior, likelihood, data and update method.
4. Add safe abstention, wait and emergency states as explicit policy actions with resource costs.
5. Integrate adaptive fidelity and decision-aware refinement from #7178, with no relaxation of the hard-risk gate.
6. Only then compare MCTS, robust K-adaptability, belief-space optimization and POMDP solvers.

## Related work

- [#7727 — non-anticipative belief-space manoeuvre policy research](https://github.com/Luminous-Dynamics/symtropy/issues/7727)
- [#7724 — shared-exogenous trajectory receipts](https://github.com/Luminous-Dynamics/symtropy/pull/7724)
- [#7726 — portfolio-specific manoeuvre execution noise](https://github.com/Luminous-Dynamics/symtropy/pull/7726)
- [#7701 — paired-objective Pareto oracle](https://github.com/Luminous-Dynamics/symtropy/pull/7701)
- [#7637 — hard mission-risk feasibility gate](https://github.com/Luminous-Dynamics/symtropy/pull/7637)
- [#7178 — decision-aware multi-fidelity refinement research](https://github.com/Luminous-Dynamics/symtropy/issues/7178)
