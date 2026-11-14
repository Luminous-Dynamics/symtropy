# CA-RCPSP Published Result Lower-Bound Audit v0.1

**Status:** source artifacts are mirrored and hash-pinned; a Rust auditor is authored, but Rust compilation/test execution remains unverified until the exact-head workflow finishes.
**Tracking:** [HABITAT-CA-RCPSP-001 #1950](https://github.com/Luminous-Dynamics/symtropy/issues/1950)
**CLI:** `cargo run --locked --manifest-path tools/habitat-calendar-oracle/Cargo.toml -- --external-audit`

## Why this audit is necessary

The CA-RCPSP dataset describes 12 synthetic scheduling instances from 5 to 1,000 tasks, with task durations, a structured work calendar, resources and disruption scenarios. The source CSV labels its output column `makespan_hours`. The paper characterizes its three solver result tables as descriptive baselines intended to demonstrate data-format compatibility, not a definitive ranking of solvers. See the [2026 Data in Brief article](https://doi.org/10.1016/j.dib.2026.113256), [data DOI](https://doi.org/10.5281/zenodo.20553817) and [upstream repository](https://github.com/aftab-fr/carm-plus-dataset).

A basic consistency check should precede any performance comparison: if a row's value is truly the whole-project makespan measured in the same work-hour units as task durations, it cannot be less than the duration of the longest required task. If the reported value is instead a different metric, the metric/units must be explicitly identified before it is treated as makespan.

## Reproduced source-level finding

An independent source scan parsed all 12 original JSON instances and the full deposited results CSV. The Rust auditor embeds the same upstream inputs unchanged, pins their SHA-256 digests, recomputes task-duration bounds from source JSON, joins results by `scenario_id`, and retains all solver rows in its report.

| Audit measure | Observed value |
|---|---:|
| Source instances | 12 |
| Declared tasks across those instances | 2,819 |
| Solver result rows in upstream CSV | 36 |
| Rows labelled successful | 30 |
| Rows labelled failed/timeout | 6 |
| Successful rows whose `makespan_hours` is below the longest task duration | **30 / 30** |
| Declared `expected_performance.makespan_hours` values below longest task duration | 0 / 12 |
| Successful rows reporting calendar compliance below 1.0 | 18 / 30 |

This is a systematic **metric/units/reproducibility warning**. It is not yet a diagnosis of why the published solver rows are small, and it should not be presented as proof that any named solver itself is incorrect. The audit deliberately reports every value and its lower bound without modifying the upstream data.

The independently published [`calrcpsp` package documentation](https://pypi.org/project/calrcpsp/) gives the manufacturing example's T006 task as 6.93 work-hours, scheduled from 08:00 to 15:55:48 with lunch excluded, while the deposited CSV reports a 1.84-hour Max-Plus result for that scenario. That provides an independent reason to reconcile the CSV's semantics with its label.

## Source and license provenance

Original JSONs and the full results CSV are mirrored under `docs/research/external/ca-rcpsp-v1.0/`. The directory README records attribution and the upstream repository's [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) license declaration. The auditor pins exact bytes for each of the 12 JSON instances and the CSV; if a byte changes, it rejects the corpus rather than continuing with stale expectations.

This mirror is intended to make the audit reproducible. It does not elevate the source solver-output CSV into ground truth or authorize a solver leaderboard.

## Acceptance contract

The Rust audit report must assert all of the following before it can be called complete:
- each of the 12 source JSON byte streams and the CSV matches its pinned SHA-256;
- each source has a unique scenario ID, unique task IDs, positive durations, matching task-count metadata and a declared expected makespan;
- all 36 CSV rows parse under the pinned format, join to one source scenario, and preserve the source success/timeout state;
- each successful row has a makespan and calendar-compliance metric; each timeout row has no makespan;
- 30 lower-bound violations and 0 expected-performance lower-bound violations are reproduced; and
- the report retains the discrepancy and warns that metric semantics must be reconciled before solver quality can be compared.

The workflow passes when the audit itself is reproducible—not because the upstream metrics pass the lower-bound check. A completed audit with violations is a valid negative finding.

## Next research

1. Ask which objective the original CSV values actually represent and reconcile them with task-level durations, calendars and `expected_performance`.
2. Reproduce at least the three tiny cases from raw instances using a separately implemented, calendar-valid schedule validator.
3. Compare solvers only after the result metric, time units, work-versus-elapsed-time convention, calendar compliance and objective definition agree.
4. Extend from this source audit to a genuine adapter and differential benchmark. Do not infer a general scheduling win from a descriptive baseline table.

All inputs are synthetic manufacturing data. Nothing in this audit represents real habitat construction performance, engineering approval or safety authority.
