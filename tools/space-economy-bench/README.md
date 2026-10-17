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
