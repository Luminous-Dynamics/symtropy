# SIM-REL-042 — preregistered paired inference for scientific benchmark claims

Status: executable first increment on a research branch. It is not yet exact-head qualified. It adds statistical analysis to the matched comparison gate; it does not claim that any Symtropy component beats a published baseline.

## 1. Why a separate statistical layer is required

SIM-REL-041 establishes that reference and candidate arms use the same task manifest, model/prompt configuration, harness, toolset, panel, scope, evidence group and resource budgets. It reports per-family paired loss deltas.

That is comparison integrity, not inferential evidence. Observing a lower loss on a handful of tasks does not establish a general improvement, and selecting only favorable mechanism families after seeing their results invalidates a superiority claim.

SIM-REL-042 therefore requires an immutable analysis plan before the sealed outcomes are inspected.

## 2. Preregistered analysis plan

The plan binds:
- analysis ID and version;
- campaign, evaluation scope and SealedHoldout group;
- fixed agent panel and task/model/prompt/harness/toolset digests;
- reference and candidate arm IDs;
- pre-registration receipt digest;
- family-wise alpha;
- maximum terminal-loss range [0,L];
- minimum valid pairs per family;
- the complete mechanism-family set.

The analysis plan must match the corresponding SIM-REL-041 campaign manifest exactly. The family set is fixed in advance; omitted or post-hoc added families are rejected. A family with no valid pairs remains visible as underpowered rather than disappearing from the report.

The current implementation accepts SealedHoldout only. Development and Calibration are not final inferential evidence.

The pre-registration receipt digest is currently a binding supplied by the caller. This module checks that it is present and carries it into the output; cryptographic authentication and trusted timestamping remain provenance-layer responsibilities.

## 3. Paired endpoint and direction

For each valid pair i:

`d_i = L_{candidate,i} - L_{reference,i}`

- `d_i < 0`: candidate wins (lower loss);
- `d_i > 0`: reference wins;
- `d_i = 0`: tie.

The exact mean paired effect is stored without floating-point rounding as:

`(sum_i d_i) / n`

Negative values favor the candidate. The loss range is fixed by the plan: each arm's terminal loss must lie in [0,L], so every paired difference lies in [-L,L].

## 4. Exact paired sign test

The two-sided exact sign test conditions on discordant pairs (wins + losses) and tests whether either system is more likely to win among non-ties.

For w candidate wins, r reference wins and n=w+r discordant pairs:

`p = min(1, 2 * sum_{k=0}^{min(w,r)} C(n,k) / 2^n)`

- Ties are excluded from this test's denominator but retained in the effect and tie-rate statistics.
- If there are no discordant pairs, p=1.
- This is a non-randomized exact binomial calculation.
- The first implementation supports up to 120 discordant pairs with checked u128 arithmetic. Larger exact samples are rejected rather than silently approximated.

The sign test establishes directional preference probability; it does **not** test the mean loss difference. Both the exact sign result and the mean effect are reported.

## 5. Distribution-free mean-loss interval

The implementation reports a pointwise 95% Hoeffding interval for the population mean paired loss difference, under the stated assumption that paired task instances are independent draws from the declared benchmark-family sampling frame.

Because both arm losses are bounded in [0,L], paired differences lie in [-L,L]. Hoeffding's inequality gives a two-sided radius:

`(2L) * sqrt(log(40)/(2n))`

Since `log(40) < 4`, the implementation uses the conservative radius:

`2L * sqrt(2/n)`

All calculations are integer/rational. The interval is rounded outward to millionths of one loss unit.

These intervals are **pointwise**, not simultaneous across mechanism families. Holm correction applies to the family-level sign-test hypotheses. Do not interpret the pointwise intervals as family-wise confidence intervals.

## 6. Holm family-wise error control

The exact sign-test p-values are sorted from smallest to largest across the complete preregistered family set. Holm's step-down thresholds are:

`alpha/(m), alpha/(m-1), ..., alpha`

where m is the number of preregistered families. Rejection stops at the first failed threshold. Underpowered families remain in the multiplicity set and cannot be removed after seeing the data.

A family is marked as rejected only when it passes both the Holm threshold and the preregistered minimum pair count. An underpowered family stops further step-down rejection; this is conservative.

The implementation does not support optional stopping, interim peeking, adaptive endpoint changes or post-hoc family selection. Those require a separately preregistered sequential alpha-spending design.

## 7. Claim boundaries

A statistically significant exact sign test is not:
- proof of a mean loss improvement;
- proof of practical importance;
- proof of model adequacy outside the declared mechanism families;
- proof of physical safety;
- proof of external-world generalization beyond the task sampling frame;
- proof that the benchmark or evidence artifact is authentic.

Report the exact mean delta, pointwise interval, directional wins/losses/ties, exact p-value, Holm-adjusted decision, valid sample count, invalid-task exclusions and all underpowered families together.

A superiority claim requires the preregistered endpoint, alpha, family set, loss range, sample size and sealed holdout to be fixed before evaluation. Report adverse strata and every family where the candidate loses.

## 8. Qualification tests

The focused suite includes:
- known exact p-value fixtures;
- exhaustive small-domain sign-test enumeration against all (2^n) sign assignments;
- symmetry and probability bounds;
- all-win, all-loss, ties and zero-discordance cases;
- maximum supported exact sample and unsupported overflow case;
- sealed-only enforcement;
- exact manifest/pre-registration binding;
- extra family and loss-range rejection;
- underpowered-family retention;
- Holm step-down correction;
- outward mean-interval containment and deterministic replay.

Qualification requires pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.
