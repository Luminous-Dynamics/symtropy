# Non-anticipative robust decision value under source ambiguity

**Status:** finite deterministic-policy reference implementation; exact-head CI pending  
**Tracking:** [#7032](https://github.com/Luminous-Dynamics/symtropy/issues/7032) · [draft PR #7034](https://github.com/Luminous-Dynamics/symtropy/pull/7034)  
**Base:** CMNI integration PR #6455, pinned core head `6b54e2473ab4f1d45c7412beb511e1edf47ffffb`  
**Scope:** finite scientific states, finite actions/outcomes, finite admitted dependence scenarios

## 1. The missing distinction

Expected information gain answers how much an observation is expected to reveal about a modeled state. It does not, by itself, answer whether that information changes a downstream action or improves its utility.

A second problem appears when source dependence is only partially identified. An observation can be perfectly informative under every admitted scenario while the mapping from the observed label to the latent state reverses between scenarios. If the scenario is not observable when the decision is made, a decision rule that knows the scenario is clairvoyant.

CMNI therefore needs two different questions:

1. **Information:** how much does this design reduce uncertainty about the declared state?
2. **Decision value:** how much does it improve the action under one policy that must work across every admitted dependence scenario?

Do not treat the first quantity as a proxy for the second.

## 2. Non-anticipative robust objective

Let states be \(\theta\in\Theta\), actions be \(a\in\mathcal A\), outcomes be \(y\in\mathcal Y_d\), and admissible dependence scenarios be \(s\in\mathcal S\). Each scenario supplies a likelihood \(p_s(y\mid\theta,d)\), and the decision utility is \(U(a,\theta)\).

For each design, choose one policy \(\pi_d:\mathcal Y_d\to\mathcal A\) *before* the hidden scenario is known:

\[
V_{\mathrm{robust}}(d)
=
\max_{\pi_d}
\min_{s\in\mathcal S}
\sum_{\theta,y}
p(\theta) p_s(y\mid\theta,d)
U(\pi_d(y),\theta).
\]

The prior decision value is

\[
V_0=\max_{a\in\mathcal A}
\sum_{\theta}p(\theta)U(a,\theta).
\]

Thus robust EVSI is \(V_{\mathrm{robust}}(d)-V_0\). The net quantity subtracts resource cost converted using an explicit caller-supplied utility rate. A candidate outside the hard remaining resource budget is never recommended.

The clairvoyant comparator is

\[
V_{\mathrm{clairvoyant}}(d)
=
\min_{s\in\mathcal S}
\max_{\pi_d}
\mathbb E_s[U(\pi_d(Y),\theta)].
\]

The non-anticipativity gap is the difference between this upper bound and the shared-policy robust value. It measures how much a scenario-aware calculation could overstate usable decision value.

## 3. Implemented v0 reference

The Rust module `robust_decision_design.rs` adds:

- A typed finite state/action/utility contract.
- Candidate-specific outcome sets with a shared, ordered finite dependence-scenario set.
- One common dependence-scope receipt reference across the candidate family. This is a scope check only; receipt authentication is external.
- Exhaustive enumeration of deterministic outcome-to-action policies, shared across all scenarios.
- Exact finite-state \(I(\theta;Y)\) per scenario, alongside robust and clairvoyant decision values.
- Explicit resource-budget feasibility and cost-to-utility conversion.
- A cumulative policy-evaluation cap. Exceeding it returns an error; no partial ranking is labelled optimal.
- Machine-readable output that keeps EIG, robust EVSI, cost, non-anticipativity gap, and feasibility as separate metrics.

The exactness claim is deliberately narrow: finite scenario set, finite states/actions/outcomes, deterministic policies, and ordinary f64 arithmetic. It is not a randomized-policy linear program, continuous ambiguity-set optimizer, theorem-level or interval-arithmetic certificate, authenticated receipt verifier, or physical validation.

## 4. Frozen EIG-versus-decision-value rank reversal

Fixture: `research/cmni/fixtures/robust-decision-value-vs-eig-rank-reversal-v1.json`.

There are two equiprobable states, two actions, and two admitted dependence scenarios. The utility is one for choosing the correct state and zero otherwise.

**Candidate A — ambiguous perfect sensor.** Each dependence scenario makes the sensor perfectly informative, so its EIG is \(\ln 2\) nats. But the outcome/state correspondence reverses between scenarios. The best single policy has worst-case utility 0.5, equal to acting before measuring. Robust EVSI is zero. A scenario-aware policy would score 1.0, exposing a 0.5 non-anticipativity gap.

**Candidate B — stable 80%-accurate sensor.** Its outcome likelihood is the same in both scenarios. It has lower EIG, approximately 0.1927447570 nats, but the common policy obtains expected utility 0.8. Robust EVSI is 0.3.

Both candidates cost 0.05 utility units. Candidate A therefore has net robust value -0.05; Candidate B has +0.25. EIG ranks A above B, while the decision objective selects B. The fixture is analytical and synthetic, not experimental physics.

## 5. Qualification and adversarial controls

The implementation includes tests for:

- The analytic EIG/decision-value ranking reversal.
- The clairvoyant upper bound and non-anticipativity gap.
- Cost and budget handling.
- Dependence receipt scope mismatch.
- Scenario order mismatch.
- Non-stochastic likelihood rows.
- Fail-closed policy-enumeration limits.

Required next checks are exact-head formatting, Clippy, unit tests and the three-platform workspace matrix. The branch is stacked on #6455 and must not be merged ahead of its core dependency.

## 6. Benchmark targets

Targets below are research goals, not achieved performance claims.

| Dimension | Benchmark target |
|---|---|
| Small finite oracle parity | Zero regret against exhaustive deterministic-policy enumeration |
| Non-anticipativity | Zero scenario-aware policies accepted as shared policies |
| Decision-value rank reversal | 100% correct selection on frozen analytic controls |
| Nuisance-only information | Zero positive net recommendation when information has no decision utility after cost |
| Resource handling | Zero hard-budget violations; policy-cap exhaustion fails closed |
| Ambiguity robustness | Report worst-case utility and regret across all admitted scenarios |
| Calibration | Source-held-out and dependence-set-held-out coverage, not random row splits |
| Scaling | Compare approximation regret against the exact finite oracle at matched policy-evaluation budgets |

Always report EIG, robust EVSI, expected utility, regret, clairvoyant gap, cost, resource use, scenario count, policy evaluations, and abstention separately. Do not collapse them into a single score.

## 7. Relationship to current methods

- **Robust Bayesian OED:** recent maximin work uses information-theoretic ambiguity sets, Sibson alpha-mutual information and PAC-Bayes lower bounds. CMNI's finite robust decision oracle is complementary: it directly optimizes declared downstream decision utility under a finite ambiguity set, rather than replacing the robust-MI objective. [Abdulsamad et al., 2026](https://arxiv.org/abs/2603.14094)
- **Constrained sequential BED:** COPEx combines amortized policy/posterior models with online multi-step scenario-tree planning for dynamic budgets and physical constraints. CMNI's exact finite-policy utility can serve as a small decision-value oracle and as a leaf/terminal-value reference for a future scenario-tree planner. [Guo et al., ICML 2026](https://proceedings.mlr.press/v306/guo26g.html)
- **Multifidelity EIG:** approximate control variates can reduce estimator variance while preserving an unbiased high-fidelity mean under their assumptions. CMNI should use such methods to estimate expensive likelihoods/utilities, while keeping the final decision comparison and stopping rule separately qualified. [Coons & Huan, 2025](https://doi.org/10.1137/25M1731812)
- **AI-designed physics experiments:** current review work emphasizes expressive design spaces, reliable simulators, scientific objectives, and exploration over discrete and continuous choices. The natural CMNI extension is to optimize apparatus configuration, calibration, controls, simulator fidelity and scientific decision utility jointly, not only select from a fixed list of observations. [Klimesch et al., Nature, 2026](https://doi.org/10.1038/s41586-026-10898-6)

## 8. Next research increments

1. Add randomized decision policies and a small LP/game-theoretic oracle; deterministic-policy enumeration is only v0.
2. Replace finite scenario lists with continuous PSD correlation ambiguity sets, using the current tangent/SDP work to bound the inner worst case.
3. Connect robust decision value to the campaign planner so evidence acquisition, compute allocation, calibration and stopping become one constrained policy problem.
4. Add decision-focused simulation-based calibration, held-out source families and physical-data replay after the data/model lineage is independently admitted.
5. Compare against EIG/maximin EIG, Sibson alpha-MI/PAC-Bayes robust OED, COPEx-style online planning, random/space-filling designs, and human expert plans under matched compute and identical utility definitions.

No SOTA superiority, physical validity, or qualified source independence is claimed by this reference module.
