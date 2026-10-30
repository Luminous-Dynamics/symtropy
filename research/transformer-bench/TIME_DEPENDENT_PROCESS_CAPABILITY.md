# Time-dependent manufacturing process capability diagnostic

**Status:** research diagnostic with synthetic fixtures. It is not an ISO 22514-2 implementation, statistical-control certificate, product acceptance rule, or release authority.

## Why a time-dependent metric

A pooled capability index collapses all samples into one mean and one spread. If the mean or dispersion changes between tooling, supplier, shift, machine, operator, recipe or material-lot windows, a single pooled index can conceal the mechanism or blur distinct regimes.

ISO 22514-2:2026 explicitly covers capability/performance estimates for time-dependent process models, including processes not in statistical control and multiple distribution types. This implementation is deliberately narrower: it exposes declared time windows, pooled descriptive Cpk, worst-window guarded Cpk, window-mean range and max/min window standard-deviation ratio. It is a benchmark baseline, not an attempt to reproduce all eight ISO distribution cases.

JCGM 106:2012 emphasizes that conformity risk depends on both the production process and the measuring system, and describes guard bands as a way to manage incorrect conformity decisions. The diagnostic therefore checks calibration identity, validity at measurement time and maximum uncertainty before including a point. It reports a guarded measurement exception when the point's expanded measurement interval crosses a specification limit.

## Input contract

Run:

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin time-capability -- \
  research/transformer-bench/fixtures/time-capability-stable-synthetic.csv \
  research/transformer-bench/fixtures/time-capability-spec-synthetic.csv \
  research/transformer-bench/fixtures/time-capability-calibration-synthetic.csv \
  /tmp/time-capability.csv
```

Samples have stable product/component/lot/station/characteristic identity, a declared window ID, unique sample ID, canonical UTC measurement timestamp, observed value, standard uncertainty, instrument ID and calibration ID. Specifications bind characteristic and unit to limits, coverage factor, maximum standard uncertainty, calibration class, minimum samples per window, maximum allowed window-mean range fraction and maximum/minimum window sigma ratio. Calibration evidence must match instrument and class and be current at the timestamp.

## Calculation

For a valid window (w), the diagnostic computes sample mean (ar{x}_w) and sample standard deviation (s_w) from the observed values. It intentionally does **not** subtract metrology variance: without a validated measurement-variance decomposition, observed variation is retained as total variation.

The guarded specification limits use the declared maximum standard uncertainty (u_{max}):

[
LSL_g = LSL + k u_{max}, qquad USL_g = USL - k u_{max}.
]

The within-window guarded descriptive index is:

[
Cpk_{g,w} = rac{min(USL_g-ar{x}_w,; ar{x}_w-LSL_g)}{3s_w}.
]

The pooled index uses all valid observed values, while the worst-window index is the minimum of the eligible window indices. The diagnostic separately reports the range of window means divided by specification width, and the maximum/minimum within-window standard-deviation ratio.

A window is not indexed unless it meets the controlled minimum sample count and has non-zero within-window sample standard deviation. A report without drift or variance alerts is still labelled `REVIEW_REQUIRED_PROCESS_STABILITY_NOT_ESTABLISHED`: no formal stability determination is inferred.

## Required interpretation

- `pooled_guarded_cpk` is descriptive only.
- `worst_window_guarded_cpk` is the conservative window comparison.
- `drift_warning` identifies a window-mean range beyond the declared threshold.
- `variance_shift_warning` identifies a window sigma ratio beyond the declared threshold.
- `invalid_metrology_records` excludes missing/expired/suspended/revoked or mismatched calibration and measurement uncertainty above the declared ceiling.
- `guarded_exceptions` counts samples whose expanded uncertainty intervals cross the raw specification limits.
- No warning does not establish statistical control, normality, independence, process capability, or product conformity.

The synthetic fixtures use four points per window only to keep the executable examples small. A real pilot must choose a sample plan using the applicable quality plan, process dynamics and measurement-system analysis; do not reuse the fixture's four-point minimum as a production threshold. Before reporting capability, assess measurement-system repeatability/reproducibility, bias, linearity, stability, autocorrelation and the appropriate distributional model. The current tool is not a substitute for Gage R&R or time-dependent control charts.

## Synthetic challenge set

- `time-capability-stable-synthetic.csv`: low window-mean range and similar within-window dispersion; diagnostic remains review-only.
- `time-capability-mean-drift-synthetic.csv`: monotonic window-mean shift; must trip the mean-drift diagnostic even though the pooled index is reportable.
- `time-capability-variance-shift-synthetic.csv`: similar window means but materially different within-window dispersion; must trip the variance-shift diagnostic.
- `time-capability-calibration-expired-negative.csv`: expired instrument calibration; all records are excluded from capability estimates.

These are synthetic software fixtures, not transformer measurements, process capability results or standards tolerances.

## Advancement benchmarks

For a real pilot, compare pooled-only decision support, windowed worst-case diagnostics and the applicable ISO 22514-2:2026 time-dependent method over held-out production campaigns. Report:
- false-negative drift and variance-shift detections against independently labelled control regimes;
- calibration and measurement-system exclusion rates;
- pooled vs worst-window capability differences;
- uncertainty and sample sufficiency by window/lot/station;
- scrap mass, rework hours, yield and energy per accepted unit;
- bottleneck cycle time and setup/queue time;
- no deterioration in critical test failures, safety escapes or field reliability.

Use independent lots/campaigns as the statistical units. The next iterations should add the relevant non-normal distribution families, autocorrelation-aware intervals, confidence intervals clustered by lot/campaign, measurement-system analysis, and a change-point/control-chart benchmark. Any acceptance or process adjustment remains under the approved quality plan and accountable manufacturing personnel.

## References

- ISO 22514-2:2026, *Statistical methods in process management — Capability and performance — Part 2: Process capability and performance of time-dependent process models*: https://www.iso.org/standard/88883.html
- JCGM 106:2012, *Evaluation of measurement data — The role of measurement uncertainty in conformity assessment*: https://www.bipm.org/en/doi/10.59161/jcgm106-2012
- NISTIR 8620 (2026), *Digital Twins Workshops Summary Report*: https://www.nist.gov/publications/digital-twins-workshops-summary-report
