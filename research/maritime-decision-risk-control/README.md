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
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin bernstein-risk-audit
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin simultaneous-hoeffding-risk-audit
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin bernstein-risk-audit
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin lipschitz-risk-audit
```



## Variance-sensitive non-monotone CRC

The fifth executable, `bernstein-risk-audit`, implements the **known-population-variance Bernstein correction** in Appendix F, Corollary S1 (Lemma S2) of Aldirawi, Li & Guo, arXiv:2604.01502v3:

https://arxiv.org/pdf/2604.01502

For bounded losses in [0,B] on a finite candidate grid of size m, with population standard deviation uniformly bounded by `sigma_max`, the paper gives:

`D_Bern(m,n) = sigma_max * sqrt(2*log(2m)/n) + B*log(2m)/(3n)`

and the non-monotone expected-risk correction:

`D_selected = min{D(m,n), D_Bern(m,n)}`.

The adjusted selector uses `alpha' = alpha - D_selected` and requires the terminal candidate's **population expected loss** to be at most alpha'. This is still an expected-risk theorem, not a high-probability statement conditional on the realized calibration set.

For loss in [0,B], Popoviciu's inequality gives `sigma_max <= B/2`. The synthetic benchmark supplies an analytically known upper bound of 376 milli-loss for its four-policy Bernoulli grid. Candidate losses share a within-schedule latent draw but complete schedule vectors are i.i.d. across episodes, as required by the theorem.

The fixture probabilities are [0.17, 0.04, 0.12, 0.05] with B=1,000 and alpha=200 milli-loss. The terminal candidate has exact population expected loss 50 milli-loss. Calibration sizes are 120, 1,200 and 50,000. The receipt reports both corrections, the selected minimum, adjusted target, selected policy, calibration mean and independent synthetic test mean.

### Important: empirical Bernstein is not the same theorem

Appendix F, Lemma S3 gives an empirical-Bernstein **high-probability uniform-deviation certificate** with confidence parameter delta. Its random variance estimate and failure event mean that it cannot simply be substituted into the expected-risk selector and called the same Corollary S1 guarantee. The paper explicitly requires additional accounting for the probability-delta failure event before an expected-risk claim can be made.

Therefore this implementation uses only the **known population variance bound** for its expected-risk arm. The empirical-Bernstein statement remains a separate high-probability comparator for #7303.

Proposed reproduction command:

```sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin bernstein-risk-audit
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



## Simultaneous high-probability finite-grid risk control

The sixth executable, `simultaneous-hoeffding-risk-audit`, is a transparent high-probability baseline. It is **not** labeled as Learn-Then-Test or RCPS.

For a fixed, predeclared finite grid of m candidate policies, complete-schedule losses in [0,B], and i.i.d. calibration schedules, the one-sided Hoeffding + union-bound radius is:

`epsilon(n,m,delta) = B * sqrt(log(m/delta)/(2n))`.

With probability at least 1-delta over the calibration sample, every candidate's population hard-failure risk is bounded by its empirical calibration mean plus this radius. Because the guarantee holds simultaneously over the entire candidate grid, the selector can choose the candidate with the lowest empirical **service loss** among those whose hard-risk upper bound is at most alpha.

This is a different guarantee class from marginal expected-risk CRC:
- CRC controls risk in expectation over calibration datasets and a future schedule.
- The simultaneous selector gives a high-probability guarantee for the realized calibration sample, conditional on the frozen i.i.d. sampling setup.
- The direct union-bound selector can be more conservative, especially at small n or small delta.
- If no policy meets the simultaneous upper bound, it returns NO_POLICY_MEETS_TARGET. It does not fall back to an uncertified policy.
- The service-loss metric is used only to select within the certified subset; it is not combined with the hard-failure axis.

The synthetic receipt uses a 10% target, delta=0.05 and a four-policy grid. The 120-schedule arm has no eligible candidate; the larger arms test whether the narrower confidence radius admits a service-efficient policy. This is a finite-grid Hoeffding baseline, not a reproduction of LTT/RCPS or an anytime-valid method.

Reproduction command:

```sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin simultaneous-hoeffding-risk-audit
```

## Independent oracle outcome adapter

The seventh research component is an adapter, not another feasibility engine: `oracle_adapter.rs` consumes versioned per-episode/per-policy adjudications from an independent oracle such as the #7231 owner. It compiles separate whole-schedule matrices for hard physical failure and service loss.

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
- The strict calibration adapter rejects an unknown physical outcome. It is never converted to a zero loss or silently dropped.
- A separate `compile_partially_identified_outcomes` diagnostic can represent unknown physical cells as [0, B] with lower (Unknown → Feasible) and upper (Unknown → Infeasible) matrices, plus per-policy unknown counts. It is explicitly marked `certification_allowed=false` and cannot be treated as CRC input or a calibrated risk result. MNAR, dependence and source missingness remain unresolved.
- Physical `Infeasible` becomes hard-failure loss 1,000; `Feasible` becomes 0. Service loss remains its own value in [0, 1,000]. They are not added or averaged.
- A reused dependence-cluster ID across any episodes or split roles is rejected. Clustered operations must first be aggregated into one complete schedule-level sampling unit or handled under a separate dependence-aware method.
- A missing/extra policy candidate, policy revision drift, evaluator revision drift, loss-contract drift, mixed operating regimes, duplicate episode IDs, malformed receipt digest, or out-of-range loss rejects the whole dataset.
- Calibration and Evaluation matrices remain separate. Evaluation outcomes cannot enter calibration selection through this adapter.
- Episode rows are sorted deterministically, while policy columns retain the exact declared CRC parameter order. Input episode order and policy-outcome order do not change the compiled result.

The adapter does **not** prove that the oracle is independent, that the evidence reference is truthful, that an external digest matches the policy-grid contents (only its SHA-256 shape is checked), or that the declared exchangeability scope holds. Those remain owner/evidence assertions. The adapter also reports empirical upper-tail CVaR by policy and loss axis. These descriptive diagnostics are not finite-sample CVaR guarantees or risk-control certificates. The broader comparison against high-probability LTT/RCPS, stability-based and anytime-valid methods is owned by [#7303](https://github.com/Luminous-Dynamics/symtropy/issues/7303). It does not implement physical feasibility, estimate importance weights, or convert a CRC expected-risk result into an authorization decision.

The root `crate::LossMatrix` and finite-grid CRC-NM grid can consume the emitted matrices independently for each loss axis. Known-weight shift selection is deliberately not wired to this adapter: a real bounded likelihood ratio must be independently justified, not inferred from these receipts.

The adapter has 21 unit tests for separate axes, deterministic permutation invariance, exact frozen policy-order preservation, authority/action-class binding, authority denial/unknown, physical unknown, bounded-loss violations, dependence-cluster leakage, policy-grid/revision drift, mixed regimes, digest validation, role separation, and empirical upper-tail CVaR.

## Important release preconditions

The future #7231 integration must establish all of the following before interpreting a risk output as operationally meaningful:

- The loss is defined over **complete schedules**, not individual vessel rows.
- Calibration schedules and the future schedule satisfy the exchangeability assumption, or a different method with a valid guarantee is used.
- The frozen policy, evaluator, resource map, constraint catalogue, and loss contract have not changed after calibration.
- For standard CRC, the policy parameter must be genuinely monotone for the declared loss, or a justified monotone upper envelope must be used. The separate finite-grid non-monotone method does not require monotonicity, but instead pays its explicit sample-size/grid-resolution correction and requires the stated population-risk condition for the terminal policy.
- A maximum-parameter policy with loss at most `α` exists **by construction**. A fail-closed abstention is not automatically a low-service-loss policy; if abstention counts as failure, its loss must be included.
- Hard authority/consent checks remain deterministic gates. A statistical risk budget cannot authorize an action.
- The exact small-instance oracle independently adjudicates physical feasibility and unauthorized actions. Its outcome receipts are consumed through `oracle_adapter.rs`; the adapter does not reimplement those constraints. The adapter preserves the pre-frozen policy parameter order rather than sorting by ID, and requires `Mutating` actions to have `Authorized` disposition while `NoMutation` may use `NotRequired`.
- Missing, censored, and outcome-dependent observations are separately identified; hidden outcomes never enter the deployed-style calibrator. Partial-identification lower/upper matrices are diagnostics only and do not restore a CRC guarantee under MNAR or dependent missingness.

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
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin lipschitz-risk-audit
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin bernstein-risk-audit
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin simultaneous-hoeffding-risk-audit
```


## Exact-head validation — 24 November 2026

<badge color="success">PASS</badge> Pinned Rust 1.96.0 exact-head workflow [#157](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45394507064), commit `4964e19e89166313e7398f141771aa2408ce9427`:
- `cargo fmt --check`: passed;
- strict `cargo clippy -- -D warnings`: passed;
- `cargo check`: passed;
- `cargo test`: **70 passed, 0 failed**;
- all six executable CSV receipts passed deterministic replay and schema/behaviour checks.

This qualifies the synthetic software/method fixtures only. It does not qualify real DCT/DGT data, the #7231 oracle's independent evidence, operational exchangeability, known density ratios, or any actual port schedule.


## Claim ceiling

Synthetic expected bounded-loss research only. This harness does not implement berth allocation, quay-crane scheduling, yard simulation, a physical terminal model, participant authorization, or a TOS integration. It does not certify DCT/DGT risk, conditional coverage, real schedule feasibility, cargo custody, vessel dispatch, receiver delivery, or operational authority.


## Conjunctive finite-scenario + statistical assurance envelope

The assurance_envelope.rs module adds a composition layer across two distinct kinds of evidence:

1. **Finite adversarial scenarios:** an exact, predeclared scenario universe is evaluated for physical feasibility, action-authority consistency, unknown outcomes and participant-level service floors.
2. **Statistical risk:** a named method contributes one explicitly typed guarantee (marginal expected, high-probability, or time-uniform) for the hard-physical-failure axis.

The statistical claim preserves the theorem class: a high-probability statement is a population-risk bound with confidence over calibration samples, not conditional validity for one realized calibration set. A time-uniform statement must name its exact monitoring horizon/sequence; the horizon reference is carried through to the decision output.

The finite-scenario contract also declares a minimum service floor for **each participant in every scenario**. A compensating gain to another participant cannot offset a breach; floor violations block composition even when the aggregate service or statistical risk bound is favourable. The floor is part of the frozen loss/contract revision.

The gate is a strict conjunction. A statistical risk bound cannot override a physical failure, an authority violation, an unknown physical outcome, an incomplete scenario universe, source-provenance failure, policy/evaluator revision drift, or a mismatched operating regime. Conversely, a clean finite stress set cannot substitute for a statistical guarantee. Diagnostic CVaR/tail metrics are never accepted as risk-control evidence.

Finite scenarios are **not** appended to the CRC calibration matrix: the finite set is an adversarial coverage claim, not an i.i.d. sample. The statistical claim retains its guarantee class and (where applicable) its failure probability; the output does not flatten marginal expected, high-probability and time-uniform guarantees into a single number. Participant A/B service remains visible separately from service-loss and hard-failure axes.

### Evidence and claim boundary

The source enum and evidence references are declarations made by the caller. This module does not authenticate an oracle, prove that the scenario universe is exhaustive for reality, verify the statistical theorem assumptions, or grant operational authority. The positive unit test demonstrates composition semantics using synthetic declarations only; it is not a real independent-oracle or deployment certificate.

### Next integration

The #7188 scarce-reserve benchmark supplies a useful six-world finite-state test domain. Its current CSV is a policy/model receipt and must not be relabeled as independently adjudicated truth. A future #7231 owner-produced projection should provide per-policy/per-scenario oracle dispositions and the required independence evidence before the IndependentOracle source path is used. Statistical calibration continues to consume separately sampled, complete schedule episodes through oracle_adapter.rs.


## Repeated-sampling risk tournament (24 November 2026)

The executable **repeated-sampling-risk-audit** adds a repeated-calibration experiment to the single-run method receipts. It uses a finite four-state synthetic population with exact state probabilities and exact population risks. The policy selector receives only sampled complete-schedule losses; population risks are used only by the evaluator after selection.

The experiment compares five fixture/method combinations:

- Monotone whole-schedule CRC.
- Fixed-grid simultaneous Hoeffding UCB on the monotone fixture.
- Monotone-envelope CRC on a deliberately non-monotone loss family.
- Finite-grid non-monotone CRC (CRC-NM) on that same family.
- Fixed-grid simultaneous Hoeffding UCB on the non-monotone fixture.

All methods use the same deterministic replicate stream (common random numbers). The default smoke run performs 2,000 independent synthetic calibration replicates at n=1,200. Every receipt row records the requested α; `delta=0.050` is recorded on fixed-grid high-probability rows and `NA` on marginal expected-risk rows. The terminal policy has exactly zero hard loss by construction in this finite toy population; this known-answer condition is not inferred from calibration data. The **--full** run performs 2,000 replicates at each calibration size n = 50, 120, 500, 1,200, 5,000, 10,000 and 50,000, for target risks α = 0.01, 0.05, 0.10 and 0.20. The policy grid is intentionally frozen at four candidates in this first increment; expansion to the 16–10,000 candidate tournament remains a separate #7303 extension.

Run the reproducible smoke receipt:

~~~sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin repeated-sampling-risk-audit
~~~

Run the complete synthetic sweep:

~~~sh
cargo run --release --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin repeated-sampling-risk-audit -- --full
~~~

Interpret the receipts by guarantee class:

- **MARGINAL_EXPECTED_RISK** rows report the mean exact population risk of the policy selected over calibration replicates, with a Monte Carlo interval. The CRC theorem is a marginal expected-risk statement; the replicate-level rate of selected policies above α is **not** its validity criterion.
- **MONOTONE_ENVELOPE_CRC** also reports the exact population risk of the suffix-maximum envelope controlled by the method, separately from the original loss. The envelope is conservative and upper-bounds the original loss.
- **FIXED_GRID_HIGH_PROBABILITY** rows report the frequency with which a selected policy's exact population risk exceeds α, its Wilson interval, the conditional rate among non-abstaining selections, and the abstention count. The conditional rate is `NA` when no policy is selected (zero denominator); the unconditional selected-unsafe event rate remains defined over all replicates. This tests the fixed-grid 1−δ selection statement; abstentions are not silently counted as safe deployed policies.
- **NO_POSITIVE_ADJUSTED_TARGET** means the CRC-NM finite-grid correction consumes the entire requested risk budget. No adjusted expected-risk claim is emitted for that configuration.

These results are synthetic method diagnostics. They do not qualify the #7231 physical/authority oracle, DCT/DGT exchangeability, operational risk, or deployment. The deterministic receipt is an experiment harness, not evidence that the 2,000 replicates correspond to 2,000 real independent port schedules.

### Additional policy-control comparator: Conformal Policy Control (ICML 2026)

Prinster et al., *Conformal Policy Control*, ICML 2026, introduces a different policy-improvement construction: constrain an optimized candidate policy relative to an already-qualified safe reference policy using a calibrated clipped likelihood ratio. It is a useful comparator for maritime policy updates, but it is not another name for CRC, LTT, RCPS or anytime-valid CRC.

Reference: [PMLR paper](https://proceedings.mlr.press/v306/prinster26a.html), [arXiv version](https://arxiv.org/abs/2603.02196), [authors' implementation](https://github.com/samuelstanton/conformal-policy-control).

A maritime adaptation is admissible only when all of the following are explicit: an independently qualified reference policy with risk at or below the target; a common and sufficiently stable context distribution; support overlap / absolute continuity between reference and candidate action distributions; and evaluable likelihood ratios for the relevant action or trajectory. A deterministic optimizer generally does not provide these quantities merely because it emits a schedule. Where the policy is deterministic or trajectory support cannot be justified, the comparator must return NOT_APPLICABLE rather than manufacturing a CPC certificate. The #7231 hard-feasibility and authority gates remain conjunctive and are not replaced by a safe-policy likelihood ratio.


## Dependence-aware calibration and cluster-unit falsification (24 November 2026)

The **clustered-dependence-risk-audit** executable is a synthetic negative control for pseudo-replication. Each experiment contains exactly 50,000 nominal schedule rows, but the rows are partitioned into 10, 25, 50, 100, 250, 500, 1,000 or 2,000 independent operating clusters. The number of schedule rows per cluster changes so that the nominal row count stays fixed. Every schedule in a cluster shares the same latent regime (intra-cluster correlation = 1.0 by construction).

For each configuration it compares:

- **SCHEDULE_ROWS_AS_IID:** an intentionally invalid baseline which treats repeated, perfectly correlated schedule rows as independent calibration units.
- **INDEPENDENT_CLUSTER_MEANS:** one bounded mean-loss vector per independent cluster. This is the valid unit for the declared synthetic estimand: expected per-schedule loss for a new independently drawn cluster with fixed cluster sizes.

Two risk-control families are kept separate:

- **FINITE_SAMPLE_CRC** reports mean selected population risk over repeated calibration runs. Its theorem is a marginal expected-risk statement; the replicate-level unsafe-selection frequency is only a secondary diagnostic.
- **SIMULTANEOUS_HOEFFDING_UCB** reports the frequency of selecting a policy whose exact population risk exceeds the target, including the Wilson interval, conditional selected-only rate and abstentions.

The CRC negative-control fixture has exact population risks [78.05, 38.05, 30.25, 0.0] milli against a target of 40 milli. The Hoeffding fixture has exact population risks [70, 38, 30, 0] milli. The final policy has exact zero loss by construction. All loss vectors are bounded in [0, 1000] milli; the candidate grid and service-cost ordering are frozen.

Run the deterministic receipt:

~~~sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin clustered-dependence-risk-audit
~~~

The negative control is deliberately extreme (intra-cluster correlation = 1), not an estimate of DGT/DCT correlation. Cluster-unit calibration assumes that the declared clusters are independently sampled. Different source systems, days or event IDs do not automatically imply independent clusters: common weather, labour, vessel-convoy, grid, equipment, broker, sensor and publication failures must be mapped before making that claim.

For unequal cluster sizes, pooled schedule weighting and equal-cluster weighting estimate different quantities. This first fixture keeps cluster sizes equal within each experiment and targets a typical randomly sampled cluster's mean schedule loss. Cluster-average risk is not a per-day maximum, worst-cohort guarantee, conditional-risk bound or tail-risk guarantee. Such outcomes remain separate and require their own contracts.

Research comparators:
- Ramos, Graziadei and Cabezas, *Conformal Prediction via Transported Beta Laws*, PMLR 329 (2026), explicitly analyzes how calibration dependence and clustering deform the i.i.d. calibration-conditional coverage law: https://proceedings.mlr.press/v329/r-ramos26a.html
- Allohibi, *Grouped Validation and Conformal Calibration for Clustered Cross-National Data*, Mathematics 14(21), 4055 (5 November 2026), compares pooled, equal-cluster and subsampling calibration: https://doi.org/10.3390/math14214055

The software experiment can falsify the assumption that 50,000 rows necessarily mean 50,000 independent units. It cannot determine a real port's cluster structure, establish DCT/DGT exchangeability, or confer an operational statistical guarantee.


## Oracle adapter: preserve cluster identity through calibration (24 November 2026)

The oracle adapter previously treated every repeated dependence-cluster ID as an error. That prevented multiple complete schedule episodes from sharing a valid same-split common-cause cluster, so an upstream caller could not aggregate their independently adjudicated per-schedule loss vectors into one cluster calibration row without bypassing the adapter.

The new adapter projection separates two controls:

1. **Split isolation:** the same dependence-cluster ID may appear on multiple complete schedule episodes within calibration, or on multiple episodes within evaluation, but it may never appear in both roles. A weather/outage/TOS incident cannot leak between calibration and held-out evaluation merely because its individual schedule IDs differ.
2. **Explicit aggregation:** the raw per-schedule matrices remain available and are not silently collapsed. The caller must explicitly call `OracleCalibrationBundle::cluster_mean_matrix(role, axis)` to obtain one row per cluster.

The returned `ClusterMeanMatrix` preserves canonical cluster IDs, source schedule counts, role and loss axis. It equal-weights cluster means (the estimand is a typical cluster's mean schedule loss), and rounds each milli-loss mean **upward** to avoid optimistic integer quantization. It is then convertible to the existing CRC `LossMatrix` interface. This is a different target from pooled schedule weighting when cluster sizes differ; the adapter does not pretend they are interchangeable.

The physical hard-failure and service-loss axes remain separate. The aggregation never averages participant authority into a risk number, and it cannot cure a falsely declared cluster boundary. Different cluster IDs do not establish independence: source-owner/common-cause review remains required for weather, labour, vessel convoys, power, equipment, sensor, gateway, broker and TOS incidents. If a cluster must be split across calibration/evaluation, the adapter rejects the dataset. If the dependence structure cannot be defended, the appropriate result remains NO_GUARANTEE / NOT_EVALUABLE.

Regression coverage:
- accepts repeated same-split episodes sharing a dependence cluster;
- aggregates their policy loss vectors into one cluster row with deterministic ordering;
- rounds fractional cluster means upward;
- retains schedule counts and cluster IDs beside the aggregated matrix;
- rejects any cluster ID reused across calibration and evaluation;
- leaves the original per-schedule matrices unchanged.

## Cluster-aware oracle receipt projection (24 November 2026)

The oracle adapter now preserves the dependence-cluster identifier beside every raw schedule row and permits several adjudicated episodes to share one cluster within the same dataset role. It rejects any cluster ID that is reused across calibration and evaluation, even if the episode IDs are different.

The bundle's matrix(role, axis) and non_monotone_grid(role, axis) methods continue to return raw per-schedule rows. Their presence does not establish IID schedule observations. When the independent unit is the disruption/day/cluster, call cluster_mean_matrix(role, axis) explicitly. It returns one row per sorted cluster ID, retains the number of source schedule rows in each cluster, uses equal cluster weighting, and rounds each bounded milli-loss mean upward. The resulting ClusterMeanMatrix matrix() or non_monotone_grid() can be supplied to the existing risk-control methods.

This implements the **typical-cluster mean schedule-loss estimand**, not the typical-schedule estimand when cluster sizes differ. It does not prove that cluster IDs are independent or that a particular real-world grouping is correct. If event records share serially dependent shocks across distinct clusters, an IID cluster theorem is still not justified. The independent-cluster basis and any block/mixing argument must be reviewed by the source/operations owners. Unknown physical outcomes and unauthorized policy outcomes are still rejected by the original adapter validation before aggregation.

The adapter's exactness tests cover:
- repeated cluster IDs within calibration: accepted and aggregated into one calibration row;
- repeated cluster IDs within evaluation: same role rule;
- cluster IDs crossing calibration/evaluation: rejected;
- schedule rows preserved in the raw matrices while a distinct cluster matrix reduces the number of rows;
- conservative upward rounding of fractional cluster mean losses;
- deterministic cluster ordering under source input permutation;
- hard failure and service axes remain separate;
- risk selector receives only the cluster-unit rows and abstains in a deliberately underpowered two-cluster fixture.

The adapter also exposes typed execution wrappers so the calibration-unit label cannot be lost when the existing algorithms are called:
- `ClusterMeanMatrix::calibrate_marginal_expected_risk` wraps the existing CRC result with the cluster-unit provenance, loss axis, independent-cluster count, source schedule count, cluster IDs, and per-cluster row counts. It checks that evaluator and loss-contract revisions match the oracle bundle.
- `ClusterMeanMatrix::select_fixed_grid_high_probability` accepts a separate cluster-level service matrix only when risk and service matrices share the exact same frozen cluster population, cluster sizes and policy order. The output preserves both provenance records and represents insufficient evidence as an explicit `Abstained(NoPolicyMeetsTarget)` outcome, not a successful empty selection.

The underlying compatibility matrix APIs remain available for research, but their returned rows do not retain the typed provenance by themselves. Prefer the typed wrapper for any reported claim.

The projection is a software contract, not an operational independence certificate. Cluster metadata, source receipts, exact physical/authority adjudication and operating regime remain caller-supplied evidence.


## Source-claim admission and append-only as-of evidence (24 November 2026)

The `source_semantics` module adds a synthetic-first admission gate for the source classes now present in the maritime source catalogue. It prevents four common category errors:

- A DGT vessel update may support a **terminal-reported vessel status**, not independent berth occupancy or a physical cargo movement.
- A DGT firm berth plan may support a **planned berth window**, not an observed alongside/departure event.
- Export stack dates and import storage snapshots may support **cargo availability windows** and **reported inventory state**, not proof of loading, gate-out, custody transfer or receiver delivery.
- A DCSA Port Call conformance report may support **API interoperability conformance**, not source capture completeness or physical truth. A third-party Dues status view remains derived evidence, not an independent witness.

The admission function also requires publication and retrieval times to be available by the requested historical as-of cutoff. A source with unknown publication time, a file retrieved after the cutoff, or a publication/revision that only became available later cannot enter an earlier replay.

### Append-only artifact revisions

`AppendOnlyArtifactLedger` retains each source snapshot as an immutable revision. A new revision must:
- have a new snapshot ID and the same logical artifact-family identity;
- name the immediate prior snapshot it supersedes;
- increase the revision sequence by exactly one;
- preserve its own raw SHA-256, source revision, publication time and retrieval time.

`latest_available_as_of(family, cutoff)` selects the highest revision that had both been published and retrieved by that cutoff. A later correction cannot rewrite what an analyst could have known at the earlier time. Unknown publication time is preserved but not selected into historical as-of evidence.

This first ledger is an in-memory typed reference implementation and a CI fixture, not yet a durable database, crawler, external source signature verifier or evidence of DGT artifact completeness. Source-owner rights and genuinely independent witnesses still need separate review.

### Run the deterministic source-claim receipt

~~~sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin source-claim-admission-audit
~~~

The receipt exercises 21 synthetic claim cases, including plan-to-actual, vessel-status-to-physical, stack-window-to-loaded, import-storage-to-receiver, third-party-to-independent-witness and DCSA-conformance-to-physical-truth negative controls; publication-time unknown; retrieval/publication after the historical cutoff; missing independence reference, missing physical event time, and missing receiver custody digest; plus positive terminal-notice and receiver-acceptance cases.

No DGT raw XLSX/PDF bytes are bundled in the fixture. The receipt validates the *class/claim contract*, not the truth of a particular vessel or cargo event.


## EPCIS 2.0 event correction semantics (24 November 2026)

The `epcis_corrections` module adds a narrow typed implementation of the GS1 EPCIS 2.0 error-declaration lifecycle. It is intentionally separate from the source-artifact revision ledger:

- The artifact ledger versions documents/snapshots (for example a DGT AM/PM source file).
- The EPCIS correction ledger versions the interpretation of a particular business event (for example a container gate event later declared erroneous).

An `EpcisEventRecord` retains:
- globally unique eventID URI (enforced as unique within this ledger; global uniqueness still requires cross-system governance);
- canonical subject/EPC identifiers;
- eventTime in UTC plus the original eventTimeZoneOffset;
- recordTime, retained separately from the physical event time;
- source-native record identity;
- digest of the original assertion payload.

An `EpcisErrorDeclaration`:
- references the same eventID as the prior event;
- requires identical subjects, eventTime, timezone offset and assertion payload digest;
- records a separate declarationTime and repository recordTime;
- preserves the original source event and lists any correctiveEventIDs;
- cannot overwrite/delete the original event.

Historical as-of replay is recordTime-aware. Before the error declaration is available in the repository, the view remains AS_REPORTED. After the declaration is available, the old event remains visible but is DECLARED_IN_ERROR. Corrective events are shown as available only after their own recordTime; unresolved correctiveEventIDs remain explicit rather than being treated as an already-captured movement.

Run the deterministic receipt:

~~~sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin epcis-correction-audit
~~~

The six synthetic receipt cases cover: original event before correction, declaration known by its declared time but not yet recorded, declaration visible while the correction is still pending, correction event later available, subject mismatch rejection, and assertion-payload mismatch rejection. Tests also retain eventTime/recordTime/timezone separately and preserve the original event after declaration.

This is a targeted EPCIS compatibility fixture, **not** the complete EPCIS parser, JSON-LD or REST implementation, CBV vocabulary validator, DCSA conformance implementation, globally unique event-ID authority or physical observation system. It does not prove that the source captured every real event.


## DCSA event classifier maturity adapter (24 November 2026)

The `dcsa_maturity` module preserves the shared DCSA event-classifier vocabulary as distinct typed values:

- `EST` → ESTIMATED_ONLY
- `REQ` → REQUESTED_ONLY
- `PLN` → PLANNED_ONLY
- `ACT` → SOURCE_REPORTED_ACTUAL_ONLY

The DCSA Information Model 3.2 reference table lists all four values at the shared classifier-entity level. Its event-family tables are narrower in places: for example, the Shipment Event field lists PLN/ACT/EST, while the Operations Event field lists PLN/REQ/ACT/EST. Consequently, the current maturity adapter is a classifier-preservation fixture, **not** an event-family-specific T&T 2.2 schema validator. Any live DCSA adapter must validate the exact event family, API release and OpenAPI enum before accepting a code. Do not infer that REQ is valid in every shipment/equipment/transport payload.

The adapter retains standard version, eventID, canonical subject ID, source-native record ID, event time, timezone offset, repository record time, publication time and retrieval time. It rejects unknown classifier codes, release-version drift, missing publication time, publication after retrieval, records unavailable by the historical as-of cutoff, and an ACT event without event time.

**Critical claim ceiling:** ACT means that the DCSA source classified its milestone as actual. It is still source-reported evidence; the adapter always sets `independent_physical_observation=false`. An ACT event cannot be used as an independently verified berth occupancy, cargo movement, custody handoff or receiver acceptance without a separate qualified witness and evidence chain.

The adapter also allows future event times for EST/REQ/PLN entries; it does not misclassify a future planned berth or arrival as an invalid actual event. Event time remains distinct from repository record time and source publication/retrieval time.

Run the deterministic maturity receipt:

~~~sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin dcsa-maturity-audit
~~~

The synthetic tournament covers all four maturity values, unknown classifier, wrong version, unknown publication, late retrieval, ACT without event time, planned event with future eventTime and publication-after-retrieval. These tests qualify our mapping, not the external DCSA API conformance. For live implementation, pin and run the official DCSA T&T 2.2 / Port Call 2.0.0 reference conformance suites.


## Cross-standard mapping contract: preserve meaning, declare loss (24 November 2026)

The `cross_standard_mapping` module treats cross-standard translation as an explicit, versioned evidence operation rather than an informal field rename. The current fixture pins **DCSA Track & Trace 2.2.0 / EquipmentEvent**, **GS1 EPCIS 2.0.1**, and **GS1 CBV 2.0.0** separately. Every rule carries source event family, source/target field and value, mapping class, reversibility, content-addressed mapping-bundle ID/revision/SHA-256/reference, declared review status, reviewer/evidence references and (for CBV fields) the target vocabulary version.

The exact manifest is `research/maritime-service-chain-benchmark/dcsa-epcis-crosswalk-v1.0.0.toml`, SHA-256 `c9d2b17e1ef041bb6787849eda725021502d9822ab4b6980f357152dd9016e96`. Exact-head CI verifies the manifest bytes against that digest before running the mapper. The receipt also binds the source event family and target EPCIS/CBV versions, so a rule for `EquipmentEvent` cannot be reused on a `ShipmentEvent` with a similarly named field.

**Review status is deliberately two-tiered.** `SYNTHETIC_FIXTURE_ONLY` rules can be exercised only against source messages explicitly marked synthetic; they are blocked for non-synthetic records. `EXTERNALLY_APPROVED` requires an approval-record reference and evidence/reviewer references. This code checks that those references are declared; an independent governance workflow must resolve and verify them before production use. It does not self-certify human review. Draft and rejected rules are blocked.

`EXACT` mappings must preserve the literal source value and be reversible. `CONTEXTUAL` and `LOSSY` mappings must be irreversible and explain the semantic gap. A key firewall prohibits DCSA `eventClassifierCode` from mapping directly to EPCIS `bizStep` or `disposition`: EST/REQ/PLN/ACT describes source milestone maturity, while CBV business-step/disposition describes business process or condition. A DCSA `equipmentEventTypeCode=LOAD` may map contextually to CBV `loading` only under an explicit event-family/version-specific rule. The target validator checks a pinned logistics subset of CBV 2.0.0 values and canonical URI forms, not the complete vocabulary. The module emits a canonical model value; it is not an EPCIS JSON/JSON-LD serializer. EPCIS 2.0.1's official artifact set includes the JSON-LD context, OpenAPI, JSON Schema and SHACL validators.

The source payload bytes are **not** copied into this mapping module. Each receipt carries the original source SHA-256 and a source-store reference; the source evidence store remains the bytes' owner. The mapper checks the reference's digest binding but not access-control or continued storage of the referenced object.

Run the deterministic crosswalk receipt:

~~~sh
cargo run --quiet --locked --manifest-path research/maritime-decision-risk-control/Cargo.toml --bin cross-standard-mapping-audit
~~~

The 25-case receipt covers exact classifier-maturity preservation, contextual DCSA equipment-event type to GS1 CBV business-step translation, forbidden classifier-to-bizStep/disposition shortcuts, invalid target vocabulary, false reversibility, draft/rejected rules, synthetic-rule misuse on non-synthetic sources, event-family mismatch, EPCIS/CBV version mismatch, missing external approval reference, exact-value mismatch, content-addressed bundle digest/reference drift, source subject identity and source payload-reference failures. The CSV has 26 lines including its header and 13 columns. This is a deterministic synthetic qualification, not an external standards conformance score.

This is not a full DCSA/EPCIS parser, event-family schema validator, JSON-LD serializer, complete CBV validator, external approval resolver or official conformance tool. It is the admissibility contract that a future parser/crosswalk must satisfy before normalized output can enter downstream assurance and risk layers.


## Current exact-head qualification — cross-standard mapping (25 November 2026)

Final documentation-complete head `146d87719433c21be1b606f72515117d8ba72cd9`, workflow #273 (`45570820389`), pinned Rust 1.96.0:
- formatting, strict Clippy, compilation: passed;
- **165 tests passed, 0 failed**;
- the 25-case DCSA↔EPCIS/CBV receipt replay, 13-column schema and adversarial semantic checks: passed;
- exact TOML mapping-bundle SHA-256 verification: passed;
- global receipt digest manifest and 30-day reproducible artifact upload: passed.

Artifact: https://github.com/Luminous-Dynamics/symtropy/actions/runs/45570820389/artifacts/15002041953

Artifact ZIP SHA-256: `353ed3ddfd25e00d44fa43b42d8dd8bc64b6138d75d503376821c33b94138d64`.

This is a synthetic software qualification. The mapping bundle is fixture-only, external approval references are not independently resolved by this crate, and no DCSA API conformance, live source truth, physical cargo movement or receiver delivery is certified.
