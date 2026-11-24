# Incumbent Anytime-Valid World-Set Receipt Bridge v0.1

**Status:** proposed synthetic receipt adapter; it does not generate or independently prove a statistical guarantee.  
**Owner:** [#7291 — sequential receipt bridge](https://github.com/Luminous-Dynamics/symtropy/issues/7291).  
**Composition:** #6887 → this adapter → #7161 → #7105.  
**Claim ceiling:** structural validation and exact inventory/cutoff composition only.

## 1. Research question

Can the incumbent-decision gate consume a time-uniform surviving-world set produced by an adaptive sequential evidence process, while preserving the upstream statistical contract exactly and preventing future observations, unregistered probe selection, or model-family gaps from being hidden at the adapter boundary?

This is an integration layer, not a new e-process, confidence sequence, sensor model, experiment planner, optimizer, evidence store, or authority service.

## 2. The gap this closes

PR #7161 currently consumes a finite compatible-world receipt from #6835 and a separate coherent joint loss-model receipt from #6939. It deliberately does not inherit #6887's anytime-valid sampling/stopping guarantee.

That separation is correct. A finite candidate-world list is not evidence that it was constructed with valid optional stopping, time-uniform error control, or predictable adaptive action selection. This bridge makes those claims explicit and binds their exact source identity and cutoff before the list can feed incumbent adequacy.

## 3. Receipt composition contract

### 3.1 Upstream provenance

Carry the upstream issuer's exact:
- receipt schema and claim ceiling;
- source commit, case/run identity, receipt ID and full-receipt SHA-256;
- candidate-world inventory and digest;
- model-family and conditional observation-kernel commitments.

The adapter does not authenticate the issuer. Digests and opaque IDs establish structural identity only. A future integration must pin the actual #6887 receipt schema and approved claim ceiling; v0.1 intentionally does not invent them.

### 3.2 Statistical contract

The consumed source must explicitly declare:
- ANYTIME_VALID_WORLD_EXCLUSION as its guarantee kind;
- a qualified guarantee disposition;
- optional-stopping coverage;
- predictable action selection and coverage of adaptive selection;
- a filtration commitment, selection-policy commitment and conditional observation-kernel commitment;
- process identity, look index and active/stopped state;
- exact rational total campaign alpha and the amount allocated by this process, plus the allocation identity and digest.

The adapter checks that the allocated alpha is positive and does not exceed the declared total, and that total alpha does not exceed one. It does not recompute e-values, infer Ville validity, add alpha budgets, or convert a fixed-look p-value into an anytime-valid claim. The mathematical validity of the source process remains #6887's responsibility.

### 3.3 Exact world inventory

The candidate set must be non-empty. Its IDs, the surviving IDs and excluded IDs must each be sorted, unique and digest-bound. The surviving and excluded sets must be disjoint and form an exact partition of the candidate set.

An empty surviving set is a valid source outcome but **not** evidence of certainty. It maps to ABSTAIN / EMPTY_SURVIVING_WORLD_SET (or the upstream model-incompatibility disposition), never to automatic re-optimization.

### 3.4 Decision cutoff and no-lookahead

The receipt binds:
- a cutoff identity and event-ledger sequence watermark;
- the maximum evidence sequence included in the source history;
- the exact history digest and knowledge-time timestamp.

The maximum included evidence sequence cannot exceed the declared decision cutoff. The adapter must not consume later evidence to reconstruct an earlier decision. Effective-time validity and evidence availability/knowledge-time remain separate in the upstream event contract.

### 3.5 Non-statistical blockers

Even a structurally valid sequential receipt cannot be consumed when:
- its source is stale, revoked, contradicted or indeterminate;
- model-family adequacy is not established;
- observation-channel dependence/common-cause treatment is unresolved;
- action realization is unknown where the profile requires readback;
- the guarantee does not cover optional stopping and adaptive selection.

The adapter does not infer that the candidate-world universe contains reality. Model-family and physical-capture adequacy remain independent upstream qualifications.

## 4. Adapter dispositions

| Condition | Adapter result | Meaning |
|---|---|---|
| All source, guarantee, inventory and cutoff checks pass; at least one world survives | CONSUMABLE | The exact source-bound inventory may be handed to the downstream decision gate |
| No worlds survive | ABSTAIN | Model incompatibility / no admissible world; do not choose the nearest world |
| Source guarantee or physical/model evidence is not qualified | ABSTAIN | Do not make a decision-quality claim from this receipt |
| Malformed identity, digest, alpha arithmetic, world partition or cutoff | INVALID_INPUT | Receipt is internally inconsistent or malformed |
| Any status | authority_granted = false | The adapter cannot authorize, recommend, or execute an action |

CONSUMABLE means only that the adapter contract is satisfied. It does not mean the incumbent is adequate; #7161/#7105 still performs its separate loss, feasibility, closure and switching-cost checks.

## 5. Frozen v0.1 challenge cases

The first executable contract harness should include:
1. a qualified active sequential receipt with a predictable adaptive policy;
2. an empty surviving-world set that abstains;
3. a fixed-look-only test incorrectly labelled as an anytime-valid source;
4. missing optional-stopping coverage;
5. unregistered or uncovered adaptive probe selection;
6. alpha allocation larger than the campaign budget;
7. malformed source commit or source receipt digest;
8. missing filtration, policy or conditional-kernel commitment;
9. duplicate/unsorted candidate or survivor IDs;
10. overlapping survivor and excluded sets;
11. candidate/survivor inventory digest tampering;
12. evidence sequence beyond the decision cutoff;
13. stale or revoked source state;
14. inadequate model family;
15. unresolved common-cause channel dependence;
16. unknown action realization when readback is required.

The contract is intentionally narrow. A later integration should add source-schema-specific replay from #6887, full event-ledger replay, and a joined #7161 loss-scenario inventory with multiple loss models per physical world.

## 6. Matched benchmark programme

Do not claim this adapter is more statistically powerful than #6887. The experiment should compare the upstream methods under the same observation streams and resource budgets:
- fixed-look likelihood-ratio test;
- naive repeated peeking as an intentionally invalid negative control;
- posterior-threshold stopping with prior assumptions stated;
- one-step EIG per cost and fixed-grid acquisition;
- #6887's qualified anytime-valid adaptive process;
- exact finite-horizon policy-tree oracle on enumerable toy domains.

Report campaign-wise false world exclusion / false singleton-decision probability, time-uniform true-world survival, power at a preregistered minimum decision separation, probes and exposure to decision identification, false abstention, and end-to-end regret/switching cost after joining the incumbent gate. Use held-out model families and preserve every attempt; do not cherry-pick the best seed.

## 7. Promotion gates and claim ceiling

V0.1 may establish:
- strict receipt shape and source pinning;
- exact rational alpha-budget comparison;
- sorted, unique, complete world-set partitioning;
- cutoff/no-lookahead discipline;
- deterministic abstention and invalid-input semantics;
- independent JavaScript challenge-oracle agreement on synthetic fixtures.

It cannot establish that the upstream statistical guarantee is true, that a physical observation channel is qualified, that the true world is represented, or that an operational decision is safe. Those require upstream independent review and, later, appropriately qualified physical evidence.

## 8. Non-goals

No new e-process or confidence sequence; no alternative sampling policy; no duplicate loss/regret engine; no hidden-world policy branch; no issuer authentication service; no autonomous authority. The bridge passes a source-bound evidence envelope downstream or refuses to pass it.
