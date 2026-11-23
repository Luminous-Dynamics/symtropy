# Randomized robust decision policies under source ambiguity

**Status:** finite randomized-policy game oracle, draft implementation; exact-head CI pending  
**Tracking:** [#7048](https://github.com/Luminous-Dynamics/symtropy/issues/7048)  
**Parent:** [#7034](https://github.com/Luminous-Dynamics/symtropy/pull/7034) / [#7032](https://github.com/Luminous-Dynamics/symtropy/issues/7032)  
**Branch:** `research/cmni-randomized-robust-policy-v0`

## Why this is a separate increment

The deterministic oracle in #7034 correctly finds the best single outcome-to-action mapping shared across all admitted source/dependence scenarios. That is a strong baseline, but it is not the full finite robust decision problem: a randomized policy can strictly improve worst-case utility by mixing between deterministic maps before the hidden scenario is known.

The recent robust Bayesian OED literature explicitly optimizes over stochastic design policies and develops PAC-Bayes lower bounds for finite-sample robust information objectives. CMNI should therefore not silently equate a deterministic reference with the complete robust solution. See [Abdulsamad et al., Maximin Robust Bayesian Experimental Design](https://arxiv.org/abs/2603.14094), accepted at NeurIPS 2026.

## Finite zero-sum game

For a candidate design, enumerate all deterministic maps \(\pi: \mathcal Y\to\mathcal A\). Let \(G_{\pi s}\) be the expected downstream utility of policy \(\pi\) under admitted dependence scenario \(s\).

The experimenter chooses a distribution \(q\) over deterministic maps before the hidden scenario is known:

\[
V_{\mathrm{primal}}
= \max_{q\in\Delta(\Pi)} \min_{s\in\mathcal S}
\sum_{\pi\in\Pi}q_\pi G_{\pi s}.
\]

The dual chooses a least-favourable mixture over the declared scenarios:

\[
V_{\mathrm{dual}}
= \min_{w\in\Delta(\mathcal S)} \max_{\pi\in\Pi}
\sum_{s\in\mathcal S}w_sG_{\pi s}.
\]

The value is reported only when both independently enumerated strategies are feasible and their primal/dual objectives agree within the caller's tolerance. The output includes primal and dual mixtures, objective values, feasibility residuals, complementary-slackness residual, and duality gap.

The scenario mixture is an adversarial dual witness over a finite ambiguity set. It is not a claim that physical source scenarios are randomly drawn from that mixture. The randomized decision policy is fixed ex ante and cannot condition on the hidden scenario.

## Frozen strict-randomization fixture

Fixture: `research/cmni/fixtures/randomized-robust-policy-strict-improvement-v1.json`.

Three states have prior \( (0.4,0.4,0.2)\); there are three actions, three outcomes and two admitted likelihood scenarios. The action utility is 1 for the matching state/action pair and 0 otherwise.

The best deterministic policy has maximin utility 0.40. The randomized primal strategy mixes two deterministic maps:

- \(\pi_A=(0,0,2)\) with probability \(2/3\), scenario utilities \( (0.40,0.48)\);
- \(\pi_B=(2,0,2)\) with probability \(1/3\), scenario utilities \( (0.56,0.40)\).

The resulting utility is \(34/75=0.453333\ldots\) in each scenario. The dual least-favourable scenario mixture is \(w=(1/3,2/3)\), also yielding \(34/75\). Thus the game has zero analytic duality gap.

Prior best action utility is 0.40, so randomized robust EVSI is \(4/75\). With resource cost 0.01 utility units, net robust value is \(13/300\approx0.043333\). The fixture is an analytic finite game, not physical data.

## Solver and scope

The Rust reference enumerates basic supports of the finite primal and dual games, solves each small basis with pivoted Gaussian elimination, and checks both strategies independently. The pay-off matrix is scaled before basis solving to improve conditioning across utility units. Policy and basis caps fail closed; there is no partial result represented as optimal.

The result is exact only for the supplied finite state/action/outcome/scenario game in the mathematical formulation. The implementation uses f64 arithmetic and caller-supplied tolerances, so the current status is numerical primal/dual verification, not a formal proof. It is not a continuous PSD ambiguity solver, a full Bayesian posterior model, an authenticated receipt verifier, or physical validation.

## Benchmarks and acceptance targets

| Dimension | Target |
|---|---|
| Strict randomization improvement | Recover the frozen \(34/75\) primal and dual values |
| Deterministic lower bound | Randomized maximin value never below deterministic maximin value, within tolerance |
| Primal/dual gap | At most preregistered relative tolerance; publish gap and residuals per case |
| Non-anticipativity | No policy or policy mixture may depend on the hidden scenario |
| Independent parity | Match a trusted LP solver on seeded small finite games |
| Feasibility | Zero simplex, game-constraint or complementary-slackness violations above tolerance |
| Resource caps | Fail closed on policy or basis cap exhaustion |
| Robust utility | Publish worst-case action utility, EVSI, regret, EIG and cost separately |

The next benchmark tier should compare deterministic #7034, this randomized finite game oracle, Sibson-\(\alpha\)/PAC-Bayes robust OED, and online constrained scenario-tree planning under the same admitted scenarios, reward matrices, resource budgets and total compute. For the latter, COPEx combines amortized posterior/policy models with online multi-step scenario-tree lookahead for dynamic budgets and physical constraints ([Guo et al., ICML 2026](https://proceedings.mlr.press/v306/guo26g.html)).

## Next step

Connect the finite randomized-policy game to the continuous PSD/elliptope dependence set and then to multi-step evidence campaigns. At that point the experiment policy, evidence acquisition sequence and compute allocation can be optimized under a shared non-anticipativity constraint, while an independent verifier checks the returned lower/upper decision-value bounds.
