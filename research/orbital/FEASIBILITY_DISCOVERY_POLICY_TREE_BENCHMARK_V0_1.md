# Safe Feasibility Discovery and Non-Anticipative Policy Benchmark v0.1

**Implementation:** `tools/orbital-observation-surface-bench/src/feasibility_discovery.rs`  
**Research issue:** [#6956](https://github.com/Luminous-Dynamics/symtropy/issues/6956)  
**Composes with:** #6835 compatible-world identifiability, #6939 joint loss scenarios, #6946 per-regime regret calibration, #6952/#6958 scenario-dependent feasibility, and #6947 assurance-budget composition.  
**Claim ceiling:** finite passive-query policy-tree algebra over synthetic deterministic query outcomes only.

## 1. Research shift: acquire information that changes the safe decision set

The previous feasibility layer computes the intersection of actions feasible in every compatible world. That is the correct baseline for one fixed action when the true world is unknown. But if a safe measurement can reveal which world is present, a policy may choose different actions *after observing the measurement*.

This must not be confused with a clairvoyant policy. The branch must depend only on a public observation, not on a hidden world/scenario identifier. This benchmark therefore treats a query as a first-stage decision and the subsequent action as a non-anticipative recourse decision.

## 2. External research comparators

- **Ley et al., OR Spectrum (2026), decision-dependent robust optimization with controllable uncertainty sets.** Their model allows paid queries to shrink uncertain intervals and studies multi-level min-max decisions and budget deflection. This is a direct comparison for query cost and decision-dependent uncertainty, but the v0.1 evaluator is an exact finite policy enumerator, not their general optimization method. https://link.springer.com/article/10.1007/s00291-026-00856-z
- **Zhu & Bertsimas (NeurIPS 2025 workshop), overfitting in adaptive robust optimization.** They show that adaptive robust policies may become brittle when realized uncertainty lies outside an assumed uncertainty set. That motivates explicit out-of-profile and held-out policy-feasibility controls; a policy tree that is optimal only for its assumed scenario list is not evidence of transportability. https://arxiv.org/abs/2509.16451
- **Poss et al., Mathematical Programming (2026), robust K-adaptability policy bounds.** Their analysis shows that the number of policies needed for exactness depends strongly on the uncertainty structure and can grow exponentially for constraint uncertainty. Our evaluator explicitly caps finite policy enumeration and emits a limit status rather than silently truncating. https://link.springer.com/article/10.1007/s10107-026-02329-1

The benchmark should be compared on matched information timing, feasible sets, query cost and policy count. A clairvoyant action chosen from the hidden world is an information-relaxation lower bound, not a deployable baseline.

## 3. v0.1 input contract

Each discovery case embeds the validated finite base decision:
- compatible physical-world set;
- source-bound finite exhaustive action-universe declaration;
- action eligibility and native action-cost receipts;
- complete joint physical-world/loss-model scenarios with per-action feasibility and loss outcomes.

It then adds a planned query universe:
- a fixed decision scope and source receipt;
- exact planned query IDs and digest;
- a complete query-record digest;
- passive-observation query kind only;
- query admissibility and evidence receipts;
- query cost in the same native dimensions as action cost;
- exact deterministic public observation ID for every compatible physical world.

A query outcome map partitions worlds by the *same observable symbol*. The evaluator never lets a policy branch directly on a world ID or a hidden loss-model ID. All query maps are deterministic in v0.1; stochastic measurement kernels, repeated observations, active interventions and state-changing probes are future work requiring separate statistical and physical contracts.

## 4. Exact policy enumeration

For a query (q), let (W_y) be the set of compatible worlds producing the same publicly observable outcome (y). For each observation class, the evaluator computes the common feasible action set:

[
F_y=igcap_{win W_y,,min M_w}F_{w,m}.
]

If any (F_y) is empty, that query has `NO_SAFE_POLICY_FOR_OUTCOME`. The evaluator will not distinguish worlds that the query itself cannot distinguish.

When every class has at least one action, v0.1 enumerates every mapping:

[
pi:ymapsto a_y,qquad a_yin F_y.
]

The policy map is keyed only by observable outcomes. The root query and every branch mapping are frozen before the hidden world is known. Enumeration is bounded at 4096 policies per query; if the cartesian product exceeds this bound, the query reports `POLICY_ENUMERATION_LIMIT_EXCEEDED` and no partial frontier is emitted.

A no-query policy is also enumerated from the intersection across all worlds and loss-model scenarios. If the static robust feasible set is empty, no no-query policy is invented.

## 5. Three non-interchangeable benchmarks

For every valid policy, report:

1. **Static robust baseline.** The best feasible fixed action when there is no observation. The policy can use one action across all worlds only if that action is feasible everywhere.
2. **Query-conditioned policy.** A single query is executed; the public outcome selects the branch action. The evaluator reports worst-case loss, outcome-class regret, wait-and-see regret and native costs.
3. **Clairvoyant wait-and-see bound.** For each hidden scenario, this lower-bound comparator selects the best action knowing the scenario in advance. It is explicitly non-deployable and may never be presented as a policy.

Wait-and-see regret compares the chosen branch action with the best action feasible in the current hidden scenario. Outcome-class regret compares it with the best action that is feasible across all worlds/loss models that produce the same observed outcome. The latter is the relevant regret against a policy that uses only the information actually available after the query.

The value-of-information field is the exact loss reduction relative to the best no-query minimax-loss policy, when that static baseline exists. Query cost, action cost, and loss remain separate dimensions. No arbitrary exchange rate converts milliseconds or exposure into loss.

## 6. Cost accounting and Pareto frontier

The query cost is paid once. Action execution cost is represented by the componentwise maximum across the policy's branches, because only one branch executes but the world is unknown at planning time. Total resource cost is the query cost plus this branch-action upper bound.

The policy frontier is computed over:
- worst-case loss;
- worst outcome-class regret;
- reference trials;
- exposure;
- elapsed time.

No weighted aggregate hides the trade-off. A redundant positive-cost query should be dominated by an equivalent no-query policy.

## 7. Frozen challenge cases

The v0.1 corpus includes:

1. **Query resolves disjoint static feasible sets.** No single action is feasible in both worlds; a safe passive query returns `north` or `south` and permits the corresponding action. The resulting policy has worst-case loss (3/10) and zero wait-and-see regret.
2. **Blind query merges incompatible worlds.** Both worlds produce `unknown`, so no common feasible action exists in that observation class. The query produces no policy.
3. **Prohibited high-information query.** Even though it would distinguish the worlds, its admissibility receipt says prohibited; no policy is generated from it.
4. **Redundant positive-cost query.** Both worlds produce the same observation and are already covered by static robust actions. The no-query baseline dominates the paid query policies.

Rust tests additionally check query and action order invariance, no hidden-world branch mapping, no authority grant, and that query costs are separate from loss.

## 8. Status semantics

- `BASE_DECISION_INDETERMINATE`: the underlying world, action-universe, feasibility or loss evidence is not admissible.
- `QUERY_PROHIBITED`: the query is not admissible; it is not executed or optimized.
- `QUERY_ADMISSIBILITY_INDETERMINATE`: query permission is unresolved.
- `QUERY_EVIDENCE_INDETERMINATE` / `QUERY_OUT_OF_PROFILE`: observation mapping is not usable.
- `NO_SAFE_POLICY_FOR_OUTCOME`: some observable class has no action feasible across all hidden worlds in that class.
- `POLICY_ENUMERATION_LIMIT_EXCEEDED`: the finite branch combination is too large; no partial optimization result.
- `QUERY_CONDITIONED_POLICIES_AVAILABLE_NO_STATIC_BASELINE`: safe policy exists after a query, but no one fixed action is robust before querying.
- `POLICY_FRONTIER_AVAILABLE`: a static baseline and/or query policies exist and the vector frontier is emitted.

## 9. Limitations

- Query outcome maps are synthetic deterministic maps, not calibrated stochastic measurement kernels.
- The module models passive observations only. A physical intervention needs a state-transition and consequence model.
- It covers one query stage, not multi-stage adaptive experiments or optional stopping.
- It does not authenticate the action universe, query-safety issuer, observation completeness or physical feasibility evidence.
- It does not prove that policies transport to unseen worlds or out-of-profile outcomes.
- It grants no query execution authority.

## 10. Prospective gates (targets, not achievements)

- 100% exact Rust/JavaScript BigInt agreement on every enumerated policy and frontier.
- Zero policy branch keyed by hidden world or loss-model ID.
- Zero inadmissible, unqualified or out-of-profile query used to generate a policy.
- Zero policy action infeasible in any world/loss-model row that shares its observable outcome class.
- Zero omitted outcome class or world in a query inventory.
- Zero partial policy frontier emitted when the enumeration cap is exceeded.
- 100% exact policy optimality against exhaustive finite-tree oracle for bounded corpora.
- Zero dominated positive-cost query on the Pareto frontier when an equivalent no-query policy exists.
- All loss, query cost and execution cost dimensions reported separately.

The desired contribution is a verifiable link between safe information acquisition and deployable robust recourse. The test is not whether a probe is informative in the abstract; it is whether its actual observable partition makes a safe, cost-accounted policy possible without clairvoyance.
