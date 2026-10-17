# Space Economy Corpus Validator

This standalone Rust crate validates the **synthetic fixture corpus** used by the Space Economy Reference Benchmark Suite. It is intentionally outside the main workspace so it can be compiled and tested without pulling in the renderer, simulation engine, Holochain, or Symthaea runtime.

## Run

```sh
cargo fmt --manifest-path tools/space-economy-bench/Cargo.toml -- --check
cargo test --manifest-path tools/space-economy-bench/Cargo.toml
cargo run --quiet --manifest-path tools/space-economy-bench/Cargo.toml -- \
  docs/tech/space-economy-benchmark-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

The command writes a JSON receipt to stdout and diagnostics to stderr. It refuses to produce a receipt if the scenario set, suite/version, units, expected mass/cost relations, hard-gate fixture, or required service-floor relationships are invalid. The receipt hashes the **exact input bytes**, binds the supplied 40-character source commit, and reports per-scenario fixture-validation results. Direct dependency versions are pinned in Cargo.toml. The focused workflow generates a lockfile before testing, runs Cargo commands with --locked, adds its SHA-256 and pinned Rust toolchain to every receipt, and retains the lockfile alongside the receipts so the resolved transitive graph is inspectable.

## Claim ceiling

This command validates the integrity and arithmetic of a synthetic benchmark corpus. It does not execute a spaceflight dynamics model, lunar resource process, market engine, traffic-management system, or economic optimizer. Its `pass` value is not a claim of real-world cost savings, physical feasibility, flight readiness, or standards compliance.

The corpus is deliberately small and inspectable. The next engineering step is to run candidate planners against the same scenarios and compare their decisions to independent reference oracles without sharing the planner's ranking implementation with the verifier.

## Determinism

The receipt has no wall-clock timestamp. Given identical corpus bytes and source-commit input, its normalized JSON payload is deterministic. SHA-256 is calculated over the exact corpus bytes, including whitespace and line endings. Do not normalize the file before hashing.

## Value-of-information reference (v0.2 research slice)

The standalone file tools/space-economy-bench/examples/voi_reference.rs is a separate analytical fixture, not part of the frozen v0.1 six-scenario corpus. It uses integer basis-point likelihoods and exact rational payoff numerators to test a decision rule before irreversible investment.

Run:

```sh
cargo test --manifest-path tools/space-economy-bench/Cargo.toml --example voi_reference
cargo run --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example voi_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

With the synthetic prior P(rich)=0.40, build payoffs (+100 if rich, −150 if poor), import payoff 0, 90% sensitivity/specificity, and survey cost 10, the analytical reference gives expected value 0 without a survey, gross value 27 after a positive-survey-only build policy, net incremental survey value +17, and perfect-information value 40. A 50%/50% uninformative survey has zero information value and must not be purchased at positive cost.

These numbers are intentionally synthetic. They validate the value-of-information arithmetic and the stop/continue decision contract, not lunar geology, a survey instrument, or an actual ISRU investment. See Symtropy issue #1738: https://github.com/Luminous-Dynamics/symtropy/issues/1738.

## Correlated-failure reference (active research slice)

`tools/space-economy-bench/examples/common_cause_reference.rs` contrasts two relay topologies that both survive either single relay failure. In the synthetic low-cost topology, both relays depend on one power bus; failure of that bus removes all capacity. The costlier topology uses separately named power domains and survives each declared individual provider or controller fault. It never claims those domains are statistically independent.

Run:

```sh
cargo test --manifest-path tools/space-economy-bench/Cargo.toml --example common_cause_reference
cargo run --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example common_cause_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

The fixture uses demand 60, two 70-capacity relays, listed synthetic cost 65 for the shared-bus option and 85 for the separated-controller option. The point is not to estimate actual relay costs; it is to show why node count is not sufficient evidence of resilience. See [BENCH-SPACE-003](https://github.com/Luminous-Dynamics/symtropy/issues/1749).

The receipt reports named fault applicability, remaining service capacity and unmet demand. A fault not present in a topology is labelled `not_applicable`, never silently counted as a pass.

## Supplier concentration and resilience reference (active research slice)

The standalone file tools/space-economy-bench/examples/supplier_concentration_reference.rs computes the capacity-share Herfindahl-Hirschman Index (HHI) with exact integer arithmetic and compares supplier portfolios under each named single-provider failure.

The synthetic concentrated portfolio has shares 50/20/15/15, HHI 3,350, lifecycle cost 100 SCU, and minimum post-outage capacity 50. The balanced portfolio has shares 25/25/25/25, HHI 2,500, lifecycle cost 118 SCU, and minimum post-outage capacity 75. Demand is 70. Therefore, if N-1 service is a hard gate, the higher-cost balanced portfolio is the only feasible option in this fixture.

Run:

```sh
cargo test --manifest-path tools/space-economy-bench/Cargo.toml --example supplier_concentration_reference
cargo run --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example supplier_concentration_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

These are synthetic capacities and costs, not actual market shares or launch procurement forecasts. The benchmark assigns no outage probabilities. See [BENCH-SPACE-004](https://github.com/Luminous-Dynamics/symtropy/issues/1750).

## Orbital sustainability as a hard-constrained common-pool resource

The separate reference `tools/space-economy-bench/examples/orbital_externality_reference.rs` makes the externality explicit. It compares lifecycle cost plus end-of-life provision, a qualified-service floor, and a synthetic residual-burden budget. The order is hard gates first, then lifecycle cost and a Pareto frontier; a lower private cost cannot buy its way out of a failed burden gate.

The negative control claims 20 synthetic units of debris removal without verified evidence. It receives zero removal credit and remains infeasible. The two feasible options—`design_for_removal` (82 synthetic cost units, 13 residual burden units) and `service_and_reuse` (88 cost units, 8 burden units)—are both retained on the Pareto frontier rather than collapsed to one weighted-score winner. The cheap disposable fixture costs 60 units but leaves 32 burden units against a budget of 15, so it is rejected.

Run:

```sh
cargo test --locked --manifest-path tools/space-economy-bench/Cargo.toml --example orbital_externality_reference
cargo run --locked --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example orbital_externality_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

**Evidence boundary:** SOBU (synthetic orbital burden unit) is a fixture-only dimensionless unit, not a physical debris-risk measure, collision probability, standard, or regulator's threshold. This reference is not an orbital-dynamics simulation and must not be transferred to lunar orbit/surface decisions without a separate scope and independently justified model. The 2026 ESA Space Environment Report motivates this research direction—orbital environment health has worsened and ESA says active debris removal is needed alongside prevention—but does not calibrate these fixture numbers: https://www.esa.int/Space_Safety/Space_Debris/ESA_Space_Environment_Report_2026. OECD similarly identifies orbital sustainability, market concentration, and critical-input dependencies as distinct systemic risks: https://www.oecd.org/en/publications/the-space-economy-at-a-glance-2026_cbf9b240-en/full-report/managing-vulnerabilities-dependencies-and-sustainability-risks-of-the-space-economy_4d97ef6d.html.

## Verified industrial capability closure

The example `tools/space-economy-bench/examples/industrial_closure_reference.rs` separates the claim that a service is operating now from the stronger claim that its required capability graph can be sustained locally. It recursively evaluates dependencies and fails closed for missing capabilities, unverified processes, nonlocal operation, recurring imports, and dependency cycles.

Run:

```sh
cargo test --locked --manifest-path tools/space-economy-bench/Cargo.toml --example industrial_closure_reference
cargo run --locked --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example industrial_closure_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

The synthetic high-local-content plan claims 92% local content and lower lifecycle cost (95 SCU), and its target mine-service is reported operating now. It nevertheless fails local closure because metal refining is unverified and the sensor module has nonlocal/recurring-import dependencies. The closure-first plan claims 78% local content, costs 120 synthetic SCU, and passes the specified closure graph because each modeled node is verified, locally operated, and free of recurring imports. The difference is a software contract oracle—not evidence that lunar sensor-electronics production is physically or economically feasible.

The receipt intentionally reports local-content percentage, current operating status, evidence verification, and local closure separately. It contains named negative controls for missing dependencies and dependency cycles. A cycle blocker is the graph walk's explicit conservative failure, not a claim to compute a globally minimal cut set.

The closure definition is specific to the declared graph and horizon: every node must be operational, evidence-verified, locally operated, and free of declared recurring external inputs, with the same requirements applying recursively to dependencies. Real applications must separately define the service horizon, stockpiles, wear-out/replacement rates, energy/material budgets, and evidence quality. See [BENCH-SPACE-006](https://github.com/Luminous-Dynamics/symtropy/issues/1765).

## Time-horizon service viability under resupply shocks

The example `tools/space-economy-bench/examples/temporal_resilience_reference.rs` moves beyond a static closure graph to a small, deterministic 12-month service-availability contract. It includes nominal and delayed imported-spare delivery, a verified local-repair path, and an unverified-repair negative control.

Run:

```sh
cargo test --locked --manifest-path tools/space-economy-bench/Cargo.toml --example temporal_resilience_reference
cargo run --locked --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example temporal_resilience_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

With synthetic failure months 2, 6, and 10 and a 75% service-availability floor over 12 months, the nominal import-spares case delivers 9 service months and consumes 3 spares. Delayed arrivals at months 7 and 11 reduce service to 7 months (58.33%), with a 2-month maximum outage, and fail the floor. Verified local repair recovers by the next month after each of three failures and achieves 9 service months with zero externally sourced spares consumed. The unverified-repair claim receives no repair credit and delivers only one service month in its negative-control case.

The model uses explicit event semantics: deliveries arrive at period start; failures happen within a period; service recovery is no earlier than the following period. The architecture-level gate requires every scenario declared for that architecture to meet the service floor before lifecycle cost is considered. Under these fixtures, the imported-spares architecture fails the delay shock; the higher-cost verified-local-repair architecture is the cheapest robust-feasible choice among the declared options.

All failures, delivery dates, repair durations, availability floor and costs are synthetic. This is not a calibrated reliability model, an inventory-optimization model, a lunar operating forecast, or mission qualification. Future empirical versions need service-specific failure evidence, lead-time distributions, wear-out/replacement models, stockpile policy, repair capacity, energy/material budgets, and independent held-out validation. Follow-up: [BENCH-SPACE-007](https://github.com/Luminous-Dynamics/symtropy/issues/1792).

## Resource-to-production chain reference

The separate example `tools/space-economy-bench/examples/resource_chain_reference.rs` explores four alternatives across three resource/process families: bulk-regolith screening for shielding aggregate, oxygen extraction from regolith, polar volatile-bearing feed to purified water, and polar volatile-bearing feed to electrolysis products. It keeps process intermediates, final products and residuals separate, and mass-balances each alternative.

Run:

```sh
cargo test --locked --manifest-path tools/space-economy-bench/Cargo.toml --example resource_chain_reference
cargo run --locked --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example resource_chain_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

### Synthetic batch oracle

- **Bulk shielding aggregate:** 1,000 kg feed → 900 kg aggregate + 100 kg screening rejects. Within this fixture, the feed, process, declared-use quality and energy gates are set to pass; this is a positive control for the gate logic, not a claim that a lunar site or process has been qualified.
- **Regolith oxygen:** synthetic 1,000 kg feed includes an assumed 400 kg oxygen inventory and 50% recovery, giving 200 kg conditional oxygen + 800 kg residue. The declared 6,000 kWh demand exceeds the 5,000 kWh site budget, and process/product gates are unqualified; firm output is therefore zero.
- **Polar volatile-bearing feed → water:** synthetic 100 kg input assumes 80% water, 75% extraction recovery, then 90% purification recovery: 54 kg purified water + 46 kg residuals. Site feed and process/quality gates are unverified, so firm output is zero.
- **Same polar batch → electrolysis products:** the alternative consumes the same 54 kg purified-water intermediate to produce 48 kg oxygen + 6 kg hydrogen, with the same 46 kg residuals. It is an alternative allocation for the same 100 kg batch, not additive output alongside the water product path. This fixture's arithmetic reflects the mass ratio of water electrolysis but not its real energy use, storage loss, purity or achievable yield.

Potential Fe/Al/Ti/Si-bearing streams in the regolith oxygen path remain explicitly uncredited until extraction, separation, purity and downstream product quality are measured. No potential coproduct is counted as refined inventory simply because it may exist in residue.

### Evidence interpretation

NASA states that lunar volatile location, form, concentration, distribution and accessibility are not fully characterized; it also notes many current ISRU analog demonstrations were short-duration and under Earth conditions. The USGS lunar-resource methodology separates the resource type from certainty and recoverability. Sources:
- [NASA: Overview of In-Situ Resource Utilization](https://www.nasa.gov/overview-in-situ-resource-utilization/)
- [USGS: Assessment of Lunar Resource Exploration in 2022](https://www.usgs.gov/publications/assessment-lunar-resource-exploration-2022)
- [NASA: Lunar Enabling Infrastructure Accelerator, 8 September 2026](https://www.nasa.gov/news-release/nasa-calls-for-proposals-to-accelerate-lunar-surface-technologies/)

Consequently, a "potential" pathway output is not automatically "firm supply". Firm output in this executable reference means only that the explicit synthetic fixture gates are satisfied: scenario feed batch verified, process qualified, product quality verified for its declared use, energy within budget, and mass balance exact. These flags do not assert real lunar qualification. The four pathway results share a single synthetic site-power budget for comparison; site batches with the same allocation group cannot have their output summed.

Recommended next inputs for real pathway comparison: site-specific grade/access measurements, mineralogical phases, extraction/purification recovery intervals, energy intensity under representative thermal/vacuum/dust conditions, product purity and acceptance specification, equipment life/wear parts, and data source/licence/epoch/resolution. Follow-up: [BENCH-SPACE-008](https://github.com/Luminous-Dynamics/symtropy/issues/1803).

For the broader resource-priority and chain roadmap, see [`docs/tech/SPACE_RESOURCE_PRODUCTION_CHAIN_MAP_V0_1.md`](../../docs/tech/SPACE_RESOURCE_PRODUCTION_CHAIN_MAP_V0_1.md), which distinguishes resource occurrence from recoverable reserves and proposes staged production chains and gates.

## Resource survey selection by decision value

The separate `tools/space-economy-bench/examples/resource_assay_selection_reference.rs` chooses a resource measurement based on exact expected value of sample information (EVSI), after survey cost, rather than data volume or nominal assay accuracy. It compares the best decision without a survey with the best action conditioned on each assay result, uses exact integer joint-likelihood weights, emits the posterior rich-site probability after positive/negative results, and checks the perfect-information bound.

Run:

```sh
cargo test --locked --manifest-path tools/space-economy-bench/Cargo.toml --example resource_assay_selection_reference
cargo run --locked --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example resource_assay_selection_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

Synthetic decision setup: the prior probability of a rich site is 50%; building an oxygen-production capability has payoff +200 units at a rich site and -100 at a poor site; imported oxygen is a zero-payoff fallback. The best decision without more data has expected value 50 units; the perfect-information value is 100, yielding EVPI = 50.

- A verified direct assay with 90% sensitivity/specificity and cost 10 has EVSI 35 and net incremental decision value +25. The policy builds after a positive result and chooses imports after a negative result.
- A verified low-resolution assay with 60% sensitivity/specificity and cost 2 does not change the optimal action in either outcome; EVSI is zero and it is not bought.
- An uninformative 50%/50% assay with positive cost has EVSI zero and is not bought.
- A claimed 90%/90% assay with no evidence backing its performance receives an effective uninformative 50%/50% likelihood model, zero information credit, and no purchase.

The resulting values are synthetic decision utility, not money, site-specific prior probabilities, survey performance, lunar geology or a real investment recommendation. The purpose is to prove the decision contract: pay only for measurements that are credibly likely to change a decision enough to justify their cost. See [BENCH-SPACE-009](https://github.com/Luminous-Dynamics/symtropy/issues/1813).

The next-measurement decision loop is covered by `tools/space-economy-bench/examples/resource_assay_selection_reference.rs`: it scores candidate assays by exact EVSI minus cost, rejects unverified assay-performance claims, and compares EVSI against EVPI. See [BENCH-SPACE-009](https://github.com/Luminous-Dynamics/symtropy/issues/1813) and the exploration section in the [resource-production-chain map](../../docs/tech/SPACE_RESOURCE_PRODUCTION_CHAIN_MAP_V0_1.md).


The direct assay produces synthetic posterior probabilities of 90% after a positive result and 10% after a negative result. The lower-resolution assay produces 60%/40%; because both outcomes still favor the same build decision under this payoff model, its EVSI remains zero. The receipt exposes these posteriors alongside the chosen conditional action so reviewers can audit why a result does or does not change the route.

## Adaptive sequential resource exploration

The separate `tools/space-economy-bench/examples/sequential_resource_exploration_reference.rs` tests a conditional measurement policy rather than a one-shot survey ranking. It first runs a cheap synthetic remote screen (70% sensitivity/specificity, cost 1), then calculates the expected value of a 90%/90% follow-up separately for each observed branch. The follow-up costs 20 synthetic units and is bought only when branch-conditional EVSI exceeds its cost.

For the fixed fixture:
- Prior rich-site probability: 50%; local-build payoff +200 if rich and -100 if poor; import payoff 0.
- The posterior after the screen is 70% following a positive result and 30% following a negative result.
- Conditional follow-up EVSI is 13 units after a positive screen, so the 20-unit follow-up is rejected; after a negative screen it is 47 units, so the follow-up is purchased.
- The adaptive policy's expected survey cost is 11 units and net expected value is 67.5; buying the high-resolution assay directly costs 20 and has net expected value 65. Thus the adaptive policy's net advantage is 2.5 synthetic units in this deliberately chosen fixture.
- A cost-60 negative control buys no follow-up on either branch.

The implementation enumerates exact integer joint probability weights under one fixed denominator, never uses floating point to select the policy, and emits a receipt binding the source commit and source SHA-256.

```sh
cargo test --locked --manifest-path tools/space-economy-bench/Cargo.toml --example sequential_resource_exploration_reference
cargo run --locked --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example sequential_resource_exploration_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

All likelihoods, payoffs and costs are synthetic—not lunar survey accuracy, geological priors, resource reserves, dollar values or mission investment advice. The fixture assumes the declared stage likelihoods are usable as given; real use must test conditional dependence, spatial sampling bias, representativeness, destructive sample limits, time/power/access constraints, and held-out calibration. Research tracking: [BENCH-SPACE-010](https://github.com/Luminous-Dynamics/symtropy/issues/1823).

## Correlation-aware assays and constrained exploration

The new `tools/space-economy-bench/examples/correlated_assay_evidence_reference.rs` addresses a common measurement-fusion error: multiplying likelihoods as if two assays were independent even though they reuse calibration standards and the same sampled patch. It also models an explicit latent spatial-representativeness state.

The synthetic oracle assumes a 20% chance that a sample patch is not representative of the site-wide resource state. In that state, results are uninformative. Conditional on a representative patch, two nominal 80% assays have a 50% shared-calibration mode. Their joint likelihood is stored explicitly and its four outcome weights sum exactly to 10,000 bps in each resource state.

| Portfolio/model | Gross expected value | EVSI | Measurement cost | Net value |
|---|---:|---:|---:|---:|
| No measurement | 50 | 0 | 0 | 50 |
| One assay A | 61 | 11 | 4 | 57 |
| A + B, correct shared-state model | 66.7 | 16.7 | 10 | 56.7 |
| A + B, false independence model | 70.62 | 20.62 | 10 | 60.62 (diagnostic only) |
| A + complementary C | 75.66 | 25.66 | 9 | **66.66** |

The false independence model raises the double-positive posterior from 81.08% under the explicit joint/shared-state model to 89.01%. It predicts a gross value 3.92 units higher than the correlated model and would incorrectly make the repeated pair look worth purchasing. Because the claimed independence conflicts with shared calibration and spatial domains, its result is rejected from portfolio selection.

The complementary assay uses a separate calibration domain and a separately modeled spatial domain; under this synthetic assumption, the effective two-measurement portfolio has the highest net expected value. Separate labels alone do not establish conditional independence in a real survey.

The finite optimizer enumerates all permitted portfolios and selects `complementary_assay_a_c` with zero regret against that enumeration. Hard resource caps are two assays, two sample units, four power units and two elapsed hours; a three-assay overrun is rejected. Five negative controls reject missing dependence metadata, uncalibrated instruments, unmodeled/nonrepresentative sampling, missing results and a false independence claim. Eleven authored tests cover table mass, expected-value oracles, posteriors, evidence gates, budget rejection and source binding.

```sh
cargo test --locked --manifest-path tools/space-economy-bench/Cargo.toml --example correlated_assay_evidence_reference
cargo run --locked --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example correlated_assay_evidence_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

All rates, latent-state probabilities, payoffs and costs are synthetic. The calibration and representativeness flags validate only the benchmark fixture contract; they do not mean real lunar samples are representative. Research motivation: [NIST correlation in measurement uncertainty](https://www.nist.gov/publications/correlation-uncertainty-measurement-discussion-state-art-techniques), [NIST covariance propagation guidance](https://www.itl.nist.gov/div898/handbook/mpc/section5/mpc55.htm), and the literature on [Bayesian data fusion with unknown correlation](https://www.sciencedirect.com/science/article/pii/S1566253517302890). Follow-up: [BENCH-SPACE-011](https://github.com/Luminous-Dynamics/symtropy/issues/1824).

## Robust exploration under likelihood-model uncertainty

The separate `tools/space-economy-bench/examples/robust_exploration_model_uncertainty_reference.rs` asks whether a good-looking measurement policy survives uncertainty about its own sensitivity/specificity. It compares three explicit models on a common exact basis-point grid: nominal 90%/90%, moderate 70%/70%, and an uninformative 50%/50% limit. Each model has the same 50% rich-site prior and synthetic build/import payoffs; it is not a site prior estimate.

| Policy | Nominal-model net value | Worst-case net value | Worst-case regret |
|---|---:|---:|---:|
| No measurement | 50 | **50** | 15 |
| Always buy assay (cost 20) | **65** | 30 | 20 |
| Calibration gate then adapt | 62 | 47 | **3** |

Three objectives intentionally select three different policies:
- **Nominal optimizer:** buy the assay, because its net value is 65 under the 90%/90% model.
- **Maximin expected utility:** take no measurement, because its minimum value is 50, versus 47 for the calibration-gated strategy and 30 for always buying.
- **Minimax regret:** choose the calibration-gated strategy, whose maximum regret is only 3 across the declared models, compared with 15 and 20.

This is the key contract: the policy objective must be visible. There is no objective-free single “best” action when the model itself is uncertain.

The calibration gate is a deliberately idealized oracle that costs 3 units and perfectly identifies which of the three performance regimes applies. It purchases the 20-unit assay only in the high-accuracy regime; in the moderate and uninformative regimes it stops and retains the baseline decision. Its values are 62 in the high regime and 47 in the other two. This oracle is a benchmark control, not a proposal for a real instrument or a claim that calibration can perfectly identify the model.

The receipt reports each model's gross information value, EVSI/EVPI bound, direct-assay net value, posterior after positive/negative observations, per-policy value vector, worst case, maximum regret, resource use, and the selected policy under each criterion. All calculations use exact integer weights under a denominator of 100,000,000. The nominal assay has positive incremental value of +15 units under the high-accuracy model but a negative incremental value of -20 under the uninformative limit, satisfying the key robustness negative control.

Hard caps are two assays, two sample units, four power units and two elapsed hours. The unverified 99%/99% claim is excluded from the uncertainty set and receives zero selection credit. Eleven authored tests check probability mass, EVSI ≤ EVPI, objective-specific winners, the nominal-positive/worst-case-negative control, evidence rejection and budgets.

```sh
cargo test --locked --manifest-path tools/space-economy-bench/Cargo.toml --example robust_exploration_model_uncertainty_reference
cargo run --locked --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example robust_exploration_model_uncertainty_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

Research grounding: the 2026 paper [Robust Bayesian Decision Making under Adversarial Uncertainty](https://proceedings.mlr.press/v337/harikumar26a.html) argues for experiment selection that targets decision stability under plausible model misspecification rather than information gain or nominal optimality alone. NIST's [measurement-uncertainty guidance](https://www.nist.gov/pml/nist-technical-note-1297/nist-tn-1297-5-combined-standard-uncertainty) also emphasizes accounting for covariances where relevant. These sources motivate the benchmark; they do not validate its synthetic values. Follow-up: [BENCH-SPACE-012](https://github.com/Luminous-Dynamics/symtropy/issues/1837).

## Choose calibration experiments by downstream decision value

The `tools/space-economy-bench/examples/decision_value_calibration_experiments_reference.rs` reference removes the perfect-calibration oracle from the previous example and gives calibration itself a finite experiment-design problem. It uses three explicit, equally weighted **synthetic** assay-quality regimes (high: net assay value 65, moderate: 35, uninformative: 30) and a baseline alternative worth 50. These equal model weights are a fixture prior, not field evidence.

Two imperfect calibration experiments have different expected utility:
- Calibration A, cost 1.00, has positive-result likelihoods 90% / 40% / 10% in the high/moderate/uninformative regimes. Alone, it yields expected net value 50.8333.
- Calibration B, cost 0.25, has positive-result likelihoods 70% / 20% / 30%. Alone, it yields expected net value 50.25.
- An uninformative 50% / 50% / 50% control costs 0.10; it cannot change a decision and is rejected.

The benchmark exhaustively enumerates the two possible test orders and whether the second test runs after the positive branch, negative branch, both branches or neither:
| Calibration policy | Expected net value |
|---|---:|
| No calibration | 50.00 |
| A only | 50.8333 |
| B only | 50.25 |
| A then B, only after A-positive | 51.4333 |
| A then B, only after A-negative | 50.70 |
| A then B, after either outcome | 51.30 |
| B then A, only after B-positive | **51.90** |
| B then A, only after B-negative | 49.65 |
| B then A, after either outcome | 51.30 |
| Uninformative test control | 49.90 |

The winner is **B first, then A only after B-positive**. B's positive branch has 40% probability under the synthetic equal model weights; running A only there makes expected calibration spend 0.65 (0.25 + 0.40 × 1.00) and preserves the gross decision value of 52.55. By contrast, running A first then B after A-positive spends about 1.1167 expected units for the same gross value.

Each possible observed branch reports its exact unnormalized model weights, posterior probability floors, posterior-mean decision, maximin action and minimax-regret action. Because every likelihood in this fixture is nonzero, all three regimes remain possible after any result: maximin and minimax-regret continue to prefer the baseline action (assay worst case 30 vs baseline 50; maximum regret 20 vs 15), while posterior-mean decisions can change by branch. The model-class EVPI is 5 and expected calibration EVSI is bounded by it.

Hard limits remain two calibration tests, two samples, four power units and two hours. The code hashes the explicit model and calibration-test specifications separately from its source. Negative controls cover a redundant test, shared standards, missing results and unsupported calibration-accuracy claims.

```sh
cargo test --locked --manifest-path tools/space-economy-bench/Cargo.toml --example decision_value_calibration_experiments_reference
cargo run --locked --quiet --manifest-path tools/space-economy-bench/Cargo.toml --example decision_value_calibration_experiments_reference -- \
  --source-commit "$(git rev-parse HEAD)"
```

Research motivation: [Hu, Ao and Mahadevan, *Calibration experimental design considering field response and model uncertainty*](https://www.sciencedirect.com/science/article/pii/S0045782516302274) explicitly studies calibration-design optimization under model uncertainty and testing budgets. NIST's [uncertainties of calibrated values guidance](https://www.itl.nist.gov/div898/handbook/mpc/section3/mpc367.htm) notes the joint contribution of limited calibration-curve data and future-measurement uncertainty. The Symtropy fixture is an independent finite reference; these sources do not validate its numerical assumptions. Follow-up: [BENCH-SPACE-013](https://github.com/Luminous-Dynamics/symtropy/issues/1847).
