# Ocean Compute Workload Benchmark v0.1

**Issue:** [OCEAN-COMPUTE-WORKLOAD-001 #6319](https://github.com/Luminous-Dynamics/symtropy/issues/6319)  
**Parent references:** [Subsea compute industry benchmark #6280](https://github.com/Luminous-Dynamics/symtropy/issues/6280), [Ocean industrial symbiosis #2100](https://github.com/Luminous-Dynamics/symtropy/issues/2100)  
**Status:** synthetic research fixture; no measured marine workload has been admitted  
**As of:** 20 November 2026

## 1. Objective

A maritime data centre should be sized against useful marine services, not speculative accelerator throughput or renewable surplus. This benchmark maps service obligations to data volume, compute, network, cooling, latency, deadlines, flexibility, locality and evidence requirements.

It does not implement a scheduler or certify a deployment. It composes existing owners: #6211 for job-level flexibility and no-new-peak admission, #6228 for cooling/cable limits, #6209 for decision-time marine replay, Mycelix #6149 for rights-aware ocean-observation lineage, and Mycelix #6343/#6344 for shared-port service equivalence.

## 2. Initial service catalogue

The companion JSON contains five synthetic workload profiles:

| Service family | Synthetic service unit | Placement hypothesis | Flexibility |
|---|---|---|---|
| Ocean observation / environmental MRV | QC-qualified observation record | onboard, port edge, coastal, nearshore | bounded deferral |
| Offshore renewable operations | turbine-hour condition assessment | port edge, coastal, nearshore, cloud | bounded deferral |
| Port and vessel services | 15-minute berth/shore-power forecast | port edge, coastal | bounded deferral |
| Marine robotics / survey processing | reviewable inspection tile | onboard, port edge, coastal, nearshore, cloud | migratable |
| Scientific and industrial batch | completed hindcast ensemble member | coastal, nearshore, subsea hypothesis, cloud | deferrable |

All five profiles explicitly set the safety-critical-control dependency to false. They are decision-support, monitoring or batch workloads. Safety-critical vessel, port, turbine, grid and robot control stays in qualified local systems.

## 3. Synthetic energy-accounting check

The JSON uses deliberately invented per-batch values so that accounting and service-equivalence checks can be built before measured traces are available. In the fixture:

| Profile | Compute (Wh) | Network (Wh) | Cooling (Wh) | Total (Wh) | Units per batch |
|---|---:|---:|---:|---:|---:|
| Wave observation QC | 120 | 15 | 18 | 153 | 1,000 |
| Offshore wind condition analytics | 900 | 60 | 135 | 1,095 | 24 |
| Port berth-demand forecast | 350 | 35 | 52.5 | 437.5 | 96 |
| AUV/ROV inspection reconstruction | 12,000 | 4,500 | 1,800 | 18,300 | 100 |
| Ocean-science hindcast ensemble | 50,000 | 12,000 | 7,500 | 69,500 | 10 |

The only arithmetic invariant is:

**batch total = compute + network + cooling**

Energy per service unit is total batch energy divided by accepted service units. A dropped output, lower-quality product, or missed deadline is not an energy saving. These values are synthetic-only test inputs and must never be cited as measured performance or used to size real infrastructure.

## 4. Placement strata to compare

- **S0 — onboard/local:** sensor or robot preprocessing and any safety-critical control.
- **S1 — port edge:** berth, terminal and local operational analytics.
- **S2 — coastal/regional:** regional storage, model inference and batch/HPC.
- **S3 — floating/nearshore:** serviceable above-water compute modules with separately qualified marine heat rejection.
- **S4 — subsea:** optional pressure modules only for workloads whose connectivity, retrieval, safety and lifecycle economics justify them.
- **S5 — external cloud/federated compute:** only where source rights, data locality, security and egress permit.

A workload may be split across strata, but data transfer, replication, network energy, cooling and failover must all be counted.

## 5. Frozen stress cases

The fixture contains eight adversarial cases: nominal operation; storm-driven observation surge; renewable-surplus scheduling; PCC-cap/rebound violation; fibre partition; unknown source QC/calibration; denied data rights/egress; and cooling/heat-sink unavailability.

Expected outcomes are specified as dispositions and assertions, not as simulated field results. In particular:
- renewable-following must not create a new PCC peak or rebound peak;
- local essential analytics must not silently depend on remote compute;
- unknown QC/calibration must not become an environmental conclusion;
- rights denial must block prohibited placement and egress;
- no heat-recovery credit is awarded without a named, measured sink.

## 6. Required baselines and measurements

Compare source-native/local processing, port-edge, conventional coastal data centre, floating/nearshore, subsea and hybrid placement under the same service traces, hardware/software identity, quality gate and deadline.

Report:
- accepted service units and quality, not raw throughput alone;
- p50/p95/p99 end-to-end latency, deadline success, backlog age, data loss and recovery;
- measured compute, network and cooling kWh separately;
- energy and lifecycle cost per accepted service unit;
- transfer bytes, locality, replication and fibre-failure behaviour;
- deferrable share and deadline-preserving shift window;
- source provenance, timebase, units, QC, calibration and rights coverage.

The scorecard is a Pareto surface with hard gates, not one weighted “ocean compute” score. Direct comparisons are invalid when service, hardware, boundary, workload quality or evidence class differs.

## 7. Research targets (not achieved results)

- 100% source/workload identity, units and provenance validation on the frozen conformance corpus.
- Zero remote dependencies for safety-critical control.
- Zero unauthorized data egress or purpose violations.
- 100% service-equivalent accounting across scored placement baselines.
- Zero PCC-cap, rebound or thermal-limit violations in admitted synthetic cases.
- For at least one real, rights-cleared service family, test the hypothesis of at least 10% lower measured energy per accepted service unit against a matched baseline; report uncertainty and failed cases.
- Demonstrate renewable-following flexibility only for bounded-deferrable work, with no missed deadlines or new peak.

These are falsifiable research goals, not promises of commercial advantage.

## 8. Real-evidence admission sequence

1. Keep the current fixture synthetic and immutable.
2. Review Mycelix #6149 for one OOI/ERDDAP series only after dataset ID, metadata, variables, units, QC, deployment, timebase, calibration and reuse terms are admitted.
3. Qualify South African CSIR WaveNet Ngqura and Durban observations independently; a public plot is not a raw, calibrated, rights-cleared trace.
4. Obtain operator-authorized offshore-wind, port, vessel, AUV/ROV and data-centre traces with synchronized clocks and metering boundaries.
5. Only then estimate real workload distributions, service demand, market size or site capacity.

## 9. Claim ceiling

The fixture can test schema, arithmetic, disposition and service-equivalence logic. It does not show real marine demand, energy efficiency, customer willingness to pay, data-centre performance, ecological safety, site suitability, grid connection, or SOTA superiority.

## 10. Placement eligibility evaluator — 20 November 2026

The first executable comparison slice is now present on branch `research/ocean-compute-workload-v0-1`:

- `docs/research/port-energy-compute/OCEAN_COMPUTE_PLACEMENT_MATRIX_V0_1.json`: 30 synthetic placement candidates (five profiles × six strata).
- `tools/ocean-compute-placement-verifier/`: standalone Rust crate using Serde/serde_json; no production Python dependency.
- The verifier rejects placements that violate allowed locality, availability, source rights, egress permission, output quality, exact accepted service units, p99 latency, completion deadline, PCC peak cap, required local fallback, safety-control isolation or energy closure.
- Eligible candidates are compared on a Pareto frontier across energy/service, carbon/service, cost/service, p99 latency and completion time. No opaque weighted score is produced.

### Independent static cross-check

The committed JSON was retrieved from the branch and independently re-evaluated against the profile contracts. All 30 candidate identities were unique; all candidate profile references resolved; all energy component sums closed; the fixtures remained synthetic-only; all five profiles had at least one eligible candidate.

The independent cross-check found these eligible candidate counts:

| Service profile | Eligible candidates | Count |
|---|---:|---:|
| Wave observation/QC | 3 of 6 | 3 |
| Offshore wind condition analytics | 3 of 6 | 3 |
| Port berth-demand forecast | 1 of 6 | 1 |
| AUV/ROV inspection reconstruction | 4 of 6 | 4 |
| Ocean-science hindcast ensemble | 3 of 6 | 3 |

Adversarial controls rejected: missing local fallback, a synthetic PCC-cap violation, source-rights and data-egress denial, failed quality, unapproved placement strata, unavailable candidate and a deadline miss.

**Verification limitation:** the independent JSON/logic cross-check was run, but the Rust crate has not been compiled or tested in this environment because `rustc` and `cargo` are unavailable. The crate includes focused unit tests; no CI result or executable Rust pass is claimed.

## 11. Current external metrology and flexibility references

- ISO/IEC 30134-2:2026, edition 2, published 16 January 2026, is the current PUE measurement reference. Its revised scope includes updated measurement boundaries, on-site generation accounting, mixed-use PUE and treatment of unaccounted energy. ISO lists Technical Corrigendum 1 as under publication after the 10 November 2026 stage update: https://www.iso.org/standard/30134-2 and https://www.iso.org/standard/95746.html. Pin the applicable text/corrigendum when a measured facility campaign is designed.
- ENTSO-E's April 2026 data-centres and power-system report classifies AI/ML training and batch/data processing as higher time/geo-shift potential than inference, web/SaaS or transactional services. This is a useful workload taxonomy, not a universal measured flexibility percentage: https://eepublicdownloads.blob.core.windows.net/public-cdn-container/clean-documents/Reports/2026/FINAL_ENTSO-E_Data_Centres_260430.pdf.
- ASHRAE's grid-interactive data-centre guidance distinguishes temporal and spatial flexibility and discusses reserve/energy-arbitrage mechanisms; local electrical protection remains separate from workload orchestration: https://www.ashrae.org/technical-resources/topics-and-initiatives/ai-data-center/grid-interactive-design-demand-flexibility.

## 12. Competitor status evidence note — Shanghai Lingang

Keep separate fields for project capacity, commissioned phase capacity, actually delivered IT load, design PUE and measured PUE.

- Shanghai municipal portal, 10 February 2026: reports the 2.3 MW first phase operating and describes 24 MW as total planned capacity across two phases: https://touch.shio.gov.cn/jsp/jjxw_detail_en.jsp?id=20260210183337707.
- Data Center Dynamics, 20 May 2026: reports 24 MW full commercial operations: https://www.datacenterdynamics.com/en/news/hiclouds-offshore-wind-powered-underwater-data-center-up-and-running-off-coast-of-shanghai-china/.

These reports can represent a later commissioning update, but the as-built phase schedule, commissioning acceptance and independently metered IT load remain unverified in this evidence set. Preserve the two claims with their dates and source classes; do not silently turn reported 24 MW project capacity into independently verified delivered IT load. Similarly, PUE ≤1.15 remains a design/project claim until measured using the pinned current standard and stated boundary.
