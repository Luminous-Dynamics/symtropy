# Joint Feasibility and Regret Benchmark v0.1

**Implementation:** `tools/orbital-observation-surface-bench/src/feasibility_regret.rs`  
**Research issue:** [#6952](https://github.com/Luminous-Dynamics/symtropy/issues/6952)  
**Composes with:** #6835 compatible worlds, #6918 point-loss regret, #6939 joint loss scenarios, #6946 per-regime regret calibration and #6947 assurance-budget composition.  
**Claim ceiling:** finite synthetic joint-feasibility and regret algebra only.

## 1. Research gap: objective uncertainty is not constraint uncertainty

Robust decision-focused learning has made important progress on uncertain objective coefficients, measurement error and deployment distribution shift. Yamao et al. (UAI 2026) use worst-case regret over coefficient uncertainty sets and Wasserstein ambiguity sets around conditional empirical distributions. This is a strong comparator for objective-side uncertainty, but the action-feasibility contract remains a separate problem. https://proceedings.mlr.press/v337/yamao26a.html

Hu et al.'s ICLR 2026 *Conformal Robustness Control* starts from an important observation: prediction-set coverage is sufficient but not necessary for robust decisions, and directly optimizing the prediction set under explicit robustness constraints can reduce conservatism while preserving the required robustness level. https://proceedings.iclr.cc/paper_files/paper/2026/hash/a80188f9b1246d4b95d396165cf99207-Abstract-Conference.html

A recent 2026 operations-research review highlights that decision-focused learning for uncertain constraints is less mature than objective-coefficient uncertainty. When processing times, capacities or service durations determine feasibility, decision quality must account for violation probability, severity or correction cost—not only ordinary objective regret. https://www.mdpi.com/2227-7390/14/22/4192

Ley et al. (OR Spectrum, June 2026) study decision-dependent robust optimization in which queries can shrink uncertainty sets at a cost. This provides a useful baseline for safe information acquisition about action availability, but a query-cost model is not proof that the feasible action universe is complete. https://link.springer.com/article/10.1007/s00291-026-00856-z

The testable systems contribution here is not a new minimax-regret theorem. It is the explicit composition of a source-bound action-universe inventory, scenario-dependent feasibility, hard static eligibility, robust feasible-set intersection and two distinct regret references.

## 2. Frozen input contract

Each case binds:
- a compatible physical-world set and source receipt identity;
- an action-universe manifest with decision scope, declared closure kind, enumeration method, evidence status, expected action IDs and digest;
- static action eligibility (`eligible`, `prohibited`, `indeterminate`), an eligibility receipt and native costs;
- a planned scenario ID inventory and digest;
- each joint physical-world / loss-model scenario, with evidence status/receipt and an exact outcome for every planned action;
- full scenario outcome digest, including feasibility status and loss assignment.

Every scenario must provide one outcome for every declared action, even when the action is prohibited or infeasible. Loss is present only for feasible actions. Infeasible, prohibited and indeterminate outcomes have null loss.

The declared finite action-universe closure is an upstream claim. The evaluator checks identity, scope, exact planned-vs-observed inventory and source receipt presence; it cannot authenticate the issuer or prove that the true domain has no omitted actions. An `open` or `indeterminate` universe cannot produce a global optimality or regret frontier.

## 3. Feasibility and regret semantics

For each admitted scenario (s), let (F_s) be the set of actions that are statically eligible and evidenced feasible in that scenario. The robust feasible action set is:

[
F_{mathrm{robust}}=igcap_{sin S} F_s.
]

Only actions in (F_{mathrm{robust}}) can be treated as one fixed action that is feasible across the entire declared scenario family. If this intersection is empty, v0.1 emits `NO_ROBUST_FEASIBLE_ACTION` and no fallback.

For each robust action (a), two regret measures are reported separately.

### Scenario-feasible / wait-and-see regret

[
R_{mathrm{WS}}(a)=max_{sin S}
left[L_s(a)-min_{bin F_s}L_s(b)ight].
]

The comparator is the best action feasible in that same scenario. This is a clairvoyant information-relaxation benchmark: it quantifies the cost of not knowing which scenario is true, but is not itself a deployable policy.

### Robust-set regret

[
R_{mathrm{robust}}(a)=max_{sin S}
left[L_s(a)-min_{bin F_{mathrm{robust}}}L_s(b)ight].
]

The comparator is restricted to actions feasible in every declared scenario. This isolates regret among actions that can be deployed without scenario-dependent feasibility risk.

Neither metric estimates a probability. Scenario counts are support enumeration counts, not frequencies or calibrated probabilities. The evaluator does not assign probabilities to the scenarios.

## 4. Status semantics

- `ACTION_UNIVERSE_OPEN`: the candidate universe is explicitly open; no global regret frontier.
- `ACTION_UNIVERSE_INDETERMINATE`: the closure kind/evidence is unresolved or out of scope.
- `ACTION_ADMISSIBILITY_INDETERMINATE`: at least one action's static rights/safety eligibility is unresolved.
- `FEASIBILITY_INDETERMINATE`: at least one eligible action has unknown scenario feasibility.
- `OUT_OF_PROFILE` / `LOSS_MODEL_INDETERMINATE`: a required scenario is outside the declared loss profile or its evidence is unresolved.
- `NO_FEASIBLE_ACTION_IN_SCENARIO`: at least one scenario has no feasible action.
- `NO_ROBUST_FEASIBLE_ACTION`: each scenario may have options, but there is no single action feasible in all scenarios.
- `ROBUST_FEASIBLE_DECISION_SET_AVAILABLE`: a nonempty intersection exists; exact regret/frontier metrics are emitted.

Statically prohibited actions are never included in feasible sets. An infeasible zero-loss action cannot influence any scenario oracle. Indeterminate evidence produces no numeric frontier.

## 5. Frozen challenge cases

The v0.1 fixture corpus has eight cases:

1. **Infeasible zero-loss comparator.** A low-loss action is feasible in one scenario but infeasible in another. It must not lower the comparator in the latter. The only robust action has wait-and-see regret (2/5) and robust-set regret (0).
2. **Action-rank reversal.** Two always-feasible actions reverse rank across scenarios; both remain in the robust set and tie on minimax regret.
3. **Disjoint feasible sets.** Each scenario has a feasible option, but the intersection is empty. No fallback is emitted.
4. **Indeterminate feasibility.** An unresolved action feasibility outcome blocks a numeric frontier.
5. **Open action universe.** A plausible-looking finite list is not treated as exhaustive when its closure kind is open.
6. **No feasible action in one scenario.** The evaluator does not hide the failure or manufacture a least-violating fallback.
7. **Multiple worlds and loss models.** Four joint scenarios across two physical worlds force the robust set intersection to span both levels.
8. **Out-of-profile loss model.** A scenario outside the admitted loss profile prevents a numeric regret frontier.

Rust tests additionally mutate the planned action inventory, omit a planned scenario while retaining world coverage, reverse input order, test static eligibility indeterminacy, and ensure prohibited actions never enter scenario feasible sets.

## 6. Comparator tournament

On matched feasibility/loss datasets, compare:
1. #6918 static-admissibility point-loss minimax/minimax-regret;
2. #6939 static-admissibility joint loss-scenario regret;
3. rectangularized regret with the same underlying loss evidence;
4. this scenario-dependent feasible-set intersection plus wait-and-see and robust-set regret;
5. soft-constraint / penalty baselines, reported separately and never accepted as a substitute for hard safety/rights gates;
6. wait-and-see lower bound (explicitly clairvoyant and non-deployable);
7. Conformal Robustness Control on matched robust prediction-set problems;
8. robust decision-focused learning on matched objective measurement error and deployment shift;
9. decision-dependent robust optimization / value-of-information baselines when a safe query can reduce uncertainty about feasibility.

Do not compare methods on mismatched constraint sets, action universes or calibration assumptions. Do not claim to beat SOTA from synthetic fixture agreement.

## 7. Metrics and benchmark targets

Report a vector frontier across:
- robust feasible action count and no-robust-action frequency;
- always/sometimes/never-feasible classifications;
- worst loss, wait-and-see regret and robust-set regret;
- false-feasible and false-infeasible classification rates on independently sampled, blinded reference outcomes;
- action-universe closure errors and missed-action rate;
- no-feasible-scenario, feasibility-indeterminate and OOD rates;
- safety/rights violations (target zero; never trade these against cost);
- native reference-trial, exposure, elapsed-time and intervention costs;
- exact receipt replay and verification overhead.

Prospective gates (targets, not achievements):
- 100% exact Rust/JavaScript BigInt oracle agreement over bounded challenge corpora.
- Zero infeasible action in any scenario comparator.
- Zero action called robustly feasible unless its feasibility is evidenced in every declared scenario.
- Zero global regret frontier with open/incomplete action universe, missing planned action/scenario or indeterminate feasibility/loss evidence.
- Zero fallback when the robust feasible-set intersection is empty.
- Zero prohibited action in any feasible set or regret comparator.
- 100% input-order invariance and exact rational tie preservation.
- Held-out false-feasible/false-infeasible intervals reported per regime with independent replication units.
- A prospective target of at least 20% lower reference-trial burden than the strongest matched baseline at equal false-feasible risk and regret bound, if independent physical evidence supports it.

The required research output is a calibrated feasibility contract, not a bigger optimizer. A mathematically optimal action over an incomplete or scenario-infeasible action set is not a robust decision.
