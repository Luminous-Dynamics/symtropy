# Paired Actuator Observability and First-to-Derate v0.1

**Status:** M0 source register and M1 evidence contract. This directory contains no new physical test or independent actuator-pair measurement.

**Research owner:** [PAIRED-ACTUATOR-OBSERVABILITY-001 #6171](https://github.com/Luminous-Dynamics/symtropy/issues/6171)  
**Thermal mission envelope:** [#6155](https://github.com/Luminous-Dynamics/symtropy/issues/6155)  
**Shared joint benchmark contract:** [#6140](https://github.com/Luminous-Dynamics/symtropy/issues/6140)  
**Physics actuator-step semantics:** [#6144](https://github.com/Luminous-Dynamics/symtropy/issues/6144)  
**J1 impact reference:** [#6134](https://github.com/Luminous-Dynamics/symtropy/issues/6134)

## 1. Research question

When multiple motors drive one joint, what fraction of the joint's useful mechanical capability can actually be relied upon over a task mission, and which individual actuator or transmission element becomes the first bottleneck?

The common shortcut

```text
pair_torque = actuator_count * single_actuator_torque
```

implicitly assumes matched motor constants, matched temperature and current limits, matched transmission losses, aligned mechanics, synchronized controls, and equal load sharing. Those are testable assumptions, not defaults.

A paired joint must retain three separate observations:

1. **Electrical effort share:** current, voltage and electrical power by drive.
2. **Mechanical output share:** independently measured generalized torque contributed by each path.
3. **Thermal burden share:** individual thermal trajectories and first-to-derate state.

Equal currents do not establish equal torque. Equal commands do not establish equal currents. A current split does not identify a mechanical torque split without additional calibrated information.

## 2. Source study and why it matters

Yang, *Upgrading a Humanoid Leg Without Replacing Actuators: Torque–Speed Limits and Bench Measurements*, Applied Sciences 16(20), 10145, 13 October 2026, DOI https://doi.org/10.3390/app162010145.

The paper publishes its acquisition/analysis code and raw logs:
https://github.com/yjh-mcut/redesign-methodology

The bench uses two ROBOTIS Dynamixel MX-106 actuators driving a common shaft through a 24:55 spur pair (N=2.2917), a 300 mm lever, and a 14.8 V / 600 W supply. The units are not the units from the retired robot.

### Published observations

| Quantity | Source result | Boundary |
|---|---:|---|
| No-load upper bound | 2.618 rad/s | Estimated from unloaded bidirectional traverses |
| Loaded maximum speed, 11.7275 N·m | 1.9688 rad/s | Mean of 3 runs, `|q| <= 8°` |
| Loaded maximum speed, 17.5904 N·m | 1.6719 rad/s | Mean of 3 runs |
| Loaded maximum speed, 23.3934 N·m | 1.3653 rad/s | Mean of 3 runs |
| Heaviest-load shortfall vs no-load bound | 47.8% | At 23.3934 N·m |
| High-load holds | 333, 351, 394 s | Ended at reported internal-device 70 °C criterion |
| Partner temperature at heavy-load stop | 51–52 °C | 18–19 °C cooler than the first unit |
| Hotter unit current fraction | 0.68–0.70 | Median during the two heavier hold strata |

The paper's fitted speed model identifies only a lumped product `n_a * beta * eta * tau_s = 21.22 N·m`. It cannot separate per-motor stall torque, gear efficiency and mechanical sharing.

The paper explicitly cautions that its inferred `beta ≈ 0.71–0.74` is conditional on equal torque constants, equal individual allowable outputs and the current split persisting at the relevant operating point. It is not a direct measurement of mechanical torque share.

At static hold, gear-train resisting torque was measured as 0.61 N·m in one direction. Consequently, successful holding is a property of the entire assembly and does not isolate the pair's actuator torque.

### Run denominator and exclusions

The public run ledger contains **49 attempted runs: 26 included and 23 invalid/excluded**. The README reports 48 raw log files. The run breakdown is:

| Experiment | Attempted | Included | Invalid/excluded |
|---|---:|---:|---:|
| Static hold | 16 | 11 | 5 |
| Maximum speed | 9 | 4 | 5 |
| Dynamic constant-rate | 18 | 9 | 9 |
| Tuning | 3 | 0 | 3 |
| No-load | 3 | 2 | 1 |
| **Total** | **49** | **26** | **23** |

Do not erase those exclusions. In particular, the ledger includes downhill regenerative overvoltage, direction/configuration errors, failed settling, aborted/partial files, and an excluded high-load hold whose load sharing was reduced by static friction. An aborted-at-thermal-limit run can still be a valid measured outcome; evidence inclusion and physical task success are different fields.

## 3. Independent raw-log reconstruction (same measurements, not a replication)

I independently reprocessed four public maximum-speed CSV logs from the author's repository using the rule stated in the paper: for each repetition, perform ordinary least-squares regression of joint angle (radians) against logged timestamp over `|q| <= 8°`, then take the absolute slope.

| Load | Raw-log reconstructed mean | Paper display | Difference |
|---:|---:|---:|---:|
| 1.1454 N·m | 2.5781276294 rad/s | 2.5781 | +0.0000276294 |
| 11.7275 N·m | 1.9687573674 rad/s | 1.9688 | -0.0000426326 |
| 17.5904 N·m | 1.6718757964 rad/s | 1.6719 | -0.0000242036 |
| 23.3934 N·m | 1.3653467243 rad/s | 1.3653 | +0.0000467243 |

This is an independent computational reconstruction of the same source data. It is **not** an independent physical measurement or replication. The raw trace path, repetition values and Git blob SHA are recorded in the JSON example.


## 4. Reusable Rust source replay tool

The same directory contains a dependency-free Rust CLI for replaying the source run ledger and maximum-speed raw traces:

- `src/lib.rs` — quoted-CSV parser, run-ledger inclusion/exclusion summary, per-repetition OLS speed reconstruction and JSON output.
- `src/main.rs` — CLI that accepts a run ledger and any number of raw encoder traces.
- `Cargo.toml` / `Cargo.lock` — isolated, no-dependency Cargo package.
- `.github/workflows/paired-actuator-source-replay.yml` — package-scoped format, test and Clippy checks using Rust 1.96.0.

Clone the public source-data repository and replay all four published maximum-speed traces:

```bash
git clone https://github.com/yjh-mcut/redesign-methodology.git /tmp/redesign-methodology

cargo run --manifest-path benchmarks/synthetic-joints/paired-actuator-observability/v0.1/Cargo.toml -- \
  --runlog /tmp/redesign-methodology/bench/sheet_B_runlog.csv \
  --trace empty=/tmp/redesign-methodology/bench/data/exp2b_HEALTH-1_20260927_151621.csv \
  --trace load-A=/tmp/redesign-methodology/bench/data/exp2b_C-A-1_20260921_203519.csv \
  --trace load-B=/tmp/redesign-methodology/bench/data/exp2b_C-B-1_20260923_184203.csv \
  --trace load-C=/tmp/redesign-methodology/bench/data/exp2b_C-C-1_20260925_160329.csv \
  --q-window-deg 8 \
  --min-points 15
```

The output is JSON containing the full attempted/included/excluded/unknown ledger counts by experiment, each repetition's point count and absolute OLS speed, the mean and population standard deviation. The default angle window is 8 degrees and the default minimum is 15 samples per repetition.

The replay is intentionally source-data agnostic. It does not infer current-to-torque conversion, identify per-motor mechanical load share, impute missing runs, or relabel a thermal/overvoltage stop as a task success. Reconstructed speed values should agree with the source report to displayed precision; discrepancies should be investigated rather than silently rounded away.

The source CSVs are not copied into this repository. This avoids duplicating a large dataset and preserves the public source repository as the data authority. Exact raw-file URLs and Git blob SHAs for the selected records remain in the machine-readable example.

## 5. Schema and example

- `paired_actuator_profile.schema.json` is a JSON Schema Draft 2020-12 contract.
- `examples/yang-mx106-paired-hiproll-source-record.json` stores selected source runs, the 49/26/23 denominator, source and derived metrics, raw-trace references, and an explicit identifiability statement.

The record separates:
- run inclusion from run outcome;
- current/electrical effort share from mechanical torque share;
- common-shaft torque/speed from individual actuator torque;
- internal-device temperature from winding temperature;
- first-to-derate/stop from pair-level task completion;
- directly measured, source-reported, derived, estimated, assumed and unknown quantities.

Unknown measurements remain null with reasons. Excluded runs remain visible. The schema cannot certify the truth of the source or quality of the instruments.

## 5. Observation model and identifiability

A simplified per-drive relation is:

```text
tau_motor_i = k_t_i(T_i, state_i) * I_i
tau_joint = sum_i (N_i * eta_i(direction, speed, load, T_i) * tau_motor_i)
            - tau_friction - tau_compliance_dynamics
```

This equation is not a measurement method by itself. It highlights why current alone is not enough:
- `k_t_i` changes with actuator state/temperature and must be calibrated;
- `eta_i` may differ across paths and operating directions;
- backlash, compliance, alignment and gear friction alter the torque transfer;
- paired control loops may saturate or synchronize imperfectly;
- static friction can support some load without the motor carrying it;
- one drive can derate while the other remains below its own limit.

If no independent per-path torque sensor or identifiable mechanical model exists, the mechanical share must remain **not identifiable**. Report bounds or a lumped parameter combination if that is all the experiment can support.

For every experiment design, evaluate:
- parameter identifiability / observation-matrix rank;
- condition number and excitation adequacy;
- correlations among `k_t`, `eta`, `beta`, friction and thermal parameters;
- prediction intervals on held-out trajectories;
- drift after reinstallation, service and aging.

Calibration fit residual is not held-out validation.

## 6. M0–M5 qualification

### M0 — source and boundary traceability
Exact actuator/model/firmware, gear ratio, common-shaft geometry, controller synchronization, voltage/current limits, thermal boundary, test method, calibration and all run exclusions.

### M1 — observation-model correctness
Known-answer fixtures for equal and unequal torque constants, efficiency mismatch, friction, gear ratio, current sensing bias, thermal sensor bias and a drive reaching a limit first. Include rank/condition checks and negative controls for falsely claiming mechanical share from current alone.

### M2 — numerical identification
Compare at least two estimators on simulated known-parameter data and frozen held-out sequences. Report identifiability, uncertainty/covariance, bias, prediction interval coverage and failure under confounded inputs.

### M3 — guarded paired component test
Use independent per-path torque sensing where mechanically possible, per-drive current/voltage, synchronized joint torque/position/speed, per-drive temperatures, calibrated supply, exact controller logs and raw traces. Otherwise explicitly state the share is not identifiable.

### M4 — installed task mission
Measure first-to-derate, joint task success, loaded speed, energy and thermal burden under payload and duty variation. Validate on hot starts and repeated partial cooldown.

### M5 — lifecycle and replacement
Repeat across multiple actuator pairs, after service/replacement, remount, gear/preload changes and cooling changes. Requalification follows changes that invalidate transfer of the original model.

## 7. Prospective benchmarks

These are targets, not achievements:
- Zero equal-sharing claims based only on equal current commands or actuator count.
- 100% of records state whether mechanical sharing is independently measured, estimated, bounded or unidentifiable.
- First-to-derate time and per-drive thermal margin are reported alongside pair-level output.
- Held-out pair model intervals have preregistered coverage once sample size and sensor uncertainty are established.
- Compare against the naive equal-share baseline at matched installed mass, supply, cooling and duty.
- Include all attempts, exclusions, aborts, derating events, partial logs and censored runs.
- Requalification after actuator, controller, current-limit, gear/preload, or cooling-boundary changes unless equivalence is demonstrated.

## 8. Claim ceiling and safety

This source record and protocol do not authorize energized tests, deliberate thermal-limit trips, stall testing, fabrication or human-contact use. Physical work requires a separately approved guarded rig, qualified operators, interlocks, manufacturer limits, hazard analysis and independent instrumentation.

A schema pass, raw-log reconstruction, simulation result, or CI pass is not an actuator-pair safety qualification.
