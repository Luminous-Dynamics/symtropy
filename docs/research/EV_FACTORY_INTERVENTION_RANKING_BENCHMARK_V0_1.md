# EV factory intervention-ranking benchmark v0.1

**Benchmark:** `EV-FACTORY-INTERVENTION-BENCH-001`  
**Input:** output report from the deterministic synthetic DES  
**Status:** counterfactual decision-ranking smoke test, not plant truth

## Paradigm shift: classify constraints and rank interventions separately

A station can be busy without being the best place to spend capital. This benchmark consumes a single, versioned simulation report and keeps two kinds of evidence separate:

- **Diagnostic signals** come only from the baseline scenario: utilization (busy fraction), a clearly labeled busy-plus-down proxy, a flow-pressure heuristic using busy/blocked/starved/down fractions, and mean changeover count.
- **Counterfactual value** comes only from each station's paired added-server intervention output distribution. The point-estimate oracle selects the intervention with the highest mean paired good-output gain in that *same synthetic model*.

No diagnostic method reads intervention-gain columns to calculate its station score. The evaluator verifies the source simulation's fixed seed list and invariants, hashes the exact report bytes, emits top-1/paired-output regret, Spearman and Kendall rank agreement, and separately compares interventions using per-seed **effect-of-effects** samples.

For each pair of interventions A and B, with each gain computed against that seed's baseline, it calculates:

`d(seed) = [output_A(seed) - baseline(seed)] - [output_B(seed) - baseline(seed)]`.

It reports the mean, sample standard deviation, empirical quantiles, and a two-sided 95% Student-`t` interval over these paired differences. A declared internal practical-equivalence margin is used to distinguish clear advantages, practical equivalence, and inconclusive comparisons. A robust best candidate is reported only if the interval clears the margin in its favor against every other candidate; otherwise the result abstains.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml

cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_simulation -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_v0_1.json \
  > ev-factory-simulation-report.json
SOURCE_SHA256=$(sha256sum ev-factory-simulation-report.json | cut -d' ' -f1)
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_intervention_benchmark -- \
  ev-factory-simulation-report.json "$SOURCE_SHA256"
```

The output carries the exact input-report SHA, seed count and dispatch policy, one feature record per station, point-estimate oracle ranking, ten pairwise intervention comparisons for five candidates, rank correlations, intervention regret and a robust-best/abstain status. Production release is hard-false.

## Metrics

- **Top-1 agreement:** did a baseline diagnostic's selected station match the highest mean paired gain from adding a server at that station?
- **Absolute regret:** oracle-best mean paired throughput gain minus the mean gain of the selected intervention, clamped at zero for numerical safety.
- **Relative regret:** absolute regret divided by the magnitude of the oracle-best mean gain when it is nonzero.
- **Spearman and Kendall τ-b:** agreement between the candidate ranking from a baseline signal and the point-estimate oracle ranking.
- **Paired effect-of-effects:** the confidence interval for `gain_A(seed) - gain_B(seed)` preserves common-seed dependence; it is not inferred from overlap of marginal intervals.
- **Practical equivalence:** the current internal margin is 0.05 good units/hour. That is a synthetic evaluator default, not an industrial materiality standard; change only as a declared experiment parameter.
- **Empirical win fraction:** the share of seeds where A's advantage over B exceeds the margin; this is a descriptive finite-sample fraction, not a posterior probability.
- **Abstention:** no station is named robust-best unless it clears the paired interval margin against every other candidate; near ties stay unresolved.
- **Model-conditioned effect:** all outputs describe one declared simulator, not a physical factory.

The source is a simulator report built from 20 seeds, with paired common-random-stream draws. The evaluator does not attempt to re-fit the simulator, change the seed list, or look at field data.

## How this relates to published SOTA

The open-access September 2026 automotive-parts study compares AP, ABAP, utilization, Turning Point and Inter-Departure Time Variance against expert-labelled shop-floor windows. It reports ABAP F1 0.89 and accuracy 0.80 on five sessions, while utilization F1 was 0.73; TP and IDTV failed to detect bottlenecks in the reported buffered plant: https://doi.org/10.3233/ATDE260693.

Those metrics evaluate **bottleneck classification against expert labels**, not whether adding a server at the predicted station gives the best gain. We must not compare our top-1 or regret to that study's F1. The two tasks are complementary:
1. diagnostic classification against expert-labeled station windows;
2. intervention-value ranking against controlled paired outcomes;
3. actual causal intervention validation on permitted measured/pilot traces.

The earlier 8-case hand-authored scorer is only a smoke test; its perfect active-period / predicted-gain scores are not evidence of state-of-the-art. This evaluator uses generated paired output to define a model-conditional intervention oracle, while baseline diagnostic features remain separate.

NIST's 2026 manufacturing digital-twin workshop summary emphasizes verification, validation, quantified uncertainty, interoperability, cybersecurity and workforce readiness: https://doi.org/10.6028/NIST.IR.8620. A reproducible score is one necessary gate, not a trust certificate.

## Limits and next upgrade

- The oracle is exact only relative to this simulator's structural assumptions and one-server intervention catalog. It does not prove a real plant effect.
- Diagnostic formulas here are transparent heuristics. `busy_plus_down_proxy` is not a faithful implementation of published AP or ABAP; `flow_pressure_heuristic` and `changeover_burden` are internal baselines, not established standards.
- The paired intervals quantify only simulated replication variation. They do not include uncertainty in process parameters, distributions, model structure, calibration error or real-world drift.
- The default margin is a provisional threshold for this synthetic benchmark. It must be defined before evaluating any future measured dataset, and sensitivity-tested rather than optimized on holdout results.
- A real SOTA comparison requires faithful published algorithms, frozen synthetic family holdouts, and eventually rights-cleared measured traces with independent temporal validation and ground truth.
- No MES/PLC/plant data was gathered; no suppliers, integrators, manufacturers, landlords or agencies were contacted; no quotes requested.

The evaluator cannot authorize production release, safety approval, quality release or machinery purchases.
