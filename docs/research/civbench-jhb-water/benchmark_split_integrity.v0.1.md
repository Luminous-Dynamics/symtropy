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
2. **Shared dimensions:** at least one identical value must be represented in all three partitions. This allows a protocol to assert that it is genuinely within the same network rather than accidentally turning into a transfer experiment.

The audit always rejects duplicate content digests, duplicate sample IDs, malformed or padded identifiers, missing policy keys, empty partitions, and contradictory shared/exclusive declarations. Exact digest syntax is checked, but the caller is responsible for computing the digest over a canonical payload and retaining the raw manifest. A digest is not a signature, source attestation, or proof of truth.

## Generalization ladder

Do not publish one headline score called "generalization." Publish separate results for these registered protocols:

| Protocol | Exclusive dimensions | Shared dimensions | What it tests |
|---|---|---|---|
| Within-network unseen event | physical event, capture session, scenario template | network, topology family | New events in a known hydraulic setting |
| New operating regime | physical event, capture session, demand profile, time block | network, topology family | Demand/season/operating-regime shift |
| Cross-network | physical event, capture session, network | — | New network instances |
| Cross-topology | physical event, capture session, network, topology family, scenario template | — | Transfer beyond familiar topology/scenario generators |
| Sensor-layout transfer | physical event, capture session, sensor layout | network, topology family | New sensor placements and sparsity |
| Asset transfer | physical event, capture session, pipe/asset | network | Unseen pipe/asset identity |
| Prospective field holdout | event, capture session, time block, sensor layout as applicable | explicitly preregistered site/network scope | Locked prospective operational evaluation |

These rows are policy templates, not proof that a dataset possesses the listed identifiers. A forward-temporal claim additionally requires an independently checked chronological ordering of time blocks; group disjointness alone does not prove that test dates occur after training dates. Cross-network is not cross-topology unless topology family is also held out. Cross-sensor-layout is not a physical-world transfer test.

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
