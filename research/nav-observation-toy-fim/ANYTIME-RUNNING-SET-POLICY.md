# Anytime Running-Average Set Policy

## Purpose

This module implements the adaptive miscoverage-level update from Fang (2026), *Anytime Valid Conformal Prediction on Streaming Time Series*, arXiv:2611.20905, Equations (34)-(35):
https://arxiv.org/abs/2611.20905

It is a policy adapter that consumes the certified upper miss sum from the evaluator-side anytime episode audit and chooses either:
- abstain for the next episode; or
- request a bounded prediction set at an exact miscoverage level.

It does not construct the prediction set, train the estimator, authenticate evidence receipts or authorize motion/actuators.

## Rule and buffer

Let alpha be the target running-average miscoverage, gamma > 0 the learning rate, t the number of completed episodes, and S^c_t the certified upper miss sum from the audit:

`S^c_t = S_t + t * omega_t`

The next requested level is

`eta_(t+1) = alpha + gamma * (alpha*t - S^c_t - B)`.

If eta is nonpositive, the algorithm abstains. Otherwise, it requests a bounded prediction set at precisely eta. The policy intentionally does not clip eta to [0,1]; the selected prediction-set family must define and test the semantics of the level supplied to it. The policy must not silently substitute a different value.

For the stitched empirical-Bernstein margin, the paper gives a one-step certified-miss increase bound

`c = 1 + 1.7*sqrt(0.72*log(5.19/delta)) + 2.45*log(5.19/delta)`.

The buffer must satisfy

`B >= alpha/gamma - alpha + c`.

The constructor computes c and rejects a configuration whose buffer is below this theorem minimum.

At the beginning of monitoring, S^c_0=0 and t=0, so the policy may initially abstain. Each completed abstention contributes Z_t=0 and leaves S^c_t unchanged; eta therefore increases by gamma*alpha for the next episode. After a bounded set is issued, its sealed hit/miss outcome updates the audit and can reduce eta.

## Correct ordering

For episode t:

1. Read only the audit snapshot from completed episodes 1..t-1.
2. Compute eta_t with the formula above.
3. If eta_t <= 0, issue the unbounded/abstention action. Otherwise, pass exactly eta_t to a conformal set builder that supports the required level semantics.
4. Seal the prediction/decision receipt and observation-set identity before truth is inspected.
5. After the full trajectory and truth are sealed, compute the complete-episode hit/miss and outcome receipt.
6. Update the evaluator audit; that snapshot becomes the input for episode t+1.

No truth, post-hoc NLOS label, future packet, or evaluator-only invalidity flag may change the already-made decision, observation inclusion, controller input ordering or requested level for episode t.

## Why this matters

Naive adaptive conformal inference aims at the realized running miss fraction and can overshoot the target before it reacts. Fang's update instead uses the certified miss sum S^c_t, adding the confidence-sequence margin to the observed miss count, and includes a fixed buffer B for the maximum one-step increase. The preprint's Theorem 4.1 proves a time-uniform true running-average miscoverage bound under arbitrary distribution shifts, under its filtration and algorithm assumptions.

This is not a guarantee on every individual conditional miscoverage rate. The preprint proves that such a guarantee is impossible under unrestricted distribution shifts without abstention/unbounded sets or additional assumptions.

## Tests in the module

- rejects invalid alpha, delta, gamma and buffer;
- enforces the theorem-minimum buffer;
- verifies that monitoring starts with an abstention at the minimum buffer;
- verifies that abstentions increase eta by gamma*alpha when the certified sum is unchanged;
- verifies that a larger certified miss sum reduces eta and can force abstention;
- preserves the exact un-clipped positive eta for the downstream set builder;
- rejects audit snapshots with mismatched alpha/delta, invalid sums or counter overflow.

## Integration status and claim boundary

The adaptive policy is separate from the episode audit. The audit must supply the exact certified upper miss sum generated from the same alpha/delta and frozen stitched-margin configuration.

Not implemented:
- a conformal quantile/set builder that accepts dynamic eta and proves its parameter semantics;
- a persistent controller that binds each eta to a specific next episode and signed decision receipt;
- rolling-average adaptation and its feasibility check;
- physical UWB truth, source rights, device runtime integration, measured availability or safety qualification.

This remains research-only. A valid policy output is not proof that the downstream predictor actually produced a bounded set with the requested coverage semantics.
