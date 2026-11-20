# Power restoration service trajectory audit v0.1

**Owner:** Symtropy #6449  
**Bridge contract:** #6235  
**Benchmark case:** #6347 / PR #6347  
**Run receipt:** #6352 / PR #6366  
**Service-cascade consumer:** CivDyn #3896  
**Programme:** #6320 / #5241

## Purpose

A completed repair task, a closed switch, an energized asset or a planner forecast is not proof that customers received electricity. This crate audits a bounded, source-linked service trajectory and independently recomputes the benchmark's priority-weighted delivered-energy objective.

It preserves four distinct streams:
1. planned forecast;
2. observed restoration work;
3. independently modelled service;
4. independently measured service.

Only modelled/measured service may support a delivered-service trajectory. Planned forecasts and observed work return explicit non-service statuses and cannot satisfy the run receipt's final service trajectory reference.

## v0.1 supported service model

Electricity only, native unit kWh per evaluation interval. For every load and every time step:
- demand denominator is derived from the frozen case active load kW multiplied by interval seconds / 3600;
- delivered kWh must be in [0, demand] with zero represented explicitly as unavailable;
- known and unavailable rows bind source evidence URI and SHA-256;
- unknown rows carry no numeric delivery and require an explicit reason;
- conflicting rows carry no numeric delivery or averaged value and require a reason plus at least two distinct source records;
- missing rows, duplicate load/step rows, noncanonical order, wrong demand, out-of-horizon timestamps and source-digest substitution fail closed.

For each load/tick, the bridge validates a single source-backed value or preserves missingness/conflict. It never silently fills missingness with zero and never averages conflicting meters.

The frozen objective is:

\[
E_{\mathrm{weighted}} =
\sum_{t=1}^{T}\sum_{i=1}^{N}
P_{i,t}^{\mathrm{delivered}}\,
\Delta t_{\mathrm{hours}}\,
w_i
\]

In v0.1 the case load kW is constant across the horizon, so each sample's delivered energy is already in kWh per interval. The scorer recomputes the sum of delivered_kWh multiplied by priorityWeight and compares the result with the exact case/run metric and weighted_kWh unit. A different objective profile is rejected rather than scored using an assumed formula.

## Time semantics

The case's declared interval and horizon are authoritative. The record carries:
- a named timezone;
- the first interval's Unix-epoch start;
- interval duration in seconds;
- a UTC offset per sample (allowing offsets to vary across daylight-saving transitions);
- the exact interval-start epoch expected for each step.

Unix epoch time is the unambiguous time authority. The timezone label is preserved as provenance; v0.1 does not consult an external IANA timezone database to independently authenticate that label. Ambiguous wall-clock timestamps must be converted to epoch time before submission.

## Producer evidence

- Modelled service requires a model ID, pinned producer source commit/configuration and source artifacts.
- Measured service requires a meter/instrument ID, calibration artifact and exact calibration digest in the source artifact inventory.
- Each known row's evidence reference/digest must occur in the same source inventory.
- Conflicting rows retain at least two distinct source artifact references/digests.
- Every source record is still a claim about a producer's method; signatures and digests authenticate identity/content, not physical truth or meter accuracy.

The service trajectory's parentServiceArtifacts must exactly equal the per-decision scored service artifacts in the qualified run receipt. A complete run's final service artifact URI/digest must equal the canonical trajectory record. The aggregate score must match the independently recomputed weighted delivered kWh.

## Incomplete runs and cascade export

An incomplete/aborted/timed-out run may still have a partial, fully mapped service trajectory. The report can expose that trajectory's native-unit totals, but it cannot produce a completed score match or a CIVDYN availability export.

Only a complete, reconciled modelled/measured trajectory can be exported. per_load_cascade_availability exposes per-load [0,1] fractions. aggregate_cascade_endpoint_availability additionally requires a separate, case-bound, digest-bound mapping from every feeder load ID to exactly one CIVDYN cascade node. Multiple feeder loads assigned to one endpoint are aggregated as delivered kWh divided by demand kWh at each interval; the bridge never assumes feeder load IDs equal cascade node IDs or applies a plain arithmetic mean. The mapping rows hash must match its source artifact digest. This is a typed adapter into the existing CivDyn service-cascade kernel; it does not reimplement dependency propagation.

Forecast/work streams, unknown/conflicting trajectories, incomplete runs or score-mismatched records cannot be exported as completed service inputs.

## Run

    cargo fmt --manifest-path tools/power-restoration-service-trajectory-audit/Cargo.toml --all -- --check
    cargo clippy --locked --manifest-path tools/power-restoration-service-trajectory-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --locked --manifest-path tools/power-restoration-service-trajectory-audit/Cargo.toml --all-targets
    cargo run --locked --manifest-path tools/power-restoration-service-trajectory-audit/Cargo.toml --bin power-restoration-service-trajectory-audit -- trajectory.json run-receipt.json benchmark-case.json [cascade-endpoint-mapping.json] [cascade-endpoint-mapping.json]

CLI exit codes:
- 0: complete modelled/measured service trajectory and aggregate weighted-energy score reconcile;
- 3: valid forecast/work-only, incomplete-run, or unknown/conflicting service record; no completed service claim;
- 1: malformed, contradictory or mismatched evidence;
- 2: report serialization failure.

## Tests

24 authored Rust tests cover:
- full measured-service weighted-kWh hand calculation, per-load cascade output and explicit multi-load endpoint aggregation;
- modelled versus measured producer roles;
- planned forecast and observed work non-promotion;
- unknown/conflicting data handling;
- exact load × interval coverage, order, timestamps and demand denominator;
- sample evidence, source inventory, calibration and parent-artifact binding;
- scorer mismatch, incomplete run and forecast-as-final negative controls;
- exact load-to-cascade mapping coverage, source-digest binding and demand-weighted multi-load endpoint aggregation.

Tests are authored, not considered passed until exact-head Rust CI executes.

## Claim boundary

This verifies the declared electricity-service trajectory, its source references and its relation to the benchmark objective. It does not prove metering truth, field measurements, net deliverability, balanced/unbalanced AC feasibility, short-circuit/protection, switching safety, operator authority or actual customer receipt of electricity. Those remain domain-specific independent qualification responsibilities.
