# Chronological Adequacy and ELCC-Like Evaluation for Flexible Data Centres

**Protocol:** GRID-SYSTEM-ELCC-001  
**Version:** 0.3  
**Status:** single-zone chronological prototype; not production system accreditation

## 1. Objective and paradigm shift

Move from three insufficient proxies—maximum observed kW, theoretical deferrable-work fraction, and repeated dispatch event pass rate—to a system-level counterfactual:

> How much additional constant system load can be served while preserving a frozen reference adequacy criterion after the data-centre portfolio's actual chronological flexibility and recovery trajectory is applied?

The calculation follows the ELCC logic used by EPRI: establish reference system adequacy risk, add the candidate resource, then increase system load until the reference risk level is restored. ELCC is system- and portfolio-dependent; it is not an intrinsic property of one site. [EPRI resource accreditation methods](https://msites.epri.com/resource-adequacy/resource-acredidation/resource-accreditation-methods).

The prototype is deliberately smaller than this target. It provides deterministic arithmetic over caller-supplied weighted chronological scenarios, a one-zone copperplate balance, and a one-dimensional load search. It is a stepping stone for a validated multi-zone probabilistic resource-adequacy engine.

## 2. V0.1 scope and equations

Each scenario is an aligned chronological path with a caller-supplied positive probability weight. Each interval has:
- reference system load (kW), including the data-centre load treated as inflexible in the reference case;
- available supply (kW), after caller-modeled generator/renewable/storage/import availability and outages;
- signed net load reduction (kW): positive for reduced data-centre load, negative for recovery/rebound;
- interval evidence ID and day index.

Reference unserved power:

    U_ref(t) = max(L(t) - G(t), 0)

Augmented unserved power for constant added system load d:

    U_flex(t, d) = max(L(t) + d - R(t) - G(t), 0)

where R(t) is signed. A negative R(t) is rebound and increases net load.

Unserved energy for interval duration Δt hours:

    EUE = Σ_t U(t) × Δt / 1000  [MWh]

Loss-of-load hours count intervals with U(t) above the configured numerical shortfall threshold. Loss-of-load days count each supplied day index once if any interval on that day has unserved load. Scenario metrics are aggregated using normalized caller-supplied scenario weights.

The caller selects exactly one primary adequacy metric for the load search: expected loss-of-load periods, elapsed loss-of-load hours, loss-of-load days, expected unserved energy, or upper-tail EUE CVaR at a declared confidence level. The report also retains the companion occurrence, duration, severity and rebound metrics. The result is the largest tested d (to the configured kW search tolerance) for which the selected augmented metric does not exceed the reference metric plus an explicit numerical/decision tolerance.

The horizon is **not annualized**. A caller may provide complete chronological years, but the engine does not infer that a partial or synthetic trace represents a year. Metrics are expected values over the supplied, aligned trace horizon.

## 3. Claim boundaries

V0.1 is not an ISO/TSO capacity accreditation, ELCC/EFC certification, LOLE standard compliance tool, or grid interconnection study. It does not model:
- transmission topology, nodal congestion, deliverability, losses or N-1 network constraints;
- unit commitment, ramp reserves, forced-outage transitions or economic dispatch;
- frequency response, voltage stability, inertia, protection or islanding;
- explicit storage state-of-charge, thermal state, workload queues, deadline constraints or compute-service degradation;
- scenario generation, statistical calibration, extreme-value distributions, confidence intervals or uncertainty in scenario probabilities;
- dynamic capacity-expansion feedback or endogenous market behavior.

Those factors must be represented in upstream scenario construction and/or added to future engine layers before the result is used as a planning or accreditation claim. In V0.1, the signed load-reduction trajectory is accepted as supplied evidence; the engine does not independently prove it can be delivered.

Zero reference risk is rejected as unidentifiable under this finite-trace method. If the selected criterion is already zero across the study, a larger system can be added without violating that observed criterion, so the finite study does not identify the capacity contribution. A search that reaches its configured maximum is reported as censored, not an exact value. If the resource trajectory worsens the selected metric at zero added load, the evaluator reports no positive capacity value.

## 4. Research and industry comparison

### EPRI: probabilistic accreditation workflow

EPRI documents ELCC, equivalent firm capacity (EFC), and equivalent conventional power (ECP) as distinct methods. Its ELCC workflow restores the baseline adequacy metric after adding a resource. It also emphasizes dependence on system composition, load, outages, weather, and resource-addition order. V0.1 follows the core counterfactual but lacks the production adequacy model needed to claim industry-equivalent ELCC. [EPRI methods](https://msites.epri.com/resource-adequacy/resource-acredidation/resource-accreditation-methods).

### LBNL: accreditation under evolving grid conditions

LBNL's July 2026 report reviews accreditation changes in California, MISO and PJM against extreme weather, energy-limited resources, retirements and data-centre load growth. This makes a single fixed site percentage an inadequate planning abstraction. The roadmap must report conditional and portfolio-dependent results, not reuse a universal flexibility fraction. [LBNL July 2026 report](https://eta.lbl.gov/publications/resource-accreditation-us-resource-adequacy).

### ESIG: integrate large loads before interconnection

ESIG's April 2026 Large Loads Task Force report proposes a six-step framework for bringing flexibility into resource-adequacy planning from the beginning, instead of treating curtailment or bring-your-own-generation as only short-term remedies. This supports an end-to-end flow from workload/service contracts to chronological resource accreditation. [ESIG report](https://www.esig.energy/reports-briefs/large-loads-resource-adequacy/).

### NLR / IEEE: probabilistic best practice

The 2026 review by Mantegna, Huang, van Caelenberg, Frew, Lynch and O'Malley surveys seven resource-adequacy best practices and finds industry is not far behind them, while identifying remaining gaps. LBNL's broader guide identifies chronological hourly simulations over all hours in a year, with better weather dependency representation, as best practice. V0.1 is substantially below that benchmark by design. [NLR review](https://doi.org/10.1109/TEMPR.2025.3640116) [LBNL guide](https://energyanalysis.lbl.gov/publications/guide-improved-resource-adequacy).

### IEA: three different flexibility products

The IEA 4E July 2026 report distinguishes market-serving, grid-serving and system-serving flexibility. A resource that can shift price-responsive workloads is not automatically a transmission relief resource or a resource-adequacy resource. Symtropy should keep product-specific constraints, evidence, and claim ceilings separate. [IEA 4E report](https://www.iea-4e.org/edna/publications/data-centres-and-flexibility/).

### Data-centre workload flexibility studies

The May 2026 LAD-Flex study used more than one million Alibaba cluster tasks and reported up to 22% load shedding during modeled flexibility windows. It is useful as a workload-model comparator, not as a universal capacity credit. The 2026 multi-region system study by Senga, Wang and Knittel found that flexibility reduced system costs but its emissions effect depended on renewable penetration and generation mix. Thus, capacity value, operating cost, carbon and service quality need separate outcomes. [LAD-Flex study](https://doi.org/10.1016/j.ijepes.2026.111940) [Senga et al.](https://cs3.mit.edu/publication/118896).

## 4A. First regional benchmark: South Africa

South Africa should be an early regional validation target rather than an afterthought to a U.S.-centric benchmark suite. NTCSA's official system-status page lists the MTSAO 2027–2036 and explains that the ten-year horizon covers demand, planned plant shutdowns, dispatchable capacity and major transmission-project delivery. The official report and underlying assumptions must be archived and reconciled before any numerical replication claim is made. [NTCSA MTSAO page](https://www.ntcsa.co.za/system-status-reports/).

There is a timely fuel-deliverability sensitivity. On 28 October 2026, the Department of Electricity and Energy reported that GASIPPPP Bid Window 1 sought 2,000 MW and received four bids totaling 2,830 MW, but appointed no preferred bidders because the required gas-supply assurances were not met. The Department also stated that it remains committed to gas-to-power and is considering the enabling conditions for a future BW1.5. The correct model treatment is therefore **not** “gas is cancelled”; it is to keep strategic procurement intent distinct from a committed, fuel-secured, commissioned and available resource. Run on-time, delayed, constrained and absent-gas cases. [DEE official statement, 28 October 2026](https://www.dee.gov.za/government-announces-gasipppp-bid-window-1-outcome-reaffirms-market-commitment-and-outlines-the-path-forward).

A government statement on 7 October 2026 identifies a first IRP 2025 section 34 determination prioritising 4.6 GW of BESS and 5 GW of gas-to-power. Treat BESS as energy-limited storage, not flat capacity: preserve power, energy, duration, efficiency, degradation and state-of-charge constraints in every chronology. [South African Government statement](https://www.gov.za/news/media-statements/minister-kgosientsho-ramokgopa-prioritises-battery-storage-and-gas-power).

A secondary analysis of the MTSAO reports High Demand trajectory values of 49.1 GW peak and 329.7 TWh sent-out energy in 2036 (versus 35.8 GW and 238.3 TWh in 2026) and reports 13.42 GWh unserved energy and 377 hours in the 2036 Reference Case. These are provisional extraction targets only: they must be reconciled to the official report's tables, metric definitions, horizon and scenario boundaries before use as benchmark truth. [Moneyweb analysis, 2 November 2026](https://www.moneyweb.co.za/moneyweb-opinion/fragile-assumptions-behind-south-africas-optimistic-electricity-outlook/).

The 2 November 2026 secondary analysis adds important stress-design detail: the MTSAO model inputs were fixed in June 2026 and did not incorporate the 28 October gas procurement outcome. Its Reference Case assumes 70% Eskom EAF from FY2031 onward and gas additions from 2032; the downside EAF trajectory is 65%. The reported Perfect Storm combines high demand, 65% EAF, no new gas and slower small-scale embedded generation uptake. It must not be interpreted as a causal estimate of gas delay alone. The experiment matrix should separate EAF and gas availability before testing their combined adverse interaction. [Moneyweb analysis](https://www.moneyweb.co.za/moneyweb-opinion/fragile-assumptions-behind-south-africas-optimistic-electricity-outlook/) [Daily Maverick analysis](https://www.dailymaverick.co.za/article/2026-11-02-the-fragile-assumptions-behind-south-africas-optimistic-electricity-outlook/).

That secondary analysis also reports approximately 16.2 GW of Eskom capacity retirements by 2036 and MTSAO assumptions for roughly 14,508 km of added transmission lines and 144 GVA of transformer capacity. Its Reduced Storage sensitivity (BESS limited to 1,744 MW) reportedly increases 2036 unserved energy from 13.42 GWh to 28.08 GWh and excess energy from 3.54 TWh to 6.37 TWh. These values remain **provisional transcription targets** until reconciled against the official report's tables and definitions; do not put them directly into a benchmark fixture yet.

The current MTSAO is the target reference rather than a weak baseline to dismiss. NTCSA's official page describes a ten-year 2027–2036 adequacy assessment of available, committed and anticipated generation against demand and ancillary services, incorporating shutdowns, dispatchable capacity and transmission-project delivery. [NTCSA System Status Reports](https://www.ntcsa.co.za/system-status-reports/). Secondary reporting says the study uses hourly network-constrained simulations and reports 2036 Reference Case EUE of 13.42 GWh over 377 shortage hours; High Demand peak is 49.1 GW and sent-out energy 329.7 TWh in 2036. These are replication targets only until directly extracted from the official PDF and reconciled to its tables and metric definitions. [Daily Maverick MTSAO analysis, 2 Nov 2026](https://www.dailymaverick.co.za/article/2026-11-02-the-fragile-assumptions-behind-south-africas-optimistic-electricity-outlook/).

The secondary analysis reports the Perfect Storm case at more than 900 GWh unserved energy in 2036, above a 20 GWh criterion, but it combines high demand, 65% EAF, no new gas and slower SSEG. That is a compound stress—not the isolated impact of losing gas. The factorial comparison must separate 70% vs 65% EAF and gas-on-time vs gas-delayed/absent, then overlay storage and transmission delivery sensitivities. The MTSAO's four scenario families (Base Case, Risk Adjusted, All Planned, Accelerated Build) should be reproduced before candidate data-centre flexibility is added. The external benchmark already includes hourly/network-constrained sophistication absent from the Symtropy copperplate prototype.

The South African benchmark should include:
- EAF sensitivities and explicit thermal forced-outage correlations;
- delayed, constrained and absent gas with fuel-network/terminal dependencies;
- planned, delayed, reduced and energy-limited BESS, including duration and reserve floor;
- low-wind/solar periods coincident with demand peaks and thermal outages;
- coal retirement, major transmission-project delay and N-1 network constraints;
- Gauteng and other candidate data-centre PCC nodes with actual transmission/substation deliverability, wheeling and tariff boundaries;
- high-demand and lower-demand trajectories, as well as high-temperature cooling derates and water stress.

Do not infer local PCC deliverability from national adequacy, nor infer a national capacity credit from the V0.1 copperplate prototype. The official MTSAO report itself has not yet been ingested and independently reconciled in this prototype branch.

## 4A. Adequacy metrics: frequency, duration, event-days, energy and tail severity

These metrics are not synonyms and must not be relabelled to make a cross-model comparison look equivalent.

- **Loss-of-load events — `LossOfLoadEvents`:** expected number of distinct contiguous shortfall spells. A spell begins when unserved power exceeds the configured epsilon and ends at the first interval in which the adequacy criterion is met. Consecutive shortage intervals remain one event across a calendar-day boundary. This is the metric concept used by NTCSA's LOLE definition.
- **Loss-of-load periods — `LossOfLoadPeriods`:** expected number of simulation intervals containing any shortfall. This is PRAS-style LOLE and is dependent on the chosen interval resolution. Two 15-minute shortfall intervals count as two periods.
- **Loss-of-load hours — `LossOfLoadHours`:** expected elapsed duration of all shortage intervals, calculated as their count multiplied by the interval duration. This corresponds to the duration concept in NTCSA LOLH. At one-hour resolution, its numeric value can equal PRAS LOLE, but the definitions remain different.
- **Loss-of-load days — `LossOfLoadDays`:** expected calendar days with one or more shortage intervals (the PRAS LOLD concept).
- **Expected unserved energy — `ExpectedUnservedEnergyMwh`:** probability-weighted mean unserved electrical energy over the supplied horizon.
- **Upper-tail EUE CVaR — `UpperTailUnservedEnergyCvar`:** weighted upper-tail mean of scenario-level EUE in MWh. This is the energy-severity statistic documented by PRAS `CVAR(:energy, ShortfallSamples(), alpha)`.
- **Upper-tail shortfall-power CVaR — `UpperTailShortfallPowerCvarKw`:** weighted upper-tail mean of interval shortfall magnitudes in kW. This keeps power severity distinct from EUE. Its V0.3 implementation weights each interval equally within a scenario and includes zero-shortfall intervals. It is a transparent prototype of an NTCSA-style power-magnitude tail, **not an exact reproduction** of NTCSA's published Monte Carlo observation population or quantile convention.

NTCSA defines LOLE as the frequency of discrete shortage events, LOLH as the number of hours with shortage, and CVaR as the mean MW shortfall magnitude among observations at or beyond the 95th percentile. PRAS defines LOLE as expected shortage event-periods, LOLD as event-days, EUE as expected energy, and its documented CVaR is energy-based. Do not compare NTCSA LOLE with PRAS LOLE without converting their definitions. [NTCSA MTSAO metric definitions, Chapter 2](https://www.ntcsa.co.za/system-status-reports/) [PRAS multi-metric documentation](https://natlabrockies.github.io/PRAS/stable/examples/pras_adequacy_metrics/).

The event-spell implementation groups consecutive entries in the supplied interval vector. It assumes each scenario is a complete, regular-cadence chronology with no missing or duplicated intervals; the current V0.3 record does not carry an absolute timestamp or explicit interval sequence index, so it cannot independently detect an omitted period within one scenario. The scenario caller must establish the timebase, and the PRAS interoperability layer must validate it against the pinned model time axis before a benchmark comparison.

A single aggregate must not replace the metric vector. Event frequency, interval frequency, elapsed duration, day dispersion, expected energy and tail severity answer different planning questions. The primary metric used by the load search must be identified in every report.

CVaR V0.3 is a weighted finite-scenario tail statistic, not a Monte Carlo confidence interval. It does not establish the statistical uncertainty of the estimated tail. A future sequential sampler must report sample count, seed, standard error/confidence interval, tail effective sample size, convergence diagnostics and sensitivity to extreme-weather representation.

## 4B. External industry baseline: NLR PRAS v0.9.0

The National Laboratory of the Rockies' Probabilistic Resource Adequacy Suite (PRAS) released v0.9.0 in October 2026. Its current documentation explicitly adds LOLD and unserved-energy CVaR and describes high-performance sequential Monte Carlo for multi-region composite reliability, including energy-limited storage. PRAS should be an independent benchmark engine, not a subsystem Symtropy attempts to replace. [PRAS v0.9 documentation](https://natlabrockies.github.io/PRAS/stable/) [Multi-metric tutorial](https://natlabrockies.github.io/PRAS/stable/examples/pras_adequacy_metrics/) [NLR PRAS overview](https://www.nlr.gov/analysis/pras).

### Metric crosswalk

| Symtropy V0.3 metric | PRAS v0.9.0 | NTCSA MTSAO | Comparison conditions |
|---|---|---|---|
| `LossOfLoadEvents` | Not PRAS LOLE | NTCSA LOLE | Same event start/end rule; one event may span midnight; equal system adequacy requirement |
| `LossOfLoadPeriods` | PRAS LOLE | Not NTCSA LOLE | Identical timestep resolution and zero-shortfall threshold |
| `LossOfLoadDays` | PRAS LOLD | Not the NTCSA metric listed in the report | Identical calendar boundaries/timezone and study horizon |
| `LossOfLoadHours` | Companion duration metric | NTCSA LOLH | Identical hourly/sub-hourly time axis and shortfall convention |
| `ExpectedUnservedEnergyMwh` | PRAS EUE | NTCSA UE | Same region scope, units, timestep durations, sample weights and horizon |
| `UpperTailUnservedEnergyCvar { alpha }` | PRAS `CVAR(:energy, ShortfallSamples(), alpha)` | Not NTCSA power CVaR | Identical sample-level EUE distribution, weights, tail convention and alpha |
| `UpperTailShortfallPowerCvarKw { alpha }` | Not PRAS `CVAR(:energy,...)` | NTCSA shortfall-magnitude CVaR (MW) | Convert kW to MW; reconcile the tail population, sampling unit, quantile and percentile convention before claiming equivalence |

The NTCSA 2027–2036 report describes an hourly chronological dispatch-based assessment with 200 Monte Carlo samples per study year, co-optimized energy and reserves, regional nodes and transmission corridors. Its reference case reports 13.42 GWh UE, 377 LOLH and 30.63 MW CVaR in 2036. Those are conditional outputs of that report's networked model, not values the single-zone prototype can reproduce by itself. The report's Perfect Storm combines lower EAF, absent gas additions, higher demand and slower SSEG uptake; it is not a single-factor gas-delay experiment. [NTCSA MTSAO, 30 October 2026](https://www.ntcsa.co.za/system-status-reports/).

The report identifies the South African data-centre load base as approximately 585 MW existing/committed in 2025/26, with 2036 estimates spanning 4.6 GW (revised applications), 5.1 GW (central) and over 7.8 GW (high). Its continuous/high-load-factor characteristics make it particularly important to model the data-centre load as a near-baseload trajectory with explicit cooling and process variation, not as a generic interruptible commercial load. [NTCSA MTSAO, data-centre demand discussion](https://www.ntcsa.co.za/system-status-reports/).

### Interchange strategy

PRAS uses Julia and a versioned HDF5 `.pras` SystemModel representation. The HDF5 model declares the data version, simulation start, timestep count/length/units, power and energy units, regions, and generation/storage/demand-response and transmission groups. This format is suitable for a benchmark adapter without introducing Julia into the production Rust runtime. [PRAS HDF5 SystemModel specification](https://natlabrockies.github.io/PRAS/stable/SystemModel_HDF5_spec/) [PRAS source repository](https://github.com/NatLabRockies/PRAS).

The adapter should:
1. Keep Symtropy's frozen scenario/input manifest in a language-neutral, digest-pinned schema.
2. Export matched bulk-system cases to PRAS `.pras` HDF5 using an explicitly pinned PRAS data version and Julia environment.
3. Run PRAS `SequentialMonteCarlo` with recorded seeds and enough samples for pre-registered precision.
4. Request `ShortfallSamples()` and calculate LOLE, LOLD, EUE and `CVAR(:energy, ...)`; request flow results when testing deliverability.
5. Compare Symtropy and PRAS only on a common scenario/time/region basis, preserving a discrepancy report rather than tuning one model to silently match the other.

PRAS's demand-response model has borrowed-load capacity, payback capacity, energy capacity, allowable payback period and failure/repair probabilities. This is a useful comparator for aggregate deferrable load, but it does not encode per-job deadlines, model semantics or service-level obligations. The Symtropy schedule verifier must remain the authority for useful compute service; a PRAS demand-response representation is an external system-adequacy abstraction, not proof of workload feasibility. [PRAS HDF5 demand-response schema](https://natlabrockies.github.io/PRAS/stable/SystemModel_HDF5_spec/).

### What Symtropy adds around PRAS

PRAS supplies a mature bulk-system probabilistic adequacy engine. Symtropy's differentiation should be the evidence-qualified data-centre resource model: verified workload contracts, independent service-equivalence replay, stateful battery/thermal trajectories, signed PCC response and rebound, event-response qualification, and receipts that can be translated into a grid study. The final capacity value still must be validated using a network-aware adequacy model such as PRAS and, for South Africa, reconciled with NTCSA's system study.

## 5. Paradigm shifts for V0.2–V1.0

1. **From event reliability to chronological resource value.** Preserve the full sequence across dispatch, recovery, consecutive stress days and common-mode failures. A service that succeeds for two hours but shifts the shortage into the next peak is not a dependable capacity resource.
2. **From "flexible percentage" to service-equivalent compute.** Bind every curtailed or shifted workload to job class, deadline, latency, completion, quality, availability and customer contract. Compare only strategies that deliver the same useful compute service.
3. **From a site to a portfolio.** Co-optimize deferral, geographic migration, DVFS, cooling, thermal storage, batteries and on-site generation while preserving each asset's state, duration, ramp, fuel/energy and failure modes. Avoid double-counting the same kW across products.
4. **From copperplate capacity to locational adequacy.** Add buses, transmission limits, contingency deliverability, import constraints and distribution bottlenecks. A site can have headroom while its local substation cannot deliver it.
5. **From independent uncertainty to joint stress.** Model weather-correlated load, renewable output, generator outages, cooling derates, water restrictions, network failures, communications loss, and workload surges together. Preserve dependence and include consecutive extreme-weather episodes.
6. **From one score to a risk surface.** Use one explicitly selected primary accreditation metric, but report LOLE/LOLD, LOLH, EUE, event duration, conditional tail shortfall, cost, carbon, water and compute-service outcomes separately. Do not collapse incompatible risks into one weighted score.
7. **From static annual claims to adaptive evidence.** Version every scenario, baseline, dispatch policy and response trace. Reconcile forecast-versus-actual, maintain exact-head data receipts, detect drift, and withdraw/reduce a claim when conditions move outside the qualified stratum.

## 6. Benchmark programme and stretch targets

These are proposed engineering targets, not claimed current performance or externally mandated standards.

| Tier | Benchmark | Pass criterion |
|---|---|---|
| Arithmetic | Synthetic one-zone, weighted scenario known truth | 100% of frozen fixtures match independent calculations within declared tolerances |
| Negative controls | Missing chronology, duplicate IDs, invalid weights, impossible reduction, unmodeled rebound | Fail closed; no optimistic output |
| Metric integrity | LOLD vs LOLH vs EUE with same trajectories | Each metric differs only as mathematically expected; no unit confusion |
| Chronological completeness | Complete hourly traces for multiple weather years | 8,760-hour non-leap year / 8,784-hour leap year checked explicitly; no silent annualization |
| Stress coverage | Heat dome, cold snap, drought, low wind/solar, generator outage, grid outage, high compute demand | Report conditional adequacy and stress-event contribution, not only averages |
| Correlation | Common-mode failures and correlated response failures | Preserve joint paths; demonstrate materially different results from an independence assumption |
| Stateful resources | Battery/thermal/workload queue/compute deadline/rebound | No energy borrowing, deadline violations, or recovery omission |
| Network | Nodal transmission and distribution constraints | Reconcile against an established power-flow / adequacy reference model on identical cases |
| External validity | Independent resource-adequacy engine and operator/utility study | Match reference risk metrics and ELCC within a predeclared tolerance/confidence band |
| Service | Same useful compute delivered | No claimed capacity gain from silently dropping workload, delaying non-deferrable work, or degrading contractual QoS |
| Sustainability | Hourly marginal emissions, water, and cost | Publish separate results; a cost reduction never implies a carbon reduction |
| Performance | Reproducible scenario scaling | Publish wall time, memory, cores, scenario count and exact input digests; compare with optimized baseline rather than an invented universal speed target |

### Suggested stretch goals for public benchmark submissions

- 100% pass on synthetic arithmetic and adversarial input fixtures.
- Zero optimistic bias in the controlled negative tests.
- For validated external reference cases, absolute ELCC error below 2% of system peak load **or** within the reference model's stated 95% uncertainty interval, whichever is wider; publish both raw and normalized errors.
- For stochastic production-scale runs, increase Monte Carlo samples until the primary risk metric's relative standard error is below 5% (or use a pre-registered precision stopping rule); report confidence intervals and tail convergence.
- Report sensitivity to weather-year selection, outage dependence, probability weights, workload SLAs, rebound windows, asset degradation and network constraints.
- Do not claim superiority to industry tools until the same input data, scenario sampling, reliability definitions, network constraints and uncertainty budgets have been used in a blinded paired comparison.

## 7. Release gates

- **V0.1 prototype:** synthetic single-zone trajectories; known-truth fixtures; explicit limitations; no production credit.
- **V0.2 chronological operations:** storage state-of-charge, thermal state, workload queues/deadlines, event-response probability and fatigue/recovery all endogenous to the trajectory.
- **V0.3 locational grid model:** nodal power balance, transmission constraints, contingencies, import limits and deliverability.
- **V0.4 probabilistic calibration:** sequential Monte Carlo or equivalent scenario engine, extreme weather sampling, correlated outages, confidence intervals, convergence diagnostics and tail-risk estimates.
- **V0.5 independent validation:** compare against an established system-adequacy model and at least one operator/utility reference study with frozen inputs.
- **V1.0 evidence-qualified planning module:** model governance, independent verification, reproducible receipts, audit trail, stress testing and jurisdiction/product-specific qualification. Regulatory/market accreditation remains with the relevant authority.

## 8. Immediate follow-on work

- Replace caller-supplied flexibility trajectories with a stateful scheduler that consumes explicit job classes, service deadlines, queue state, cooling constraints, storage state and response reliability.
- Add clustered-event or block-bootstrap reliability estimates; the prior V0.1 Hoeffding bound assumes independent Bernoulli events and does not prove independence.
- Generate deterministic benchmark scenarios with heatwave, low-renewable, outages and rebound; add adversarial cases with coincident cooling derate and workload spikes.
- Add a multi-zone linear network model and compare nodal deliverability against copperplate results.
- Produce a complete-year benchmark set before annual LOLE/LOLP/EUE labels are permitted.
- Keep current CI status separate from authored fixture/test counts; only exact-head passing results may be represented as passed.

## References

- EPRI, [Resource Accreditation Methods](https://msites.epri.com/resource-adequacy/resource-acredidation/resource-accreditation-methods).
- Kahrl, Fredrich and Lin (2026), [Resource Accreditation in U.S. Resource Adequacy Mechanisms](https://eta.lbl.gov/publications/resource-accreditation-us-resource-adequacy).
- ESIG (2026), [Resource Adequacy with Large Loads](https://www.esig.energy/reports-briefs/large-loads-resource-adequacy/).
- Mantegna et al. (2026), [Electric Grid Reliability in an Era of Unprecedented Uncertainty](https://doi.org/10.1109/TEMPR.2025.3640116).
- LBNL (2023), [A Guide for Improved Resource Adequacy Assessments](https://energyanalysis.lbl.gov/publications/guide-improved-resource-adequacy).
- IEA 4E (2026), [Data Centres and Flexibility](https://www.iea-4e.org/edna/publications/data-centres-and-flexibility/).
- Caprara et al. (2026), [Data center workload flexibility for power system demand response](https://doi.org/10.1016/j.ijepes.2026.111940).
- Senga, Wang and Knittel (2026), [Flexible data centers reduce power system costs but can increase emissions](https://cs3.mit.edu/publication/118896).
