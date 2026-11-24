# Observable Incumbent Policy Adequacy v0.1

**Status:** synthetic adapter and single known-answer smoke corpus; exact-head qualification in progress.  
**Owner:** #7295 — ORBITAL-INCUMBENT-POLICY-ADEQUACY-001.  
**Implementation:** `tools/orbital-assurance-bundle-bench/src/policy_adequacy.rs`.  
**Claim ceiling:** `synthetic-observable-policy-maintenance-algebra-only`.

## 1. Research question

A deployed static action may remain adequate under a common-action comparison, yet a safe, admissible observation can reveal information that permits a better non-anticipative policy. Conversely, a per-world hindsight selector can appear arbitrarily good by using hidden-world identity unavailable to the decision-maker.

This benchmark asks:

> Does the incumbent policy remain within its declared worst-case terminal-loss and resource envelope when compared with already-qualified policy candidates whose branches depend only on observable outcomes?

It is decision maintenance, not another optimizer or policy-tree generator.

## 2. Composition and ownership

Consume, do not reimplement:
- **#6997 — feasibility discovery:** observable outcome classes, branch-to-action mapping, per-scenario terminal losses/regrets, query and branch costs, and the finite policy Pareto frontier.
- **#6958 — feasibility/regret:** scenario-specific feasible actions, joint loss-model semantics and the exact loss units.
- **#6835 / #6887 / #7291:** compatible-world identity and sequential evidence/cutoff. The #7291 receipt bridge is separate; this adapter does not manufacture or inherit an anytime-valid guarantee.
- **#6939 / #7161:** coherent joint physical-world/loss-model inventory. Several loss models may share one physical world.
- **#7001 / #7093 / #6598:** independent action-domain closure, dependency-graph adequacy and selective evidence invalidation.
- **#7105 / #7117:** static common-action incumbent adequacy remains the required no-query baseline.

This adapter consumes an already evaluated frontier. It does not enumerate policy trees, generate candidate policies, optimize a new query, calculate EIG, infer observation likelihoods, or execute queries/actions.

## 3. Input and source-binding contract

The normalized case binds:
- upstream #6997 receipt schema, claim ceiling, source commit, receipt ID/digest, source case ID and source scope ID;
- scope digest and decision cutoff identity;
- #6835 compatible-world receipt identity;
- #6939 joint loss receipt identity and joint scenario manifest digest;
- exact joint scenario rows: `(scenario_id, world_id, loss_model_id)`;
- a canonical SHA-256 over the full joint scenario identity records and a separate digest over scenario IDs;
- the complete normalized policy-frontier payload, including per-scenario rows and summary metrics, by source-receipt digest;
- the incumbent policy ID, not merely the best no-query comparator;
- separate action-domain, dependency-graph, evidence-currentness and model-family states.

**Important source boundary:** the current v0.1 schema is a normalized typed projection of #6997's `PolicyMetric` and `CaseResult`, not a direct deserialization of the raw upstream workflow receipt. The adapter requires the upstream metadata to be mapped into this normalized envelope. The next integration step is a source-specific extraction/replay adapter that verifies the actual #6997 artifact and maps its exact receipt/case/frontier fields; the synthetic all-a/all-b source pins in the smoke fixture are not real source evidence.

SHA-256 and opaque IDs provide structural binding only. They do not authenticate an issuer, prove receipt truth, or establish that the scenario family includes reality.

## 4. Preserving the upstream policy semantics

Each policy preserves the upstream fields:
- `policy_id`, `query_id`;
- `branch_actions`, keyed only by public observed outcome (or `__NO_QUERY__`);
- `observation_class_world_ids`, the distinct physical worlds in each public observation class;
- `worst_case_loss`, `worst_case_wait_and_see_regret`, `worst_case_outcome_class_regret`;
- the three corresponding worst-case scenario-ID sets;
- `worst_loss_reduction_vs_best_no_query`, where provided;
- query cost, worst-branch action cost and total cost upper bound;
- per-scenario world/model IDs, observed outcome, selected action, loss, wait-and-see regret and outcome-class regret.

The adapter checks:
1. Every scenario row is in the exact planned joint inventory, with the same world and loss-model identity.
2. Every planned joint scenario is covered once; duplicate, missing and unknown rows fail closed.
3. Every scenario's selected action equals the branch action keyed by its observable outcome.
4. The branch domain exactly equals the observed outcome domain; missing and extra branches are invalid.
5. The declared outcome-class world inventory exactly equals the worlds reconstructed from per-scenario rows.
6. Multiple loss models for the same physical world share the same observation outcome. A policy cannot separate them by hidden loss-model ID.
7. Scenario/world/loss-model identifiers cannot be smuggled in as observation labels.
8. Declared maximum loss and regret values, plus their worst-case scenario-ID sets, agree with the per-scenario values.
9. The complete normalized frontier payload digest matches the source pin.

The clairvoyant world-revealed policy is never used as a reference policy. A review result does not select or return a candidate policy.

## 5. Decision and typed resource rule

Let `L(pi,s)` be the terminal loss of policy `pi` in joint scenario `s`. For every policy:

`M(pi) = max_s L(pi,s)`

Let `pi_0` be the identified incumbent policy, `P_frontier` the source-pinned upstream Pareto frontier, and `B` the resource budget vector.

A candidate policy is eligible only when:

`query_cost + worst_branch_action_cost + policy_transition_cost <= B`

The comparison is dimension-by-dimension, not a weighted scalar. The current native vector is:
- reference trials;
- exposure micro-units;
- elapsed milliseconds.

Energy or additional domain costs must be added as explicit typed dimensions before use; they are not silently treated as zero. Loss remains in its own declared loss unit. Policy switching cost is a native resource vector and is **not** added to the terminal-loss materiality margin.

Among eligible frontier policies, the evaluator calculates the best reference worst-case loss:

`M_ref = min M(pi)` over eligible frontier policies other than the incumbent.

It compares `M(pi_0) - M_ref` to the declared materiality margin:
- **RETAIN_INCUMBENT_POLICY:** no eligible alternative exists, or the best eligible alternative does not exceed the materiality margin. Equality retains.
- **POLICY_REVIEW_WARRANTED:** the incumbent is infeasible, exceeds its declared resource envelope, or an eligible observable policy demonstrates improvement strictly above materiality.
- **ABSTAIN:** source/world/loss/model/evidence/closure state is not current or qualified, or the incumbent feasibility state is indeterminate.
- **INVALID_INPUT:** malformed source pins, inventory/payload digest mismatch, hidden-state branch, incomplete outcome/scenario map, inconsistent regret summaries, arithmetic overflow or malformed costs.

The comparator is analytical evidence for a review, never a recommendation, authorization or executable action.

## 6. Current frozen smoke example

The initial synthetic known-answer case deliberately demonstrates:
- incumbent static policy worst-case loss: 10;
- candidate observable policy worst-case loss: 1;
- declared materiality margin: 2;
- query + branch execution + transition costs fit the native resource budget;
- improvement: 9, strictly above the margin.

Expected result: `POLICY_REVIEW_WARRANTED` with zero authority grants. The output does not return the candidate ID as an action recommendation.

The Rust unit tests additionally cover:
- exact materiality equality;
- candidate resource-budget rejection in multiple dimensions;
- incumbent infeasibility;
- stale source receipt;
- wrong scope and cutoff;
- changed joint scenario world/model mapping;
- changed policy payload;
- hidden-world IDs as branch labels;
- same physical world with different loss models but a single observable outcome;
- conflicting outcomes across loss models in one physical world;
- incomplete scenario rows and observation branches;
- altered observation-class world map;
- inconsistent per-scenario regret and worst-case scenario-ID summaries.

## 7. Benchmark tournament and targets

Use identical compatible worlds, joint loss-model scenarios, loss units, decision cutoff and resource budgets:
1. retain the incumbent forever;
2. static common-action comparator from #7117;
3. fixed-calendar query/replanning policy;
4. #6997's full non-anticipative Pareto frontier;
5. decision-class EIG-per-cost and maximin information baselines only where their probabilistic assumptions are explicitly supplied;
6. RADAR-style regret monitoring under its stated distribution/preference assumptions;
7. exact exhaustive policy oracle on tiny finite domains;
8. hidden-world clairvoyant selector as an unattainable information upper bound only.

Primary metrics (no aggregate score):
- false retain when the exact policy oracle shows material eligible improvement;
- false review when all candidate policies are dominated or outside resource limits;
- worst-case terminal loss and exact regret;
- query trials, exposure, elapsed time and transition costs separately;
- frontier completeness and dominated-policy rate;
- hidden-state branch leakage escapes;
- incorrect observation-class world inventories;
- source/payload digest tampering escape;
- abstention and invalid-input precision;
- runtime/memory p50/p95/p99 at increasing world × loss-model × policy counts;
- reviewer time and defect discovery on blinded engineering tasks.

Prospective gates: zero hidden-world branch leakage on exhaustive finite cases; zero frontier-summary inconsistencies; zero false retains/reviews against the tiny exact oracle; zero resource-envelope escapes; zero receipt tampering escapes; zero authority grants. Stochastic power/false-alarm targets require preregistered error budgets and independent held-out trials. These are research targets, not results from the single smoke case.

## 8. SOTA and industry positioning

#6997's finite policy-tree algebra is a local upstream capability; this adapter adds incumbent identity, source-pinned maintenance, joint scenario consistency, typed transition-cost constraints and a review-only disposition. It does not claim to beat PEAR on regret-gradient efficiency, robust DFL on uncertainty modelling, or RADAR's statistical operational-suboptimality guarantees.

For a fair external tournament, run PEAR and robust DFL on their matched published LP/QP/resource-allocation tasks, and RADAR on matched incumbent-shift tests. Separate native metrics and assumptions. In particular, do not use a finite synthetic policy-tree result to claim a SOTA statistical advantage.

The strategic differentiation remains the connected evidence execution plane: independently challenged domains, current and correctly scoped source evidence, decision-quality comparators, non-anticipative information acquisition, selective invalidation and reproducible maintenance across the decision lifecycle. Integrate with Ansys/medini, Polarion and SACM/GSN rather than pretending to replace their domain workflows or standards.

## 9. Claim ceiling and promotion gates

The current implementation can establish only deterministic finite algebra and receipt-contract checks over synthetic declared outcome classes and joint scenarios. It does not establish:
- that any query's observation mapping is physically true or safe;
- that a query is statistically informative under stochastic/overlapping supports;
- an anytime-valid or optional-stopping guarantee;
- that the candidate world/model family contains reality;
- source issuer authenticity or independent external authority;
- safety of any deployment or execution.

Before production composition: pin and replay the actual #6997 source artifact/schema, implement the exact extraction bridge, connect the actual #6887/#7291 sequential receipt and cutoff, and independently qualify the metrology/query-safety/authority path.

No production action, query or replacement policy is selected or executed.
