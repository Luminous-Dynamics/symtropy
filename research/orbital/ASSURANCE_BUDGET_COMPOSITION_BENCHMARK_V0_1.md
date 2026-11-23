# Exact Assurance-Budget Composition Benchmark v0.1

**Implementation:** `tools/orbital-observation-surface-bench/src/assurance_budget.rs`  
**Research issue:** [#6947](https://github.com/Luminous-Dynamics/symtropy/issues/6947)  
**Composes:** observation/world coverage (#6887), joint loss-model coverage (#6922/#6939), regret-score calibration (#6942/#6946)  
**Claim ceiling:** exact finite union-bound budget algebra over synthetic claim manifests only.

## 1. Why a composition layer is needed

The stack contains three distinct evidence contracts:

1. **Observation/world coverage.** Does the admitted candidate-world family contain the real physical/observation world at the requested time or over the requested horizon?
2. **Loss-model coverage.** Does the declared coherent loss-scenario family contain the real consequence model for each relevant world and operating regime?
3. **Regret-score calibration.** Does the calibrated regret score upper-bound actual decision regret with the stated finite-sample or sequential guarantee?

None implies the others. A SHA-256 digest binds bytes; it does not establish statistical coverage. An anytime-valid e-process for physical worlds does not make a fixed-split regret calibration anytime-valid.

## 2. Exact fixed-evaluation union bound

Suppose independently justified statistical claims have events (E_1,ldots,E_m), each applying to the same evaluation scope, with:

[
P(E_i^c)leq alpha_i.
]

No independence assumption is needed for the union bound:

[
Pleft(igcap_i E_iight)
geq 1-sum_ialpha_i.
]

The manifest declares a total error budget (alpha_{total}) and per-claim allocations before outcomes. v0.1 computes the exact rational sum (alpha_{used}), the unused amount (alpha_{remaining}=alpha_{total}-alpha_{used}), and the lower confidence bound (1-alpha_{used}). It refuses to report a joint lower bound when a required contract is missing, stale, out of scope, not statistical, or when the allocation sum exceeds the budget.

This is **not** a new probability theorem. The testable engineering contribution is exact provenance, contract and scope matching, no double-counting and replayable composition across the evidence stack.

## 3. Fixed versus anytime scope

The caller requests either:
- `FIXED_EVALUATION`: a predeclared evaluation point / scope ID. A time-uniform guarantee can support a fixed-time request, as can a valid fixed-time claim.
- `ANYTIME`: every required statistical contract must itself be time-uniform over the same declared scope ID. A fixed-split calibration cannot satisfy this request merely because an upstream world-survival e-process is anytime-valid.

v0.1 does not implement finite-horizon alpha spending schedules or adaptively selected subgroup families. Those remain separate future work. A repeated-look procedure needs its own sequential validity or predeclared time/multiplicity allocation for every affected statistical layer.

## 4. Claim types and evidence identity

Required statistical classes:
- `OBSERVATION_WORLD_COVERAGE`
- `LOSS_MODEL_COVERAGE`
- `REGRET_SCORE_CALIBRATION`

Optional non-statistical classes may include:
- `TRANSPORTABILITY`
- `MULTIPLICITY_FAMILY`

Each statistical claim binds:
- unique claim ID and class;
- guarantee kind `STATISTICAL_GUARANTEE`;
- guarantee scope (`FIXED_EVALUATION` or `ANYTIME`) and scope ID;
- evidence status and provenance receipt;
- statistical claim receipt identity;
- multiplicity family and allocation receipt;
- exact alpha allocation;
- source contract digest.

A structural-only claim can carry a provenance receipt and source digest, but no alpha or statistical claim receipt. It is reported as non-composable and does not satisfy a required statistical class. A structural digest cannot spend or earn alpha.

Duplicate statistical claim receipt IDs are rejected, so the same underlying guarantee cannot be counted twice under two labels. The planned claim-ID inventory and full claim digest bind the input record set; they do not authenticate the issuer or prove the underlying theorem.

## 5. Status semantics

- `ASSURANCE_COMPOSED_FIXED_EVALUATION`: every required statistical class is evidence-admissible and scope-compatible, and (sum_ialpha_ileqalpha_{total}). Reports the union-bound lower confidence.
- `ASSURANCE_COMPOSED_ANYTIME`: same, with every required statistical claim declared anytime-valid for the requested scope.
- `ASSURANCE_BUDGET_EXCEEDED`: all required claims are otherwise composable, but the exact alpha sum exceeds the total budget. No joint lower confidence is reported.
- `ASSURANCE_SCOPE_MISMATCH`: at least one statistical claim is scoped to another evaluation/horizon, or a fixed-time claim is being used for an anytime request.
- `ASSURANCE_INDETERMINATE`: required statistical class is missing, or evidence status is indeterminate/out-of-scope, or only structural evidence is present.

A composed status means only that the *declared* statistical guarantees can be combined algebraically under their declared scopes. It does not certify that their assumptions hold in the real system.

## 6. Frozen challenge corpora

1. **Fixed success:** world alpha (0.01), loss alpha (0.02), regret alpha (0.02), total (0.05). Expected (alpha_{used}=0.05), remaining (0), lower confidence (0.95).
2. **Budget overrun:** (0.02+0.02+0.02>0.05). No joint confidence is emitted.
3. **Missing loss coverage:** no statistical loss-model claim. `ASSURANCE_INDETERMINATE`.
4. **Fixed-time claims for an anytime request:** `ASSURANCE_SCOPE_MISMATCH`.
5. **Anytime success:** all three claims are anytime-valid for the same predeclared scope; only then may `ASSURANCE_COMPOSED_ANYTIME` be emitted.

Rust unit tests additionally cover structural-only claims, receipt double counting, exact arithmetic and no authority.

## 7. Limitations and non-claims

- Every component claim must already be statistically valid for the stated population, dependence structure, selection process and stopping rule. The budget evaluator does not verify the theorem or calibration data.
- The union bound may be conservative; v0.1 does not assume independence or use covariance/conditional dependence to reduce the sum.
- Multiplicity family IDs and allocation receipts are required structural references; a family-wide theorem is still the issuer's responsibility.
- Fixed-split conformal does not become time-uniform after composition with an e-process. An anytime request requires every required layer to provide an anytime guarantee.
- A confidence budget is not physical qualification, safety certification, dispatch authority or proof that a source-world/ loss-model set contains reality.

## 8. Prospective gates (targets, not achievements)

- 100% exact Rust/JavaScript agreement on claim sets, rational alpha sum and composed status.
- Zero budget overruns accepted as composed.
- Zero duplicate claim receipts counted twice.
- Zero structural-only receipts accepted as statistical guarantees.
- Zero fixed-time guarantees used to support an anytime request.
- 100% detection of missing world, loss or regret guarantees.
- No alpha reallocation after outcomes; unused alpha remains unused.
- All multiplicity families, scopes and claim receipts replayable and source-bound.

The desired end state is a system that can say not only “these layers passed,” but “these specific statistical contracts cover this exact scope, their combined failure budget is at most this declared amount, and no guarantee has been borrowed from an unrelated layer.”
