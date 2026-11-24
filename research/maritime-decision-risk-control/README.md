# Maritime decision-risk control v0.1

A standalone, dependency-free Rust research harness for **calibrating bounded downstream decision loss at the whole-schedule unit**.

This is a follow-on to:

- [#7221 — DCT empirical reproduction / data rights](https://github.com/Luminous-Dynamics/symtropy/issues/7221)
- [#7224 — schedule-level simultaneous coverage](https://github.com/Luminous-Dynamics/symtropy/issues/7224)
- [#7231 — exact schedule feasibility and false-safe oracle](https://github.com/Luminous-Dynamics/symtropy/issues/7231)
- [#7252 — decision-risk calibration and authorized recourse](https://github.com/Luminous-Dynamics/symtropy/issues/7252)
- [#7229 — schedule-level conformal coverage](https://github.com/Luminous-Dynamics/symtropy/pull/7229)

## Research shift

Prediction coverage answers whether an outcome lies in a prediction set. It does not, by itself, answer whether the frozen scheduling policy will produce a feasible schedule, avoid a critical connection miss, or preserve participant-authorized recourse.

This harness tests **Conformal Risk Control (CRC)** on one bounded loss per complete synthetic schedule. It does not claim that CRC is a new method. The research opportunity is the disciplined maritime composition:

1. Freeze a policy, evaluator, and loss contract before calibration.
2. Treat the entire schedule as one exchangeable sampling unit.
3. Calibrate the expected bounded loss of the downstream policy.
4. Keep hard physical failure, authority failure, service loss, and cost as separate losses.
5. Refuse a statistical claim when the mathematical preconditions are not met.

## The theorem boundary matters

The CRC result of Angelopoulos et al. (ICLR 2024) controls **expected loss** on a future exchangeable unit:

`E[L_{n+1}(\hat\lambda)] \leq \alpha`.

It is not a high-probability upper confidence bound conditional on the realized calibration set. A Wilson interval or another sampling interval for observed test outcomes is descriptive/inferential reporting, not the CRC theorem itself.

For a binary loss `L=1{any hard schedule failure}`, expected loss equals marginal failure probability. The guarantee is still over the randomness of calibration schedules and the future schedule, under exchangeability and the theorem's other assumptions. It is not conditional coverage by vessel class, temporal regime, or participant.

Primary reference: [Conformal Risk Control, ICLR 2024](https://proceedings.iclr.cc/paper_files/paper/2024/hash/f3549ef9b5ff520a7e41ff3cc306ab2b-Abstract-Conference.html).

## Algorithm

For a bounded loss `L_i(λ) ≤ B` that is non-increasing in a conservative policy parameter `λ`, CRC chooses the smallest grid value satisfying:

`(n/(n+1)) · empirical_mean_loss(λ) + B/(n+1) ≤ α`.

If no grid point satisfies that empirical inequality, the algorithm returns `λ_max`. The theorem requires `L_i(λ_max) ≤ α` by design for all possible future units. The caller-supplied maximum-parameter contract in this harness is an explicit assumption, **not a proof**; its scope and evidence reference must be independently qualified. The implementation also checks that the observed calibration outcomes do not contradict the declared bound.

### Non-monotone scheduling effects

A larger buffer or more aggressive reserve policy does not necessarily reduce schedule failure: it can create berth conflicts, crane interference, yard overflow, or participant displacement.

CRC's paper gives a monotone-envelope construction for non-monotone losses:

`L̃_i(λ) = sup_{λ' ≥ λ} L_i(λ')`.

The envelope is non-increasing and upper-bounds the original loss at the selected parameter. This is safe but can be conservative. The harness compares:

- `REQUIRE_MONOTONE`: reject a loss family that violates monotonicity;
- `MONOTONE_ENVELOPE`: replace each schedule's loss curve with its upper monotone envelope before calibration.

The non-monotone method is now represented by a separate executable comparator: finite-grid CRC-NM, based on Aldirawi, Li & Guo (arXiv:2604.01502v3, 11 November 2026). This is an arXiv preprint, not treated here as a peer-reviewed result. Its theorem provides an expected-risk bound with an explicit finite-grid correction; its assumptions and endpoint population-risk requirement are kept separate from the standard monotone CRC claim.

## Synthetic fixture

Two deterministic whole-schedule loss streams are included:

- **MONOTONE_HARD_FAILURE**: an explicitly nested policy family with binary schedule-failure loss.
- **NON_MONOTONE_WITH_ENVELOPE**: a constructed loss family where adding buffer/recourse options can worsen some schedules. The monotone envelope is used before standard CRC.
- **FINITE_GRID_CRC_NM**: a separate finite-grid non-monotone risk-control experiment using pseudo-random synthetic whole-schedule loss vectors, with calibration sizes of 1,200 and 50,000 schedules.

The generated test outcomes are descriptive only. All data are synthetic, deterministic, and deliberately small. They are not estimates of DCT, DGT, or any operating terminal's risk.


## Finite-grid non-monotone CRC comparator (November 2026)

The second executable, `non-monotone-risk-audit`, implements Theorem 1 from Aldirawi, Li & Guo, *Conformal Risk Control under Non-Monotone Losses: Theory and Finite-Sample Guarantees*, arXiv:2604.01502v3 (11 November 2026):

https://arxiv.org/abs/2604.01502

This is an **arXiv preprint**, not a peer-reviewed SOTA result. It is nevertheless directly relevant to maritime recourse, where increasing a buffer or changing a recovery policy can reduce one failure mode while increasing another.

For a finite candidate grid of size `m`, calibration size `n`, and bounded loss in `[0,B]`, the paper defines:

`D(m,n) = B * sqrt(log(2m)/(2n)) + B/(2*sqrt(2n*log(2m)))`.

For non-monotone loss vectors that are i.i.d. across complete schedules and a terminal candidate whose population expected loss is at most `alpha`, the unadjusted CRC selector has:

`E[L_{n+1}(lambda_hat)] <= alpha + D(m,n)`.

The adjusted selector uses `alpha' = alpha - D(m,n)`. If `alpha' > 0` **and** the terminal candidate's population expected loss is at most `alpha'`, then:

`E[L_{n+1}(lambda_hat_adj)] <= alpha`.

Important distinctions:

- The guarantee is on **marginal expected loss**, not a high-probability bound conditional on the realized calibration set.
- The schedule-loss vectors must be i.i.d. under the theorem; temporal drift and dependent schedules are stress tests, not covered guarantees.
- The endpoint population-risk condition is an external assumption requiring structural evidence. A calibration-sample average does not prove it.
- The correction grows with the candidate-grid resolution and shrinks with calibration sample size. If `D >= alpha`, the implementation emits `NO_POSITIVE_ADJUSTED_TARGET`, not a fabricated certificate.
- Authority/consent remains a deterministic gate; no statistical guarantee can authorize an action.

The deterministic receipt compares three finite-grid cases: 1,200 schedules at 20% target risk, 120 at 10%, and 50,000 at 10%. The 120/10% arm deliberately demonstrates that D(m,n) can consume the entire risk budget and must produce NO_POSITIVE_ADJUSTED_TARGET; the 1,200/20% arm shows a positive adjusted budget; the 50,000/10% arm tests whether a useful adjusted target returns at a much larger calibration size. These counts are synthetic illustrations of the correction's effect, not DCT sample-size recommendations or operational risk estimates.

Standard monotone CRC, CRC with a monotone upper envelope, finite-grid non-monotone CRC-NM, known-weight shift CRC and the Lipschitz-structured correction are distinct comparators with different theorem assumptions. Compare them at matched loss definitions and report each guarantee type; do not treat them as interchangeable. In particular, known-weight shift CRC requires an exact bounded likelihood ratio, while Lipschitz CRC requires an almost-sure uniform K across schedule losses. Neither assumption may be inferred from observed smoothness or estimated density ratios.


## Known-weight distribution-shift comparator (November 2026)

The third executable, `weighted-shift-risk-audit`, implements the **known, bounded importance-weight** case in Proposition 4 of the same arXiv preprint (Section 7):

https://arxiv.org/abs/2604.01502

It intentionally uses an exactly known synthetic source-to-target likelihood ratio. Source calibration episodes have P_train(severe)=0.10; independent target episodes have P_test(severe)=0.50. The exact ratios are:

- severe episode: w = 0.50 / 0.10 = 5;
- normal episode: w = 0.50 / 0.90 = 5/9.

Therefore P_test is absolutely continuous with respect to P_train and 0 <= w <= W=5 in this constructed fixture. This is a controlled mathematical test, not an estimate of the DCT-to-DGT regime ratio.

For bounded per-schedule loss 0 <= L <= B, the weighted empirical risk is:

`Rhat_w(lambda) = (1/n) sum_i w_i L_i(lambda)`

The paper's selector is:

`lambda_hat = inf {lambda : n/(n+1) * Rhat_w(lambda) + W*B/(n+1) <= alpha}`

with the terminal-policy convention when the set is empty. Under Proposition 4's assumptions, including the independent target episode and its explicit terminal-risk margin, the expected target risk is bounded by:

`alpha + D_W(m,n) + (B-alpha) * exp(-2*n*a^2/(W^2*B^2))`

where:

`D_W(m,n) = W*B*sqrt(log(2m)/(2n)) + W*B/(2*sqrt(2n*log(2m)))`.

This is an **expected-risk** guarantee over calibration and the independent target observation. It is not a high-probability bound conditional on the realized calibration set. A large W increases the correction and may make the bound uninformative.

### Synthetic counterexample: source-calibrated policy can fail after shift

The four-policy fixture has non-monotone schedule-level hard-failure losses:

- deterministic policy: fails in severe episodes;
- buffered policy: avoids the severe failure but introduces a smaller failure in normal episodes;
- third policy: has a different severe-regime trade-off;
- terminal fallback: zero hard-failure loss by construction.

The unweighted source calibration can select the deterministic policy because severe episodes are rare in P_train. On the independent target distribution, that selected policy has a much higher hard-failure rate. The known-weight selector instead selects the buffered policy. The receipt reports both policies' target hard-failure loss and their separate service-loss metric.

The terminal fallback's zero hard-failure loss does **not** mean zero service loss: its service-loss metric is maximal because it performs no service. This is deliberately separate from the CRC risk bound. A risk certificate for one failure outcome is not a certificate for delivered cargo, service quality, participant fairness or cost.

The three calibration sizes are 120, 1,200 and 50,000 complete synthetic schedules. Small-sample or overly large-weight cases can return an uninformative upper bound; the receipt records this rather than silently clipping it.

### What this does not establish

The theorem assumes a **known** likelihood ratio w=dP_test/dP_train and a finite upper bound W. The paper's remark on estimated weights provides a fixed-policy L1 approximation relation, but explicitly leaves a finite-sample guarantee for the data-dependent selector with estimated weights to future work. Consequently:

- a domain-classifier probability is not a known density ratio;
- a capped/truncated estimated ratio does not satisfy the theorem without a bias/error bound;
- ESS and observed weight histograms do not prove W or support overlap;
- P_test not being absolutely continuous with respect to P_train means the historical data cannot certify unseen target regimes;
- covariate weighting does not fix conditional/concept shift, measurement/ontology shift, or MNAR source loss.

The DCT-to-DGT transfer therefore remains a research question owned by [#7274](https://github.com/Luminous-Dynamics/symtropy/issues/7274), not an achieved result.

Reproduction command:

```sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin weighted-shift-risk-audit
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin lipschitz-risk-audit
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin lipschitz-risk-audit
```


## Lipschitz-structured non-monotone CRC comparator

The fourth executable, `lipschitz-risk-audit`, implements the correction in Proposition 2 of Aldirawi, Li & Guo (arXiv:2604.01502v3, 11 Nov 2026):

`D_e(n,K,b-a) = inf_{N>=2} { D(N,n) + K(b-a)/(N-1) }`

where `D(N,n)` is the finite-grid non-monotone correction evaluated on an auxiliary grid of size `N`. The resulting expected-risk bound is:

`E[L_{n+1}(lambda_hat)] <= alpha + min{D(m,n), D_e(n,K,b-a)}`.

Unlike `D(m,n)`, the Lipschitz correction does not depend on the original candidate-grid size `m`. It is useful when a fine continuous policy grid would otherwise enlarge the selection correction.

### Synthetic domain and strict scope

This comparator deliberately uses a **continuous smooth service-cost surrogate**, not a hard-feasibility indicator. Its synthetic loss is:

`L_i(lambda) = 0.15 + 0.02 sin(2 pi (lambda + phase_i))`

over `lambda in [0,1]`. Its derivative is bounded by `2 pi * 0.02 < 0.126`, so the module declares `K=0.126` as a uniform bound for every synthetic schedule. The terminal population risk is at most 0.17, below the 0.20 target, by construction.

The auxiliary-grid minimization enumerates integer `N>=2` and stops when the increasing concentration term `D(N,n)` is already no smaller than the best objective found. The receipt reports the finite-grid correction, Lipschitz correction, selected minimum and auxiliary grid size separately.

**Do not transfer this assumption to physical feasibility.** A binary berth overlap, crane conflict, yard overflow or missed-deadline indicator can jump discontinuously when a continuous buffer crosses a threshold. A smooth empirical average does not prove an almost-sure uniform K-Lipschitz condition. The physical hard-failure stream remains on the #7231 exact-oracle and finite-grid CRC-NM track unless a valid Lipschitz envelope is independently established.

The experiment uses 6 configurations (1,200 / 10,000 / 50,000 schedules × 64 / 10,000 policy-grid points). These are synthetic mathematical comparisons, not sample-size prescriptions for DCT/DGT operations.

Reproduction command:

```sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin lipschitz-risk-audit
```


## Independent oracle outcome adapter

The fifth research component is an adapter, not another feasibility engine: `oracle_adapter.rs` consumes versioned per-episode/per-policy adjudications from an independent oracle such as the #7231 owner. It compiles separate whole-schedule matrices for hard physical failure and service loss.

### Input contract

Each dataset declares:
- schema version `maritime.oracle-outcomes.v1`;
- oracle identity/revision and an evidence reference;
- an independence-evidence reference describing separation from policy/production feasibility helpers;
- one declared exchangeability scope and one operating-regime ID;
- frozen evaluator and loss-contract revisions;
- an explicit ordered candidate-policy grid with contiguous parameter indices, immutable policy IDs and revisions, plus an externally supplied SHA-256-shaped digest. Parameter order is preserved exactly; it is not sorted by policy ID.

Each complete scheduling episode declares its unique episode ID, unique dependence-cluster ID, scenario family, regime ID, and Calibration or Evaluation role. Each episode carries exactly one oracle outcome for every candidate policy. Each policy outcome includes policy/evaluator/loss revisions, an action class (`NoMutation` or `Mutating`), an authority disposition, physical feasibility disposition, bounded service-loss value, and receipt digest. `NotRequired` is only accepted for `NoMutation`; a `Mutating` action requires `Authorized`.

### Fail-closed rules

- A denied or unknown authority disposition rejects the dataset. `NotRequired` is reserved for a policy that made no mutating action; the adapter does not infer consent.
- An unknown physical outcome rejects the dataset. It is never converted to a zero loss or silently dropped.
- Physical `Infeasible` becomes hard-failure loss 1,000; `Feasible` becomes 0. Service loss remains its own value in [0, 1,000]. They are not added or averaged.
- A reused dependence-cluster ID across any episodes or split roles is rejected. Clustered operations must first be aggregated into one complete schedule-level sampling unit or handled under a separate dependence-aware method.
- A missing/extra policy candidate, policy revision drift, evaluator revision drift, loss-contract drift, mixed operating regimes, duplicate episode IDs, malformed receipt digest, or out-of-range loss rejects the whole dataset.
- Calibration and Evaluation matrices remain separate. Evaluation outcomes cannot enter calibration selection through this adapter.
- Episode rows are sorted deterministically, while policy columns retain the exact declared CRC parameter order. Input episode order and policy-outcome order do not change the compiled result.

The adapter does **not** prove that the oracle is independent, that the evidence reference is truthful, that an external digest matches the policy-grid contents (only its SHA-256 shape is checked), or that the declared exchangeability scope holds. Those remain owner/evidence assertions. The adapter also reports empirical upper-tail CVaR by policy and loss axis. These descriptive diagnostics are not finite-sample CVaR guarantees or risk-control certificates. The broader comparison against high-probability LTT/RCPS, stability-based and anytime-valid methods is owned by [#7303](https://github.com/Luminous-Dynamics/symtropy/issues/7303). It does not implement physical feasibility, estimate importance weights, or convert a CRC expected-risk result into an authorization decision.

The root `crate::LossMatrix` and finite-grid CRC-NM grid can consume the emitted matrices independently for each loss axis. Known-weight shift selection is deliberately not wired to this adapter: a real bounded likelihood ratio must be independently justified, not inferred from these receipts.

The adapter has 18 unit tests for separate axes, deterministic permutation invariance, authority denial/unknown, physical unknown, bounded-loss violations, dependence-cluster leakage, policy-grid/revision drift, mixed regimes, digest validation, role separation, and empirical upper-tail CVaR.

## Important release preconditions

The future #7231 integration must establish all of the following before interpreting a risk output as operationally meaningful:

- The loss is defined over **complete schedules**, not individual vessel rows.
- Calibration schedules and the future schedule satisfy the exchangeability assumption, or a different method with a valid guarantee is used.
- The frozen policy, evaluator, resource map, constraint catalogue, and loss contract have not changed after calibration.
- For standard CRC, the policy parameter must be genuinely monotone for the declared loss, or a justified monotone upper envelope must be used. The separate finite-grid non-monotone method does not require monotonicity, but instead pays its explicit sample-size/grid-resolution correction and requires the stated population-risk condition for the terminal policy.
- A maximum-parameter policy with loss at most `α` exists **by construction**. A fail-closed abstention is not automatically a low-service-loss policy; if abstention counts as failure, its loss must be included.
- Hard authority/consent checks remain deterministic gates. A statistical risk budget cannot authorize an action.
- The exact small-instance oracle independently adjudicates physical feasibility and unauthorized actions. Its outcome receipts are consumed through `oracle_adapter.rs`; the adapter does not reimplement those constraints.
- Missing, censored, and outcome-dependent observations are separately identified; hidden outcomes never enter the deployed-style calibrator.

The maximum-parameter assumption is especially strong. If the exact evaluator cannot establish a safe fallback within the declared scope, the correct result is **NO_MAX_PARAMETER_CONTRACT**, not an extrapolated certificate.

## Output

The executable writes a deterministic CSV receipt with:

- experiment and calibration unit;
- frozen policy/evaluator/loss-contract revision labels;
- target risk and bounded-loss maximum;
- calibration schedule count and selected policy-parameter index;
- original monotonicity violations and whether an envelope was applied;
- observed calibration loss at the selected parameter;
- descriptive synthetic test loss;
- theorem/claim type and explicit claim ceiling.

Revision labels are identifiers supplied by the caller, not cryptographic attestation. Production receipts must additionally bind content digests and an immutable evidence manifest.

## Reproduction

From the repository root:

```sh
cargo fmt --manifest-path research/maritime-decision-risk-control/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --all-targets
cargo test --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --all-targets
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin decision-risk-audit
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin non-monotone-risk-audit
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin weighted-shift-risk-audit
```

## Claim ceiling

Synthetic expected bounded-loss research only. This harness does not implement berth allocation, quay-crane scheduling, yard simulation, a physical terminal model, participant authorization, or a TOS integration. It does not certify DCT/DGT risk, conditional coverage, real schedule feasibility, cargo custody, vessel dispatch, receiver delivery, or operational authority.
