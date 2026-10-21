# EV factory evidence-aware KPI contract v0.1

**Benchmark:** EV-FACTORY-KPI-001  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2460  
**Implementation:** tools/ev-passenger-cost-frontier/src/bin/ev_factory_kpi_contract.rs  
**Status:** synthetic metrics contract; informative ISO 22400 crosswalk only

## Paradigm shift: a KPI must carry its measurement contract

A production dashboard can be misleading when ratios with different denominators are presented as comparable, missing data is treated as zero, or a station-state fraction is renamed "availability" without a planned-time model. Every metric emitted here includes a stable formula ID/version, unit, reference window, numerator and denominator definitions when computable, source field paths, evidence class, computability status, and interpretation limits.

The contract intentionally distinguishes two states:
- **Computed:** formula inputs were available and passed basic validation.
- **Not estimable:** the metric is meaningful, but required evidence is missing. The value and distribution are null, with a reason; this is not zero.
- Invalid input reports are rejected rather than returned as not-estimable metrics.

## Research and standards alignment

ISO 22400-1 provides an industry-neutral framework for defining, composing, exchanging and using manufacturing operations-management KPIs. ISO 22400-2 specifies KPI concepts with reference-time, production, logistics, quality and energy elements. ISO 22400-1:2014 was reviewed and confirmed in 2025 as the current edition.
- ISO 22400-1: https://www.iso.org/standard/56847.html
- ISO 22400-2: https://www.iso.org/standard/54497.html
- NIST's KPI hierarchy paper notes that production KPIs are interrelated and should not be treated as independent standalone measurements: https://www.nist.gov/publications/hierarchical-structure-key-performance-indicators-operation-improvement-production
- NIST's 2026 manufacturing digital-twin report identifies VVUQ, interoperability, cybersecurity and workforce readiness as cross-cutting priorities: https://doi.org/10.6028/NIST.IR.8620

This tool borrows the idea of explicit definitions and reference periods; it does not reproduce standard text, use an exhaustive ISO model, or claim official conformance.

## Computable in the current synthetic DES report

| KPI | Unit / window | Source and caveat |
|---|---|---|
| Good output per hour | good units/hour, 20 replications after warm-up | Per-seed baseline good output divided by measured time |
| Linearized annualized output | good units/year, 20 replications | Throughput × declared productive seconds/year; assumes constant rate, not a demand or production forecast |
| Total scrapped jobs | jobs, 20 replications over full horizon including warm-up | Count only; not a quality yield ratio |
| Rework attempts | attempts, 20 replications after warm-up | Count only; first-pass yield is separately computed from first-attempt outcomes with job linkage |
| Per-seed median and P95 good-job lead time | seconds/job, distribution across seed-level quantiles | Uses the eligible job-release cohort and final-good transfer acceptance timestamp; jobs still open at horizon are reported as right-censored |
| Right-censored jobs at horizon | jobs, 20 replications | Nonterminal eligible jobs at the horizon; never imputed as completed |
| Per-station observed first-pass yield | fraction, seed-level ratios | Unique jobs first attempted after warm-up; denominator uses observed first-attempt outcomes only, with unresolved attempts disclosed separately |
| Busy / blocked / starved / down fraction | fraction of server-seconds, primary seed only | Uses the simulator's all-server-seconds denominator; not planned-time availability, utilization under a plant's calendar, or OEE |
| Average source-buffer / supplier-backlog WIP | units, primary seed only | Time-average inventory/queue level in the measured horizon |
| Average interstation-buffer occupancy | units, primary seed only | Time-average finite-buffer WIP; no replication interval in v0.1 |
| Changeover count, output-transfer count, out-of-order transfer proxy | counts, 20 replications after warm-up | Station-level synthetic counters; job-ID order inversions are a coarse sequence proxy |

All computed metrics identify the exact source catalog SHA-256 and simulation report SHA-256. Distribution summaries report sample size, mean, sample standard deviation, empirical P05/P50/P95, min/max and a Student-t mean interval when at least two independent seed-level observations exist. The primary-seed station-state/WIP values explicitly have sample count one and no confidence interval. The event ledger is retained for the primary-seed baseline only; the other seeds and station interventions still contribute to per-seed lifecycle summaries.

## Intentionally not estimable yet

The report explicitly returns null/not-estimable for:

- **OEE:** requires compatible planned production time, ideal cycle reference, total quantity and good quantity over one common defined window.
- **Scheduled-time equipment availability:** requires planned production calendar and a declared planned-maintenance inclusion rule.
- **Performance efficiency versus ideal cycle:** current cycle inputs are synthetic model parameters, not approved ideal-cycle standards.
- **First-pass yield:** now computable per station when the job lifecycle ledger validates; it is an observed first-attempt fraction and unresolved first attempts are counted separately. It is not final yield.
- **Energy per good unit:** no meter or validated station energy model in the DES output.
- **Good units per paid labor hour:** no worker roster, labor-role scope, or paid hours.
- **Audited cost per good unit:** no verified labor, tariff/FX, material, scrap, overhead or maintenance cost inputs.
- **Lead-time uncertainty:** the ledger enables per-seed median/P95 good-job lead time with explicit right-censoring. This is still model-conditional and is not validated lead time from a physical production system.
- **On-time delivery rate:** no customer order/due-date cohort or promised timestamp.

The tool does not invent OEE by multiplying the current state fractions; those denominators do not match a standard OEE formula. Nor does it label the synthetic mean cycle seconds as an approved ideal-cycle reference.

## Run

Use cargo fmt and cargo test with manifest path tools/ev-passenger-cost-frontier/Cargo.toml, then run the ev_factory_simulation binary against the versioned synthetic catalog to create a report. Pass the exact catalog file and resulting report to the ev_factory_kpi_contract binary.

## Acceptance gates

- The KPI CLI rejects a report whose catalog digest does not match the exact catalog bytes.
- Every computed KPI includes its formula ID, unit, reference window, source fields, evidence class and computability status.
- Missing prerequisites yield value=null, distribution=null and a reason; not zero.
- The same input files produce a byte-identical KPI report.
- No KPI output can authorize quality, machinery, site, local legal or production release.

This is a provenance-and-measurement contract, not a statement of ISO conformance or industry-leading performance. All sample reports are synthetic; no manufacturing data was collected and no supplier, operator, integrator, landlord, manufacturer or agency was contacted.
