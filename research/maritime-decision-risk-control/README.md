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

The deterministic receipt compares three finite-grid cases: 1,200 schedules at 20% target risk, 1,200 at 10%, and 50,000 at 10%. These counts are synthetic illustrations of the correction's effect, not DCT sample-size recommendations or operational risk estimates. The 1,200/10% arm is expected to show that the correction can consume the entire risk budget; the 50,000/10% arm tests whether a useful adjusted target returns at a larger calibration size.

The original monotone CRC, CRC with a monotone envelope, and finite-grid CRC-NM are different methods with different theorem assumptions. They must be compared at matched loss definitions and clearly reported guarantee types; do not treat them as interchangeable.

## Important release preconditions

The future #7231 integration must establish all of the following before interpreting a risk output as operationally meaningful:

- The loss is defined over **complete schedules**, not individual vessel rows.
- Calibration schedules and the future schedule satisfy the exchangeability assumption, or a different method with a valid guarantee is used.
- The frozen policy, evaluator, resource map, constraint catalogue, and loss contract have not changed after calibration.
- For standard CRC, the policy parameter must be genuinely monotone for the declared loss, or a justified monotone upper envelope must be used. The separate finite-grid non-monotone method does not require monotonicity, but instead pays its explicit sample-size/grid-resolution correction and requires the stated population-risk condition for the terminal policy.
- A maximum-parameter policy with loss at most `α` exists **by construction**. A fail-closed abstention is not automatically a low-service-loss policy; if abstention counts as failure, its loss must be included.
- Hard authority/consent checks remain deterministic gates. A statistical risk budget cannot authorize an action.
- The exact small-instance oracle independently adjudicates physical feasibility and unauthorized actions.
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
```

## Claim ceiling

Synthetic expected bounded-loss research only. This harness does not implement berth allocation, quay-crane scheduling, yard simulation, a physical terminal model, participant authorization, or a TOS integration. It does not certify DCT/DGT risk, conditional coverage, real schedule feasibility, cargo custody, vessel dispatch, receiver delivery, or operational authority.
