# TransformerBench v0.1 — lifecycle loss screening

**Status:** research starter / analytic screening only. Not a transformer design tool, standards checker, digital-twin qualification, or safety approval.

This standalone Rust binary compares declared candidate loss data against explicit full-year operating scenarios. It is intentionally outside Symtropy's product workspace until the experiment has been independently reviewed and qualified.

## Scope and physics boundary

For each interval, the harness uses the first-order screening approximation:

P_loss = P0 + Pk × mean_square_current_pu2

P0 is declared no-load loss in watts, Pk is declared rated-load loss in watts, and `mean_square_current_pu2` is the time-average of squared per-unit current over that interval, measured relative to the reference rated current. It is **not** the square of the arithmetic mean current. Because E[I²] ≥ E[I]², reducing a varying interval to its mean current and then squaring can understate load losses and can change candidate rankings. The input contract therefore requires both `current_pu_mean` and `mean_square_current_pu2`; constant-current intervals set the latter to the former squared. The harness checks the moment inequality but cannot reconstruct or verify the underlying current trace. Interval energy is P_loss × hours / 1000 kWh. Interval loss cost multiplies that energy by the supplied tariff. It calculates expected values over declared scenario weights and a worst-case value across those scenarios. Lifetime loss cost is the NPV of repeating each scenario's annual tariff/current profile for the configured life with a constant real discount rate.

This is a screening approximation only. It does not model temperature-dependent resistance, harmonics, stray losses, saturation, inrush, frequency/voltage variation, insulation fields, partial discharge, hot spots, oil flow, short-circuit forces, tank behavior, sound, ageing, fault probability, or manufacturing tolerances. It cannot establish IEC/SANS compliance. Declared P0 and Pk must have a named/versioned source and matching reference conditions before engineering use.

## Run

No third-party Rust crates are required. From this directory, run:

cargo test --locked

cargo run --release -- fixtures/candidates.csv fixtures/scenario_profiles.csv fixtures/required_gates.csv fixtures/gate_records.csv /tmp/transformer-report.csv 30 0.08

Optional arguments are lifetime years (default 30) and real discount rate (default 0.08). Candidate capital cost and tariff must use the same currency unit. The tariff must be stated in the same real-price basis as the discount rate.

Fixtures are synthetic and are not based on any commercial transformer. The gate-record fixture contains no passing evidence, so candidates intentionally remain INCOMPLETE and none appears on the manifest-complete frontier. That is expected fail-closed behavior.

## Input contracts

### candidates.csv

Exact header: candidate_id,capex,p0_w,pk_w. Numeric values must be finite and non-negative; candidate IDs must be unique. P0 and Pk are declared inputs, not independently verified outcomes by virtue of being accepted by this binary.

### scenario_profiles.csv

Exact header: scenario_id,weight,interval_id,hours,current_pu_mean,mean_square_current_pu2,price_per_kwh. Each scenario must cover 8,760 hours, scenario weights must be consistent per scenario and sum to 1, and interval IDs must be unique within each scenario. Weights are in (0,1]; durations are positive; current moments and price are non-negative and finite; the second moment must be at least the squared mean within a declared numerical tolerance. This verifies only row-level IDs, moments and annual duration totals. It does not verify timestamp coverage, interval overlap, daylight-saving-time handling, meter quality, calibration, or representativeness of the tariff/profile. Mean current alone is insufficient: provide `mean_square_current_pu2` from the underlying trace before aggregation; do not fill it with the square of a varying interval's average unless the interval truly has constant current.

### required_gates.csv

Exact header: gate_id. Every listed gate is required for a candidate to enter the manifest-complete frontier. The applicable gate set must be selected for the exact transformer/application; the binary does not define applicable standards.

### gate_records.csv

Exact header: candidate_id,gate_id,status,evidence_sha256. Status is exactly PASS, FAIL, or UNKNOWN. A PASS counts toward manifest completeness only if its digest is exactly 64 hexadecimal characters. Missing records and UNKNOWN remain incomplete. Any required FAIL produces REJECTED. Duplicate records, unknown candidates/gates, and malformed non-empty digests are errors.

**Critical limitation:** the program checks only manifest shape and digest syntax. It does not read or hash the referenced artifact, authenticate its producer, prove a test ran, inspect the underlying report, or establish that PASS is justified. MANIFEST_COMPLETE means only that declared gate rows meet this input contract; it does not mean safe, qualified, certified, or ready to manufacture.

## Report semantics

- economic_pareto: nondominated on capital cost, expected NPV loss cost, and worst-case NPV loss cost, considering all candidates regardless of gate status. This is economics-only diagnostic output.
- manifest_complete_pareto: nondominated on the same objectives among candidates whose required-gate manifest is complete. This is not an independently verified feasible set or engineering approval.
- expected_lifecycle_cost: capital cost plus weighted expected NPV loss cost.
- worst_case_lifecycle_cost: capital cost plus the maximum NPV loss cost among the declared scenarios.

Report rows are sorted by candidate ID. Pareto decisions use full-precision values before six-decimal formatting. Record input/profile/report hashes, exact arguments, Rust toolchain, and source revision externally for each run (for example with sha256sum).

## Hand-checkable equation

For P0 = 1,000 W, Pk = 4,000 W, mean current = 0.5 pu, mean-square current = 0.25 pu², and 8,760 hours: P_loss = 1,000 + 4,000 × 0.25 = 2,000 W, so annual loss energy is 17,520 kWh. A 50/50 current trace at 0 and 1.0 pu has the same mean current but mean-square current 0.5 pu²; in the P0 = 0 case it has 17,520 kWh of load loss versus 8,760 kWh for constant 0.5 pu current. This fixture is a model-arithmetic example, not field evidence.

Unit tests cover the arithmetic, second-moment inequality, mean-vs-mean-square loss underestimation, interval splitting invariance, scenario weights and annual coverage, malformed CSV, non-finite values, missing/failed gates, digest syntax, Pareto dominance, and exclusion of incomplete manifests from the eligible frontier.

## Next qualification steps

1. Replace synthetic fixtures with a named incumbent transformer and traceable loss data including test conditions; preserve source evidence immutably and hash it.
2. Add timestamp-aware profile verification, temperature/harmonic-adjusted losses, and uncertainty only after reference models/test cases are defined.
3. Reuse Symtropy's existing thermal primitives; add FEM baselines with pinned solver versions and mesh/convergence checks rather than cloning another thermal model.
4. Expand the loss model to temperature-corrected load losses, harmonic/stray-loss components and transformer thermal dynamics only after their reference data and verification oracles are defined.
5. Implement separate independent receipt validators. The candidate generator and report must never certify their own evidence.
6. Benchmark manual/incumbent design, deterministic grid or space-filling sweep, conventional constrained multi-objective optimization, and Symthaea-guided search under the same candidate budget and frozen holdouts.
7. Only after held-out manufacturer/lab data qualify prediction error should results nominate a prototype for competent manufacture and formal testing.

## Surrogate evaluation CLI

A second binary evaluates a frozen reference/prediction table and reports error metrics for the overall set, plus geometry-family, material, frequency, excitation-class and exact operating-domain strata. Candidate-ranking metrics are only reported where every row has the same frequency and excitation class; ranking designs across incomparable operating domains is suppressed.

Example:

```sh
cargo run --release --manifest-path research/transformer-bench/Cargo.toml --bin surrogate-eval -- \
  research/transformer-bench/fixtures/surrogate_predictions_synthetic.csv \
  /tmp/transformer-surrogate-report.csv 3000 3000 2
```

Exact input header:

`candidate_id,geometry_family,material_id,frequency_hz,excitation_class,reference_core_loss_w,predicted_core_loss_w,prediction_lower_w,prediction_upper_w,model_fea_queries`

The row-level `model_fea_queries` value must be constant: it is the total costly FE calls attributed to that model/search run, including training, shortlist checks, failed/retried jobs and mesh refinements. The third CLI argument is the candidate-universe size. The fourth is `full_pool_fea_queries`: the total calls required to evaluate every candidate with the same pinned solver and convergence/mesh-refinement protocol. The reported FEA-call reduction is `1 - model_fea_queries / full_pool_fea_queries`, not `1 - shortlist_size / candidate_count`. This baseline must include comparable full-pool retries/refinements, and the model count must include training, calibration, shortlist verification, failed/retried jobs and mesh refinements. The reported values are still a caller-declared ledger, not cryptographically audited accounting; exact solver receipts and query logs remain required. Held-out reference values may be produced by an evaluator's independent oracle and are not counted as model-initiated calls.

Metrics include mean/median/P95/maximum absolute relative error, Spearman rank correlation, top-k recall of the lowest-loss candidates, regret of the predicted best candidate against the best reference candidate, prediction-interval coverage, and mean interval width relative to reference loss. The accepted excitation classes are SINUSOIDAL, DISTORTED and OTHER_DECLARED; frequency must be a finite positive number in Hz. The evaluator rejects duplicate candidate IDs, inconsistent query counts, non-finite or physically nonsensical inputs, invalid prediction intervals, and derived non-finite metrics. Results are stratified by geometry family, material, frequency, excitation class and their combined operating domain to expose transfer failures hidden by aggregate scores.

`fixtures/surrogate_predictions_synthetic.csv` is deliberately synthetic and tests the metric implementation only. `fixtures/surrogate_predictions_domains_synthetic.csv` tests pooling safeguards across 50 Hz sinusoidal, 60 Hz sinusoidal and 50 Hz distorted-excitation domains. Neither fixture replicates the published paper, represents FEA output, or may be cited as a model result. The evaluator assumes the split and source rows were frozen before model tuning; it cannot infer or authenticate data leakage from a CSV alone.

Global deployment contract: see [GLOBAL_APPLICABILITY.md](GLOBAL_APPLICABILITY.md) and [PWR-TX-GLOBAL-001 #2872](https://github.com/Luminous-Dynamics/symtropy/issues/2872). This is a global architecture/schema plan; no regional profile is yet certified.

## State-of-the-art comparator (not a result from this repository)

Zou et al., “A geometry-aware surrogate model for fast transformer core loss prediction and design optimization,” Electric Power Systems Research, DOI [10.1016/j.epsr.2026.113713](https://doi.org/10.1016/j.epsr.2026.113713), report one screening case that compressed 3,000 candidate schemes to 8 FE-verified schemes and reports a 99.7% reduction in FE calls, with 2.6% mean and 4.6% maximum relative error on shortlisted schemes. The simple 8/3,000 ratio is about 99.73% candidate compression, but our reproduction must match the paper's FEA-call accounting and separately expose training, calibration, shortlist verification, retries, failed runs and mesh refinements instead of assuming the shortlist ratio alone establishes total cost savings. Their stated scope is GOES, 50 Hz sinusoidal steady state, and a three-phase three-limb transformer. Their results also report that comparative gains are conditional on material and validation split; random-split performance must not be treated as proof of transfer to unseen geometry families. Source: https://www.sciencedirect.com/science/article/abs/pii/S0378779626010060.

Follow-on benchmark specification: [PWR-TX-SOTA-001 #2859](https://github.com/Luminous-Dynamics/symtropy/issues/2859). Its 3% mean / 5% maximum screening targets and 99% FE-call reduction target are aspirational replication/extension gates, not this harness's results. A faithful reproduction requires compatible public candidate data, FEA settings, and geometry/material splits. Substitute synthetic examples cannot reproduce or validate the published result.

## References

- Li et al. (2025), “A Review of Data-Driven Models for Electromagnetic Devices Design and Analysis,” IEEE Access 13, 142161–142185, DOI 10.1109/ACCESS.2025.3591965. Covers data-driven models for electromagnetic devices including transformers and FEA acceleration; it does not validate this harness.
- IEC 60076-1 — General: https://webstore.iec.ch/en/publication/588
- IEC 60076-2 — Temperature rise for liquid-immersed transformers: https://webstore.iec.ch/en/publication/599
- IEC 60076-3 — Insulation levels and dielectric tests: https://webstore.iec.ch/en/publication/601
- IEC 60076-5 — Ability to withstand short circuit: https://webstore.iec.ch/en/publication/603
- SANS 780:2019 catalogue copy: https://irp.cdn-website.com/4e0aecb6/files/uploaded/SANS780_2019_Ed5.pdf. Confirm the actual current edition and exact utility/customer specification for any deployment; old tender documents do not prove current applicability.

## License

MIT OR Apache-2.0.
