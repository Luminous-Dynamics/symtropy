# WPT four-quadrant common-mode invariance oracle v0

**Status:** executable synthetic metamorphic control  
**Research parent:** [WPT foreign-object resilience #3496](https://github.com/Luminous-Dynamics/symtropy/issues/3496)

## Why this exists

Yan, Xing, Liu & Sun (2026), “Power-Decoupled Metal Object Detection Using Four-Quadrant Flux-Coupling-Ratio Sensing for Wireless Power Transfer,” *IEEE Transactions on Instrumentation and Measurement*, DOI [10.1109/TIM.2026.3716472](https://doi.org/10.1109/TIM.2026.3716472), report an 85 kHz LCC-S experimental prototype. Their described approach compensates receiver contribution, suppresses primary-side common scaling through four-channel normalization and applies an adaptive ordered-statistic CFAR threshold. The abstract reports object-free residual returning to its band under tested primary input-voltage and secondary-load variations, with stable behaviour under tested fixed Tx–Rx misalignment; object intrusion produces a sustained separated response.

That does not establish broad moving-vehicle, new-coil, temperature, ageing or sensor-fault performance. The article summary does not provide the complete raw per-channel dataset, class-wise false-negative rate, latency distribution or full condition matrix. Those are open benchmark requirements.

## What this crate does—and does not do

This crate tests a generic metamorphic property of a four-channel normalization pipeline:

`N(αx) = N(x)` for positive common scale `α`.

It uses non-negative synthetic quadrant magnitudes normalized by their sum, after overflow-safe maximum scaling. It then measures total-variation distance against a frozen object-free baseline. It verifies two controls:
- common-mode scaling does not create a false object signature;
- a localized channel-pattern perturbation remains detectable after the same scaling.

The crate does **not** implement the paper's exact 4Q-FCR equation, its receiver-side compensation, its OS-CFAR logic, or a calibrated hardware detector. Its normalization is a generic mathematical oracle, not a reproduction of the paper's algorithm. No values are extracted as measured sensor observations.

## Safety and fail-closed behaviour

- Any faulted or unknown quadrant health returns NOT_EVALUABLE; remaining channels are not silently renormalized.
- Non-finite, negative and all-zero signals are rejected.
- Channel order is semantic; permutation is not silently canonicalized.
- A threshold is a synthetic fixture parameter, not an FOD operating limit.
- Object-signature presence is not itself authorization to energize or continue transfer. The operational policy remains the separately qualified detect/localize/predict/mitigate/verify sequence in #3496.

## Tests

Twelve Rust tests cover common-scale invariance, preserved object signature, object-free no-trigger, localized perturbation detection, quadrant permutation, unknown/faulted channel abstention, invalid signals, threshold validation and extreme finite magnitudes.

Run:

```sh
cargo test --locked -p wpt-fod-invariance-oracle
```

## Next experimental comparison

Use the same frozen object/session-held-out data to compare:
- impedance baseline;
- planar magnetic-inductance tomography;
- published-style 4Q flux-coupling-ratio sensing;
- thermal sensing;
- hybrid detection.

Sweep primary input voltage, receiver load, Tx–Rx gap/misalignment, temperature and receiver geometry with and without objects. Report object-free residual drift and object-present residual separation by operating cell, plus class-wise FNR/FPR, p99 latency, localization, calibration, OOD abstention, channel-fault abstention and thermal-response verification. Do not tune a detector threshold on held-out object/session data.

The 4Q-FCR paper's fixed-misalignment results are a starting baseline, not evidence for the full dynamic operating envelope.
