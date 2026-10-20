# EV factory intervention-ranking benchmark v0.1

**Benchmark:** `EV-FACTORY-INTERVENTION-BENCH-001`  
**Input:** output report from the deterministic synthetic DES  
**Status:** counterfactual decision-ranking smoke test, not plant truth

## Paradigm shift: classify constraints and rank interventions separately

A station can be busy without being the best place to spend capital. This benchmark consumes a single, versioned simulation report and keeps two kinds of evidence separate:

- **Diagnostic signals** come only from the baseline scenario: utilization (busy fraction), a clearly labeled busy-plus-down proxy, a flow-pressure heuristic using busy/blocked/starved/down fractions, and mean changeover count.
- **Counterfactual value** comes only from each station's paired added-server intervention output distribution. The point-estimate oracle selects the intervention with the highest mean paired good-output gain in that *same synthetic model*.

No diagnostic method reads intervention-gain columns to calculate its station score. The evaluator verifies the source simulation's fixed seed list and invariants, uses the SHA-256 digest of the exact input report, emits top-1/paired-output regret, Spearman and Kendall rank agreement, and abstains on ties or if the top two oracle candidates do not pass a simple individual-confidence-interval separation screen.

## Important statistical nuance

The best-vs-runner-up non-overlap screen is descriptive only. It is **not** a formal paired test of the difference between intervention effects. Since all interventions share per-seed inputs, the next statistics upgrade should compute a per-seed *difference between two intervention deltas* and its own confidence interval. Until then the report explicitly says not to make a firm intervention recommendation when the screen does not separate the two best point estimates.

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

The output carries the input report SHA, number of seeds and dispatch policy, one feature record per station, point-estimate oracle ranking, a top-two interval-screen flag, method rank correlations and intervention regret. Production release is hard-false.

## Metrics

- **Top-1 agreement:** did a baseline diagnostic's selected station match the highest mean paired gain from adding a server at that station?
- **Absolute regret:** oracle-best mean paired throughput gain minus the mean gain of the selected intervention, clamped at zero for numerical safety.
- **Relative regret:** absolute regret divided by the magnitude of the oracle-best mean gain when it is nonzero.
- **Spearman and Kendall τ-b:** agreement between the candidate ranking from a baseline signal and the point-estimate oracle ranking.
- **Abstention:** ties abstain; overlapping / missing top intervention intervals trigger a no-recommendation disposition.
- **Model-conditioned effect:** all these outputs describe one declared simulator, not a physical factory.

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
- The current confidence gate compares two individual confidence intervals, which is not equivalent to a paired confidence interval for the difference between interventions. Add joint paired-difference summaries next.
- A real SOTA comparison requires faithful published algorithms, frozen synthetic family holdouts, and eventually rights-cleared measured traces with independent temporal validation and ground truth.
- No MES/PLC/plant data was gathered; no suppliers, integrators, manufacturers, landlords or agencies were contacted; no quotes requested.

The evaluator cannot authorize production release, safety approval, quality release or machinery purchases.
