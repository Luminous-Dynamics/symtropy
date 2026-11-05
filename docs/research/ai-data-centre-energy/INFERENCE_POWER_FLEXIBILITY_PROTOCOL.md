# Inference Power–Service Flexibility Envelope Protocol

**Protocol:** GRID-INFERENCE-POWER-QOS-001  
**Version:** 0.1  
**Status:** paired offline measurement evaluator; no GPU controller, workload scheduler, or facility actuation

## 1. Objective and paradigm shift

A data-centre inference control is not a flexibility resource merely because GPU power falls under a cap or clock change. It is only a qualified flexibility point when the same useful request obligations remain completed under owner-defined service constraints and the power effect is measured at an explicit, calibrated boundary.

The desired object is a **multi-dimensional response surface**:

    control action × workload/model × hardware × operating stratum
      -> (boundary kW trajectory, response delay, sustain, recovery,
          useful tokens, TTFT/latency distributions, failures, tokens/J)

Keep the Pareto frontier. Do not compress severe tail-latency regressions, lost useful work, or rebound into a single weighted score.

## 2. Research and industry baselines

### Argonne JLSE Data Center Flexibility Dataset

Argonne's current JLSE project description says no public dataset presently answers how much power an AI data centre can shed, how fast, and at what service-quality cost. It plans inference experiments on B200/H100 nodes, sweeping GPU clock, GPU/CPU power caps, batch size and request concurrency, collecting GPU/CPU/DRAM power-energy traces, clock/utilization/temperature and per-request TTFT, latency and tokens/s. It plans to fit power-flexibility and performance-flexibility functions:
https://www.jlse.anl.gov/projects/benchmarking/data-center-flexibility-dataset

This is a project specification, not a released dataset and not a result we claim to have reproduced. V0.1 establishes the ingestion and evaluation contract now; add a version-pinned adapter when the data are public or owner-authorized equivalent measurements are available.

### MLPerf Power

The June 2026 IEEE Micro paper presents cross-scale reproducible power measurement, including samples/J and tokens/J, and reports 1,841 reproducible submissions from 60 systems. Use MLPerf Power as an energy-efficiency comparator only when workload rules, hardware and measurement boundary match. MLPerf Power does not itself qualify dispatch response, service-preserving load shedding or rebound:
https://ieeexplore.ieee.org/abstract/document/11551311

### Operational inter-data-centre migration study

The 22 June 2026 ACM Sustainability Week field demonstration measured LLM inference workload migration across commercial data centres. It found pre-launched read-only patterns fastest with least downtime; on-demand copy was strongly bandwidth-sensitive (401 seconds at 1 Gbps best effort vs 23 seconds at 10 Gbps guaranteed in its experiment). The authors distinguish measured response-time feasibility from complete market qualification:
https://doi.org/10.1145/3765611.3815063

This is a migration/response benchmark, whereas the Argonne JLSE work targets accelerator power-control sweeps and QoS. Keep them as complementary evidence sources.

## 3. V0.1 matched experiment contract

### Frozen identity

Pin:
- hardware SKU and node ID;
- workload and model digest;
- runtime/driver digest;
- exact request-manifest digest;
- condition stratum (ambient, cooling state, initial load/thermal condition);
- baseline and candidate run IDs;
- declared measurement boundary ID and boundary level;
- meter calibration evidence ID.

The caller-supplied digests are provenance labels; the module does not authenticate the source files or measurement device.

### Request obligations and outcomes

Each request obligation has a stable request ID and expected useful output-token count. Both baseline and candidate must contain exactly one result for every request, with no missing, extra or duplicated IDs.

Each result reports success/failure, output tokens, time-to-first-token and completion latency. Candidate service metrics include:
- request success fraction;
- useful-work completion fraction (completed output tokens capped at the manifest obligation);
- total output tokens and useful tokens;
- useful tokens/s, raw tokens/s and useful tokens/J;
- nearest-rank p95/p99 TTFT;
- nearest-rank p95/p99 completion latency.

The baseline must satisfy the service contract before comparison. The candidate must satisfy absolute success/work/throughput/latency thresholds and relative throughput-retention and p99-regression bounds. A failed service gate is reported; it cannot be traded against kW or energy.

### Power and telemetry

Each power interval includes:
- interval-average authoritative boundary power in W;
- separately metered GPU, CPU and DRAM power in W;
- GPU clock, GPU/CPU utilization and GPU/DRAM temperatures;
- exact elapsed start and duration.

The boundary reading is authoritative. GPU+CPU+DRAM values are diagnostics and are not summed into the boundary power unless an independently documented meter topology explicitly proves they are non-overlapping. Baseline and candidate time axes must match exactly and be contiguous.

Boundary energy is integrated as:

    E_boundary[Wh] = Σ P_boundary[W] × duration[ms] / 3,600,000

Useful tokens/J is reported separately from boundary kW flexibility. Useful tokens/J and useful tokens/s use completed useful tokens, capped at each request's obligation; output from failed requests is diagnostic and does not count as completed useful work. Raw output tokens, including partial failed outputs, remain a separate diagnostic. Minimum-throughput and throughput-retention gates use useful tokens/s, not raw output tokens/s.

### Response and recovery

Signed reduction is:

    ΔP(t) = P_boundary,baseline(t) - P_boundary,candidate(t)

Positive means reduced import; negative means increased import/rebound. Gross reduction Wh, rebound Wh and net boundary energy reduction Wh are separate.

A response is qualified only when:
- the reduction exceeds a configured minimum W;
- it meets the configured consecutive-sample requirement;
- the same reduction persists for the minimum elapsed sustained duration;
- total recovery/rebound power and energy remain within explicit owner-defined bounds;
- candidate service gates pass.

Any post-response rebound remains in the report and is checked independently against maximum rebound power and energy. A sustained reduction may still fail the response envelope because recovery is excessive. A short power dip does not qualify as sustained response. The stabilization timestamp is when the response sample-count and duration thresholds are first both met.

## 4. V0.1 implementation scope

The Rust module `inference_power_flexibility.rs` implements:
- frozen experiment identity and typed control action;
- exact request-obligation matching across baseline/candidate runs;
- per-request success, useful tokens, TTFT and completion latency;
- nearest-rank p95/p99 metrics and hard/relative service gates;
- exact contiguous baseline/candidate power trace alignment;
- boundary energy, component energy diagnostics, average power and tokens/J;
- signed per-slot power reduction, gross energy reduction and rebound;
- minimum consecutive-sample and elapsed-duration response qualification;
- explicit claim ceiling tied to declared boundary type.

It does not implement GPU control, telemetry collection, network ingestion, calibration, scheduling, facility power flow or grid dispatch.

## 5. Benchmark targets

Proposed engineering targets, not formal standards or achieved results:

- 100% agreement with independent known-truth arithmetic.
- Zero accepted candidates with a missing/extra/duplicate request.
- Zero service-equivalent credit when a hard p95/p99, success, useful-work or throughput guard fails.
- Zero unreported negative power deltas or rebound energy.
- Exact paired trace time-axis alignment.
- p95/p99 computed using a declared nearest-rank definition and accompanied by request count.
- Report useful tokens/J and raw tokens/J separately.
- Paired repeated-run bootstrap confidence intervals across condition strata before publishing a frontier point.
- Compare controls one at a time (GPU clock, GPU cap, CPU cap, batch size, concurrency) before combined action.
- Compare only identical meter boundaries; separately measure accelerator, server inlet, rack PDU, facility and PCC where available.
- No SOTA or market qualification claim until measurements on production-representative B200/H100 workloads and independent power meters are available.

## 6. Integration

- #3673 paired workload trajectory proves workload-level job obligations and full schedule shift.
- #3675 paired stateful resource trajectory reconciles battery/thermal terminal state.
- #3694 owner agent admits only an already-qualified capability offer and requires local controller review.
- #3649/#3652 validate dispatch response and repeated-event reliability.
- #3667/#3687 evaluate system adequacy with PRAS as the independent multi-region engine.

The inference evaluator is a lower layer than a grid-facing flexibility offer. Node-level evidence cannot be promoted to PCC capacity without boundary reconciliation.

## 7. Limits and next work

- Synthetic tests only; no actual JLSE data are present.
- Request/model/runtime hashes are caller inputs and not authenticated.
- Component telemetry is not a substitute for facility/PCC meters.
- Quantiles from a small synthetic request set are not statistically robust tail estimates.
- No uncertainty intervals or experimental randomization are implemented yet.
- No control actuator, safety guard, battery/thermal plant, network or market protocol is connected.

Next: ingest the Argonne JLSE dataset when released; add paired block bootstrap for QoS/power uncertainty; add power-meter calibration and time-synchronization audits; then integrate only service-qualified and boundary-reconciled points into owner-controlled offers.
