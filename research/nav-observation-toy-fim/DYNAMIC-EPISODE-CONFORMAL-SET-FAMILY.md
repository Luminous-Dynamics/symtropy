# Dynamic Episode-Level Conformal Set Family

## Purpose

This module bridges the sequential running/rolling miscoverage policies to an explicit geometric prediction set. It takes a requested whole-episode miscoverage level `eta` and returns either:

- a constant-radius 2D tube around every predicted position in the episode; or
- an explicit abstention when the requested level is unsupported or the frozen calibration cohort is too small.

The score is the **maximum Euclidean position error over the entire calibration trajectory episode**. It is not a packet-level residual score.

This is a conformal set constructor. It does not itself provide an anytime guarantee under distribution shift. That guarantee comes from a separate sequential policy, confidence sequence, predictable decision filtration and sealed post-decision whole-episode outcome.

## Finite-sample split-conformal selection

For n independent calibration episode scores and requested miscoverage eta, let target coverage be (p=1-eta). The one-based rank is:

`k = ceil((n+1)*p)`.

Sort the episode maximum-error scores in ascending order and select the k-th score as the radius. If k>n, there is no finite rank and the family abstains.

Examples:

| Calibration episodes | Requested miscoverage | Rank | Result |
|---:|---:|---:|---|
| 98 | 1% | 99 | abstain: insufficient calibration |
| 99 | 1% | 99 | finite radius: maximum score |
| 99 | 20% | 80 | finite radius: 80th ordered score |
| 199 | 0.5% | 199 | finite radius: maximum score |

The calibration cohort must be disjoint from evaluation by both episode identity and independence-group identity. The calibration predictor, preprocessing and score definition are frozen and digest-bound. A changed model or score requires a fresh calibration family.

The usual split-conformal marginal statement assumes the calibration and future episode scores are exchangeable. It is not a conditional guarantee for every environment, device, anchor geometry, obstacle or fault regime. The separate anytime policies target a time-uniform average of conditional whole-episode miscoverage; they do not transform split conformal into an unrestricted per-episode conditional guarantee.

## Whole-episode set

For predicted positions (hat x_1,ldots,hat x_T) and selected radius r, the set is represented as a sequence of disks:

`C_t = { x in R^2 : ||x - x_hat_t||_2 <= r }, t=1,...,T`.

The event is a whole-trajectory hit only when every evaluated truth position is inside its corresponding disk. The `contains_complete_trajectory` method rejects a missing/truncated truth trajectory or a non-finite truth point. A partial trajectory must never be counted as a hit.

The returned tube retains:
- episode and independence-group identities;
- every predicted center and the selected radius;
- requested miscoverage and implied coverage;
- calibration episode count and selected order-statistic rank;
- calibration cohort, predictor and score-definition commitments;
- decision sequence ID and decision receipt digest.

The receipt digest is supplied by the caller. The module checks that it is nonzero and preserves it, but does not hash or authenticate the underlying receipt bytes.

## Connection to anytime running and rolling control

The sequential policies use eta chosen before the current truth is observed:

- [#6976](https://github.com/Luminous-Dynamics/symtropy/pull/6976) selects an adaptive running-average level or abstains, based on the certified upper miss sum and theorem buffer.
- [#6977](https://github.com/Luminous-Dynamics/symtropy/pull/6977) selects an adaptive rolling-window level only after both-outcome prospective feasibility across every affected future window.

This module maps levels without clipping: a non-finite or nonpositive eta produces an unbounded/unsupported abstention; (0<eta<1) selects the finite-sample radius; and (eta>=1) produces an explicit empty bounded set (coverage zero, whole-episode miss one). The empty set is important for the sequential ACI feedback loop: treating eta>=1 as an abstention would feed a false zero miss back into the policy and could drive eta upward indefinitely. A rank that exceeds the calibration cohort for (0<eta<1) also produces an unbounded/unsupported abstention. The caller must record these three outcomes distinctly.

Importantly, constructing a tube at a requested eta is not proof that the running or rolling theorem holds. The overall sequential claim still requires:
1. eta is selected using only prior completed episodes;
2. the complete episode prediction is sealed before truth is observed;
3. the family and predictor remain frozen to calibration;
4. the outcome is scored once after the whole trajectory and truth are sealed;
5. the outcome updates the correct evaluator state only for future decisions.

## Limitations and future work

- This first set family is 2D and isotropic (constant Euclidean radius); it is not an anisotropic tube or a polytope/range-set posterior.
- The calibration score and source truth are caller-supplied. This module does not parse raw CIR or establish ground-truth quality.
- No environment-conditional or dependence-adjusted calibration is implemented here.
- No source signature, receipt hashing or trusted time verification is implemented.
- The family does not persist a sequence ledger; the caller/controller must enforce monotonic decision and episode IDs.
- The empty bounded set for eta>=1 intentionally records a miss for every finite truth trajectory; it is not a fail-safe abstention.
- Adaptive/rolling policies are not yet joined into a single runtime issuer. Their integration must preserve one shared policy sequence, receipt and outcome contract.
- Real Bregar, IDLab, outdoor or MILUV trajectory data have not been admitted or scored in this module.
- This is not a PHMI, physical availability or safety qualification.

Keep this research-only until the set family is integrated with the sequential policy using sealed receipts and evaluated on source-complete, independently held-out episodes.
