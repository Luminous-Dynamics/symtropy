# Subsea Compute Industry Benchmark v0.1

**Issue:** SUBSEA-COMPUTE-INDUSTRY-BENCH-001 (#6280)  
**Snapshot:** 19 November 2026  
**Evidence class:** source-attributed public competitor register and matched benchmark contract.

## Strategic correction

Subsea compute is already a commercial market, not just a research curiosity. China's Hainan UDC has been reported in operation since March 2023 and was expanded in February 2025. HiCloud's Shanghai Lingang offshore-wind-connected underwater data centre was reported at full commercial operation in May 2026 with 24 MW total project capacity. Microsoft Project Natick remains an important research/pilot comparator, but should not be treated as the sole industry baseline.

That changes our competitive question. The winning proposition cannot be "put servers in sealed modules and use seawater as a heat sink". Competitors already report that. Our ambition should be to outperform on the **whole coastal-port system**: grid-aware service delivery, multi-domain workload orchestration, qualified heat/water use, resilience, marine evidence, repairability, common-cause independence and cyber-safe autonomy.

## What the public industry record currently says

### Hainan — commercial subsea cluster

China's official government/Xinhua page reports:
- the first Hainan UDC began operating near Lingshui at the end of March 2023;
- a new module was added in February 2025, capable of housing more than 400 high-performance servers;
- the reporting described ten companies as signed up for use;
- reported use cases include AI training/inference, industrial simulation, game development and marine science.

The same page includes throughput-style claims such as 7,000 AI assistant queries per second and compute equivalent to 30,000 high-end gaming PCs. Those are not comparable performance metrics without workload definition, quality, accelerator configuration, latency, availability, energy boundary and independent tests.

Source: [China government / Xinhua, 19 Feb 2025](https://english.www.gov.cn/news/202502/19/content_WS67b5a14dc6d0868f4e8efcaa.html).

### Shanghai Lingang — 24 MW underwater/offshore-wind project

Data Center Dynamics (20 May 2026) reports the Shanghai Lingang facility launched in June 2025, construction complete in October 2025, initial trials in February 2026 and full commercial operation the week before publication. It reports 24 MW total capacity, pressure-resistant modules around 35 metres below the surface, passive seawater cooling, a substantial share of offshore-wind power, and a PUE claim below 1.15 attributed to Chinese media reports. It also reports a possible larger 500 MW programme, which is a plan and not installed capacity.

Offshore Wind (18 May 2026) reports a 2.3 MW first-phase demonstration scaled to 24 MW, with the modules approximately 10 metres underwater. It reports developer claims of PUE around 1.15, 22.8% lower electricity use, zero water use and more than 90% lower land use; these are not independently audited in the source.

**Material source conflict:** DCD reports around 35 m depth; Offshore Wind reports 10 m. The difference could reflect phase, definition or reporting error, but it is unresolved. The benchmark fixture deliberately records CONFLICTED_UNKNOWN; do not use either depth in hydrostatic pressure, cable, bathymetry or installation calculations until primary as-built drawings and owner data resolve it.

Sources: [DCD, 20 May 2026](https://www.datacenterdynamics.com/en/news/hiclouds-offshore-wind-powered-underwater-data-center-up-and-running-off-coast-of-shanghai-china/); [Offshore Wind, 18 May 2026](https://www.offshorewind.biz/2026/05/18/china-puts-worlds-first-offshore-wind-powered-underwater-data-centre-into-operation/).

### Microsoft Project Natick — experimental reliability baseline

Microsoft describes Project Natick as a feasibility research project. Its Phase 2 subsea module operated for more than two years, was retrieved in July 2020 and, according to Microsoft's summary, showed a server failure rate one-eighth that of the land-based control group. Microsoft also reports plans to recycle the vessel/equipment and restore the seabed.

This is a useful research comparison, not a commercial 24 MW facility, and the 1/8 failure ratio is specific to its sample, module, environment and control. It does not prove higher availability, lower lifecycle cost, lower MTTR or better environmental outcomes at commercial scale.

Sources: [Microsoft Research — Project Natick](https://www.microsoft.com/en-us/research/project/natick/); [Natick Phase 2 timeline](https://www.microsoft.com/en-us/research/project/natick/phase-2-timeline/).

### Adjacent technologies — not the same architecture

Nautilus's EcoCore facility cooling distribution unit is a relevant liquid-cooling technology comparator, but it is not by itself a submerged UDC deployment. Panthalassa's Ocean-3 and Aikido's floating offshore wind/data-centre proposals should be tracked separately as floating-platform architectures, not counted as commercial seabed pressure modules without deployment and operating evidence.

Source examples: [Nautilus EcoCore performance announcement, May 2026](https://nautilusdt.com/press-release/nautilus-data-technologies-facility-cooling-distribution-unit-exceeds-performance-benchmarks-for-next-generation-ai-gpu-workloads/); [DCD's 2026 offshore/subsea comparison](https://www.datacenterdynamics.com/en/news/hiclouds-offshore-wind-powered-underwater-data-center-up-and-running-off-coast-of-shanghai-china/).

## Paradigm shift: a distributed Port-City Compute and Ocean-Energy Commons

A subsea pressure module should be one optional tier in a distributed coastal system, not the only data centre and not a single point of failure.

1. **Onshore operational core:** local orchestration, security, customer/network interconnect, maintenance access and resilient compute for latency-sensitive or physically dependent workloads.
2. **Subsea compute tier:** selected batch/HPC/AI workloads that tolerate latency and data movement, subject to subsea maintenance and repair costs.
3. **Floating/offshore energy tier:** renewable generation and storage integrated with a shared, metered PCC/grid boundary; renewable nameplate is not firm supply.
4. **Thermal and water network:** liquid-cooling heat can serve a real water/process sink only when temperature, flow, demand, delivery meter and displaced alternative are qualified. Heat rejection and water output are distinct claims.
5. **Resilience fabric:** multiple cable routes, diverse landings, local fallback and repair logistics, assessed through transitive shared dependencies rather than cable count.
6. **Evidence and authority fabric:** Sol-Atlas spatial/environmental truth; Symtropy physical and chronological replay; Symthaea bounded proposal/abstention; Mycelix authorization, provenance and reconciliation.

This is more difficult than a stand-alone UDC, but offers a more defensible competitive advantage: performance of the complete service system rather than a single module.

## Benchmark design

### Comparator matrix

| Comparator | Evidence status | What to benchmark |
|---|---|---|
| Hainan HiCloud UDC | Commercial operation reported since 2023; expanded 2025 | customer/workload service, capacity actually used, uptime, water/energy boundaries, maintenance |
| Shanghai Lingang HiCloud UDC | 24 MW commercial operations reported May 2026 | whole-facility PUE, hourly offshore-wind share, service, repairability, depth/as-built, marine impacts |
| Microsoft Natick | Research pilot retrieved 2020 | reliability experimental method, failure definition, sample/control, module lifecycle |
| Nautilus EcoCore | Adjacent liquid-cooling technology | heat exchanger/CDU performance, flow/temperature envelope, maintainability |
| Floating offshore compute concepts | Proposed/unveiled platform class | readiness, mooring, marine access, grid and fibre interface |
| Conventional coastal/onshore liquid-cooled DC | Commercial baseline | equal workload, climate, cooling and grid boundary |

### Metrics

**Compute and service**
- useful workload completed per kWh, by workload/precision/model/accelerator;
- p50/p95/p99 latency and data movement;
- on-time completion, queue/rebound, checkpoint/restart and lost work;
- service availability, planned/unplanned downtime and recovery.

**Energy**
- whole-facility PUE with exact meter boundary and time window;
- IT kWh, facility kWh and grid-import kWh per useful work;
- hourly wind/PV share, curtailment, firming, storage losses and PCC/feeder peaks;
- demand response, ramp and rebound.

**Cooling and water**
- cooling electricity and pump energy;
- WUE using a declared standard boundary, plus separate seawater intake/discharge volume;
- seawater temperature/flow/depth profiles and rejected heat flux;
- useful heat delivered to a named external sink, source heat, transfer loss and external energy displaced;
- fouling, corrosion, biofouling, chemical treatment and maintenance interval.

**Reliability and maintainability**
- server/module failure frequency with sample size/confidence intervals;
- service availability and MTTR—not failure ratio alone;
- retrieval, repair-vessel, module-swap and spare capacity;
- common-cause grid/fibre/cooling/landing failures and local fallback.

**Environment and lifecycle**
- bathymetry, seabed, protected habitat and receiving water;
- validated thermal plume and current interaction;
- cable burial/route/repair access and fishing/anchoring exposure;
- capex, opex, vessels, cables, maintenance labour, insurance, retrieval, recycling and decommissioning.

### Metric boundary discipline

ASHRAE recommends pursuing heat reuse where there is a viable sink and tracking useful reuse with PUE plus ERE/ERF-type metrics; it also emphasizes low/no-water cooling strategies and liquid-cooling at AI densities. The benchmark must distinguish PUE, WUE, ERF/ERE, grid import and marine heat rejection rather than condensing them into a single efficiency number. [ASHRAE AI Data Center Energy & Thermal Efficiency](https://www.ashrae.org/technical-resources/topics-and-initiatives/ai-data-center/energy-and-thermal-efficiency).

ITU-T L.1328 (2025) defines ERF using energy that is reused outside the data-centre boundary and substitutes for outside energy use; the boundary and the electricity used to raise heat temperature need separate accounting. Do not count mere heat extraction as useful reuse. [ITU-T L.1328](https://www.itu.int/rec/dologin_pub.asp?id=T-REC-L.1328-202509-I%21%21PDF-E&lang=e&type=items).

## Candidate target ladder (hypotheses, not achieved results)

- **Capacity parity:** independently qualify a 2.3 MW demonstrator envelope and a 24 MW campus-scale design/reference; no capacity claim from press releases alone.
- **Whole-facility efficiency:** establish independently metered PUE at or below 1.15 on the same boundary as the comparator; stretch research target <=1.10 only after demonstrating no reliability/service loss.
- **Renewable integration:** publish hourly renewable fraction, grid imports, curtailment and firming cost. Annual matching alone does not demonstrate 24/7 renewable supply.
- **Workload value:** exceed conventional and current subsea comparator cost/energy per useful workload, controlling for accelerator generation and service class.
- **Heat/water:** count only useful heat delivered to a real sink and water produced at a qualified quality/volume boundary; no RO benefit credited from thermal balance alone.
- **Resilience:** target no hard service failure under one qualified grid, fibre, cooling or module fault; report MTTR and repair dependencies.
- **Marine:** no design/site performance claim without source-qualified depth, currents, temperature/salinity, bathymetry, habitat, outfall and independent plume monitoring.
- **Whole-network advantage:** target 5–10% improvement in a pre-registered service-equivalent lifecycle objective versus the strongest matched comparator, with no hard safety, environmental or service-floor regression and paired confidence intervals excluding zero.

These are research hypotheses; competitor media/vendor claims are not used as audited truth.

## Ownership

This issue defines industry evidence and matched-comparison rules only. Reuse:
- Symtropy #6228/#6255/#6258/#6259/#4582/#6211/#6209 for cooling, trigger, heat-water, port energy, compute and marine replay.
- Mycelix #6343/#6344/#5974/#5762 for service equivalence, authority, provenance and common cause.
- Sol-Atlas #654/#942/#948 for site, marine, habitat, cable, rights and South African paired-site evidence.
- Symthaea #5199/#11572/#11473 for candidate proposals/abstention and maritime services.

## Claim ceiling

This is an industry evidence register and benchmark protocol. It does not prove Luminous is better than commercial UDCs, does not certify public competitor claims, and does not qualify a South African subsea site, grid connection, water system, environmental permit or construction design.
