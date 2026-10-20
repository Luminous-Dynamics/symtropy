# EV bottleneck diagnostic scoring benchmark v0.1

**Benchmark:** `EV-FACTORY-BENCH-001`  
**Status:** synthetic harness and hand-authored counterexamples; not an industry-comparable result  
**Implementation:** `tools/ev-passenger-cost-frontier/src/bin/ev_factory_bottleneck_benchmark.rs`

## Purpose

The question is not "which station is busiest?" It is "which resource is constraining saleable output in this time window, what intervention would actually relieve that constraint, and how often does the diagnosis fail when buffers, starvation, blocking, rework or the bottleneck shift change?"

This first version provides a versioned scoring harness over eight synthetic diagnostic challenge cases. Each case declares a construction-time ground-truth label and station-window features; every baseline makes a single-label prediction, or explicitly abstains on a tie. The output reports exact accuracy, macro-F1, class-level precision/recall/F1 and coverage.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_bottleneck_benchmark -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_bottleneck_diagnostic_benchmark_v0_1.json
```

Methods implemented in this harness:
- **Utilization max:** predict the station with highest supplied utilization fraction.
- **Active-period max:** predict the station with highest supplied active-period fraction.
- **Inter-departure CV max heuristic:** predict the station with highest supplied inter-departure coefficient-of-variation feature. This is *not* a reproduction of the published IDTV algorithm.
- **Flow-pressure heuristic:** score each station as `utilization + 0.5 × blocked − starved − down` and choose the highest. This is an internal candidate, not an established standard.
- **Predicted marginal good-output gain max:** choose the highest declared predicted gain signal. In v0.1 that field is also synthetic and hand-authored; it is not the existing DES's independently measured treatment effect.

Any tie within tolerance becomes abstention rather than depending on station ordering. Abstentions remain misses for exact accuracy and false negatives for the labeled class. The score is reported with coverage so a method cannot hide mistakes behind abstention.

## Public research comparator

A September 2026 automotive-parts plant study used expert-labeled bottleneck windows to compare Active Period (AP), ABAP (Algorithm Based on Active Period Theory), Utilization, Turning Point and Inter-Departure Time Variance (IDTV). It reported F1 0.89 and accuracy 80% for ABAP on its own five-session dataset; utilization's reported F1 was 0.73. Those figures are **specific to that study's boundary, labels and data** and are not universal SOTA: https://doi.org/10.3233/ATDE260693.

This harness does not claim to reproduce AP, ABAP, TP or IDTV. Reproducing those methods faithfully requires implementing their published definitions and evaluation protocol. The CV-max baseline is deliberately labeled a heuristic rather than rebranded as IDTV.

WEFTSIM, a 2026 process-mining-based factory simulation paper, reports around 80% activity-level coverage and 90% trace similarity for its own evaluation: https://doi.org/10.1016/j.dajour.2026.100729. Those scores measure a different task (model derivation/trace similarity), and must not be compared directly with station-label accuracy or macro-F1. Trace validity, model fidelity, bottleneck classification and intervention effect are separate capabilities.

NIST's 2026 manufacturing digital-twin research prioritizes interoperability and VVUQ, including uncertainty and measurement practice: https://doi.org/10.6028/NIST.IR.8620 and https://doi.org/10.6028/NIST.AMS.100-85. A model's reproducibility is necessary but insufficient for trust.

## Next benchmark maturity steps

1. Replace hand-authored features with output generated from the DES across a declared grid of process cycle-time, finite-buffer, quality, supplier and failure conditions. Store the generating parameters and source digest beside each scenario.
2. Define ground truth by controlled counterfactual runs with paired exogenous inputs and repeated replications, not by assuming one utilization feature equals the constraint. Keep the prediction window separate from the intervention outcome window.
3. Add sensitivity cases for simultaneous/multiple bottlenecks, supplier/energy constraints, no identifiable constraint, stochastic ties, changing variants, time-varying constraints and distribution shift.
4. Reproduce fully specified published baselines; use frozen scenario holdouts. Report per-class precision/recall/F1, macro-F1, calibration/abstention, delay-to-detection, false intervention rate and top-k/regret on measured output effects.
5. Only compare with a published value when system boundary, event windows, ground truth, class distribution, data splitting and metric definition match. For real logs, require rights-cleared data and an independent temporal holdout; no supplier or plant contact is required for this synthetic step.

## Claim ceiling and safety

The eight cases are hand-authored synthetic counterexamples to exercise scoring and regression logic. Their labels are true only by construction. The values are not observations from an industrial process and the predicted gain signal is not a proven causal effect. No current method score in this fixture is evidence of real-world superiority or SOTA.

The evaluator cannot authorize factory release or machinery purchases. Calibration, production-quality validation, product safety, electrical/battery safeguards and jurisdiction-specific compliance remain separate gates. No supplier, integrator, plant, landlord or agency was contacted; no quote was requested.
