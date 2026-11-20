# Ocean Reserve Compute: Recovery Topology and Industry Benchmark v0.1

**Status:** synthetic reference architecture and qualification protocol; not a site design, operator topology, recovery certification or procurement specification.  
**Date:** 2026-11-20  
**Owner-native work:** [SUBSEA-RESERVE-RECOVERY-001 #6360](https://github.com/Luminous-Dynamics/symtropy/issues/6360)

## Executive decision

The field should be advanced from "subsea data centre" to **service recovery infrastructure with evidence-qualified failure independence**.

The distinctive proposition is not a pressure vessel hidden at depth. It is a productive, distributed reserve whose local, warm and immutable recovery paths remain trustworthy when sites, identity providers, keys, cable routes, grid supply, cooling trains, operators or software supply chains are compromised.

The v0.1 reference contains:
- three recovery service paths: port-local continuity, warm recovery, and immutable clean-room recovery;
- 39 synthetic components across 13 component classes;
- 42 synthetic failure domains;
- 14 fault / adversarial scenarios, including a compound two-region event and an application-integrity quarantine;
- a Rust CLI that verifies declared independence, exhaustively injects every single declared failure domain, replays the scenario fixture and has a negative-control unit test for a shared identity dependency.

**All sites, utilities, carriers, operators, identities, keys and hardware stacks are synthetic placeholders.** The graph is an executable contract shape, not proof that any real-world dependency is independent.

## 1. Paradigm shifts

### 1.1 From a bunker to a federated recovery fabric

A single "hardened" facility remains a single site, supply chain, jurisdiction, cable and management problem. A recovery fabric uses independent failure domains and service-specific paths.

The architecture separates:
- local continuity: a bounded subset of port and marine services, close to sensors and users;
- warm recovery: a recent, separately administered replica and compute capacity at a geographically independent site;
- immutable recovery: an offline or deletion-resistant baseline plus a clean-room environment, independent keys and recovery authority.

Do not count two sites as independent until their geography, utility feeders, carriers, landing stations, ducts, identity, key custody, storage administration, software approval, operators and cooling dependencies have been evidenced.

### 1.2 From idle backup to productive reserve

A warm reserve can run useful, deferrable work during normal operation—ocean hindcasts, archive validation, image reconstruction, condition analytics—provided a measurable reserve envelope is preserved.

A "productive reserve" is not allowed to sell the same compute twice. Every accepted job must declare:
- maximum pre-emption latency;
- checkpoint durability and restart cost;
- deadline and quality threshold;
- minimum compute, memory, storage, network and cooling capacity held back;
- maximum resource contention and power ramp;
- the workload's rights and data-egress constraints.

The reserve-capacity claim is proven by an interruption test: launch the nominated failover load while the reserve is running its most demanding allowed background workload. Measure time-to-capacity, time-to-first-valid-result, completion quality, data age and rebound peak. An idle capacity spreadsheet is not evidence.

### 1.3 From server uptime to accepted-service survival

The primary unit is **accepted service restored**, not servers online, pods reachable, containers scheduled or a control plane returning HTTP 200.

Measure the trajectory from declared event to verified service:
1. incident declaration and fault detection;
2. authority and evidence reconciliation;
3. selected recovery path;
4. restore and data-integrity verification;
5. application-level quality and deadline checks;
6. service acceptance by an independent oracle.

Failed, partial, corrupted, stale or unverified results do not count as restored service.

### 1.4 From redundancy counts to causal independence

Two cables, two clouds or two data centres can still share a landing station, duct, utility substation, provider control plane, identity provider, key custodian, geographic hazard or update-signing authority.

Represent each path as component dependencies and failure-domain memberships. Compute intersections and cut sets. A single declared critical failure domain must not disable all three paths in the synthetic reference. Real deployment credit requires as-built evidence.

### 1.5 From passive cooling to thermal resource intelligence

"Cold ocean" is not a design specification. Measure seasonal temperature profiles, salinity, currents, heat-exchanger approach temperature, fouling, pump power, differential pressure, corrosion, intake/outfall and fallback cooling.

The near-term experiment should be a closed secondary coolant loop connected through a serviceable coastal heat exchanger. A subsea compute pod is a later option only if the matched whole-facility comparison establishes a real advantage.

### 1.6 From backup power to energy-native compute

Use workload flexibility, thermal flexibility and storage deliberately—but keep their timescales and authorities separate:
- power-electronics and protection response: milliseconds to seconds;
- BESS and microgrid reserve: seconds to minutes;
- cooling and thermal storage: minutes to hours;
- deferrable batch workloads: minutes to hours or days, subject to deadlines.

Do not claim a compute scheduler provides millisecond frequency response. Fast grid response belongs to qualified inverter, UPS, protection and storage systems; the scheduler supplies slower energy and capacity flexibility. Grid export or market participation requires local utility approval and contractual qualification.

## 2. Industry and state-of-the-art comparison

### 2.1 Data-centre efficiency context

Uptime Institute's Global Data Center Survey 2026 reports:
- annual facility-average PUE: **1.52**;
- capacity-weighted PUE analysis: **1.36**;
- newer, leading facilities commonly report PUE around **1.30 or lower**.

At equal IT energy and a consistent meter boundary, PUE 1.15 corresponds arithmetically to about 24.3% less total facility energy than PUE 1.52, and 11.5% less than PUE 1.30. This is a comparison of the PUE ratio, not proof of lower energy per useful workload: compute efficiency, utilization, climate, network, water and service quality still need matching.

Shanghai Lingang:
- official Shanghai reporting says the 2.3 MW first-phase demonstration entered operation in May 2026;
- the overall two-phase project plan is 24 MW;
- an August 2026 Shanghai update reports more than 95% renewable electricity for the project;
- project PUE claims around/below 1.15 are a useful reference, but the public material is not independent annual metering of the complete life-cycle boundary.

Hainan:
- China's State Council/Xinhua reported the commercial underwater data centre had begun operation in March 2023;
- a 2025 module was reported as capable of housing more than 400 high-performance servers;
- the operator/government report describes AI training/inference and marine-science use, but does not provide a matched, independently verified public benchmark for the complete facility.

Microsoft Project Natick:
- the Northern Isles research module had 240 kW electrical consumption, 12 racks, 864 servers and 27.6 PB storage;
- it was designed for a one-atmosphere dry-nitrogen interior;
- up to five years' maintenance-free operation was a design objective, not the duration of a completed five-year operating trial.

**Implication:** PUE ≤1.15 is an ambitious parity target against Shanghai's published project figure and substantially better than the 2026 broad industry average. It is not a sufficient differentiator or proof of subsea superiority. Do not compare pilot reliability rates from unmatched hardware cohorts as if they were directly comparable.

### 2.2 The v0.1 benchmark ladder

| Measure | Initial target | Stretch target | Evidence required |
|---|---:|---:|---|
| Whole-facility PUE | ≤1.15 annual measured | ≤1.10 annual measured | ISO/IEC 30134-2:2026 meter boundary, calibrated meters, full seasonal cycle |
| Energy per accepted service unit vs matched shore control | ≥10% lower | ≥20% lower | Same accepted output, quality, volume, workload, deadline and lifecycle boundary |
| Hourly carbon-free electricity matching | ≥90% | ≥95% | Hourly consumption and source-matched generation/contract evidence; no annual-average substitution |
| Eligible batch workload shift | ≥10% without missed deadlines | ≥15% | Job-level time/geo flexibility, no service regression or rebound peak |
| Warm recovery RPO | ≤15 min on nominated critical datasets | Service-specific improvement | Event-level last-accepted-write evidence |
| Warm recovery RTO | ≤4 h | ≤1 h where service tier supports it | Clock starts at declared incident; stop only after independent service acceptance |
| Immutable clean-room RTO | ≤72 h for frozen recovery set | ≤24 h | Blind restore of data, catalogue, keys, software and configuration |
| Single critical-domain failure | At least two recovery paths remain | All three remain where feasible | Exhaustive single-domain fault injection and independent as-built evidence |
| Two-region compound fault | One independent path remains in nominated scenario | Two paths for selected threat pairs | Common-cause model plus replay evidence |
| Destructive backup action by production credentials | Zero successful actions | Zero | Negative-control tests against every delete, retention and key path |
| Cooling resilience | No required-service loss under one cooling train failure | Maintain full agreed service | Seasonal thermal data, pump energy, fault injection and fallback test |
| Environmental compliance | Zero unapproved habitat disturbance or discharge | Regenerative/beneficial reuse only after evidence | Baseline ecology, noise, sediment, thermal plume, intake/outfall and permits |
| Lifecycle cost per accepted service | No worse than matched shore control | ≥10% lower | Vessel, retrieval, cable, repair, replacement, storage losses and decommissioning |

These are proposed qualification targets, not achieved results or universal industry requirements.

### 2.3 Measurement rule

Use the published **ISO/IEC 30134-2:2026** edition for PUE definition, measurement categories and reporting. Pay particular attention to mixed-use facility treatment, on-site generation and unaccounted energy. State whether the reported result is PUE or the mixed-use mPUE variant; never mix boundaries.

PUE must be reported beside:
- energy per accepted service unit;
- annual and hourly carbon matching;
- fresh-water consumption and seawater withdrawal/discharge as separate quantities;
- pump and cooling parasitics;
- storage losses and renewable curtailment;
- reliability and service-quality outcomes;
- embodied and repair/replacement lifecycle burden.

A low PUE does not compensate for a failed restore, rights breach, unsafe dependency or ecological gate.

## 3. Recovery topology contract

### 3.1 Three path roles

| Path | Intended function | Synthetic target RPO | Synthetic target RTO |
|---|---|---:|---:|
| Port-local continuity | Reduced local marine/port analytics | 0 min for locally captured accepted records | 5 min |
| Warm recovery | Restore nominated services from a recent independent replica | 15 min | 4 h |
| Immutable clean-room recovery | Restore a frozen, independently verified baseline after destructive compromise | 24 h | 72 h |

These are objectives only. The JSON deliberately sets `targets_are_measured=false`. A real deployment must replace all synthetic site/provider assumptions with evidence.

### 3.2 Component classes whose independence must be proven

The reference graph explicitly includes 13 classes:
- site and geography;
- compute and storage;
- identity and key custody;
- network path;
- power and cooling;
- operator authority;
- control plane;
- runtime and artifact approval;
- time source.

A path is not independent when it shares a critical failure domain with another path. The verifier rejects a common identity or key domain rather than awarding partial credit for having different hardware labels.

### 3.3 Fault corpus

The 14 synthetic scenarios include:
- production site loss;
- production identity/key/control-plane compromise;
- each of the three site/region losses;
- production export fibre/landing loss;
- warm identity compromise;
- immutable vault key compromise;
- port cooling failure;
- public DNS and upstream repository unavailability;
- compound loss of port and warm regions;
- invalid workload image;
- compromise of a hypothetical shared backup administrator;
- correlated port/warm power and cooling failure.

A key distinction is RT-012: recovery infrastructure can remain available while service must still be blocked because the workload image failed integrity validation. Infrastructure reachability is not service recovery.

### 3.4 Verifier semantics

The Rust CLI:
1. parses a strict schema with unknown fields rejected;
2. checks unique component IDs and failure-domain declarations;
3. requires every path to contain exactly one of each required component class;
4. rejects shared critical failure domains across paths;
5. exhaustively removes each declared failure domain and requires at least two paths to remain;
6. simulates the 14 named scenarios and checks their expected infrastructure and service path sets;
7. emits a deterministic JSON receipt whose status explicitly says this is a synthetic reference, not site proof.

The negative-control test mutates the warm identity dependency to share the port identity domain and expects rejection.

Run from the repository root:

```bash
cargo test --manifest-path tools/ocean-recovery-topology-verifier/Cargo.toml
cargo run --manifest-path tools/ocean-recovery-topology-verifier/Cargo.toml
```

The current execution environment has no `rustc` or `cargo`; authored Rust tests are therefore pending execution. The JSON and scenario semantics have been independently cross-checked in JavaScript against the committed GitHub contents, without claiming a Rust build or CI pass.

## 4. Productive reserve: new benchmark

A reserve site should contribute useful work during normal operations without consuming capacity promised for recovery.

Define:

`R = firm_recovery_capacity / measured_peak_recovery_demand`

Do not publish an assumed scalar without a benchmark. Measure at each minute of a 30-day trace:
- reserved CPU/GPU, memory, storage IOPS and network bandwidth;
- power headroom at the facility point of connection;
- cooling headroom and maximum thermal trajectory;
- background workload pre-emption and checkpoint latency;
- transfer time for the frozen recovery data set;
- time to first valid service result after failover;
- user workload quality and deadline success.

**Acceptance:** the failover load starts while the reserve runs its maximum permitted background workload; every nominated service reaches its acceptance oracle within its RTO; no facility PCC cap is violated; no workload is silently dropped; and the reserve replenishes only after data integrity and service state are reconciled.

A reserve that is "available" only when idle is not a productive reserve. A reserve whose background jobs cannot be pre-empted is not a reserve at all.

### 4.1 Machine-readable preemption contract

The synthetic fixture `OCEAN_PRODUCTIVE_RESERVE_CONTRACT_V0_1.json` pairs the existing five workload profiles with three protected services (observation QC, port forecast, offshore-wind condition analytics) and two preemptible background jobs (AUV/ROV reconstruction and ocean-science hindcast).

With 100 normalized capacity units in each of compute, memory, network, cooling and power:
- protected recovery load is 70 / 40 / 33 / 55 / 65 units;
- normal background load is 95 / 75 / 70 / 55 / 65 units;
- running both groups concurrently exceeds at least one resource ceiling, so admission is not double-counted;
- after the background jobs are fenced and checkpointed, the protected recovery service vector fits all five capacity dimensions.

The slowest synthetic quiesce-plus-checkpoint contract is 180 + 300 = 480 seconds. The sequential recovery budget sums to 6,000 seconds against a 14,400-second (4-hour) RTO target. These normalized resource units and timings are test inputs only, not hardware measurements or an SLA.

The standalone verifier also rejects protected-service overcommit, checkpoint timeout beyond the recovery budget, missing checkpoint integrity, profile/contract mismatches and undeclared shared assumptions.

Run from the repository root:

```bash
cargo test --manifest-path tools/ocean-productive-reserve-verifier/Cargo.toml
cargo run --manifest-path tools/ocean-productive-reserve-verifier/Cargo.toml
```


## 5. Energy and workload orchestration

Use ENTSO-E's 2026 workload flexibility taxonomy: AI/ML training and batch/data processing have greater time/geographic shifting potential; inference has lower potential; transactional real-time work is least flexible. Backup/disaster-recovery load is flexible in normal mode but becomes priority load after failover. This must be reflected in job contracts rather than applied as one blanket scheduling rule.

The scheduler may move only eligible work. Fast grid response is handled by qualified power electronics and storage, not by waiting for a container scheduler.

A proposed whole-system target is ≥10% lower energy per accepted service unit than a matched shore-based control, with ≥20% as a stretch objective. The denominator must be identical and useful: e.g., accepted inspection area reconstructed at declared resolution, quality-passed observation records, or validated hindcast member-hours. Do not count raw FLOPs or bytes as service.

## 6. South African implementation and cable caution

Ngqura / Coega remains the shore-first integration study; Durban remains a comparator. Eco Wave Power/AGFDL announced a feasibility result for approximately 8.3 MW of potential installed wave capacity on the Ngqura breakwater in February 2026, subject to further development and permitting. It is not an installed plant, average-output forecast, firm-energy resource or connection offer. Require hourly output, capacity-factor and storm-recovery evidence before granting energy credit.

Public cable directories list multiple South African international cable systems and landing locations, including Gqeberha, Mtunzini and Amanzimtoti. This is useful for an inventory, but **it does not demonstrate diverse inland backhaul, independent carriers, independent landing-station power or independent routes to a candidate data-centre site**. Obtain route-level carrier attestations, landing-station dependencies, duct and wayleave records, power diagrams and failure-history data. The same test applies to subsea power routes.

Ngqura must be compared against:
- shore-only data centre with conventional cooling;
- coastal data centre with a closed seawater secondary loop;
- floating/nearshore module;
- subsea module;
- regional cloud/colocation.

Use the same workload, quality, SLA, redundancy, seasonal weather and lifecycle boundary.

## 7. Physical and environmental design gates

- No industrial compute or heat extraction in, on, or immediately adjacent to active hydrothermal vent habitat in this programme.
- No credit for vent heat as power without an independently measured and ecologically acceptable resource case; no vent interference.
- No shallow subsea pod until a non-critical instrument pod proves pressure, connectors, corrosion, fouling, retrieval and loss containment.
- No claim that depth prevents professional subsea intervention or a cyberattack.
- No seawater "free cooling" claim without pump, fouling, thermal approach and fallback energy measurements.
- No site recommendation without bathymetry, seabed geotechnics, currents, storm conditions, navigation/fishing use, habitat baseline and cable/power route evidence.
- No environmental performance claim without intake/outfall, thermal plume, noise, sediment and lifecycle monitoring.

## 8. Next execution sequence

**P0 — software and desk qualification**
- Keep the graph, scenario fixture and strict verifier in a public draft PR.
- Add scenario coverage for credential revocation, key rotation, corrupted catalogue, partial restore and restore under BESS degradation.
- Add matched workload and facility-meter data contract to the placement matrix.
- Independently review Rust source and execute tests on a local Rust-capable machine; CI is supplementary, not an excuse to delay the next research slice.

**P1 — data and topology evidence**
- Acquire licensed marine observation time series with calibration and QC metadata.
- Request as-built carrier routes, landing dependencies and service-level histories for Ngqura and Durban.
- Obtain utility point-of-connection studies and hourly grid-carbon/resource data.
- Obtain an environmental baseline and local seawater thermal series.

**P2 — physical sequence**
- Instrumented shore-side cooling loop with independent metering.
- Port microgrid / BESS and controlled failover exercise.
- Non-critical subsea instrument pod at a serviceable depth.
- Only then a matched compute pod if it beats the shore control on useful service, lifecycle cost and resilience.

## References

- Uptime Institute, Global Data Center Survey 2026: https://uptimeinstitute.com/uptime_assets/77a6191bef6c85cb4ac9bfb169ddc313cae517030b48709697b1f80bc10edc90-uptime-institute-global-data-center-survey-results-2026.pdf
- Uptime Institute, capacity-weighted PUE analysis (August 2026): https://intelligence.uptimeinstitute.com/resource/growing-pue-advantage-larger-data-centers
- ISO/IEC 30134-2:2026: https://www.iso.org/standard/30134-2
- Shanghai municipal portal, Lingang underwater data-centre operation (May 2026): https://english.shanghai.gov.cn/en-Latest-WhatsNew/20260601/89a94ac072f842eeacf82fcf0cce8261.html
- Shanghai municipal portal, renewable direct-supply share (>95%) and 24 MW two-phase plan (August 2026): https://english.shanghai.gov.cn/en-Policies/20260807/e366514466bc4c86b51a8d13bd3797fd.html
- China State Council/Xinhua, Hainan underwater computing cluster (February 2025): https://english.www.gov.cn/news/202502/19/content_WS67b5a14dc6d0868f4e8efcaa.html
- Microsoft Research, Project Natick facts and figures: https://www.microsoft.com/en-us/research/project/natick/facts-and-figures/
- ENTSO-E, Data centres and the power system (April 2026): https://eepublicdownloads.blob.core.windows.net/public-cdn-container/clean-documents/Reports/2026/FINAL_ENTSO-E_Data_Centres_260430.pdf
- ASHRAE, Grid-interactive design and demand flexibility: https://www.ashrae.org/technical-resources/topics-and-initiatives/ai-data-center/grid-interactive-design-demand-flexibility
- IEA 4E EDNA, Data Centres and Flexibility (July 2026): https://www.iea-4e.org/edna/publications/data-centres-and-flexibility/
- NIST SP 800-209 (final): https://csrc.nist.gov/pubs/sp/800/209/final
- NIST SP 800-209 Rev. 1 initial public draft (July 2026): https://csrc.nist.gov/pubs/sp/800/209/r1/ipd
- Canadian Centre for Cyber Security, submarine cable security guidance: https://www.cyber.gc.ca/en/guidance/cyber-security-considerations-submarine-cables-itsp20001
- Eco Wave Power / AGFDL, Ngqura breakwater feasibility announcement (17 February 2026): https://www.sec.gov/Archives/edgar/data/1846715/000121390026016925/ea027651301ex99-1_ecowave.htm
- South African cable landing inventory (third-party directory; verify against carrier as-built data): https://www.submarinenetworks.com/en/stations/africa/south-africa

---

**Claim ceiling:** synthetic reference architecture, design targets and reproducible calculations only. No real recovery topology, security independence, CI pass, field performance, grid connection, environmental approval, site suitability or SOTA superiority is established.
