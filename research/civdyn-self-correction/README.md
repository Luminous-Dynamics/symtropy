# CIVDYN Self-Correction v0.1

A small, deterministic, ledger-backed reference benchmark for the ability of a socio-ecological controller to detect a bad signal or a developing policy failure, diagnose a mechanism, test a discriminating counterfactual, update its policy, intervene, and independently audit the consequences.

It composes `civdyn-resource-ledger`; it does not create a second stock-flow engine or a general-purpose civilization simulator.

See [benchmark_crosswalk.md](benchmark_crosswalk.md) for the external benchmark landscape, staged comparison gates and proposed targets. Those targets are protocol proposals, not current performance claims.

## Paradigm

Evaluate the correction process, not just the final target KPI:

`observe → detect → explain → discriminate → update → intervene → audit`

A correction is not qualified merely because one reported metric improves. The benchmark reports household service, industrial service, reserve stock, maintenance energy served and unmet, unnecessary dispatch, and conservation separately. It does not emit a composite welfare score.

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
- mechanism-aware: 70 m³ reserve after maintenance-assisted recovery, 0 m³ household unmet, 30 m³ industrial unmet, 10 kWh maintenance energy served and 50 kWh maintenance-energy obligation unmet.

The outcome-reactive controller waits for actual household service failure before curtailing industry; by then the reserve is exhausted. The mechanism-aware controller uses paired maintenance and demand-ablation counterfactuals before failure.

## Metrics

The CSV exposes:
- minimum household coverage and household unmet water;
- industrial unmet water;
- unnecessary emergency dispatch;
- reserve before, after growth, after recovery, and after repeated shock;
- maintenance energy served and unmet;
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

## Scenario C: causal drift versus observational shift

The synthetic structural outcome equation is:

`Y = 10 + U + τ_t A`

where \(U\) is a common cause, \(A\) is the intervention, and \(\\tau_t\) is the time-varying causal effect. The observational treated group has a +5 confounder imbalance, so the observational association is \(\\tau_t + 5\). After the causal effect reverses from +2 to -3 at tick 4, the observed association remains +2. An outcome-reactive controller therefore continues the intervention despite its negative causal effect.

The mechanism-aware controller periodically runs a deterministic matched-pair probe. Each treatment/control pair shares the same confounder value; the probe estimate is computed from the paired synthetic outcomes, not read directly from the hidden regime parameter. The first known-answer version has no sampling noise, so this demonstrates the protocol logic, not statistical power or real-world identification.

Five regimes test different failure modes:
- `stable_positive`: effect remains +2; the controller must not over-correct or shut down a still-beneficial intervention.
- `sign_reversal`: effect changes from +2 to -3 at tick 4 while the observational association remains positive; the controller must detect the change and suspend the intervention on the next tick.
- `gradual_sign_reversal`: the effect moves from +2 to +1 to -1 to -2 across eight ticks while the confounder keeps the observational association positive; the controller must detect the transition at tick 6.
- `confounded_null`: true effect is zero while observational association remains +5; the controller must reject the spurious benefit.
- `confounder_shift`: true effect remains +2, but a common-cause imbalance moves from +5 to -5 at tick 5, making the observational association move from +7 to -3. A CUSUM detector correctly flags a signal shift but cannot identify the mechanism; its signal-only policy turns off a still-beneficial intervention. The matched-probe controller diagnoses a confounder shift and retains the intervention.

The portfolio compares frozen, outcome-reactive, CUSUM signal-only and mechanism-aware controllers across all five regimes. It reports signal detection separately from causal diagnosis, proxy-driven action under a non-positive causal effect, missed beneficial-action ticks after over-correction, matched probe estimates, active ticks after the effect becomes non-positive, action and probe costs, native net utility, oracle utility and regret. The oracle is a ground-truth upper bound, not a deployable baseline.

Known-answer controls: CUSUM detects abrupt observational change at tick 4, gradual change at tick 5, and confounder shift at tick 5. The mechanism-aware controller diagnoses abrupt reversal at tick 4, gradual reversal at tick 6, confounded null at tick 2, and confounder-only shift at tick 6. It does not disable the stable-positive mechanism or the still-beneficial intervention under confounder shift.

## Scenario D: noisy sequential causal diagnosis

Scenario C is a deterministic known-answer protocol control. Scenario D tests whether its conclusions survive finite samples, random outcome noise and repeated looks.

The seeded Monte Carlo runner uses:
- 256 fixed seeds per regime (1,280 complete episode rows);
- SplitMix64 with Box-Muller normal variates, using a frozen base seed and deterministic regime/seed mapping;
- independent treatment/control outcome noise with standard deviation 2.0;
- 2,048 matched treatment/control pairs per look, sharing the same confounder value within each pair;
- four predeclared looks at ticks 2, 4, 6 and 8;
- an estimated standard error and a conservative one-sided critical value of 2.5 for the upper confidence bound against the declared action-benefit threshold of 0.5.

The repeated-look rule is conservative relative to a one-sided Bonferroni allocation across four planned looks under the stated approximately Gaussian paired-difference model. It uses a normal critical value with estimated variance, not an exact finite-sample randomization test. Its statistical interpretation depends on the generator and assumptions being correctly specified. A future version should add exact/permutation inference, heteroskedastic noise, missing outcomes and blinded randomization checks.

Every seed/regime row is exported. The receipt includes signal-detection tick, diagnosis and tick, policy action counts, probe look count, final paired estimate, standard error, lower/upper bound, sample size, confidence critical value, false sign diagnosis flag, and the paired deterministic comparator.

The confirmatory software gate requires:
- no false policy stops on the 256 stable-positive episodes;
- at least 250/256 correct sign-reversal diagnoses;
- at least 250/256 correct gradual-reversal diagnoses;
- at least 250/256 non-beneficial diagnoses for the confounded-null regime, with at most five false sign labels;
- at least 250/256 correct confounder-shift diagnoses while retaining the beneficial action.

These are predeclared synthetic-generator operating-characteristic gates, not estimates of real-world performance or guarantees for other distributions. Failed seeds remain in the CSV and cannot be discarded.

## Run

```sh
cargo fmt --manifest-path research/civdyn-self-correction/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-self-correction/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-self-correction/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-self-correction/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-self-correction/Cargo.toml --bin self-correction-sweep
cargo run --locked --manifest-path research/civdyn-self-correction/Cargo.toml --bin causal-drift-sweep
cargo run --locked --manifest-path research/civdyn-self-correction/Cargo.toml --bin noisy-causal-drift-sweep
```

## Claim ceiling

This benchmark demonstrates deterministic accounting, sensor-drift controls, a confounded causal sign-reversal control, matched synthetic counterfactuals, and explicit policy trade-offs under declared synthetic scenarios. It does not establish:
- empirical causal identification;
- calibrated hydrological, ecological, economic or institutional behaviour;
- real-world policy effectiveness;
- biodiversity, ecosystem health or environmental-flow adequacy;
- a universal normative priority ordering;
- empirical causal identification or drift-detection performance under sampling noise;
- superiority to SimBench, SSDataBench, LiveCultureBench or any other SOTA benchmark.

Green software CI validates this reference implementation only.
