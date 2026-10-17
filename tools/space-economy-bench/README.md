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
