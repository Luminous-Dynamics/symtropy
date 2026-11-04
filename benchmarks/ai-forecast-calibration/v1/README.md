# Prequential online-calibration known-truth fixtures

These fixtures pin the deterministic bounded ACI replay arithmetic implemented by `symtropy-energy-systems::evaluate_online_adaptive_calibration`.

**Evidence class:** synthetic known truth only. No values originate from an operating data centre, utility meter, weather forecast, or production workload.

## Contract

- Calibration pool: 99 synthetic block-max scores from 0.1 to 9.9.
- Target joint coverage: 950,000 ppm (95%).
- Initial miscoverage: 50,000 ppm (5%).
- ACI step size: 10,000 ppm (gamma = 0.01).
- Default alpha bounds: 10,000 to 300,000 ppm.
- Quantile: exact split-conformal finite-sample rank using integer ceiling arithmetic.

## Vectors

- `finite_quantile`: 99 calibration blocks, minimum finite alpha 10,000 ppm, selected rank 95, score 9.5.
- `miss_then_recovery`: one joint miss reduces alpha to 40,500 ppm and raises the next quantile rank to 96; a covered second block moves alpha to 41,000 ppm.
- `lower_bound_saturation`: five sustained misses reach the declared lower alpha bound; the final receipt must say it was clamped.
- `upper_bound_saturation`: covered blocks move alpha to the declared upper bound; the final receipt must say it was clamped.
- `delayed_feedback_negative_control`: next forecast issuance before the previous block's outcome is available must be rejected.

## Reproduction

Construct 99 calibration blocks whose block-max normalized absolute errors match the score list, then construct the evaluation blocks in each vector. Every block must carry a complete declared channel × horizon grid with unique sample identities and forecast, realization and normalization-scale evidence IDs. Forecast issue times must precede their target windows. For online replay, each evaluation forecast after the first must be issued no earlier than the preceding evaluation window's end.

These score-level fixtures validate arithmetic and event sequencing only. They do not validate split-conformal exchangeability, online coverage, regime robustness, interval sharpness on real traces, a forecaster, optimizer, or physical energy system.
