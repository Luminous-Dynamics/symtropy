# Magnetic encoder benchmark programme

This directory contains preregistered, evidence-aware comparison protocols for magnetic absolute-angle encoders. The first programme targets an 80 mm off-axis installation:

- **RLS AksIM-2 MRA080 / MB080** — current MBD01 Issue 15 datasheet baseline.
- **RLS AksIM-4 HIPERFACE DSL MC080 / MCD06** — provisional comparator; the exact current MCD06 PDF revision and ordering code must be obtained from RLS before the comparison can be called matched.

The objective is not to win a single datasheet-number contest. It is to compare system accuracy, calibrated and uncalibrated installed accuracy, speed/bandwidth, installation sensitivity, thermal behaviour, stray-field tolerance, diagnostics, power and total integration burden under matched and traceable conditions.

## Files

- `BENCHMARK_PROTOCOL_80MM.md`: test procedure, uncertainty, blind holdout, acceptance metrics and decision gates.
- `benchmark-plan.toml`: machine-readable draft configuration, frozen known fields and explicit unknowns.

## Ground rules

1. **Accuracy labels remain distinct.** Vendor system accuracy, calibrated installed accuracy and uncalibrated installed accuracy are separate measurands.
2. **Bits are not accuracy.** Resolution, SDE/noise, repeatability, hysteresis, bandwidth and latency are reported independently.
3. **No synthetic missing specifications.** AksIM-4 MCD06 speed, bandwidth, installed accuracy, ordering code and exact PDF revision remain vendor-confirmation gates until supported by a current document.
4. **No leakage.** Calibration/fit data, adaptive exploration, blind holdout and post-selection qualification are separate roles.
5. **Measurement uncertainty is part of the result.** Raw encoder data, independent reference data, calibration chain, uncertainty budget, firmware/configuration and reduction-code digests travel with the report.
6. **Not an ISO conformity claim.** ISO 230-2, 230-7 and 230-3 are methodological references for relevant positioning, rotary-axis and thermal tests. The actual test plan must be reviewed against licensed copies of the standards before claiming conformity.

## Research targets, not achieved results

For the 80 mm magnetic challenge, the current proposal is:

- system accuracy: parity ≤±0.004°; stretch ≤±0.003°;
- calibrated installed accuracy: ≤±0.007° against MRA080's published typical ±0.009°;
- uncalibrated installed accuracy: stretch ≤±0.020° against MRA080's published typical ±0.029°;
- zero false-safe spatial classifications on the preregistered challenge set.

These remain aspirational until hardware exists, the protocol is frozen and independently measured results are available.


## Comparator classes and computational frontier (reviewed 2026-11-27)

Keep four scoreboards separate:

1. **Magnetic absolute encoders:** RLS AksIM-2 MRA080 is the primary frozen comparator. Its MBD01 Issue 15 gives ±0.004° system accuracy, typical ±0.009° installed accuracy after calibration and ±0.029° uncalibrated. AksIM-4 MC080/MCD06 is a provisional comparator with a public catalogue ±0.004° variant; the exact current MCD06 PDF and order code must be obtained because the accessible page has also served legacy MCD05 documentation.
2. **Optical metrology:** Renishaw RESOLUTE installed accuracy varies with ring diameter (examples: ±12.7 arcsec at 52 mm, ±4.3 at 200 mm, ±1.9 at 550 mm). It is a metrology ceiling/reference candidate, not a like-for-like 80 mm magnetic competitor. Resolution below noise floor must not be treated as useful accuracy.
3. **Incremental Hall latches:** TI TMAG5111 (2D dual-channel, 40 kHz bandwidth) benchmarks digital edge, speed/direction and threshold integrity, not absolute angle.
4. **Field solvers:** Magpylib Material Response 0.15.0 (25 November 2026) reports high-precision checked WebAssembly kernels and JAX/PyTorch differentiation for its Python face-charge kernel. Freeze it as an external solver/differentiation arm rather than duplicate its kernels. Compare solver residuals and gradient/reference coverage independently from encoder system accuracy.

The next step up the competitive stack is **decision assurance**: propagate physically bounded field and sensor transfer through distinct phase and amplitude requirements, certify unsampled spatial cells, and test the final result against sealed angle/field observations. This remains conditional until the sensor, reference and physical bounds are independently validated.

Useful external methods: RLS MBD01 Issue 15, TI TMAG5111, Renishaw RESOLUTE/REXA30, Magpylib 0.15.0, and EURAMET Calibration Guide No. 23. Exact source URLs and comparison caveats are in `BENCHMARK_PROTOCOL_80MM.md`.
