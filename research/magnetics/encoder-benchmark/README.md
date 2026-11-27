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
