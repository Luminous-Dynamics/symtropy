# Maritime Industrial Benchmark Matrix v0.1

Companion to [MCSC #7686](https://github.com/Luminous-Dynamics/symtropy/issues/7686). This is a benchmark protocol, not a claim that any target has been achieved.

## 1. Adopt the industry's shared language; differentiate above it

| Reference | What it standardizes/measures | Programme treatment |
|---|---|---|
| [DCSA Port Call 2.0.0](https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope) | Standardized operational information exchange among carriers, terminals, ports and service providers. Its interface scope focuses on port-call planning through berth arrival; cargo operations are separate parallel processes. | Implement/adopt the standard and its conformance scenarios. Do not invent a second port-call API or claim that port-call conformance proves cargo delivery. |
| [IAPH/IHMA Port Call Optimization Guide](https://www.iaphworldports.org/pickup/21900/) | A minimum high-value nautical/operational data set, including terminal/berth identity and key planned times, with port-to-port trials. | Consume its semantics and identifiers. Differentiate through resource feasibility, authority-preserving replanning and measured outcomes. |
| [World Bank / S&P CPPI methodology](https://thedocs.worldbank.org/en/doc/aac122f6df85534428d66a7b9af4b7f6-0400012026/original/CPPI-Methodology-Note.pdf) | Port-level vessel-time index based on six port-call milestones and carrier records matched with AIS. | Preserve a CPPI-compatible port-time scorecard where the source data supports it. Add receiver-accepted service as a separate outcome, not a replacement score. |
| [UNCTAD TrainForTrade Port Performance Scorecard (2026)](https://unctad.org/system/files/official-document/rmt2026_en.pdf) | Global port benchmarking programme with more than 70 reporting port entities in 2026; published KPI families span financial performance, human resources, vessel/cargo operations, gender participation and environment (2016–2025). | Map to and report compatible PPS indicators alongside CPPI. Map to and report compatible PPS indicators alongside CPPI. Differentiate through a traceable per-unit receiver-acceptance chain, time-windowed productive capability and disruption recoverability; do not claim to replace UNCTAD's broader port scorecard. |
| [UNCTAD Review of Maritime Transport 2026, Chapter 4](https://unctad.org/publication/review-maritime-transport-2026) | Broad port-performance and trade-facilitation comparator: container-ship berth waiting in developing countries was about twice that in developed countries by end-2025; berth waiting accounted for 31% of the increase in port waiting time comparing the first seven months of 2019 and 2026; capacity at nine major ports grew 21% from 2019–2026 against 28% growth in trade volumes. It calls for continuity plans across ports and hinterland corridors. | Preserve berth waiting and port-capacity indicators; add receiver acceptance, inland corridor continuity, qualified local substitutes and time-to-recovery. Test rerouting and common-cause disruptions rather than assuming a port-level dwell improvement means goods reached the receiver. |

The 26 November 2026 edition also puts the resilience claim in a broader corridor context: berth waiting accounted for 31% of the increase in port waiting time between the first seven months of 2019 and 2026; cargo-handling capacity at nine major ports grew 21% from 2019 to 2026, against 28% growth in trade volumes; and the report calls for continuity plans agreed in advance across ports, inland routes, customs and neighbouring jurisdictions. This supports a benchmark based on service continuity across port and hinterland corridors, not just local berth-time optimization. Source: [UNCTAD Review of Maritime Transport 2026](https://unctad.org/publication/review-maritime-transport-2026), published 26 November 2026.

### CPPI-compatible metric boundary

CPPI's six timestamps are: arrival at anchorage/pilot station; movement to berth; start of cargo operations; end of cargo operations; departure from berth; exit from port limits. CPPI is a port-level benchmark, not an individual terminal audit. Do not fold receiver acceptance, inland delivery, post-berth repair or lifecycle maintenance into the CPPI number. Report those separately with their own denominators, sources and acceptance rules.

DCSA's current implementation guidance also points adopters to its conformance framework and sandbox. We should use the conformance scenarios as the interop acceptance gate; an internally successful adapter is not proof of DCSA conformance.

## Port digital-twin benchmarking and synchronization

A 2026 European port digital-twin benchmarking study reviewed seven port/logistics initiatives and found recurring interoperability and modular-architecture principles, but persistent gaps in data availability, model transparency, operational-performance reporting and synchronization mechanisms. [Digital twin architectures for smart port operations: Lessons from European benchmarking](https://doi.org/10.1016/j.rineng.2026.110485).

This creates a concrete comparative test for MCSC: do not merely assert that a port digital twin is integrated. Measure:
- event-time, first-available time, ingestion time and decision cutoff separately;
- cross-owner receipt and model-version synchronization;
- stale/late/revoked evidence admission;
- time from disruption event to admissible replacement plan;
- service-equivalent outputs preserved across owners;
- operational performance and uncertainty bounds on held-out events.

The 2026 NIST digital-twin workshops also emphasize VVUQ, interoperability, cybersecurity and workforce readiness. MCSC's claim ceiling remains explicit: a synthetically synchronized trace is not proof of a synchronized physical port twin.

## 2. Manufacturing intelligence and digital-twin assurance

The NIST comparator has advanced beyond a broad trustworthy-manufacturing agenda. Its **2026 Roadmap on AI and ML for Smart Manufacturing** covers industrial big-data, sensing, autonomous systems, robotics, supply-chain/logistics optimization, advanced digital twins, reliability/maintainability and safety. NIST's **NISTIR 8620 Digital Twins Workshops Summary Report** identifies interoperability, VVUQ, cybersecurity and workforce readiness as persistent adoption barriers. [Roadmap](https://doi.org/10.1088/3049-4761/ae5967) · [NISTIR 8620](https://doi.org/10.6028/NIST.IR.8620)

For maritime industrial capability, translate this into explicit cross-layer metrics: asset/process qualification, facility scheduling and resource evidence, site-level utility/capacity integration, enterprise supplier/repair network resilience, human approval/override quality, and continuous digital-twin credibility monitoring. This is a stronger comparator than model accuracy alone.

[NIST SI for Manufacturing](https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing) is a measurement-science comparator, not a product competitor. Its current research threads include trustworthy manufacturing intelligence, human-machine teaming, contextualized manufacturing data, integration test benches/benchmark studies and digital-twin verification, validation and uncertainty quantification (VVUQ). Its TIDAL framing connects asset-, site- and enterprise-level intelligence with human roles, operational systems and digital-twin foundations.

Programme requirement:

[NIST SI for Manufacturing](https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing) is a measurement-science comparator, not a product competitor. Its current research threads include trustworthy manufacturing intelligence, human-machine teaming, contextualized manufacturing data, integration test benches/benchmark studies and digital-twin verification, validation and uncertainty quantification (VVUQ). Its TIDAL framing connects asset-, site- and enterprise-level intelligence with human roles, operational systems and digital-twin foundations.

Programme requirement:
- Treat NIST's evaluation methods, metrics, datasets, test-bench concepts and standards work as a design constraint for our benchmark protocol.
- Provide an integration seam with owner and authority for every decision, source and observation.
- Separate model-predicted state, observed physical state, accepted output and human authorization.
- Calibrate model fidelity to the decision: no digital-twin claim without a declared validation domain, uncertainty budget and measurement trace.
- Compare human-machine teaming on inspectable work products, correction rate, safe overrides, time-to-competence and responsibility clarity—not only model task completion.

## 3. Published port-energy comparators

These results show why matched reproduction must precede any superiority claim. They are reported case-study outcomes, not universal port performance limits and not directly comparable to one another.

| Published method | Reported result in its study | Required reproduction |
|---|---|---|
| Li et al., *Microgrid energy optimal scheduling in ports incorporating the bidirectional event trigger mechanism*, Journal of Cleaner Production 568 (15 Jun 2026), DOI [10.1016/j.jclepro.2026.148666](https://doi.org/10.1016/j.jclepro.2026.148666) | Tianjin Dongjiang case: operating cost reduced 18.64% vs MPC and 22.44% vs a logistics event-triggered approach; reported online-solve-time reductions 88.90% and 77.76%; near-zero renewable curtailment. | Reproduce the day-ahead/15-minute real-time cadence, 1-hour reoptimization horizon, event triggers, forecast deviations, storage, tariffs and exact baselines. Match all input traces and solver budgets before comparing. |
| Li & Hu, *Integrated optimization of berth allocation, crane assignment, and onshore power supply with multi-source energy dispatch for green ports*, Journal of Cleaner Production 574 (18 Aug 2026), DOI [10.1016/j.jclepro.2026.149105](https://doi.org/10.1016/j.jclepro.2026.149105) | Integrated berth, crane, shore-power and energy dispatch; reports cost reductions up to 19.87% in its cases. | Match berth/crane/OPS decisions, energy sources, power tracing, carbon accounting, MILP linearization, constraints and all objective weights. |
| Wang et al., *An integrated robust framework for berth, yard, and shore power planning considering vessel arrival uncertainty*, Ocean Engineering 358 (15 Jun 2026), article 125850, DOI [10.1016/j.oceaneng.2026.125850](https://doi.org/10.1016/j.oceaneng.2026.125850) | Joint berth/yard/OPS plan under uncertain vessel arrivals; reports 4–13% case-specific cost reduction against deterministic planning using a min-max robust model and adversarial hierarchical multi-agent SAC. | Reproduce the vessel-arrival uncertainty set, yard/pre-stacking, OPS compatibility, carbon terms, exact deterministic baseline and training/compute budget. Report robustness vs out-of-sample empirical service, not just worst-case objective. |
| Zhang et al., *Scenario-driven day-ahead–intraday coordination of seaport virtual power plants under heterogeneous load uncertainty*, Sustainable Energy Research 13, 38 (11 Sep 2026), DOI [10.1186/s40807-026-00269-0](https://doi.org/10.1186/s40807-026-00269-0) | Reports 9.79% lower total operating cost and 52.11% lower maximum electricity-purchase deviation with storage; day-ahead/intraday coordination reports 18.4% lower execution-stage cost. Its source data are restricted and renewable profiles are forecast-driven in the main case. | Obtain rights-cleared source traces; reconstruct load scenarios, crane demand, storage parameterization, deviation penalty, forecast updates, and all three baseline cases. Do not compare with a different scenario set. |
| Li & Hu, *Collaborative optimization of berth scheduling and multi-energy dispatch for green ports*, Omega 143 (Sep 2026), DOI [10.1016/j.omega.2026.103549](https://doi.org/10.1016/j.omega.2026.103549) | Joint berth and multi-energy dispatch; reports 1.61–2.17% cost savings, 0.32–14.44% emissions reductions and zero renewable curtailment against two-stage scheduling in its reported cases. Includes logistics-driven load response, renewables, CCHP, electrical/thermal storage and bidirectional grid trading. | Reconstruct formulation, demand profiles, energy prices, renewable profiles, storage, emission factors and network constraints before using it as a baseline. |
| Yang et al., *Enabling grid-interactive data center-desalination coordination through thermo-electric coupling-based waste heat utilization*, Applied Energy 408 (1 Apr 2026), DOI [10.1016/j.apenergy.2026.127428](https://doi.org/10.1016/j.apenergy.2026.127428) | Reports 86.5% reduction in total annualized cost for its case; the authors attribute 17% savings specifically to waste-heat synergy. The full 86.5% must not be represented as a heat-recovery effect. | Reconstruct the thermo-electric and temperature-dependent RO model, two-stage stochastic planning, Benders/McCormick formulation, source/receiver temperature and heat-exchanger constraints, and baseline configuration. Require a real time-matched sink and verified water quality in any physical extension. |

**Attribution warning.** These results are reported outcomes from distinct case studies, not universal performance limits and not directly comparable across studies. The 86.5% total annualized-cost result is particularly easy to misuse: the paper's abstract attributes 17% to waste-heat synergy alone.

### Mandatory parity sheet for every paper reproduction

Freeze and publish: original source version; all data/input files and checksums; time horizon and discretization; vessel/cargo set; berth/crane/AGV/yard/OPS decisions; power/heat/fuel sources; tariff/carbon assumptions; operational constraints; uncertainty set and distribution; objective formula and units; solver/version; hardware; thread count; time limit; warm start; stopping tolerance; best bound/gap; random seeds; and exact baseline implementation.

If any item cannot be recovered, label the result a partial reconstruction, not a faithful reproduction. A 20% result on a different problem is not evidence of beating a paper's 19% result.

## 4. Shipyard scheduling baseline

[Symtropy #4204](https://github.com/Luminous-Dynamics/symtropy/issues/4204) owns the English/Chinese Mendeley ship-block data admission and faithful result reproduction. The English dataset record is [Mendeley Data, DOI 10.17632/2bmbmbf8hm.1](https://doi.org/10.17632/2bmbmbf8hm.1), metadata published 3 March 2026 and licensed CC BY 4.0. It contains block and block-position input data and result files for DDQN, EDDQN and five heuristics; the source says the data is adapted from real-world shipyard production data with anonymity/experimental adjustments.

The source assumptions include:
- minimum-bounding-rectangle representation;
- one block per position at a time;
- sufficient labour, with no absenteeism/labour anomalies;
- a suitable position assumed to exist for each block;
- each block completed at one position;
- all block positions empty at schedule start.

Those assumptions must be preserved for Layer A faithful reproduction. Only after Layer A and independent feasibility reconstruction pass should we run Layer C controlled extensions: non-empty initial occupancy, labour absenteeism/skills/shifts, lift/crane capacity, transport fleets, inspection holds, rework, material delay and machine outage. Each extension receives a distinct scenario-family label and is not mixed into the published baseline score.

## 5. Connection to the existing port-city golden trace

Mycelix [#6343](https://github.com/Luminous-Dynamics/mycelix/issues/6343) already owns the four-owner service-equivalent golden trace for shared PCC/feeder demand, shore power, terminal equipment, compute, cooling, desalination, thermal storage, common-cause risk and participant authority. Its synthetic v0.1 trace has four 15-minute intervals and includes a negative control where objective-only compute shifting creates a new PCC peak, alongside a no-new-peak candidate that preserves the same compute and vessel service energy. Its later fixture work expands to 48 expected dispositions, including source/clock/authority, heat, water, cable/common-cause and South African demand-response product controls.

MCSC must consume those owner-native traces rather than implement another PCC verifier, workload scheduler, RO solver, heat ledger or dependency graph. Geographic proximity of a private industrial energy asset is not evidence of a shared PCC, wheeling/export entitlement, reserve accreditation, grid availability, or dispatch rights. The Ngqura/Coega/Durban pilot must keep each connection boundary and utility/participant authority explicit.

## South African paired-site evidence acquisition protocol

Keep Ngqura/Coega and Durban as paired research environments, not a premature site competition. The first tests industrial integration and productive-capability formation; the second tests service continuity and constrained infrastructure. Neither site is an approved demonstrator until the relevant port, utility, participant, environmental and data-rights holders authorize the work.

### Public South African baseline: capacity estimates vs estimated shore-power demand

Two September 2025 CSIR/GIZ studies provide a useful published baseline for the paired-site design. Their values are useful for selecting stress cases and requesting the right data, but they are **not meter readings or connection offers**.

**Load Connection Capacity Assessment of South African National Ports** (CSIR/GIZ/PtX Hub, September 2025) is explicitly a pre-feasibility desktop assessment. It estimates:
- Ngqura: firm supply 25 MVA, NMD 6 MVA, estimated spare capacity 19 MVA, low connection-risk category.
- Durban: firm supply 17 MVA, NMD 13 MVA and estimated spare capacity 4 MVA, medium connection-risk category. The report recommends using the 4 MVA at lower distribution voltages first; if shore-power demand exceeds 4 MVA, a second dedicated feeder may be required.
- The connection-risk class is based on estimated spare capacity alone. The report explicitly excludes other factors such as substation space, accessibility, electrical servitudes, routing and grid-connection requirements; municipalities/Eskom and a formal load application are required for feasibility confirmation.

**Shore Power Demand for South African Ports** (CSIR/GIZ/PtX Hub, September 2025) uses TNPA vessel-activity records and vessel-type power reference data to estimate time-series demand. Its published absolute-peak / average-peak estimates are:

| Port | Financial year | Absolute peak estimate | Average peak estimate |
|---|---|---:|---:|
| Durban | 2018–19 | 47 MW | 21 MW |
| Durban | 2019–20 | 42 MW | 19 MW |
| Durban | 2020–21 | 41 MW | 17 MW |
| Durban | 2021–22 | Not reported | Not reported |
| Durban | 2022–23 | Not reported | Not reported |
| Durban | 2023–24 | 37 MW | 17 MW |
| Durban | 2024–25 | 42 MW | 20 MW |
| Ngqura / Coega | 2018–19 | 10.6 MW | 4.7 MW |
| Ngqura / Coega | 2019–20 | 12.6 MW | 5.8 MW |
| Ngqura / Coega | 2020–21 | 9.8 MW | 5.0 MW |
| Ngqura / Coega | 2021–22 | 11.5 MW | 5.2 MW |
| Ngqura / Coega | 2022–23 | 9.5 MW | 4.3 MW |
| Ngqura / Coega | 2023–24 | Not reported | Not reported |
| Ngqura / Coega | 2024–25 | Not reported | Not reported |

The report defines the absolute peak as the maximum demand using vessels' maximum reference power; its average-peak series uses average reference power. These are reconstructed estimates, not metered coincident shore-power load. It notes data-cleaning and classing limitations, and some port/year combinations are absent. A dash is **UNKNOWN / NOT_REPORTED**, never zero.

**What this comparison suggests—and does not prove.** The two studies expose a potentially important alignment problem: reported shore-power demand is in MW, while the load-connection assessment reports spare capacity in MVA. Those quantities cannot be compared directly without power-factor evidence and matched electrical boundaries. The shore-power series is reconstructed from TNPA vessel logs and reference-power assumptions, not interval-metered coincident load; the spare-capacity figures are pre-feasibility estimates, not firm allocations. Therefore do not subtract one from the other or label either port in deficit/surplus. The correct follow-on is to acquire utility-confirmed PCC/feeder limits, calibrated time-series meters, actual simultaneous berth occupancy, power factor, voltage and contingency limits, and shore-power connector/ship compatibility.

The public aggregates are frozen in [south-african-port-energy-evidence-v0.tsv](fixtures/south-african-port-energy-evidence-v0.tsv). Keep the original reports, publication versions and claim ceilings attached to every use:
- [CSIR/GIZ Load Connection Capacity Assessment (2025)](https://www.international-climate-initiative.com/en/iki-media/publication/load-connection-capacity-assessment-of-south-african-national-ports-new696e3fd8d74ab589496258/) · [PDF](https://ptx-hub.org/wp-content/uploads/2025/11/PtX-Hub-Load-Connection-Capacity-Assessment-of-South-African-National-Ports_final-1.pdf)
- [CSIR/GIZ Shore Power Demand for South African Ports (2025)](https://ptx-hub.org/publication/shore-power-demand-for-south-african-ports/) · [PDF](https://ptx-hub.org/wp-content/uploads/2025/10/PtX-Hub-Shore-Power-Demand-for-South-African-Ports-2.pdf)

### Required evidence packets

| Packet | Minimum evidence | Admission rule |
|---|---|---|
| Port and vessel service | Port-call event IDs/timestamps, terminal/berth identifiers, vessel and cargo service obligations, terminal work events, inland handoff and receiver acceptance | Use DCSA/IAPH-compatible semantics where applicable; record the actual event source and accepted service denominator. Port-call timestamps alone do not establish inland delivery. |
| Grid and PCC | Utility-confirmed PCC identity/topology, single-line diagram revision, connection/contracted/import/export limits, voltage level, transformer/feeder constraints, protection and metering boundaries, time-series import/export, outages/deratings and power quality | No pre-feasibility figure or nameplate value becomes an available capacity offer. Require utility/source approval, meter class/calibration and the validity period. |
| Shore power | Ship/berth interface compatibility, connector readiness, power rating and ramp/quality limits, slot windows, metered concurrent kW and integrated kWh, authorization/contract and unserved-service events | Keep kW instantaneous service, kWh delivered, connector availability, vessel compatibility and authority as separate gates. |
| Generation and storage | Asset identity/site boundary, MW and MWh separately, commissioning certificate/date, operating envelope, SOC/health, round-trip efficiency, metering, availability history, network connection, control owner and dispatch/aggregation rights | Planned, near-commissioning or nearby assets are not available capacity. No shared PCC, wheeling/export, reserve or dispatch entitlement is inferred from geography. |
| Water and thermal | Feedwater/source rights, flows, feed/product quality, RO process energy, brine/outfall limits and monitoring; heat source/sink temperature, flow, grade, time-matched sink demand, exchange losses and metered useful delivery | A modelled heat flow or nominal RO rating cannot earn product-water or recovered-heat credit. Keep water quality and environmental authorization explicit. |
| Regional manufacturing | Supplier/facility identity, material grade and provenance, exact process/tolerance, qualified workforce and shift, equipment availability, capacity by time window, inspection/NDT capacity, quality/rework, transport and repair history | Supplier registration or theoretical machine capacity is not qualified capability. Inspection/receiver acceptance must close for the exact configuration/revision. |
| Spatial, rights and ecology | Land/sea/port boundary, corridor/access, title/lease/permit, indigenous/community/participant rights where applicable, ecological constraints, sensitive habitat and environmental limits | Preserve no-go areas, rights conflicts and uncertainty; an optimizer cannot trade away statutory or participant authority. |
| Inland logistics and receivers | Rail/road/barge path, capacity/window, transshipment, customs/clearance, inland terminal, receiving facility, proof of receipt and acceptance | Distinguish dispatched, delivered, received and accepted states; unknown receiver status is not success. |

### Evidence and temporal integrity

Every admitted datum must carry:
- stable source and observation identity;
- measurement/event time, first-available time, ingestion time and decision cutoff;
- source revision/digest, unit, coordinate reference or asset boundary;
- method, calibration/uncertainty and validity interval;
- owner, rights/permission, revocation/expiry and allowed purpose;
- explicit missingness and claim ceiling.

Historical replays may use only information available at the decision cutoff. Later-corrected measurements can support a restated report, not silently alter what the original policy knew.

### Read-only pilot gate

1. Obtain written data-sharing, use, privacy and publication permissions from each source owner.
2. Secure utility/port-confirmed connection boundaries, capacity offers and operating constraints.
3. Validate time synchronization, meters, units, coordinate references and event joins.
4. Run a read-only baseline and independent reconciliation against owner-native ledgers.
5. Freeze a matched scenario family and human-adjudicated acceptance outcomes.
6. Only after that, compare candidate policies offline. No connection request, dispatch, shore-power switching, water treatment, cargo allocation or procurement authority is created by this benchmark.

The output is a site evidence-readiness report: admitted, missing, stale, disputed, rights-restricted and not-applicable evidence by service chain. It is not a ranking of Ngqura/Coega vs Durban, and it must not infer physical grid capacity from pre-feasibility or nameplate figures. The 2025 CSIR/GIZ load-connection and shore-power reports are admitted only as source-qualified historical estimates; their paired-site values and absence markers are frozen in `south-african-port-energy-evidence-v0.tsv`. They provide initial stress-case ranges, not evidence of a current firm connection allocation or metered demand.

## 6. End-to-end service and productive capability

Consume the canonical owners:
- Mycelix #6773: regional maritime productive capability and time-windowed qualification.
- Mycelix #4814 and #5762: supplier/resource and common-cause dependency semantics.
- Symtropy #3683: expected-service denominator, cargo disposition and receiver acceptance.
- Symtropy #4204: public ship-block data admission and faithful scheduling reproduction.
- Symtropy #2048/#2050/#2734: build, spatial feasibility and non-anticipative recovery owners.
- Sol-Atlas #370/#377/#948: spatial, rights-aware and site evidence.

The MCSC contribution is only the counterfactual join: hold the service obligation and exogenous shocks fixed, vary the qualified capability portfolio, and evaluate the difference in receiver-delivered service and recovery. No second dependency graph, yard scheduler or cargo ledger.

### Required outcome vector (publish each component)

1. **Receiver service:** expected units, accepted units, on-time accepted units, partial units, late units, diverted/cancelled units, unresolved/unknown units; receiver elapsed-time percentiles calculated only over the declared eligible set.
2. **Port/asset operation:** CPPI-compatible six-milestone times; berth/vessel wait; cargo operations and equipment utilization; shore-power service; power peak/curtailment; service shortfall.
3. **Productive capability:** exact qualified grades/processes/tolerances/volume; capacity by time window; first-pass yield; inspection queue/rework; qualified repair and substitute activation time.
4. **Resilience:** direct and transitive shared dependencies; minimal cut sets under the declared fault model; accepted-service loss under common-cause shocks; time to minimum viable service and full recovery.
5. **Economics and distribution:** whole-life cost per accepted unit; gross local spend, local value added and retained capability separately; working capital; training vs qualified competency; supplier concentration.
6. **Physical/environmental:** mass and energy balances; grid/renewable/fuel/water/heat use; material yield, waste and embodied/operational emissions with declared boundaries.
7. **Evidence quality:** provenance completeness, freshness, measurement uncertainty, model VVUQ coverage, human approvals, invalidated assumptions and unresolved evidence.

Do not collapse these into one weighted scalar. Preserve hard gates, service floors, Pareto alternatives and distributional floors; publish trade-offs and dominated portfolios separately.

## 7. Benchmark ladder and targets

Targets below are proposed qualification thresholds, not achieved results.

| Stage | Minimum evidence | Gate |
|---|---|---|
| 0. Deterministic micro-oracle | 10 independently adjudicated tiny instances; all finite alternatives enumerated | 100% outcome/oracle agreement; zero false feasibility/acceptance admissions |
| 1. Frozen corpus | 30 cases minimum, expanding to 100+; exact source and fixture digests | 100% inputs accounted for; no silently dropped units, options, failed runs or constraints |
| 2. Matched reproductions | At least three strong baselines across port-energy, shipyard scheduling and service orchestration | Reproduce source formulation, data, objective, units, hardware/compute limits and bounds; disclose all mismatches |
| 3. Robust generalization | Hold out full facilities, shock families, arrival regimes and supplier topologies | No future-information leakage; report paired results, confidence intervals, failure cases and timeouts |
| 4. Physical/industrial validation | Controlled hardware/operational data with rights, safety and authority approvals | Independent VVUQ, calibrated sensors, measured acceptance, traceable intervention and rollback |
| 5. External qualification | Independent evaluator and reproducible public report | Third-party replication; never infer generality from one demonstrator |

### Specific proposed targets

- **Hard-gate integrity:** zero false admissions on all frozen invalid/stale/revoked/unqualified dependency controls; zero unknown or late units relabelled as receiver-accepted.
- **Exactness:** 100% agreement on tiny exact cases across two separately implemented evaluators.
- **Corpus:** at least 30 frozen cases and 10 independently adjudicated tiny cases for initial qualification; 100+ cases for broader screening. Count scenario families and facilities, not merely random seeds.
- **Optimization:** report best-known feasible and best bound, optimality gap, runtime, memory, timeouts and compute. A proposed ≤5% gap applies only where a valid bound is available; it is not a claim of global optimality.
- **Held-out improvement:** only after faithful reproduction, target ≥5% improvement on a predeclared service/cost/recovery measure over the strongest matched baseline, with paired uncertainty reported and no worsening of protected service/safety/environmental floors. Require improvement on more than one held-out scenario family. If the strongest baseline wins, publish that.
- **Operational assurance:** all decisions have authority, timestamped source state, declared validity, constraints, rejected alternatives and a replayable receipt.

Do not set a numeric target against the 18–22% port-energy results until those specific studies have been faithfully reproduced. The first win is a correct, matched and independently replayable baseline—not a flattering number.

## 8. Paradigm-shift hypothesis

The research hypothesis to test—not assume—is that **regional productive capability has marginal service value only when its complete qualified path is independent enough, available in the required time window, and connected to an accepted receiver outcome.**

That reframes local industrial policy from an expenditure quota into a portfolio of service-capable options, and reframes maritime optimization from isolated asset efficiency into whole-chain capability formation, operation and recovery.



### Quality-qualified vs receiver-accepted service

The micro-reference uses two distinct event records:
- `INSPECTION`: PASS/FAIL/UNKNOWN, configuration revision and qualified quantity.
- `RECEIVER_ACCEPTANCE`: ACCEPTED/REJECTED/UNKNOWN, the same configuration revision and accepted quantity.

Accepted quantity cannot exceed inspection-qualified quantity or expected units. FAIL/UNKNOWN inspection yields zero qualified units; REJECTED/UNKNOWN receiver disposition yields zero accepted units. The expected-service denominator is never reduced to match the accepted quantity. Late receiver acceptance remains accepted-but-late. This is still a single batch completion-time model; per-unit arrival distributions and custody timelines are follow-on work.

The distinction prevents an internal NDT/QA pass from being promoted into proof that an intended receiver took custody and accepted the output. Production adapters must bind these records to Mycelix-owned custody, dispatch, handoff and participant-authority evidence.

## Port-city service-equivalence and shared-PCC comparator

MCSC now consumes the exact 48-disposition port-city service-equivalence trace from Mycelix #6343 / draft PR #6344. Its source is pinned by commit `654d37bc1aac06ece189c8fb0c0a0202c69bf6eb` and blob `ee9ecb7fc42009c6658e644b57e8f459aa68b979`; the checked-in JSON is unchanged and the TSV is a deterministic projection of dispositions, policy vectors and heat partitions.

The three source policies offer a useful negative-control experiment:

| Policy | PCC peak | PCC energy | Cap breach | Useful compute work |
|---|---:|---:|---:|---:|
| Baseline inflexible | 7.4 MW | 6.125 MWh | 0 | 100 |
| Naive objective-only shift | 9.2 MW | 6.125 MWh | 1.0 MW | 100 |
| No-new-peak admitted | 7.2 MW | 6.125 MWh | 0 | 100 |

The MCSC integration test reconstructs every interval's PCC load from the independent resource profiles, recalculates energy, checks service equivalence and validates the three heat partitions. It also verifies all 48 source dispositions remain individually identifiable and keep their Positive/Negative/Unknown/Conflict classes.

This benchmark is a cross-owner contract and hand-computable synthetic known-answer trace. It is not a site simulation, real power-flow model, RO process qualification or physical heat-recovery measurement. Water product quality/volume and environmental limits remain unknown. The independent owner engines remain authoritative for PCC verification, compute flexibility, cooling/thermal storage, RO, marine forecasts, rights and common-cause dependency closure.

The frontier is therefore no longer simply “co-optimize more assets.” It is to prove that a coordinated schedule preserves the same vessel, terminal, compute and water obligations under one shared grid envelope, and that its evidence/time/authority frontiers are consistent across independent owners.

## Causal interpretation and evidence ladder

The option-value vector is a **model counterfactual** until the intervention is identified from actual assignments and independently measured outcomes. Do not call modelled regional value, predicted jobs or simulated service recovery a causal industrial impact.

Compose the canonical causal owners:
- Symtropy #3152 for known-truth synthetic causal estimation and digital-twin intervention falsification.
- Symtropy #5108 for maritime repair effect, maintenance recurrence and time-varying treatment.
- Symtropy #5094 for independently measured physical restoration outcomes.
- MCSC #7686 only for the joined capability-to-receiver-service estimand and its denominator.

The detailed [causal evaluation protocol](causal-evaluation-protocol.md) freezes interference-aware estimands, cluster/network exposure, randomized and phased rollouts, target-trial emulation, falsification controls, and the paired-site limitations. In particular, Ngqura and Durban are evidence-acquisition environments; two non-randomized sites alone cannot identify a causal regional-development effect.

## Common-cause recovery and decision regret

The next MCSC lane is tracked in [#7747](https://github.com/Luminous-Dynamics/symtropy/issues/7747). It composes Mycelix #5762's transitive common-cause dependencies, Mycelix #6533 offer lifecycle and Symtropy #5710 requalification outputs with canonical #7554 receiver-service results.

A 20-case frozen synthetic manifest covers shared feeder/substation, mill, NDT/calibration, specialist workforce, rail, fibre, common water intake, heat sink loss, firmware/material/skill/calibration expiry, revoked execution authority, post-deadline qualification, partial restoration, sequential events during recovery, late evidence and unknown dependency remainder.

The research question is **decision regret under correlated recovery**, not provider diversity. Compare three current recovery paradigms—bounded disruption management, full rescheduling and proactive–reactive hybrid—alongside no-build, independence-assumption (negative control), common-cause-aware, lifecycle-aware, finite-scenario robust and clairvoyant per-scenario upper-bound policies. A 2026 systematic review identifies these three paradigms and notes the lack of systematic comparisons in performance, computational efficiency and practical applicability. The MCSC regret vector now adds replan invocation count, plan churn and event-to-admissible-plan latency to service loss, unit-hours below floor, recovery/requalification time and cost, emissions and common-cause false-independence. Do not assign probabilities without source-grounded incident data or create a scalar winner. [2026 recovery-strategy systematic review](https://doi.org/10.1080/00207543.2026.2719841).

The initial manifest and protocol qualify only scenario coverage and claim ceilings; they do not yet implement a recovery optimizer or numeric regret engine.

## Capability lifecycle, qualification decay and offer authority

MCSC's next paradigm shift is from a static capability graph to a **time-bounded, configuration-bound capability option**. Reuse:
- Mycelix #6533 for immutable, versioned manufacturing resource offers, provider/site/process references, capacity calendars, issuer authority, validity and revocation;
- Symtropy #5710 for change-triggered claim invalidation, dependency closure and minimum requalification;
- Symtropy #3121 for component/interface/composition/end-to-end VVUQ.

An offer published is not a resolved resource; a resolved resource is not a qualified process; a qualified capability is not calendar availability; calendar availability is not a reservation; a reservation is not job authority; job execution is not inspection acceptance; and inspection acceptance is not receiver acceptance.

The [capability lifecycle protocol](capability-lifecycle-protocol.md) defines these states, source/time/configuration joins, expiry and revocation, change-triggered requalification, SOVQC across a horizon, 20 micro-cases and proposed targets. MCSC consumes those owner-native transitions rather than recreating their registry or assurance graph.

### Investment and renewal option value

Compare not only the immediate service result of a qualified portfolio, but the time it becomes available, time it expires, cost to maintain/renew it, and service at risk while requalification occurs. Capability that becomes qualified after the required receiver deadline has no current-service credit. Skills, tools, materials, calibration and permits must all remain valid for the execution window. This is how we measure durable regional capability rather than counting machines or certificates.

## 9. New construct: service option value of qualified capability

Treat a qualified regional capability as an **option to preserve or restore a service**, not simply as locally sourced spend.

For a candidate capability investment/qualification package (c), baseline capability portfolio (P_0), and a fixed set of demand/disruption scenarios (Omega), define the per-scenario marginal outcome vector:

[
\Delta \mathbf{Y}_c(\omega) =
\mathbf{Y}(P_0 \cup c,\omega) -
\mathbf{Y}(P_0,\omega)
]

where \(\mathbf{Y}\) separately contains receiver-accepted and on-time units, time to minimum service and full recovery, lifecycle cost, emissions, regional value added and capability retention. The result is a **vector-valued service option profile**, not a single scalar score. The underlying service contract distinguishes inspection-qualified quantity from receiver-accepted quantity. A quality pass alone is not a delivery or acceptance event; on-time accepted service is counted only when the receiver record is configuration-matched, explicitly ACCEPTED and within the deadline.

Rules:
- The capability is available only from its demonstrated qualification/commissioning date and only within its evidenced material, labour, process, inspection, energy and logistics windows.
- Compare the candidate with no-build, import, local-assembly, centralized, distributed and repair-first alternatives on identical demand and shock realizations.
- Publish scenario-by-scenario \(\Delta\mathbf{Y}\), downside exposure and Pareto dominance. If credible probability distributions exist, additionally report expected value and CVaR; if they do not, do not invent probabilities—publish the complete stress-set profile and worst-case outcomes.
- Include the cost and time to form the capability, idle/standby cost, workforce qualification and obsolescence. Do not count training enrolment, machine nameplate capacity or supplier registration as available capability.
- Count capability persistence only while its evidence, qualifications, inputs and authority remain valid; expired or revoked evidence removes the option until requalified.

This construct makes the central research question measurable: **what does one additional qualified regional capability buy in accepted service and recoverability, under which shocks, at what whole-life cost, and for whom?**

A synthetic PASS validates only the frozen model and reference calculations. It does not prove actual regional industrial capability, economic additionality, employment impact, physical safety, port performance or causal impact.


## 10. Information-set, online-recovery and port-reliability benchmark (KITE)

KITE is specified in [information-set-regret-protocol.md](information-set-regret-protocol.md) and tracked in [#7775](https://github.com/Luminous-Dynamics/symtropy/issues/7775). It addresses a fairness defect in common offline comparisons: a hindsight optimizer can consume events and corrected data that were not available when an online policy had to decide.

The comparator hierarchy is explicit:
- **R0 perfect-information bound:** knows the realized scenario; useful only as an information upper/lower bound.
- **R1 same-information decision oracle:** sees exactly the evidence vintage available at each decision cutoff and obeys the same hard constraints. It is an offline decision-quality reference; report its compute time and do not imply deployability.
- **R2 compute-budget-matched policies:** receive the same as-of evidence and cutoffs, with the same declared wall-clock/resource/authority envelope.

Report the information gap (R0→R1) separately from the decision gap (R1→R2), with vector outcomes and execution/authorization delay shown independently. The synthetic 20-case negative-control manifest in `fixtures/information-set-replay-v0.tsv` covers late observation, late ingestion/validation, backfill, correction, authority revocation, expiry, rights, conflict, censoring, future-derived features, clock uncertainty, plan publication and authorization latency. A Rust integration conformance test parses the 24-field, 20-case manifest and recomputes declared visibility, admissibility and action-execution outcomes. The follow-on `src/kite.rs` library API now validates complete append-only correction lineages, selects the latest fully available and validated vintage as of a cutoff, preserves late revocations for present-time evaluation without rewriting historical snapshots, and evaluates required-evidence/action timing. The API is a reusable benchmark primitive, not a production evidence-vintage store or proof of source authenticity; source-ID authentication, cross-source provenance deduplication and owner-authorized rights checks remain external.

### Strong 2026 comparison targets

| Research comparator | Published scope/result | What MCSC should reproduce or add |
|---|---|---|
| [Wang et al., two-stage rolling port scheduling (2026)](https://doi.org/10.1016/j.cie.2026.112035) | Real-data Huanghua Coal Port case; hybrid rolling strategies for uncertain arrival/departure and vessel/berth/equipment coordination; compares a tailored method with GA, FCFS and CPLEX and reports case-study cost gaps of 5.7–89.1%. | Reproduce the same problem and objective before comparing. Add common-cause and lifecycle evidence only as explicit extensions; publish feasibility, objective gap, runtime, disruption-event response and receiver-service outcomes. The paper's study-specific range is not a transferable MCSC target. |
| [Zhang et al., data-driven liner schedule recovery (2026)](https://doi.org/10.1016/j.tre.2026.105009) | AIS-based recovery-behaviour data covering 3,559 container vessels and 700+ ports from 2018–2023; identifies port skipping, port swapping and speed-up behaviours and predicts suitable strategies. | Treat observed carrier behaviour and model prediction as separate baselines. Extend the outcome contract to accepted downstream service, substitution feasibility, common causes, authority, carbon/energy and post-action outcomes. A predicted strategy is not automatically a beneficial intervention. |
| [Chhetri et al., port time variability (2026)](https://doi.org/10.1186/s12544-026-00862-w) | Published 13 November 2026; derives a coefficient-of-variation port-time reliability measure from AIS for 401 ports in 119 countries; explains why a mean-only benchmark can conceal operational variability. | Preserve mean/median and tail delay, and report variability with censoring and vessel/cargo segmentation. Include anchorage waiting consistently; never collapse congestion, delay and schedule reliability into one measure. |
| [Disrupted seaside-planning systematic review (2026)](https://doi.org/10.1080/00207543.2026.2719841) | Reviews 45 studies (2007–2025); identifies disruption management, rescheduling and proactive–reactive hybrid policies, and gaps in propagation effects, integrated recovery and broad operational evaluation. | Retain all three as mandatory matched policy classes. Add common-cause propagation, dynamic qualification, decision-time information, schedule stability and service-floor outcomes. |
| [European port digital-twin benchmark (2026)](https://doi.org/10.1016/j.rineng.2026.110485) | Reviews seven port/logistics digital-twin initiatives; identifies interoperability/modularity patterns and persistent gaps in data availability, model transparency, operational reporting and synchronization mechanisms. | Measure evidence-vintage synchronization, source coverage, model/receipt version drift, first-available-to-ingestion delay, and event-to-admissible-plan latency. “Connected” is not the same as synchronized or operationally credible. |

### MCSC benchmark ladder

1. **Contract conformance:** owner identity, rights, authority, time/version binding, unit preservation, expected denominator and fail-closed UNKNOWN behavior.
2. **Synthetic known answers:** exact reproducibility, adversarial mutations, common-cause and lifecycle cases, and zero no-lookahead violations.
3. **Owner-native historical replay:** signed or digest-bound source receipts with evidence-vintage replay and no post-cutoff leakage.
4. **Held-out retrospective benchmark:** temporal, facility, supplier, corridor and event-family holdouts; publish all policy baselines and uncertainty.
5. **Shadow-mode operation:** prospective decisions are computed but not executed; measure data latency, override, admissibility, compute budget and forecast drift.
6. **Controlled operational pilot:** only after safety, authority and independent review; compare on predeclared outcomes with a valid control/comparison strategy and preserve all service denominators.

### What “better than SOTA” would mean

A credible claim requires a matched, reproducible result, not a larger architecture diagram or one attractive percentage. MCSC should aim to demonstrate a *joint frontier*:

- no false admission of stale, expired, revoked, wrong-configuration or unauthorized capability;
- no future-information leakage and deterministic replay of identical evidence vintages;
- no protected-service denominator deletion and no conversion of UNKNOWN to zero;
- at least matched baseline feasibility and objective quality, with exact runtimes and compute budgets;
- lower service regret or faster admissible recovery without unacceptable churn, emissions, lifecycle cost or safety trade-offs;
- better tail reliability and receiver-accepted service while retaining CPPI/PPS-compatible port indicators;
- generalization to held-out facilities and unseen disruption families.

These are qualification targets. Until the R2 methods are run on source-qualified and held-out port data, MCSC has not demonstrated superiority to any published method or industry KPI.
