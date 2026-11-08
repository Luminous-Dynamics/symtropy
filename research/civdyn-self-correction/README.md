# CIVDYN Self-Correction v0.1

A small, deterministic, ledger-backed reference benchmark for the ability of a socio-ecological controller to detect a bad signal or a developing policy failure, diagnose a mechanism, test a discriminating counterfactual, update its policy, intervene, and independently audit the consequences.

It composes `civdyn-resource-ledger`; it does not create a second stock-flow engine or a general-purpose civilization simulator.

## Paradigm

Evaluate the correction process, not just the final target KPI:

`observe → detect → explain → discriminate → update → intervene → audit`

A correction is not qualified merely because one reported metric improves. The benchmark reports household service, industrial service, reserve stock, maintenance energy, unnecessary dispatch, and conservation separately. It does not emit a composite welfare score.

## Scenario A: sensor-drift sham

The physical system is unchanged: a household receives exactly its 10 m³/tick requirement and the ledger reports 100% service. Starting on tick 3, the primary sensor reports 60% coverage while an independent meter remains at 100%.

The portfolio compares:
- `frozen_policy`: ignores the corrupted primary sensor;
- `outcome_reactive`: treats the primary reading as physical shortage and sends emergency water;
- `mechanism_aware`: uses the independent meter to identify sensor drift, calibrates the signal, and does not send unnecessary water.

Known-answer control: outcome-reactive dispatches 12 m³ unnecessarily over the three drifted ticks; mechanism-aware dispatches zero additional water. True household service remains 100% in every arm.

## Scenario B: delayed externality and repeated shock

A source basin begins with 80 m³ of water and 100 kWh of energy. The synthetic system runs:
- 4 growth ticks: 10 m³/tick recharge, 10 m³/tick household demand, 5 m³/tick industrial demand;
- 2 recovery ticks;
- 6 repeated-shock ticks with zero recharge.

The mechanism-aware recovery action transfers up to 5 kWh/tick to a pump-maintenance node. The declared synthetic pump relation is 2 m³ of additional basin recharge per delivered kWh. This is an explicit scenario input, not calibrated hydrology. The mechanism-aware controller keeps industry online during recovery, then uses a matched counterfactual from the exact post-recovery snapshot to test whether industrial withdrawals compromise the household service floor during the shock.

The declared policy constraint is household essential-service coverage of at least 0.95. The policy protects that constraint by curtailing industrial deliveries during the repeated shock. This is a normative priority declared by the fixture, not a universal recommendation. All industrial unmet demand is reported separately.

Expected synthetic known answers:
- frozen/outcome-reactive: reserve 60 m³ after growth, 50 m³ after recovery, 25 m³ household unmet under the repeated shock, 15 m³ industrial unmet;
- mechanism-aware: 70 m³ reserve after maintenance-assisted recovery, 0 m³ household unmet, 30 m³ industrial unmet, 10 kWh maintenance energy consumed.

The outcome-reactive controller waits for actual household service failure before curtailing industry; by then the reserve is exhausted. The mechanism-aware controller uses paired maintenance and demand-ablation counterfactuals before failure.

## Metrics

The CSV exposes:
- minimum household coverage and household unmet water;
- industrial unmet water;
- unnecessary emergency dispatch;
- reserve before, after growth, after recovery, and after repeated shock;
- maintenance energy consumed;
- household unmet demand in matched industry-on and industry-off shock branches;
- household unmet demand without maintenance, even when industry is curtailed;
- mechanism identification, correction latency, event-trace validity;
- maximum water and energy conservation residuals, each in its native unit.

Do not aggregate these into one score. In particular, the mechanism-aware policy's improvement in household service is accompanied by higher industrial unmet demand. That trade-off is a central result, not a nuisance to hide.

## Baselines and extension targets

The first comparison is intentionally simple: frozen policy, outcome-reactive threshold control, and a mechanism-aware controller with explicit counterfactual evidence. The next research stages should add:
1. matched human/expert policy references where a documented protocol exists;
2. stochastic replications and uncertainty intervals, not only deterministic known answers;
3. held-out mechanisms and population/context shifts;
4. a learned diagnosis/controller compared at equal observation/action/compute budgets;
5. source-backed real-world calibration and independent hydrological/environmental validation;
6. composition with the existing causal-path and robust-policy evaluators once their branches are integrated.

## Run

```sh
cargo fmt --manifest-path research/civdyn-self-correction/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-self-correction/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-self-correction/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-self-correction/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-self-correction/Cargo.toml --bin self-correction-sweep
```

## Claim ceiling

This benchmark demonstrates deterministic accounting, known-answer sensor-drift controls, matched counterfactual branches, and an explicit policy trade-off under a declared synthetic scenario. It does not establish:
- empirical causal identification;
- calibrated hydrological, ecological, economic or institutional behaviour;
- real-world policy effectiveness;
- biodiversity, ecosystem health or environmental-flow adequacy;
- a universal normative priority ordering;
- superiority to SimBench, SSDataBench, LiveCultureBench or any other SOTA benchmark.

Green software CI validates this reference implementation only.
