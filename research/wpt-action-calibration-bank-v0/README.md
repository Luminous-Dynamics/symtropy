# WPT action-conditioned calibration bank v0

**Status:** exact registry lookup for precomputed, fixed-stratum, action-conditioned split-conformal calibrator records. It does not compute pose scores, create prediction sets, choose a probe or command hardware.

## Purpose

A pose score distribution can change with measurement action, action order, contact state, probe dwell and receiver/sensor configuration. A calibrator made under one action must not be reused for a different action because the receiver hardware happens to be the same.

This builds on #4297 / PR #4300 but keeps the calibration computation there. This registry selects an exact calibrator key and verifies the recorded conformal rank.

## Calibration key

The registry binds each calibrator to:
- evidence class and contact/state stratum;
- exact action digest and full sequence digest;
- receiver hardware and coordinate frame digests;
- joint nonconformity metric digest;
- predictor/observation-model digest;
- environment digest;
- alpha in parts per million.

The action digest commits to the specific action configuration. The sequence digest commits to the exact ordered probe plan, including order/dwell where upstream encodes it. No fuzzy matching, "nearest" action, wildcard, or device-family fallback is allowed.

## Calibration rank

For sample count n and miscoverage alpha:

    k = ceil((n + 1) * (1 - alpha))

A registry entry is valid only if its quantile_rank equals k and k <= n. If no finite order statistic exists, the calibrator cannot be registered as a usable finite calibrator. The threshold score itself comes from the upstream #4300 artifact; this registry does not fetch or recompute raw calibration episodes.

## Request and result

A request includes the exact same key fields plus decision time and upstream shift disposition:
- SUPPORTED: lookup may proceed;
- SHIFT_DETECTED: abstain;
- UNKNOWN: abstain.

A successful result means only CALIBRATOR_MATCHED. The caller still applies the threshold to upstream-supplied pose/model/fault scores using #4300. This bank never produces a pose set, makes no conditional-coverage claim, and provides no operational authority.

## Tests

- Exact V-guide probe-A lookup with known rank/threshold.
- Exact probe-B lookup resolves a different calibrator.
- Unseen action and unseen sequence abstain.
- Changed contact stratum, receiver, frame, predictor, metric, environment or alpha cannot fall back.
- Stale calibration and upstream shift status abstain.
- Rank mismatch, insufficient finite sample count and duplicate keys are rejected.
- Row permutation preserves lookup result.

## Claim ceiling

A pass validates only exact-key registry integrity, rank arithmetic and abstention semantics. It does not validate conformal score quality, exchangeability, state/action coverage, physical sensing, probe energy/ripple/thermal safety, actual pose calibration, energy transfer or flight authority.
