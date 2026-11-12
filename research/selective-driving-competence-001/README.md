# SELECTIVE-DRIVING-COMPETENCE-001

**Status:** offline protocol and Rust reference evaluator; no driving model has been assessed  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4938  
**Related capability frontier:** https://github.com/Luminous-Dynamics/symtropy/issues/4639  
**Related renderer / camera input:** https://github.com/Luminous-Dynamics/symtropy/pull/4616  
**Related Symthaea camera bridge:** https://github.com/Luminous-Dynamics/symthaea/pull/11480  
**Related branch evidence auditor:** https://github.com/Luminous-Dynamics/symtropy/pull/4556

## Paradigm shift

A driving score alone does not answer whether a policy knows when it is unreliable. This protocol evaluates the **risk-coverage frontier**: how much autonomy coverage can a policy retain at a declared observed-failure rate, and how does the policy's predicted failure probability rank actual failures?

It also measures calibration, worst-family outcomes, actual abstention behaviour, late abstentions, fallback completion and the human-intervention cost. It reports these separately; no weighted "safety score" is invented.

The evaluator is offline and has no vehicle, actuator, network or host-control interface. It does not approve actions or provide a minimal-risk manoeuvre.

## Current industry/benchmark context (12 Nov 2026)

| Comparator | Published result | Interpretation |
|---|---:|---|
| Bench2Drive v0.0.3 self-reported table — BridgeDrive (LEAD) | 96.34 DS / 89.25 SR | Public leaderboard comparator; data, experts, training and compute vary |
| Bench2Drive v0.0.3 self-reported table — TFv6 | 95.28 DS / 86.81 SR | Non-privileged listed comparator |
| Bench2Drive v0.0.3 privileged section — PDM-Lite | 97.02 DS / 92.27 SR | Ground-truth/privileged results are not a fair vision-only matched comparator |
| NAVSIM v2 end-to-end test — NTR (2026.05) | 90.9 EPDMS | Native NAVSIM v2 end-to-end metric; not comparable to Bench2Drive DS |
| NVIDIA Alpamayo 2 Edge | 2B params; reported 160–190 ms pixel-to-trajectory on DRIVE AGX Thor; reported 48.9% uplift from closed-loop RL post-training | Industry technical-blog claims; not independently reproduced here |
| Waymo Safety Impact dashboard through June 2026 | 271.3M rider-only miles; company-reported 95% fewer serious-injury-or-worse crashes, 82% fewer injury-causing crashes | Operational retrospective safety evidence; distinct from its formal safety case |

Sources:
- Bench2Drive leaderboard: https://github.com/autonomousvision/Bench2Drive-Leaderboard
- NAVSIM v2 end-to-end test: https://www.sota2.com/research/sota/end-to-end-driving-on-navsim-v2-test
- NVIDIA Alpamayo 2 Edge, 10 Nov 2026: https://developer.nvidia.com/blog/running-av-reasoning-models-within-onboard-compute-and-latency-limits/
- Waymo safety impact: https://waymo.com/safety/impact/
- Sensor-centric uncertainty and integrity survey (Sensors, June 2026): https://pmc.ncbi.nlm.nih.gov/articles/PMC13306937/

These results are distinct protocols and evidence classes. Do not pool or rank them as though they form one leaderboard.

## Protocol

Each decision record has:
- a unique decision and episode ID plus a scenario-family label;
- prediction time and outcome time;
- a predicted failure probability in [0, 1];
- an observed binary failure label and observed normalized loss in [0, 1];
- explicit policy-visible input fields with their availability timestamps;
- actual abstention state and, when abstained, a decision deadline and observed fallback outcome/duration;
- measured latency, energy and memory where available.

The evaluator rejects:
- missing or censored outcomes (no zero imputation or silent exclusion);
- duplicate or unexpected decision IDs and missing expected episodes;
- evaluator-only or future policy inputs;
- outcome timestamps before the prediction;
- abstention records without explicit fallback outcome and duration;
- invalid probabilities, losses, resource values or inconsistent fields.

The policy-input availability check is a declared-record integrity test. It does not authenticate upstream timestamp claims or prove the record source.

## Metrics

1. **Risk-coverage curve:** sort by predicted failure probability ascending. Every equal-score group enters together; ties are never split by decision ID.
2. **AURC:** right-continuous step integral over the coverage increments, computed separately for binary observed failure rate and mean observed loss.
3. **Fixed operating points:** observed failure rate at predeclared target coverage, and maximum coverage under a predeclared observed-failure-rate limit. Tied scores may overshoot the target coverage; the report shows achieved coverage.
4. **Calibration:** Brier score, log loss, fixed-width-bin ECE and maximum bin gap.
5. **Worst-family exposure:** per-family accepted coverage and accepted failure/loss metrics; an overall average cannot hide a weak family.
6. **Actual abstention:** actual autonomy coverage, accepted failure/loss, fallback success/failure, late abstentions, fallback duration and human interventions.
7. **Resources:** average measured latency/energy/memory and the number of observations available for each. Missing measurements remain null, never zero.

AURC is a ranking metric, not a deployment-risk guarantee. ECE depends on binning. A favourable point estimate without episode-level uncertainty intervals is not evidence of statistical superiority. This tool does not implement bootstrap confidence intervals; the analysis protocol requires episode-level resampling before research claims.

## Build and run

Rust 1.96.0 is pinned in `rust-toolchain.toml`.

```sh
cargo fmt --manifest-path research/selective-driving-competence-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/selective-driving-competence-001/Cargo.toml
cargo clippy --locked --manifest-path research/selective-driving-competence-001/Cargo.toml --all-targets --all-features -- -D warnings
cargo run --release --locked \
  --manifest-path research/selective-driving-competence-001/Cargo.toml -- \
  research/selective-driving-competence-001/examples/selective-driving-demo-v0_1.json \
  evaluation-report.json
```

The CLI accepts one JSON input file and writes a machine-readable report. The input is limited to 256 MiB. Exit code 0 means the declared records passed structural validation and metrics were computed; exit code 2 indicates input/evaluation/invocation failure. It is not an approval to drive.

A six-decision, three-family example with one documented abstention/fallback is provided at `examples/selective-driving-demo-v0_1.json` and parsed/evaluated by the unit suite.

Input schema: `selective-driving-evaluation-v0_1.schema.json`  
Protocol/comparator manifest: `manifest-v0_1.json`

## Prospective target ladder (not measured)

- exact agreement on analytic risk/coverage fixtures and 100% denominator reconciliation;
- rejection of every declared leakage, missing-outcome and fallback-evidence negative control;
- risk-coverage and calibration gains versus matched random-score, inverted-score and calibrated baseline arms;
- improved useful coverage under a predeclared risk limit without worst-family regression;
- measured abstention timeliness and fallback recoverability at fixed latency/energy budgets;
- later, independently governed HIL and closed-course evidence.

No driving model has been benchmarked by this crate. Targets are prospective.

## Claim ceiling

A successful report establishes only that the exact finite records passed this evaluator's structural checks and that the described statistics were computed. It does not establish calibrated uncertainty outside the sampled distribution, a safe runtime monitor, physical fallback performance, lower real-world crash rates, roadworthiness, regulatory approval or SAE L4/L5 capability.
