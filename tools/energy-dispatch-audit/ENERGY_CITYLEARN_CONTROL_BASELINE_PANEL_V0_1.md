# ENERGY-CITYLEARN-CONTROL-001: pinned baseline panel v0.1

## Intent

This is the first executable closed-loop benchmark panel for an external energy simulator. It deliberately starts with four official fixed policies rather than claiming Symtropy is already better than the industry. The goal is to establish a reproducible floor with pinned inputs, exact time windows, native simulator KPIs, decision-latency measurements, action receipts and no forecast-feature leakage.

## Data and source pin

- Repository: https://github.com/citylearn-project/CityLearn
- Release tag: \`v3.0.2\`
- Source commit: \`834575c1a0194c8ae9d648ae858376a94dfceb78\`
- Dataset: \`citylearn_challenge_2022_phase_all_plus_evs\`
- Schema Git blob SHA-1: \`5d58d31fd75e4845d73d319c69df34d64223f533\`
- 17 buildings, 8,760 hourly steps from 2024-08-01, 30 CSV files.
- Machine-readable contract: \`tools/energy-dispatch-audit/fixtures/energy-citylearn-control-benchmark-protocol-v0.1.json\`
- Source integrity: PR #2821's \`energy_citylearn_source_snapshot_audit\` must verify the exact source commit, schema and all 30 data files, calculate logical CSV rows and emit raw-content SHA-256 receipts.

The control run is blocked unless the operator supplies a dataset-terms-review acknowledgment and reference. The acknowledgment is a recorded gate, not a legal conclusion by the benchmark code. The source CSV files are fetched into a transient runner checkout and are not added to the Symtropy repository or uploaded in benchmark artifacts.

## Baselines

| ID | Official policy | What it measures |
|---|---|---|
| \`passive_no_control\` | \`citylearn.agents.base.BaselineAgent\` | Passive/no-active-control reference. Not operational business-as-usual. |
| \`business_as_usual\` | \`citylearn.agents.baseline.BusinessAsUsualAgent\` | Conservative operational behavior for EV charge targets, deferrable appliances and PV self-consumption. |
| \`basic_rbc\` | \`citylearn.agents.rbc.BasicRBC\` | Fixed hour-of-use rule controller. |
| \`optimized_rbc\` | \`citylearn.agents.rbc.OptimizedRBC\` | Published optimized rule-map reference. It is treated as off-the-shelf, not retuned on held-out data. |

The benchmark adapter is the only Python layer because CityLearn's simulator and reference agents expose a Python API. It is a research/CI integration shim, not production control code. The benchmark protocol, source audit, digest contracts and independent feasibility verification remain in Rust.

## Chronological windows

| Window | Inclusive steps | Count | Use |
|---|---:|---:|---|
| Early block | 0–5,255 | 5,256 | Early-year reference |
| Middle block | 5,256–6,131 | 876 | Middle-year reference |
| Held-out block | 6,132–8,759 | 2,628 | Frozen final comparison |

All four policies run on all three windows using fresh environment instances and the same fixed seed. They are not tuned in this panel. Each window starts from CityLearn's default reset state; state is not carried between windows. This avoids mismatched test setup but means the three windows are independent reset episodes, not a single continuously-operated annual battery trajectory.

Because the dataset spans one simulated year only, the held-out block is exploratory within-dataset evidence—not multi-year seasonal generalization and not SOTA proof.

## Causal feature isolation

The adapter extracts every top-level observation name containing \`_predicted_\` or ending in \`_predicted\` from the pinned schema and disables those observations in CityLearn before agent creation. After each agent initializes, the adapter fails closed if any prediction-named observation remains in the agent's observation registry.

This is intentionally stricter than assuming column names imply forecast issue-time semantics. In v0.1, no reference agent may consume the dataset's prediction-named inputs. Future forecast-based controllers require their own evidence for forecast issue time, publication time, target time and decision availability.

## Metrics and evidence

The harness runs \`CityLearnEnv.evaluate_v2(include_business_as_usual=False)\` after each controller/window episode and preserves the native KPI rows instead of guessing KPI formulas or collapsing everything into one scalar. It separately records:

- controller prediction latency p50/p95/p99, measured around \`predict\` only;
- simulation-loop wall time and process peak RSS;
- count of steps actually completed;
- per-controller/window SHA-256 of the full action trace;
- full benchmark action-trace digest, source-audit report digest, protocol digest, Python version, resolved package inventory and source commit.

The native KPIs should be interpreted according to the exact CityLearn version and level they describe (building vs district). Do not claim that a policy prevents essential-load outages if the simulator's base building services are guaranteed independently of that policy. A CityLearn result is a simulator result, not a physical feeder power-flow certificate or a South African field reliability result.

## Promotion gates before a controller superiority claim

1. Source terms are reviewed for the intended use and the runner receives the review reference.
2. Source commit, all Git blob identities, raw CSV SHA-256 values, row counts and file sizes pass.
3. The CityLearn package version is exactly 3.0.2 and its runtime environment is recorded.
4. The three windows × four baseline matrix completes with zero predicted-named observations exposed and with 12 reproducible action traces.
5. Raw native KPI rows and timing/memory measurements are preserved.
6. A Symtropy policy is evaluated with the same environment and split setup, but any parameter selection is confined to the early/validation blocks; the held-out block is evaluated once.
7. The comparison uses paired windows/seeds, uncertainty analysis and a multi-objective scoreboard. Critical physical constraints remain hard gates. Beating a weak rule baseline alone is not SOTA evidence.

## Known limitations

- No baseline results are claimed before a rights-reviewed manual run completes.
- No raw external dataset bytes are committed to Symtropy.
- The CityLearn schema's forecast issue/publication times are unverified; forecast-named observations are disabled.
- No physical microgrid simulation, grid power-flow audit, measured field result or SOTA result is produced by this panel.
