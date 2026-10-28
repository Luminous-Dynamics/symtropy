# Polar resilience benchmark targets and industry-comparison protocol

**Status: proposed research protocol. Targets below are hypotheses, not achieved results.**  
**Current evidence ceiling: synthetic scorer correctness only.** This crate scores supplied traces; it does not simulate electrical/thermal dynamics, optimize dispatch, certify a controller, or validate an Arctic/Antarctic site.

## New: source-backed environmental trace replay

The next implementation boundary is a reproducible environmental trace, not just another scenario label. See [ENVIRONMENT_TRACE_REPLAY_PROTOCOL_2026.md](ENVIRONMENT_TRACE_REPLAY_PROTOCOL_2026.md), the [trace-manifest schema](schemas/environment-trace-manifest-0.1.0.schema.json), and the [Namib trace manifest template](fixtures/environment-trace-manifest-template-2026.json). The template intentionally contains no source URL, coordinates, dates, hashes, or observed values; it is not an ingested trace. The [NASA POWER acquisition adapter](scripts/fetch_and_normalize_nasa_power_daily.sh) and [normalizer](scripts/normalize_nasa_power_daily.sh) provide a source-pinned daily pilot, while a clearly synthetic offline fixture exercises conversions and missing-data cases. [The artifact verifier](scripts/verify_environment_trace_artifacts.sh) recomputes artifact digests and checks byte size/row count. The [Environment Trace Contract workflow](../../../.github/workflows/environment-trace-contract.yml) runs those tests alongside the Rust metadata gate. These checks do not replace physical-data/model validation.

The protocol prioritizes chronology-preserving weather/solar traces, explicit gaps and uncertainty, whole-episode temporal holdouts, source-specific field observations for fog/salt/dust, fair baseline information/compute budgets, and distinct integrity/model/service-resilience gates. Performance superiority is a hypothesis until a suitable open baseline is reproduced and a locked comparison passes.

## Environment domain coverage: deserts and the full solar-system trade space

The [Earth Desert and Solar-System Environment Domain Catalog](ENVIRONMENT_DOMAIN_CATALOG_2026.md) defines the proposed environment taxonomy, and the [machine-readable catalog](fixtures/environment-domain-catalog-2026.json) records scenario IDs, coupled-hazard hypotheses, required model domains, metrics, source links, evidence limitations, and implementation status. Its schema is [environment-domain-catalog-0.1.0.schema.json](schemas/environment-domain-catalog-0.1.0.schema.json). The [compound-test matrix](fixtures/environment-compound-test-matrix-2026.json) provides 51 per-profile coverage templates plus 19 cross-profile interaction templates. All 70 are design-only and have no fabricated numeric environment values.

Deserts are a first-class Earth benchmark family: hot arid, hyper-arid, coastal fog desert, cold desert, high-altitude dry terrain, salt flat, dust storm, and compound heatwave/drought each stress different assumptions. The first test queue should prioritize dust deposition plus thermal derating; dust storm plus water scarcity and delayed maintenance; cold-desert night plus depleted storage/cold start; coastal salt plus dust; and high-altitude derating.

The initial catalog also covers cross-cutting space conditions (vacuum/thermal cycling, solar weather, radiation, low gravity, communication latency and planetary protection), terrestrial bodies, gas/ice-giant atmospheric levels, key moons, asteroids/comets, and outer-system objects. It intentionally does not collapse them into one "environment score". Venus's surface, a Venus cloud-layer vehicle, Titan's cryogenic hydrocarbon surface, Europa's radiation-exposed ice, a lunar permanently shadowed region, and Neptune at a specified atmospheric pressure are separate physical contracts, not interchangeable variants of a generic hostile climate.

Current status: these are **scenario families to parameterize and validate**, not 51 validated models or executed tests. All 51 catalog profiles are currently marked planned. CI checks catalog IDs, status/evidence metadata, and one-to-one coverage-template presence, not physical correctness or habitat feasibility. Unmodelled conditions remain unknown; cross-domain transfer requires an explicit model and held-out validation.

## Why this benchmark should differ

A conventional asset-availability or energy-cost metric can look good while people lose heat, water, sanitation, or communications. Evaluate delivered services over time against owner-approved floors; report fuel, cost, repair burden, compute, and other trade-offs separately. Lower fuel use never compensates for a breached critical-service floor.

Distinguish two questions. Where credible scenario probabilities exist, use probability-weighted metrics and tail measures with uncertainty. Where probabilities are not defensible, report each enumerated stress scenario and worst-case outcomes; do not relabel an unweighted stress suite as expected risk. This reflects recent microgrid reliability research emphasizing chronological adequacy and correlated scarcity rather than a single headline indicator. HIL, commissioning, and periodic controller testing are separate evidence stages, not certificates implied by a passing unit test.

## Three linked, separately scored benchmark tracks

Do not try to make one metric cover slow energy adequacy, fast inverter stability, and cyber-physical integrity. Share provenance, scenario identity, and service-delivery traces across tracks, but keep each domain's physical metrics and acceptance criteria distinct.

### Track A — long-horizon critical-service resilience

This is the existing scorer's scope: delivery of owner-approved heat, water, communications, sanitation, and other declared services through chronological renewable scarcity, reserve depletion, equipment faults, delayed resupply, repair queues, and recovery. Physical unit-hours remain separated by service; the normalized service-deficit summary supplements rather than replaces them. Repeated-event tests must carry state across events.

### Track B — fast electrical dynamics and controller implementation

A suitable electrical benchmark should separately measure frequency and voltage excursions, settling/recovery, current-limit behavior, synchronization/stability margins, protection interactions, computation/decision deadlines, and sensitivity to controller tuning. It must use a matched plant, disturbance sequence, controller-visible input, protection envelope, and metric extractor for all comparator arms; low-order models should be checked against an appropriate EMT or laboratory reference before strong electrical claims are made.

A relevant recent comparison is [A Reproducible Weak-Grid Benchmark with Switching-Averaged EMT Validation for Battery-Backed Grid-Forming Control in PV Microgrids (Energies, 25 June 2026)](https://doi.org/10.3390/en19133017). It compares droop, virtual-synchronous-machine, and power-synchronization control under matched plant data, load disturbance, grid-strength reduction, voltage sag, and current limits, then checks the low-order response with a switching-averaged EMT layer and tuning sensitivity. Its result is a benchmark-specific trade-off among frequency moderation, restoration speed, stability, and implementation burden—not a universal controller ranking. Symtropy should reproduce an appropriate public baseline before extending it to polar constraints and compound events, and should not claim superiority over that paper without a matched, independently reproduced experiment.

### Track C — cross-layer cyber-physical integrity

This track tests whether telemetry freshness, sensor disagreement, command validity, actuator response, controller fallback, and state re-synchronization preserve service delivery under safe, isolated fault injection. Keep evidence-integrity metrics (stale/invalid command handling, detection/fallback latency, command-versus-plant mismatch) separate from the resulting service losses. A detector firing is not proof of containment; stable network uptime is not proof that commands were physically correct.

### Cross-track experiment contract

Every paired comparison should share a stable study ID, scenario/trajectory ID, initial-state digest, model revision, service-contract digest, and policy-visible input trace. Each track may add its own validated model/testbed digest and domain-specific measurements. A cross-layer case must explicitly record which layers were active and which were not covered. Never infer electrical stability from energy-service averages, infer multi-day energy adequacy from a short EMT trace, or infer cybersecurity from correct service metrics alone.

## Comparison arms

Use identical episode boundaries, service contracts, input data, constraints, hardware, and reporting rules. Pre-register tuning budgets and freeze the final hold-out set.

| Arm | Purpose | Fairness requirement |
|---|---|---|
| Documented incumbent/operator policy | Real operational reference | Version the policy and record operator overrides |
| Transparent rule-based policy | Reproducible low-complexity baseline | Publish thresholds, priorities, fallback behavior |
| LP/MILP planner, when the model supports it | Optimization baseline | Same asset limits, forecasts, horizon, input information |
| MPC/receding-horizon optimizer, when feasible | Test value of replanning | Match decision deadlines and forecast access |
| Robust/scenario optimizer, when uncertainty is modeled | Uncertainty-aware comparator | Same stress cases and documented uncertainty set |
| Proposed controller/planner | Candidate | Freeze the candidate before final evaluation |
| Negative controls | Demonstrate fault detection | Floor violator, failed actuator, stale sensor, invalid evidence |

Do not claim to beat “industry” using a straw-man baseline. Name specific commercial controllers only when version, configuration, model boundary, inputs, hardware and results are documented and reproducible. A paper, vendor claim, simulation, and HIL result are different evidence classes.

### Additional electrical-track targets (separate from service resilience)

These also need a frozen study-specific protocol; they are not universal limits.

| Dimension | Acceptance approach |
|---|---|
| Baseline reproduction | Reproduce an appropriate published/open baseline and its primary metrics within declared tolerances before changing the plant or adding polar hazards |
| Matched comparisons | Identical plant, perturbations, protection/current limits, input visibility, solver settings, output sampling and metric extraction across controller arms |
| Dynamic constraints | Zero violations of explicitly declared voltage/frequency/current/angle/protection envelopes in the selected test suite, subject to validated model boundaries |
| Model cross-check | Report discrepancies between reduced-order trajectories and EMT/reference results; explain any controller rank reversal rather than selecting the more favorable model |
| Tuning robustness | Report performance and failure-boundary sensitivity under preregistered parameter perturbations and tuning budgets |
| Real-time execution | No missed control deadlines in the declared test suite on named hardware; publish latency distribution and fallback for missed deadlines |

## Proposed quantitative targets

These are goals for a future preregistered experiment—not current results. Freeze service floors, engineering tolerances, hardware, and primary outcomes before running comparisons.

| Dimension | Proposed target | Pass/fail interpretation |
|---|---|---|
| Scoring integrity | 100% of defined negative controls rejected; zero false-pass outcomes in the test set | Any known floor-breaching control reported as eligible fails the benchmark |
| Critical-service performance | At least **10% lower relative criticality-weighted unserved service-fraction-hours** than the strongest feasible baseline on held-out episodes | This time-integrated normalized deficit is not a physical unit sum; publish per-service physical shortfalls separately and require uncertainty to support improvement |
| Outage persistence | Lower longest consecutive system-level service-floor breach duration than the strongest eligible baseline on held-out compound events, without increasing breach counts | Preserve timeline and event boundaries; equal total breach-hours must not hide longer continuous loss |
| Endurance before failure | Longer time from event start to the first critical-service floor breach than the strongest eligible baseline on held-out outage trajectories | Treat traces without an observed breach as right-censored at the trace horizon; do not interpret them as infinite endurance |
| Critical-floor protection | No new candidate floor failure on an episode/service where the comparator passes, unless explicitly approved through prior safety review | New critical-floor breach blocks an unqualified “better” claim |
| Tail performance with credible probabilities | Aim for **15% lower CVaR at 95% confidence** for preregistered shortfall loss than the strongest feasible baseline | Report event-level outcomes, uncertainty, probability provenance, and sensitivity |
| Service-specific tail safety | Report expected loss, CVaR95 where probabilities are defensible, worst-case shortfall, and breach probability for every declared critical service; no aggregate improvement claim may hide a critical-service regression | Service-specific hard floors remain independently binding; inactive services report null losses rather than zeros |
| Probability-free stress performance | Lower worst-case shortfall and no increase in floor-breach scenarios relative to the strongest baseline | Same frozen stress set; make no frequency or expected-value claim |
| Fuel/resupply | Improve the fuel/resupply Pareto frontier at matched service outcomes; pre-approve any tolerated trade-off | Fuel or cost gains cannot excuse critical-service failure |
| Generalization | Improve on held-out compound hazards and report degradation by hazard family and geography | Keep Arctic land, Antarctic stations, and mobile/ice platforms distinct unless transfer is justified |
| Decision execution | 100% valid outputs inside the frozen decision deadline on named target hardware | Publish p50/p95/p99 latency, peak memory, platform energy, misses, and fallback behavior |
| Evidence reproducibility | 100% of runs link exact commit, input/config hashes, scenario IDs, units, model versions and result hashes | Missing lineage makes a run non-comparable and not independently reproducible |
| HIL progression | 100% of applicable, selected test cases pass at the agreed HIL boundary before a site trial | Project gate only—not a claim of IEC conformance |

Compare against the strongest *eligible* baseline, never a selectively weak one. Report absolute values beside percentages. The system-weighted tail score must be accompanied by per-service tails: in a synthetic example, total shortfall CVaR can be materially lower than water shortfall CVaR while the system average still looks acceptable. A 10% relative gain may be trivial at low absolute risk; owners must also set absolute service minima. Use episode/block-level uncertainty methods that preserve temporal correlation; do not treat each timestep as an independent sample. If too few independent sites or severe events exist, state that limitation rather than manufacture statistical power.

## Cyber-physical integrity under injected faults

The complement to long-horizon weather/reserve testing is a controlled cyber-physical fault suite. A controller can have good communications uptime yet act on stale or biased telemetry, or fail to meet service demand when an actuator does not follow its command. Test stale/replayed samples, sensor drift/disagreement, communications partitions, command/actuator mismatch, common-cause dependencies, and safe recovery after state resynchronization in isolated replay, simulation, or suitable HIL environments. Measure detection/fallback latency and unsafe-command emissions independently from delivered-service loss.

The detailed [cyber-physical stress-test protocol](CYBERPHYSICAL_STRESS_TEST_PROTOCOL.md) defines proposed comparison arms, negative controls, metrics, and stage gates. This is proposed work, not a implemented detection/control feature. Recent research highlights end-to-end cyber resilience for renewable-heavy grids with digitally connected devices ([Nature Reviews Electrical Engineering, 2026](https://www.nature.com/articles/s44287-026-00334-2)); that informs our test design but is not evidence of capability in this project.

## Long-horizon resilience: benchmark the trajectory, not just the storm

A further paradigm shift is to evaluate a **continuous operating trajectory across seasons and repeated hazards**, not just one representative outage at a time. Recent research argues that event-by-event resilience can be hard to interpret for climate risks that evolve across decades; the benchmark should therefore keep event-specific tests, while also measuring system performance across long horizons and multiple plausible futures ([Nature Reviews Electrical Engineering, 2026](https://www.nature.com/articles/s44287-026-00322-6)). Related resilience-curve research emphasizes event-level outage volume, nadir, duration, and restoration dynamics that time-averaged statistics can conceal ([2026 review](https://pmc.ncbi.nlm.nih.gov/articles/PMC12905357/)).

Protocol requirements for a future trajectory-capable simulator:
- Carry state continuously across events: battery state of charge, thermal storage, fuel, water reserves, equipment wear, repair queues, spare inventory, and forecast/telemetry state. Never silently reset resources between storms.
- Separate seasonal weather sequences from independent stress-injection tests. Label whether the sequence is observed, calibrated, generated, or adversarially constructed.
- Score the whole horizon and each event window: critical-service shortfall, total and longest consecutive breach duration, breach-window counts, time to first breach, recovery, reserve exhaustion, lifecycle fuel/resupply, and maintenance backlog.
- Use multiple climate/resource futures and sensitivity bounds. Do not claim future occurrence probabilities unless their estimation is justified; use probability-free scenario envelopes where they are not.
- Preserve cold-start, black-start, and replenishment transitions between windows. A later storm should encounter whatever state the earlier event leaves behind.
- Freeze the scenario generator and seeds before final evaluation; separately report coverage of hazard families and compound dependencies rather than assuming a finite scenario list is exhaustive.

This protocol is a proposed next step, not a feature already implemented by this trace scorer. The current scorer preserves temporal ordering in provided traces, but it neither generates weather nor simulates equipment state or multi-year asset degradation.

## Frozen hazard and fault matrix

Start from `../STRESSOR_MEASUREMENT_MATRIX.md`. Use traceable cases and compound event sequences, not only isolated faults:

- Renewable scarcity during polar night, low wind, icing, and cold-related battery derating.
- Prime-power or inverter/control loss while critical demand remains high.
- Delayed fuel/spares resupply coincident with a prolonged outage.
- Communications loss, stale telemetry, sensor drift, and common-cause failures.
- Black-start/cold-restart, frozen water/wastewater systems, and snow/ice access blockage.
- Maintenance delays, limited spares, and technician constraints in cold/gloved conditions.

For a non-applicable stressor, record **not applicable and why**; never silently drop it. Keep calibration data, tuning data, and final hold-out episodes separate. Preserve event duration and state carry-over, and test both single faults and correlated/compound events.

## Evidence ladder: what a result is allowed to claim

| Level | Evidence | Permitted conclusion |
|---|---|---|
| E0 — contract | Unit tests, schema checks, negative controls, metamorphic tests | Scorer/report contract behaves as tested |
| E1 — synthetic traces | Hand-authored or generated fixtures with stated assumptions | Metric pipeline works on those synthetic inputs |
| E2 — calibrated model | Traceable inputs, validated component/system models, held-out episodes and sensitivity | Model-conditional comparison within validated scope |
| E3 — HIL/lab | Documented testbed, real controller hardware or power hardware as appropriate, repeatable fault injection | Behavior validated at stated lab/HIL boundary |
| E4 — field/commissioning | Qualified review, site-representative tests, operating constraints and monitored outcomes | Evidence limited to tested configuration, site, and period |

Evidence does not automatically rise because a workflow passes. Synthetic-only output is not physical validation, controller certification, engineering safety evidence, or superiority to state of the art.

## Industry anchors and limitations

- [IEC TS 62898-3-5:2026](https://webstore.iec.ch/en/publication/84175) covers HIL, commissioning, and periodic testing for microgrid monitoring, control, and energy-management systems. Use it to build a gap map; this benchmark alone does not show compliance.
- [NREL/NLR, Hardware-in-the-Loop Test Bed and Test Methodology for Microgrid Controller Evaluation](https://www.nrel.gov/docs/fy20osti/71338.pdf) provides controller-evaluation methodology.
- [Reliability as a design principle: A systematic review and integrated framework for renewable-based microgrids (2026)](https://www.sciencedirect.com/science/article/pii/S2590174526003612) motivates time-coupled adequacy, correlated scarcity, and service-critical tail measures.
- [Quantifying Microgrid Resilience by Determining Survivable Durations of Random Outages (IEEE Access, 2025)](https://doi.org/10.1109/ACCESS.2025.3626579) motivates reporting how long critical service can be sustained before failure, rather than only average service fractions.
- [IEEE 2030.8-2018: Testing of Microgrid Controllers](https://standards.ieee.org/ieee/2030.8/6169/) is an existing test reference; its successor [P2030.8](https://standards.ieee.org/ieee/2030.8/12543/) was approved as an active revision project on 26 March 2026.
- [A Reproducible Weak-Grid Benchmark ... (Energies, 2026)](https://doi.org/10.3390/en19133017) is a reproducibility pattern for matched grid-forming controller comparison and a model-to-EMT consistency check, not a polar microgrid validation.

The current code demonstrates metric calculation on synthetic fixtures only. The next scientific milestone is not a higher dashboard score; it is a frozen, reproducible, site-relevant comparison whose evidence can falsify the candidate's value proposition.
