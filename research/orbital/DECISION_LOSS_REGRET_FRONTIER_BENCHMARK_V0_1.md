# Decision Loss and Regret Frontier Benchmark v0.1

**Owner:** orbital decision-identifiability research, composed with [#6804](https://github.com/Luminous-Dynamics/symtropy/issues/6804) and [#6887](https://github.com/Luminous-Dynamics/symtropy/issues/6887)  
**Implementation:** `tools/orbital-observation-surface-bench/src/robust_decision.rs`  
**Input:** a finite compatible-world set supplied by the upstream identifiability layer  
**Claim ceiling:** exact loss/regret algebra over a declared synthetic compatible-world set only.

## 1. Why another layer is necessary

The finite-world identifiability evaluator distinguishes parameter ambiguity from decision-class ambiguity. It answers whether all worlds consistent with admitted evidence imply the same domain-supplied decision class.

That is necessary but not sufficient for real decision support. In many problems, no single action is optimal in every compatible world. Treating this as a binary failure to identify a decision discards useful structure; collapsing it to a point estimate, MAP world, or weighted score hides the downside of the remaining ambiguity.

This layer accepts the upstream compatible-world IDs and a domain-owned action-loss matrix. It does not reconstruct the world set, infer probabilities, authenticate the upstream receipt, decide whether losses are ethically or operationally appropriate, or authorize an action.

## 2. Three separate decision questions

For finite compatible worlds (W), admissible actions (A), and a declared loss (L(a,w)in[0,1]):

1. **Uniform optimum:** an action (a) is uniformly optimal when it minimizes loss in every compatible world:
   [
   ainigcap_{win W}argmin_{bin A}L(b,w).
   ]
   A nonempty set means the same action remains optimal despite unresolved world ambiguity. It does not imply parameter identification.

2. **Minimax loss:** minimize the worst absolute loss:
   [
   min_{ain A}max_{win W}L(a,w).
   ]
   This is the conservative absolute-risk baseline.

3. **Minimax regret:** define the per-world oracle loss (L^*(w)=min_{bin A}L(b,w)), then minimize:
   [
   min_{ain A}max_{win W}igl(L(a,w)-L^*(w)igr).
   ]
   This asks how much worse the selected action can be than the action that would have been best if the world were known.

These criteria can disagree. The evaluator preserves all ties and returns the joint Pareto frontier over worst-case loss, worst-case regret, reference-trial cost, exposure and elapsed time. It never scalarizes these dimensions and never silently chooses one frontier point.

A minimax or Pareto-optimal action is a robust choice relative to the supplied set and loss matrix; it is **not** a claim that the action is identified as the uniquely correct physical action.

## 3. Admission and evidence binding

Every case must carry:
- an opaque upstream compatible-world receipt identity;
- an explicit finite set of compatible-world IDs;
- a SHA-256 digest recomputed over the sorted compact JSON array of those IDs;
- a complete loss for each action in each declared world;
- an upstream admissibility status and receipt identity for every action labelled admissible;
- native cost dimensions kept separate.

Actions labelled inadmissible or indeterminate are excluded before computing per-world optima, minimax loss, minimax regret or the Pareto frontier. Their diagnostic IDs and statuses remain in the receipt. If no action is admissible, the result is `NO_ADMISSIBLE_ACTION`; the evaluator does not fall back to the lowest-loss excluded action.

The world-set digest proves only that the supplied ID list has not changed relative to the digest in this corpus. The opaque upstream receipt ID is not cryptographically authenticated here. The complete upstream receipt chain remains the responsibility of the evidence owner.

Losses are exact rational values in the normalized unit interval. Exact arithmetic avoids tolerance-driven tie changes. The current v0.1 envelope deliberately supports finite, fully specified loss matrices; uncertain loss intervals, randomization, stochastic policies, distributional ambiguity and multi-stage utility are follow-on work.

## 4. Frozen challenge families

The v0.1 corpus includes:
1. A uniformly optimal action that remains optimal across two parameter-distinct worlds.
2. A loss/regret disagreement where minimax loss chooses `balanced` while minimax regret chooses `aggressive`.
3. A zero-loss but inadmissible action that must never enter any frontier.
4. A case in which every action is inadmissible or indeterminate and no metric is fabricated.

The disagreement fixture is deliberate. In world (w_1), losses are (L(aggressive,w_1)=0) and (L(balanced,w_1)=0.45). In world (w_2), they are (0.80) and (0.45). Thus worst-case loss is 0.80 versus 0.45, while worst-case regret is 0.35 versus 0.45. The minimax-loss and minimax-regret actions therefore differ. A third, strictly dominated action checks that the cost-aware Pareto frontier removes dominated options without erasing the risk/regret tradeoff.

The Rust evaluator is checked against a separate JavaScript `BigInt` oracle, with receipt schemas and a field-by-field comparator. The challenge corpus is a conformance suite, not a statistical sample and not a benchmark ranking against real industrial systems.

## 5. Comparison with current research and industry

The relevant literature already contains partial-identification decision theory, minimax-regret rules, robust online decision-making and decision-focused learning. This benchmark does not claim to invent those concepts.

- Montiel Olea, Qiu and Stoye (Review of Economic Studies, 2026) study treatment choice under partial identification and show that minimax-regret optimal decision rules can be non-unique, with randomization relevant in some settings: https://doi.org/10.1093/restud/rdag015
- Christensen, Moon and Schorfheide (Review of Economic Studies, 2026) develop decision rules when payoffs are partially identified, combining minimax treatment of ambiguity with average-risk minimization for point-identified components: https://doi.org/10.1093/restud/rdag017
- Appel and Kosoy (COLT 2025) study robust online decision-making with set-valued probability models and adversarial, history-dependent distributions: https://proceedings.mlr.press/v291/appel25a.html
- Yamao et al. (UAI 2026) study robust decision-focused learning using worst-case regret under measurement uncertainty and deployment distribution shift: https://proceedings.mlr.press/v337/yamao26a.html
- Wen et al. (ICML 2026) study robust sequential experiment design under model misspecification, with synthetic and real-world A/B testing evaluations: https://proceedings.mlr.press/v306/wen26i.html
- NIST TN 2119 and ASTM E2862-23 remain domain-relevant POD and instrument-testing comparators only when the measurement and examination assumptions match; they are not substitutes for a compatible-world decision-loss evaluator.

The differentiator to test is therefore not “minimax regret is new.” It is the composition of (a) explicit observation-process identifiability, (b) a source-linked compatible-world set, (c) upstream action admissibility, (d) exact decision-risk and regret envelopes, (e) resource-aware Pareto reporting, and (f) independently replayable receipts. Each component must retain its own claim boundary.

## 6. Tournament design

On identical world sets, action sets, loss matrices, evidence scopes and resource budgets, compare:

- point estimate / MAP-world action;
- minimax absolute loss;
- minimax regret;
- uniform-optimum-only policy;
- joint Pareto frontier;
- Bayes expected loss only when an independently justified prior is supplied;
- exact exhaustive oracle on bounded finite cases (upper bound only).

Report separately:
- worst-case and per-world loss;
- worst-case and per-world regret;
- uniformly optimal action count;
- minimax-loss and minimax-regret tie counts;
- Pareto-frontier size and dominated-action count;
- excluded inadmissible / indeterminate action counts;
- false robust commits on negative controls;
- false abstention / no-admissible-action frequency;
- reference trials, exposure, elapsed time, compute and operator cost;
- replay determinism and receipt-verification overhead.

Do not compare one method on expected loss and another on worst-case regret as though the scores were commensurable. Match the utility/loss assumptions before ranking methods. Publish the complete cost-risk-regret frontier, not one aggregate score.

## 7. Prospective gates (targets, not achieved results)

- 100% exact Rust/JavaScript agreement on every frozen finite case.
- 100% recomputation agreement for the canonical compatible-world ID digest.
- Zero inadmissible or indeterminate actions in metric or frontier output.
- Zero fallback action when no admissible action exists.
- 100% exact preservation of all ties under rationally equivalent encodings.
- 100% order invariance under permutation of cases, actions and world IDs.
- 100% correct divergence on the minimax-loss / minimax-regret negative control.
- No scalar composite score in the reference receipt.
- Every output remains bound to source commit, corpus digest and upstream world-set receipt identity.

These gates establish bounded algorithmic conformance only. They do not establish that the compatible-world set contains reality, that the loss matrix is ethically justified or physically calibrated, that a recommended action is safe, or that a physical system will achieve the modeled loss.

## 8. Follow-on research

1. Integrate this loss layer with #6835's exact compatible-world receipt without giving it authority to expand or rewrite that set.
2. Compose it with #6887's time-uniform surviving-world sets. The loss and regret frontier must be recomputed as (W_t) shrinks; the guarantee for any reported decision must inherit the upstream anytime-valid coverage assumptions and alpha allocation.
3. Extend from point losses to interval-valued / set-valued losses, with explicit lower/upper regret envelopes and robust dominance.
4. Add mixed and randomized actions only where the domain semantics justify them; preserve the distinction between deterministic action IDs and policies.
5. Add adversarial loss misspecification, OOD worlds and held-out deployment shifts. A loss function is itself a model and must not be treated as ground truth by default.
6. Run matched experiments against minimax, minimax regret, Bayesian and robust decision-focused baselines before making performance claims.

The intended paradigm shift is from “which model is most likely?” to “what can we guarantee, and what does it cost, across the worlds we have not ruled out?” The answer must remain conditional on the evidence, action-admissibility and loss contracts that were actually supplied.
