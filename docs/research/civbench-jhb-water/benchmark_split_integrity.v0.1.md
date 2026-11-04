# CivBench benchmark split integrity v0.1

**Status:** engineering protocol proposal; not a claim of validated utility performance  
**Owner:** Symtropy / CivBench evaluation  
**Hydraulic data and topology owner:** Sol-Atlas  
**Decision closure owner:** Symtropy  
**Authority, appeal, and institutional-governance owner:** Mycelix

## Why this exists

High sample counts and random sample splits can overstate generalization when related observations are distributed across training and evaluation. Water-network studies commonly generate synthetic scenarios with EPANET/WNTR; the 2026 systematic review found 36 of 90 reviewed studies (40%) used bespoke hydraulic simulation and reported substantial variation in leak definitions, demand patterns, sensor configurations, noise and evaluation practice. A separate 2026 decadal review warns that random splits can place highly similar hydraulic scenarios or segments from the same event, sensor or experimental site on both sides of a split.

CivBench therefore treats split validity as a precondition for interpreting a score. The Rust module `benchmark_split` validates a caller-frozen manifest against explicit provenance-group firewalls. It does not split data automatically, inspect raw telemetry, recalculate content hashes, detect near-duplicates, determine causal validity, or decide whether a test set represents a utility.

## Audited contract

For every sample, the manifest must identify:

- a unique sample ID;
- exactly one of train, validation or test;
- a caller-computed `sha256:<64 hex>` digest of the canonical sample payload;
- one value for each provenance dimension required by the frozen policy.

Supported leakage dimensions are physical event, capture session, pipe/asset, network, topology family, scenario template, sensor layout, hydraulic model/calibration lineage, demand profile and time block.

The policy distinguishes two different properties:

1. **Exclusive dimensions:** a value may occur in only one partition. For example, physical-event and capture-session IDs must not cross train/validation/test.
2. **Shared dimensions:** every distinct value in each listed dimension must be represented in all three partitions. This prevents a purported within-network protocol from quietly including test-only networks or layouts.

The audit always rejects duplicate content digests, duplicate sample IDs, malformed or padded identifiers, missing policy keys, empty partitions, and contradictory shared/exclusive declarations. Exact digest syntax is checked, but the caller is responsible for computing the digest over a canonical payload and retaining the raw manifest. A digest is not a signature, source attestation, or proof of truth.

## Registered split protocols

The Rust audit binds each protocol ID to a registered preset. Callers cannot provide an arbitrary protocol label with a weaker dimension set and still receive a passing audit.

| Registered preset | Exclusive dimensions | Shared dimensions | Valid interpretation |
|---|---|---|---|
| `within_network_unseen_event_v1` | physical event, capture session | network | New events/sessions within the same network |
| `new_operating_regime_v1` | physical event, capture session, demand profile, time block | network, topology family | New demand/temporal regimes within known network/topology |
| `cross_network_v1` | physical event, capture session, network | — | New network instances |
| `cross_topology_v1` | physical event, capture session, network, topology family, scenario template | — | New network and topology/scenario families |
| `sensor_layout_transfer_v1` | physical event, capture session, sensor layout | network, topology family | New sensor placements on known network/topology |
| `asset_transfer_v1` | physical event, capture session, pipe/asset | network | Unseen pipe/asset identities within known network |

The registered requirements are exact in v0.1: adding or omitting a dimension is not silently accepted. A genuinely different protocol needs a new registered preset and explicit review rather than an arbitrary string. For a new-operating-regime study, temporal ordering still requires an independent check that held-out time blocks occur after the training blocks; this module only checks group membership.

## Generalization ladder

Do not publish one headline score called "generalization." Publish separate results for these registered protocols:

| Protocol | Exclusive dimensions | Shared dimensions | What it tests |
|---|---|---|---|
| Within-network unseen event | physical event, capture session, scenario template | network, topology family | New events in a known hydraulic setting |
| New operating regime | physical event, capture session, scenario template, demand profile, time block | network, topology family | Demand/season/operating-regime shift |
| Cross-network | physical event, capture session, scenario template, network | topology family | New network instances under a shared topology family |
| Cross-topology | physical event, capture session, network, topology family, scenario template | — | Transfer beyond familiar topology/scenario generators |
| Sensor-layout transfer | physical event, capture session, scenario template, sensor layout | network, topology family | New sensor placements and sparsity |
| Asset transfer | physical event, capture session, scenario template, pipe/asset | network | Unseen pipe/asset identity |
| Prospective field holdout | event, capture session, time block, sensor layout as applicable | explicitly preregistered site/network scope | Locked prospective operational evaluation |

All registered protocol presets hold out scenario-generator template lineage. This matters for synthetic EPANET/WNTR corpora: a test set can contain new event IDs but still reuse the generator templates and parameterization families exposed during training. A protocol that lacks trustworthy scenario-template identifiers cannot claim this firewall; it must disclose the missing dimension as an unresolved limitation rather than fabricate a value.

These rows are policy templates, not proof that a dataset possesses the listed identifiers. A forward-temporal claim additionally requires an independently checked chronological ordering of time blocks; group disjointness alone does not prove that test dates occur after training dates. Cross-network now holds topology family constant while holding out network IDs; cross-topology holds out both network IDs and topology-family IDs. These are intentionally orthogonal generalization tests. Cross-sensor-layout is not a physical-world transfer test.

A study should first state its target deployment claim, then select and freeze the matching protocol before model tuning. Do not silently change the protocol after seeing the test results.

## Evaluation stack

Detection and localization are separate claims. Report a hierarchy instead of collapsing it into one score:

1. event detection: event-onset precision/recall, PR-AUC, timely/late/missed events, false alarm episodes per monitored system-day, delay distribution;
2. severity: calibrated uncertainty and error by leak magnitude and operating regime;
3. zone: top-k zone localization and coverage;
4. asset/pipe: top-k pipe localization, distance error, candidate-set size and abstention;
5. decision: recommendation acceptance/rejection/deferral, dispatch burden, feasible intervention rate and resource consumption;
6. service: independently verified restoration, essential-service deficit, interruption duration, quality constraints and worst-group service floor.

For localization, pair accuracy with **coverage**: report the fraction of cases where the model issues a localization and its conditional accuracy. Include an explicit abstention option and a risk–coverage curve. A system must not receive credit for high conditional accuracy after silently abstaining on the hardest cases.

For interventions, compare policies under identical crew-hour, part, vehicle, access, shift, energy and spend envelopes. Information gain is not value of information unless it changes the selected feasible action or a consequentially safer decision. Additional sensing with no feasible action change is not automatically beneficial.

## Baselines and fair comparisons

Every protocol should compare at least:

- hydraulic residual / pressure-flow threshold baseline;
- classical statistical or tree model (for example, Random Forest or gradient boosting);
- a representative GNN;
- a representative temporal/Transformer model;
- an acoustic or field-sensor baseline where that modality exists;
- a calibrated hydraulic/digital-twin decision baseline;
- a model-free control baseline for the control lane, where comparable service objectives and constraints exist.

For transfer experiments add a target-only model, same-architecture random initialization, and source-trained/no-adaptation control. Keep preprocessing, tuning budgets, labels, sensor placement, test leakage groups and compute budgets matched. Report raw uncertainty and the number of independent networks/events/sites, not only scenario count.

The 2026 Net3 benchmark's reported detection above 95% on 47,000+ EPANET/WNTR scenarios is a useful synthetic comparator, not a field performance target. A 2026 sparse pressure/flow study reported detection AUPRC 0.901 but Top-5 pipe accuracy 0.122 across 171 candidate pipes on a calibrated Moroccan district network with ten pressure and five flow sensors. This motivates measuring observability and sensor coverage before treating pipe localization as an algorithm-only contest. A 2026 model-free controller reported performance on its studied small-scale systems using 200 samples and 90% lower compute than a typical MPC; it belongs in the control baseline set, not as an assumed universal winner.

## Proposed CivBench acceptance targets

Targets below are **proposed research gates**, not achieved performance:

- Event detection: at least +5 percentage points timely event recall at matched false-alarm episode rate and matched inspection budget, or at least 30% fewer false-alarm episodes at matched recall.
- Dispatch: at least 20% fewer avoidable dispatches at matched missed-event rate under identical resource envelopes.
- Localization: at least 20% relative improvement over the strongest matched baseline at the same coverage and abstention rate; report zone and pipe levels separately. Where the observability ceiling is low, the success criterion is calibrated abstention plus correct pre-localization, not fabricated pipe precision.
- Calibration: predeclare confidence levels and report coverage, Brier/log score or suitable interval coverage, calibration by network/regime and worst-group error. Never hide OOD cases in aggregate calibration.
- Transfer: exceed both target-only and same-architecture random-init/no-transfer controls on held-out networks/topologies, in both directions where data allow.
- Operations: at least 10% lower essential-service deficit area under the same resources, no regression to declared protected-group service floors, and no worsening of p95 inspection/restoration time or water-quality constraints.
- Reproducibility: frozen manifest, source and code digests, split receipt, seeds, model and simulator versions, label provenance, tuning budget, predeclared metrics and negative controls.

Do not combine sample accuracy, event recall, false alarms/month, pipe localization, crew productivity, service outcomes, or litres saved into a synthetic industry leaderboard. Their units and evidence provenance differ.

## Johannesburg operating envelope

Johannesburg Water reported on 23 July 2026 that 53 smart pressure controllers had been commissioned/installed by April 2025, with 306 additional controllers then being installed, and 225 noise loggers installed with 125 more planned for October 2026. The stated target dates have elapsed; the final installed counts are not confirmed by that release and must not be inferred from procurement or planned deployment.

On 29 October 2026 Johannesburg Water described constrained reservoirs/towers, low pressure, intermittent supply and outages, with network reconfiguration, targeted throttling and a proposed Level 1 restrictions package targeting approximately 15% demand reduction. It also states that basic/essential service safeguards and alternative supplies are part of the response. This makes reservoir recovery, spatial service deficit, pressure constraints and essential-supply floors important scenario variables. It does not supply an audited event-level ground-truth dataset or causal intervention outcomes.

The 3 November 2026 systems update is published as image-based daily updates. This is useful operational context, but it is not a public structured telemetry feed and should not be treated as one. Any Johannesburg pilot requires formal data access, privacy and OT-security controls, independent ground truth, and a pre-registered prospective protocol.

## Industry outcome comparators

Industry metrics should be used as operational reference points, not mixed into a machine-learning leaderboard:

- **England, FY 2025/26 (official regulator):** leakage ranged from roughly 11% to 22% of distribution input (DI), 24–55 litres per person per day, and 60–137 litres per property per day. The reported national average was 18.6% of DI; Affinity Water was 15.5%. The regulator explicitly notes that the DI percentage is affected by consumption as well as leakage, so a falling ratio does not necessarily mean absolute leakage fell.
- **Affinity Water / SUEZ / ASTERRA (provider case study):** satellite SAR screened 9,045 km; 763 locations were prioritised; field teams identified 902 leaks; less than 4% of the network was walked; and the case study reports 7.2 leaks per crew-day versus a stated 1.3 industry benchmark. This is a strong operational comparator for *prioritised field productivity*, but it is provider-published and not an independently audited randomized comparison. Satellite anomalies were followed by acoustic confirmation; do not treat all 763 alerts as confirmed leaks.
- **Affinity's regulator outcome is a useful counterweight:** the 2025/26 regulator report lists leakage at 93.4 litres/property/day, better than the 104.5 industry average, while total leakage increased 0.7% year-on-year and average minutes lost per property to supply interruptions increased 37%. A promising detection programme does not guarantee all system-level outcomes improve simultaneously.
- **Johannesburg Water's own programme baseline:** the public programme description reports 15 daily technical teams, over 10,500 km inspected annually (approximately 85% of the network), and annual averages of 1,456 burst pipes, 5,227 leaking meters, 341 leaking valves and 217 leaking hydrants, alongside approximately 8,100 ML/year of estimated water savings. Those programme figures are not directly comparable to ASTERRA's case-study crew-day rate or to model-level event recall; their time windows, definitions and audit basis differ.

A real field evaluation should therefore report **confirmed leaks per crew-day, kilometres inspected per independently verified leak, fraction of prioritised alerts confirmed, false investigations, time-to-confirmation, repair closure, verified water recovered, and service interruption impact**. Match crews, survey area, leak severity, detection modality and time window. Do not assume that 7.2 leaks/crew-day is a transferable target without matching the vendor case's selection and confirmation process.

## Sources

- Farah, E. & Shahrour, I. (2026), "Standardization and Benchmarking of Datasets and Evaluation Protocols for Machine Learning-Based Water Leak Detection: A Systematic Review", *Water* 18(18), 2267. DOI: https://doi.org/10.3390/w18182267
- "AI-based leak detection in water distribution networks: a decadal review across sensing and monitoring scales", *Journal of Hydroinformatics* (2026). https://iwaponline.com/jh/article/28/9/1143/113064/AI-based-leak-detection-in-water-distribution
- Ranganathan et al. (2026), "Benchmarking Diverse Learning Architectures for Leak Detection and Localization in Water Distribution Systems", *Journal of Water Process Engineering* 87, 110114. DOI: https://doi.org/10.1016/j.jwpe.2026.110114
- "Detection and localization limits of sparse pressure-flow monitoring", *AQUA — Water Infrastructure, Ecosystems and Society* (2026). DOI: https://doi.org/10.2166/aqua.2026.300
- Villacres, Putri & Moazeni (2026), "Data-driven, model-free control for reliable operation of water distribution systems: Implementation, benchmarking and validation", *Water Research* 304, 126295. DOI: https://doi.org/10.1016/j.watres.2026.126295
- Chenevey & Cooper (2026), "Model or Digital Twin? AWWA Survey on Water Distribution Technology", *Journal AWWA* 118, 36–45. DOI: https://doi.org/10.1002/awwa.70075
- Johannesburg Water, "Smart technology rollout to reduce water losses" (23 July 2026): https://www.johannesburgwater.co.za/blog/2026/07/23/johannesburg-water-accelerates-smart-technology-rollout-to-reduce-water-losses/
- Johannesburg Water, "Urgent interventions to stabilise severely constrained systems" (29 October 2026): https://www.johannesburgwater.co.za/blog/2026/10/29/johannesburg-water-implements-urgent-interventions-to-stabilise-severely-constrained-systems/
- Johannesburg Water, "System Updates 3 November 2026": https://www.johannesburgwater.co.za/blog/2026/11/03/system-updates-3-november-2026/

- UK Environment Agency / Ofwat / DWI / NRW, "Water resources 2025 to 2026: analysis of the water industry's annual water resources performance" (updated 29 October 2026): https://www.gov.uk/government/publications/water-resources-2025-2026-analysis-of-the-water-industrys-annual-water-resources-performance/water-resources-2025-to-2026-analysis-of-the-water-industrys-annual-water-resources-performance
- UK joint regulator report, "Water industry performance insights" (29 October 2026): https://www.gov.uk/government/publications/water-industry-performance-insights-joint-regulator-report/water-industry-performance-insights-joint-regulator-report
- SUEZ / ASTERRA / Affinity Water, "Affinity Water Uses Satellite Intelligence to Accelerate Leak Detection": https://www.suez.com/en/uk/case-studies/satellite-leak-detection-delivers-results-affinity-water
- Johannesburg Water, "Active Leak Detection Programme": https://www.johannesburgwater.co.za/about-us/campaigns/active-leak-detection-programme/
