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
- an estimated standard error and a conservative one-sided critical value of 2.8 for the upper confidence bound against the declared action-benefit threshold of 0.5.

The repeated-look rule is conservative relative to a one-sided Bonferroni allocation across four planned looks under the stated approximately Gaussian paired-difference model; the normal-approximation union bound is about 1.1% family-wise, more conservative than a 5% budget. It uses a normal critical value with estimated variance, not an exact finite-sample randomization test. Its statistical interpretation depends on the generator and assumptions being correctly specified. A future version should add exact/permutation inference, heteroskedastic noise, missing outcomes and blinded randomization checks.

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

## Scenario E: interference and spillover identifiability

The first exact interference twin-world control is implemented in `interference_identifiability.rs`, advancing [CIVDYN-ID-001 #4251](https://github.com/Luminous-Dynamics/symtropy/issues/4251).

It separates a two-unit network into a source neighbor (A) and a focal neighbor (B):
- no-spillover world: `Y_B = T_B`;
- positive-spillover world: `Y_B = T_B OR T_A`.

The observational reference only records focal-neighbor outcomes when the source neighbor is untreated. Across focal treatment assignments `T_B ∈ {0,1}`, both worlds produce `[0,1]`; their observed distributions are exactly equal under this declared observation design.

Three exact interventions demonstrate why a full rollout is not automatically the most informative experiment:
- baseline `do(T_A=0,T_B=0)`: both worlds predict 0; no discrimination;
- neighbor-only probe `do(T_A=1,T_B=0)`: no-spillover predicts 0 and positive-spillover predicts 1; total variation is 1 and expected information gain is 1 bit under a 0.5/0.5 world prior;
- full rollout `do(T_A=1,T_B=1)`: both worlds predict 1 because direct treatment saturates the binary outcome; no discrimination.

The selector respects a declared intervention budget, excludes non-discriminating probes, and returns no selected experiment when the permitted set cannot separate the candidate worlds. It keeps observational ambiguity and the identified spillover-effect set `{0,1}` explicit.

This is an exact two-node protocol control, not a general network causal estimator. It does not cover partial-interference assumptions, weighted/directed networks, exposure mappings beyond the OR rule, treatment compliance, endogenous network formation, heterogeneous spillover effects, equilibrium feedback, or empirical social-network data. Those are the next benchmark families in #4251.


## Scenario F: adequacy before policy interpretation

The first executable slice of CIVDYN-ADEQUACY-001 is in src/model_adequacy.rs and the deterministic model-adequacy-sweep binary.

The evaluator consumes an explicit statistic registry, a split manifest, source-identified observations and source-identified predictive draws. It does not create another simulator or fit parameters. Each statistic carries its construct, native measurement unit, population/context, source family and digest identity, independent sampling unit, split role, primary/required status and minimum draw count.

The split contract is explicit:

- Calibration units are named separately from validation units.
- Challenge units are a third, optional partition and can only be evaluated by an explicit challenge call.
- Independent units cannot appear in more than one partition.
- Split-predictive draws must carry the exact calibration-data digest pinned by the manifest.
- Each observed statistic must carry the exact source digest registered for that statistic; a substituted source fails closed.
- Prior-predictive and split-predictive draws are separate kinds.
- Draws are matched to the observed sample size; unmatched draws do not get pooled.
- Model and draw-set identities must remain constant within each statistic/unit draw set.
- Units, source identity, finite values, unique draw indices and required-statistic coverage are checked fail-closed.

The output is one record per statistic and independent unit. It retains native units, the registered source digest, the observed value, measurement standard error, reference interval, empirical mid-rank and draw count. A separate per-statistic coverage summary reports the numerator, assessed/expected independent-unit counts, declared sampling-unit type, empirical coverage and a 95% Wilson interval. Missing units are counted as not assessed, not silently treated as covered. Measurement uncertainty is exposed separately from structural model discrepancy; this first slice does not combine them into one width or score.

The reference interval is an empirical central interval of the supplied finite draws. Its mid-rank is descriptive. Wilson intervals summarize coverage across the declared independent units; they do not prove that the chosen units are truly independent, and their precision depends on the number of such units. Neither output is claimed to be a calibrated frequentist p-value or a full implementation of the JASA split predictive check theory. In particular, this v0.1 does not yet implement the single/divided SPC test statistic calibration, power guarantees, joint trajectory discrepancy, or multiplicity control. It also does not verify that caller-supplied digest strings are cryptographically correct; upstream ingestion must hash and independently verify the bytes.

### Frozen known-answer fixture

The model-adequacy-sweep binary emits six rows: prior reachability and split-predictive diagnostics for three independent target statistics.

- Household water mean: inside both the prior and split-predictive references.
- Health-income dependence: outside both references despite a plausible marginal service statistic.
- Rare service-failure rate: inside the prior reference but outside the split-predictive reference.

This deliberately shows that passing a marginal statistic cannot mask a failed joint or long-tail target, and that prior reachability and held-out predictive adequacy answer different questions. It is a software contract fixture, not empirical CIVDYN model validation.

### Current claim ceiling and next steps

A no_primary_discrepancy_detected result means only that no discrepancy was detected for the registered primary statistics, units, sample size, data split and finite predictive draw set. It is not a declaration that the model is true or universally adequate. not_assessed remains distinct from a pass; detected failures remain visible even when a different primary statistic is missing.

This slice does not yet implement:
1. Joint and trajectory discrepancies with dependence-aware reference distributions.
2. Prospective sample-size/power planning and dependence-aware uncertainty beyond the current Wilson coverage summary.
3. Explicit treatment of measurement-error distributions rather than retaining the supplied measurement SE.
4. Multiple source-family, site, cohort and temporal transfer tests.
5. A diagnosis/repair lineage with fresh challenge sets and leakage receipts.
6. Policy-rank, feasibility and native-resource sensitivity under alternative discrepancy models.
7. Empirical data adapters or independent domain qualification.

Run the model-adequacy-sweep binary using Cargo with the locked manifest at research/civdyn-self-correction/Cargo.toml. The existing CIVDYN workflow also runs this binary after formatting, strict Clippy, compilation and tests. These are engineering checks against frozen synthetic contracts only.
