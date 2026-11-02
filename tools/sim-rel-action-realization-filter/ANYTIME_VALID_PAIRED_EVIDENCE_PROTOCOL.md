# SIM-REL-045 — anytime-valid paired benchmark evidence

Status: exact-rational paired sign e-process and event-replay receipts implemented on a research branch. Exact-head qualification is pending. It does not replace the fixed-sample SIM-REL-042 analysis.

## 1. Why a distinct sequential procedure is needed

SIM-REL-042 is a fixed-sample inferential procedure. Repeatedly inspecting its p-values and stopping when a result looks favorable would invalidate the nominal Type-I error guarantee.

SIM-REL-045 adds an explicitly sequential path. It permits continued monitoring and optional stopping under a preregistered conditional sign-symmetry null. It retains a complete append-only event transcript and reconstructs every intermediate e-value.

The fixed-sample sign test and Hoeffding mean-loss interval remain useful separate endpoints. The e-process below tests directional win probability, not mean loss.

## 2. Paired sign sequence

For each valid matched pair:

- candidate lower terminal loss: one candidate win;
- reference lower terminal loss: one reference win;
- equal terminal loss: one tie.

Ties remain in the reporting stream but do not update the directional sign process. Let w be candidate wins and l be reference wins among discordant pairs.

The null assumption is conditional sign symmetry: given the preregistered task and the observation that the pair is discordant, the candidate/reference sign is conditionally fair under the null:

`P(candidate winmid discordance,mathcal F_{t-1}) = 1/2.`

This is a substantive statistical assumption. It is not automatic merely because the tasks are matched. The benchmark sampling and task-selection policy must preserve it.

## 3. Exact mixture e-process

Use a fixed prior uniform over alternative candidate-win probabilities `theta in [1/2,1]`. A fixed-`theta` likelihood ratio against the fair-sign null is:

`(2 theta)^w (2(1-theta))^l.`

Mixing the likelihood ratios under that fixed prior gives:

`E(w,l) = 2 integral_{1/2}^{1} (2 theta)^w (2(1-theta))^l d theta.`

With `u = 2 theta - 1`, this becomes:

`E(w,l) = integral_0^1 (1+u)^w (1-u)^l du.`

This fixed-prior mixture of nonnegative likelihood-ratio martingales is itself a nonnegative martingale under the conditional fair-sign null. It starts at one.

The executable implementation expands:

`(1+u)^w (1-u)^l`

as an alternating polynomial and integrates each term exactly as a rational. It uses checked u128 arithmetic and fails closed beyond 40 discordant pairs. It does not replace overflow with floating-point approximations.

Useful exact fixtures:

| Candidate wins | Reference wins | E-value |
|---:|---:|---:|
| 0 | 0 | 1 |
| 1 | 0 | 3/2 |
| 0 | 1 | 1/2 |
| 1 | 1 | 2/3 |
| 2 | 0 | 7/3 |

## 4. Anytime crossing and optional stopping

For a single preregistered family with allocated alpha `alpha_i`, Ville's inequality gives:

`P_0(sup_t E_{i,t} >= 1/alpha_i) <= alpha_i.`

The first qualifying crossing sequence is stored. Further observations may lower the current e-value, but they cannot erase the historical fact that the threshold was crossed.

A preregistered minimum discordant-pair count can delay the *reportable* crossing. The implementation still uses the same e-process and only records a claim after that minimum. Since this rule is a subset of all possible crossings, it does not increase the crossing probability.

The output stores a per-event replay receipt with the e-value, wins/losses/ties and first-crossing sequence after every event, including ties and invalid-task exclusions.

## 5. Multiple mechanism families

Fix the complete family set and total family-wise alpha before the sealed stream begins. With m preregistered families, allocate:

`alpha_i = alpha/m`

to each family and use threshold:

`1/alpha_i = m/alpha.`

For any null families, a union bound controls the probability of at least one false crossing by:

`sum_i alpha_i <= alpha.`

No independence between mechanism-family streams is required for this Bonferroni family-wise guarantee. This is intentionally conservative and avoids applying an invalid multiple-testing rule to running suprema.

All preregistered families remain in the receipt, including families with zero observations or no crossing.

## 6. Stream and replay integrity

The event transcript is bound to one SIM-REL-041 matched campaign manifest and includes:

- contiguous sequence number and unique event ID;
- unique pair/task/seed identity from the underlying matched trial;
- registered mechanism family;
- SealedHoldout group and split;
- reference/candidate arm, model, prompt, harness, toolset, scope and resource budgets;
- per-arm evidence artifact IDs;
- terminal-loss observations and validity status.

The verifier rejects Development/Calibration campaigns, split or manifest mismatch, unregistered families, out-of-range loss, duplicate/reordered events, and discordant streams above the exact arithmetic cap.

The output keeps both the final current e-value and the first crossing sequence. The append-only replay record can reconstruct every intermediate e-value from the immutable plan and trial events.

## 7. Claim boundaries

This process is valid under the declared conditional sign-symmetry null for the paired sequence in each family. It does not test mean loss, expected regret or practical effect size. Use SIM-REL-042 for the fixed-sample mean-loss endpoint.

The result does not establish:
- sign symmetry under arbitrary adaptive task selection;
- validity when the task selected at time t depends on the current pair's outcome;
- independence/exchangeability when repeated tasks share hidden seeds or correlated realizations;
- model adequacy outside the registered family set;
- generalization beyond the declared benchmark sampling frame;
- physical safety or apparatus correctness.

Optional stopping is only supported for the specified e-process and conditional null. It is not permission to inspect and repeatedly rerun arbitrary benchmark metrics.

## 8. Qualification plan

The focused suite includes:
- known exact rational e-value fixtures;
- independent double-polynomial expansion oracle for small win/loss counts;
- exact one-step martingale identity;
- exhaustive fair-sign path enumeration with a finite-horizon crossing-frequency bound;
- ties leave the e-value unchanged;
- historical crossing persists after further losses;
- fixed Bonferroni alpha allocation across family streams;
- SealedHoldout-only enforcement;
- family, manifest, event ordering and loss-range mutation controls;
- unsupported exact sample-size rejection.

Qualification requires pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree verification.
