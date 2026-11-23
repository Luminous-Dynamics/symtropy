# Incumbent Decision Adequacy: Joint Loss-Model Receipt Bridge v0.1

**Status:** synthetic integration adapter; exact-head CI pending.  
**Owner:** [#7105 — incumbent decision adequacy](https://github.com/Luminous-Dynamics/symtropy/issues/7105).  
**Implementation:** draft PR to be stacked on #7117.  
**Upstream contracts:** [#6835 — finite decision-identifiability receipt](https://github.com/Luminous-Dynamics/symtropy/pull/6835), [#6939 — coherent joint loss-model scenarios](https://github.com/Luminous-Dynamics/symtropy/pull/6939), and [#6922 — joint loss-scenario regret](https://github.com/Luminous-Dynamics/symtropy/issues/6922).  
**Claim ceiling:** synthetic receipt composition and exact rational-to-integer projection only. No issuer authentication, loss-model calibration, physical qualification or authority.

## 1. Why this bridge exists

The incumbent-retention evaluator must not treat “the compatible physical-world set is current” as equivalent to “the decision loss-model set is current and complete.”

Those are distinct inventories:

- A **physical/observation world** describes a possible state of the system.
- A **loss model** describes the consequence mapping applied to actions in that world.
- A **joint decision scenario** is one pair ((w,m)), with a unique scenario ID, a physical-world ID, a loss-model ID, a coherent vector of action losses, evidence status and source receipt.

One physical world may legitimately have multiple admitted loss models. All such joint scenarios must be represented in the incumbent comparison. Collapsing them to one “representative” model or counting physical-world coverage as loss-model coverage would understate uncertainty.

This bridge therefore separates and pins:
1. the exact finite compatible-world receipt;
2. the exact coherent joint loss-scenario receipt;
3. the projection of those rational losses into the integer loss unit consumed by the incumbent comparator.

## 2. Upstream receipt contracts

### Physical-world receipt

V0.1 consumes the exact finite-world decision-identifiability receipt:
- receipt schema: `luminous.orbital-decision-identifiability.receipt.v0.1`;
- claim ceiling: `exact-finite-world-decision-set-algebra-only`;
- exact source commit and case ID;
- opaque receipt ID and full-receipt SHA-256;
- exact sorted compatible-world ID inventory and its SHA-256.

This source currently represents the bounded finite-world contract from #6835. It is **not** an adapter for #6887's anytime-valid sequential evidence receipt. A future #6887 integration must bind its own receipt schema, time-uniform error contract, stopping rule, source commit and exact surviving-world set. No such sequential guarantee is inferred from this finite-world receipt.

### Joint loss-model receipt

V0.1 consumes the #6939 joint loss-model receipt:
- receipt schema: `luminous.orbital-joint-loss-scenario.receipt.v0.1`;
- claim ceiling: `finite-joint-loss-scenario-regret-algebra-only`;
- exact source commit and upstream case ID;
- full receipt SHA-256, loss manifest ID and source joint-scenario digest;
- exact sorted planned joint-scenario ID inventory and its digest.

The bridge requires the planned scenario ID set to equal the observed scenario ID set. It separately requires the set of physical-world IDs represented by the joint scenarios to equal the compatible-world inventory. Repeating a physical-world ID is permitted; repeating a joint scenario ID is not.

Receipt IDs and digests are structural pins only. They do not authenticate the issuer or prove that the scenario family is calibrated, exhaustive or physically true.

## 3. Exact rational-to-integer loss projection

The upstream joint-loss evaluator stores exact rational losses. The incumbent v0.1 evaluator operates on exact non-negative integer losses in one declared target unit. A silent float conversion or rounding step would break the evidentiary chain.

For each source loss (n/d), the bridge carries:
- source numerator (n);
- source denominator (d>0);
- explicit projection identifier;
- explicit scale (p/q), with (p>0, q>0);
- projected integer loss (z);
- source and target unit identities.

The conversion is accepted only when:

[
z = rac{n,p}{d,q} in mathbb{Z}_{geq 0}
]

The evaluator computes this using integer arithmetic and rejects:
- zero denominators;
- scale overflow or unsafe JSON integer values;
- a non-integral quotient;
- a quotient beyond the signed 64-bit loss range;
- a projected integer that differs from the exact quotient.

There is no rounding mode and no floating-point arithmetic. The projected-scenario digest binds the scenario identity, physical-world ID, loss-model ID, evidence status/receipt, scope, feasible set, source rational numerator/denominator, and projected integer value. This lets the incumbent receipt be replayed against the exact loss projection it consumed.

The projection scale is part of the receipt. “Same numbers” in different units cannot be compared without an explicit conversion contract.

## 4. Evaluation semantics

The incumbent comparator remains the common-action worst-case improvement gate:
- one action must be feasible in every joint physical-world/loss-model scenario;
- compare the incumbent's worst-case loss with the best worst-case loss of one common feasible action;
- threshold = materiality margin + switching cost;
- equality retains; strictly greater improvement triggers review;
- an infeasible incumbent triggers review;
- an empty surviving-world set abstains.

Before any numeric result, all of these must be current:
- finite physical-world receipt and exact world inventory;
- planned joint loss-scenario inventory and full loss-manifest digest;
- projected scenario digest and exact rational conversion;
- each joint scenario's admissible evidence status and non-empty evidence receipt;
- domain closure and dependency-graph adequacy;
- model-family adequacy and current evidence state.

Stale/indeterminate/out-of-profile/inadequate loss-model sets abstain. Per-scenario inadmissible, indeterminate or out-of-profile evidence blocks numeric decision metrics. Missing planned joint scenarios or missing physical-world coverage is invalid input, not an invitation to calculate over the smaller observed set.

## 5. Frozen v0.1 bridge controls

The 29-case corpus now includes the prior 16 incumbent cases plus:
- stale, inadequate and out-of-profile loss-model manifests;
- an out-of-profile per-scenario loss receipt;
- an indeterminate per-scenario loss receipt;
- a missing joint scenario despite a matching physical-world ID;
- missing physical-world coverage despite a self-consistent reduced scenario list;
- malformed upstream joint-loss receipt hash;
- missing per-scenario receipt ID;
- multiple loss models for one physical world;
- exact fractional rational projection (e.g. (1/10 	imes 1000 = 100));
- a non-integral conversion (e.g. (1/3 	imes 1)), which must be rejected;
- a projected integer that does not equal its source rational conversion, which must be rejected.

The Rust suite includes exact conversion unit tests, negative projection mutations, input permutation invariance, and the earlier anti-clairvoyance common-action control. An independent JavaScript oracle uses BigInt for exact rational arithmetic and recomputes the full projection digest. Corpus validator, receipt schema and workflow checks require the positive multi-model case and all three classes of loss-model blockers.

## 6. Research comparisons and field positioning

The bridge does not recreate #6939's optimizer or #6958's robust feasible-set regret. Those owners retain their respective responsibilities. This adapter makes the loss-model evidence a mandatory dependency of the incumbent-retention gate.

The external comparison should preserve distinct research questions:
- PEAR / decision-focused learning: how well does a predictor train for downstream regret on matched LP/QP workloads?
- Robust DFL / Wasserstein-DRO: how do performance and conservatism change under objective and distributional uncertainty?
- RADAR-style operational-suboptimality monitoring: when has an incumbent become materially suboptimal under a current distribution?
- #6939 joint vs rectangular regret: what is the cost of breaking coherent action-loss dependence?
- This bridge: can every joint loss scenario and exact projected value be traced into the incumbent assessment without dropping uncertainty or rounding away distinctions?

Do not call this an SOTA win. Compare matched physical worlds, admitted loss models, action sets, constraints, units, solver budgets and held-out shifts. Report worst-case common-action improvement, joint and rectangular regret, false retention, false re-optimization, stale-scenario escape, scenario omission detection, reviewer time and compute separately.

## 7. Prospective benchmark gates

Targets, not measured field achievements:
- zero unplanned joint scenarios accepted;
- zero omitted planned joint scenarios accepted;
- zero physical-world coverage omissions accepted;
- zero stale/out-of-profile/indeterminate loss models producing numeric decision metrics;
- zero source rationals rounded during conversion;
- 100% exact conversion agreement on finite fixtures;
- 100% Rust/independent BigInt oracle agreement;
- zero retention with missing or stale loss-model evidence;
- zero world-set coverage claims treated as loss-model-set coverage;
- zero authority grants.

Measure p50/p95/p99 runtime and memory at 10, 100, 1,000, 10,000 and 100,000 joint scenarios. Also measure scenario-to-world fan-out, number of model IDs per physical world, projection cost, and sensitivity to added loss models.

## 8. Known limitations and next adapter

The source pins do not verify raw upstream artifact bytes, signatures, issuer credentials or revocation. A real deployment needs the trust-root and receipt-authentication owner (#6519/#6598) to verify those artifacts independently.

The #6835 finite-world receipt is the current v0.1 world-set source. To compose #6887's anytime-valid surviving-world receipt, create a separate schema-typed adapter carrying its error probability, filtration, sampling protocol, stopping rule, exact source receipt and evaluation cutoff. Do not translate a sequential guarantee into a generic `CURRENT` boolean.

A passing synthetic run proves only that the declared finite receipt inventories and exact rational projection satisfy this adapter's contract. It does not prove that the physical-world or loss-model sets cover reality, calibrate the consequence law, or authorize retaining an operational decision.
