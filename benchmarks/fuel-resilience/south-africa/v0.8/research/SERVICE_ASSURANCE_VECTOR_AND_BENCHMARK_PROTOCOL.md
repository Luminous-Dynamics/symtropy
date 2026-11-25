# SAFRB v0.1 — service-assurance vector and matched benchmark protocol

**Research cut:** 24 November 2026  
**Status:** research and benchmark contract; no field qualification or SOTA superiority claimed  
**Composes with:** SAFRB v0.8 (#7294), temporal assurance (#7337), Power Fabric service trajectory (#6450), restoration bridge (#6235), run receipt (#6366), independent artifact/claim verification (#6405/#6427), branch/beneficiary risk qualification (#6317), HIL ladder (#6451), native three-phase readiness (#6544)

## 1. Executive thesis: certify usable service, not an energy number

SAFRB v0.8 makes a meaningful advance: it evaluates a time series of lower/upper delivered-energy bounds against critical-service floors and only confirms restoration after a continuous recovery interval. The remaining limitation is deliberate: Wh at one boundary does not establish that the service was usable at the required instantaneous power, voltage, frequency, waveform quality or downstream process output.

The next research object should be a **non-compensatory service-assurance vector**. It must not collapse all properties into one weighted scalar. A high energy total cannot compensate for voltage outside limits; a healthy feeder cannot compensate for no fuel at the generator; a completed repair cannot compensate for missing commissioning; and a valid-looking measurement digest cannot compensate for an unverified source.

For service \(s\), boundary \(b\), and interval \(t\), define the evidence-qualified state:

\[
\mathcal{S}_{s,b,t} =
\left(
E_{\mathrm{Wh}},
P_{\mathrm{kW}},
V_{A,B,C,\mathrm{pu}},
f_{\mathrm{Hz}},
Q_{\mathrm{kvar}},
\mathrm{THD}_{\%},
Y_{\mathrm{process}},
\mathcal{E}
\right)
\]

The actual fields are profile-dependent. A single-phase boundary does not invent three-phase values; a non-electrical service does not pretend to have voltage. Each required field carries its own unit, boundary, time interval, value or interval, evidence state, provenance, calibration/model identity and qualification predicate.

**Decision rule:** a service interval is **GUARANTEED** only when every required hard predicate is independently admissible and passes; **FAILED** only when a required predicate is independently established to fail; otherwise **INDETERMINATE**. Missing or unqualified measurements are not passes and are not silently converted to zero.

## 2. Paradigm shifts worth pursuing

### 2.1 From commodity availability to usable-service deliverability

Barrels in storage, fuel at a terminal, generator nameplate MW, installed battery kWh and an energized feeder are intermediate state variables. None alone establishes service at the beneficiary boundary.

Model the chain as typed, time-indexed transformations:

1. **Gross stock:** product, grade, tank identity, measured volume/mass, temperature/density correction and measurement uncertainty.
2. **Usable stock:** gross stock minus separately evidenced quarantine, contamination, unusable heel, operating reserve and other physical restrictions. Do not subtract a category unless its quantity and rule are evidenced.
3. **Accessible stock:** usable stock constrained by ownership, legal authority, contractual encumbrances, release permissions and custody.
4. **Deliverable stock:** accessible stock constrained by sustainable drawdown, pumping, loading, berth, pipeline, road/rail and transfer time.
5. **Convertible energy:** crude/refined-product compatibility, assay, process configuration, yield, efficiency, operating envelope, feedstock availability and commissioning state.
6. **Dispatchable service capacity:** generator or plant fuel compatibility, derating, minimum load, ramp, reserve, storage state of charge, maintenance state and firm operating duration.
7. **Network-deliverable service:** network topology, congestion, losses, voltage/frequency, reactive-power, phase and protection constraints.
8. **Beneficiary service:** independently measured or qualified modelled output at the actual service boundary, with its own demand denominator and critical floor.

Every stage produces a typed receipt and a bounded flow, not an unqualified scalar. For a serial path, throughput is limited by the narrowest effective bottleneck; for alternate routes or sources, use the actual constrained network-flow model rather than a simplistic minimum. The report should identify the binding cut set and the evidence that supports each capacity.

This creates a **deliverability gap**: the difference between nominal resource inventory and service that can be delivered, at a named boundary, by a stated deadline. Quantify it by time and bottleneck; do not turn it into one days-of-cover number.

### 2.2 From scalar service scores to a non-compensatory vector

Recommended profile fields for electricity:

| Dimension | Required interpretation | Owner / qualification boundary |
|---|---|---|
| Delivered energy | Lower/upper Wh or kWh over exact interval | SAFRB bound evaluator; upstream meter/accounting verifier remains separate |
| Instantaneous / sustained power | kW lower/upper, peak, ramp and duration | Electrical/plant model or calibrated measurement |
| Voltage | Per-phase pu and/or declared line-to-line values; min/max over interval | Balanced or native three-phase specialist solver / calibrated recorder |
| Frequency | Hz bounds, excursions and duration | Grid-forming / system-dynamics validator or qualified recorder |
| Reactive power | kvar, power factor and inverter capability where relevant | Electrical solver and equipment capability evidence |
| Waveform quality | THD and applicable disturbance metrics with instrument bandwidth/sample rate | Power-quality instrument and accepted limits |
| Process output | Native unit such as m³/h, pressure, temperature, telecom throughput or clinical service | Domain-specific downstream validator |
| Evidence | Source bytes, digest, signer/provider, calibration/model scope, time, freshness, independence | Existing evidence and claim-verification owners |

Do not require every field for every service. Define versioned service profiles that state which predicates are mandatory, optional or not applicable. **NOT_ASSESSED** is not equivalent to **PASS**; **NOT_APPLICABLE** requires a declared profile reason.

A conservative logical composition is:

\[
\mathrm{usable}_{s,b,t} =
\bigwedge_{j \in R(s,b)} \mathrm{predicate}_{j,t}
\]

where \(R(s,b)\) is the set of required predicates for that service profile. If any required predicate is false, usable service fails. If none fail but at least one required predicate is unknown or unverified, the outcome is indeterminate. Weighted scores may describe economic performance after hard predicates pass; they may not override the hard result.

### 2.3 From snapshot validation to counterfactual, time-dependent assurance

Keep four timelines separate: planned forecast, observed work, modelled service, and measured service. Then evaluate the same frozen disruption under counterfactual alternatives:

- no intervention;
- existing operator or rule-based restoration;
- optimizer plan without SAFRB;
- identical optimizer run with SAFRB bound/floor qualification;
- same run with independent artifact and claim verification;
- additional information acquisition before a decision, where its latency and value are included.

The aim is not to produce the prettiest restoration plan. It is to determine which claims and decisions remain defensible after uncertainty, missingness, common-cause shocks and alternative explanations are considered.

### 2.4 From more data to decision-value measurement

An observation should be requested because it can change a service or resource decision, not merely because it is available. For each proposed observation, estimate:

- which uncertainty or competing hypothesis it can resolve;
- the probability that its outcome changes the admissible action set;
- the cost, diagnostic latency and operational disruption;
- whether it shares a common failure mode or source with existing observations;
- the cost of waiting versus acting conservatively now.

The output is a request/response pair with separate timestamps and provenance. A request is not evidence; a returned value is not automatically admissible evidence. An unresolved outcome keeps the system in an indeterminate or mutation-blocked state as appropriate.

## 3. Assurance contract: preserve the bounds and the reason for every verdict

The #7338 adapter should remain deliberately thin. It must reuse #6450's accepted case, run-receipt, service trajectory, source inventory, load reconciliation and evidence-stream semantics. It must not create a second weighted scorer, service schema, endpoint map, signature verifier or beneficiary risk engine.

### 3.1 Projection rules

For each SAFRB window, bind exactly one frozen case load and one case step:

- case digest, run ID, service ID, boundary ID, load ID, step, horizon and timestamps must match;
- target Wh must reconcile with the frozen case demand for that exact interval;
- convert Wh to kWh only by checked division by 1,000, and seconds to milliseconds only by checked multiplication;
- preserve lower bound, upper bound, critical floor, target, original evidence references and the evaluator assessment in a separate SAFRB certificate;
- only a strictly positive and admitted lower bound may be projected as **known** delivered kWh into the generic scorer;
- a zero lower bound with positive upper bound is **unknown**, not **unavailable**, and blocks a completed score;
- zero/zero does not establish outage without independently qualified explicit zero-delivery evidence;
- upper bound, midpoint and interval width must never substitute for the delivered lower bound;
- the critical floor verdict remains a separate hard predicate;
- scenario and reported-only evidence cannot be promoted to measured/modelled field service.

The current auditor treats uncertaintyKwh as descriptive metadata. It does not cause the weighted scorer to integrate an interval. The adapter and reporting must say this explicitly.

### 3.2 Evidence and identity gates

A SAFRB Measured enum and a 64-hex digest-shaped string do not prove bytes, signature, calibration, instrument installation or issuer authority. The accepted Power Fabric measured-service path and independent #6405/#6427 verifier must qualify exact bytes, identity, role, calibration interval and parent relationships.

The adapter must reject:
- wrong case/run/load/boundary identities;
- duplicate or many-to-one implicit load mappings;
- unmatched or overlapping intervals;
- 1 Wh demand mismatch;
- seconds/milliseconds or Wh/kWh scale errors;
- source digest not present in the accepted source inventory;
- missing or extra parent service artifacts relative to #6352;
- a reported/scenario stream represented as measured/modelled service;
- a weighted score that passes while the SAFRB critical floor fails or is indeterminate.

No new result may be described as signed, independently verified, field-measured or physically qualified unless the relevant verifier and accountable domain reviewer actually performed that work.

## 4. Benchmark design: compare like with like, then measure what the assurance layer adds

SAFRB v0.8 is an evaluator. It is not an optimizer and should not be put on the same direct leaderboard as LARA, FLARE, HAPPO, MILP or PyPSA. There should be distinct benchmark tracks with shared frozen cases only where their physical assumptions and output semantics genuinely match.

### 4.1 Matched assurance ablation

For each planner, run the same frozen inputs, action budget, solver time limit, observation cut-off, random seed and physics tier:

| Arm | Configuration | Question answered |
|---|---|---|
| A | Planner + its existing validator | What does the current stack accept? |
| B | Same exact planner outputs + SAFRB interval/floor/temporal qualification | Which service claims change when interval and sustained-restoration semantics are enforced? |
| C | Same run + SAFRB + independent artifact/claim verifier | Which claims survive byte, signature, role, calibration and parent-chain verification? |

The planner output must be byte-identical across A/B/C. Otherwise this is not an assurance ablation. The denominator must include failed attempts, incomplete runs, abstentions, unknown observations and rejected proposals. Do not drop inconvenient cases from the score.

### 4.2 Separate physical benchmark lanes

| Lane | Benchmark / comparator | Required discipline |
|---|---|---|
| Language-conditioned planning | LARA on its IEEE-13 and IEEE-33 cases | Reproduce its action catalogue, case assumptions, time budget and MILP comparator; do not compare against SAFRB as a planner |
| Degraded observation and guard quality | FLARE's 800 AC-feasible cases and its reported 78,750 episodes, if source data and rights permit | Preserve its observation profiles, physics, seeds, rejected-action set and tail metrics |
| Multi-agent RL | HAPPO/OpenDSS on IEEE-123 and the reported 8,500-node setting | Match three-phase/DER assumptions, generation caps, training seeds and compute budget |
| CoT/LoRA and explainable multi-agent | Their modified IEEE-123 and IEEE-33/69/118 cases respectively | Keep each paper's topology and scenario conversion as a distinct lane |
| Optimization | HiGHS/Gurobi MILP and appropriate PyPSA deterministic/stochastic/CVaR or robust formulation | Same feasible set, objective, horizon and wall-clock/solver budget |
| Physical qualification | Balanced AC, native unbalanced three-phase, protection/transient, HIL and commissioning tiers | No promotion from topology-only or balanced results into three-phase/protection/HIL claims |
| Fuel-to-service | Frozen South African synthetic supply-chain cases, then operator-approved historical/field data | Track crude/refined grade, custody, logistics, conversion, generation, network and beneficiary boundary separately |

FLARE's 2026 report is especially instructive: it reports 78,750 episodes across topology, distributed-generation and loading shifts; five models had zero median regret but P95 regret from 0.814 to 1.000, while shadow replay found that 96.4% of 15,706 guard-rejected actions would have caused a physical violation. Average results therefore do not tell the risk story. Treat these as paper-reported benchmark results, not external ground truth for SAFRB.

LARA reports near-MILP performance on IEEE-13 and about 13% better performance than a time-capped MILP on IEEE-33 at less than 1% of its wall-clock runtime. That is a planning result on its own benchmark; it does not test bound-preserving service certification. The appropriate comparison is to reproduce its planner track and add the same SAFRB/verification overlay to the same output runs.

### 4.3 Required metric families

**Assurance correctness**
- false continuity claims;
- premature sustained-restoration confirmations;
- critical-floor false passes;
- zero-lower/positive-upper promotion errors;
- upper-bound or midpoint score substitutions;
- false measured/modelled evidence promotions;
- parent-artifact and load/time mapping errors;
- right-censoring errors;
- false rejects on known-admissible positive controls;
- unknown/abstention rate and unresolved exposure duration.

**Service outcomes**
- bounded critical-floor and full-target shortfall Wh;
- EENS/EUE in the correct native boundary and denominator;
- guaranteed and indeterminate service-hours;
- time-to-first-candidate and time-to-sustained restoration;
- p50/p95/p99 and CVaR95 by service class, beneficiary cohort and geography;
- power-quality and process-output predicates as separate dimensions;
- SAIDI/SAIFI/CAIDI only where customer-level event and denominator data actually exist.

**Planner and operational outcomes**
- mean, p95 and CVaR95 regret against a clearly specified oracle;
- cost, emissions, fuel use, curtailed load and schedule churn;
- topology/constraint feasibility at the declared physics tier;
- observation-to-decision latency and time-to-reconciled evidence;
- planner tokens/API cost, solver runtime, memory and energy use.

**Assurance overhead**
- adapter/verifier p50, p95 and p99 latency;
- artifact bytes and verification cost;
- false-reject rate and reason distribution;
- percentage of time the system must abstain pending reconciliation;
- full failed-attempt denominator and audit completeness.

Do not collapse these into one opaque score. If a policy ranking is required, publish the Pareto frontier and the hard constraints first.

## 5. Proposed gates and research targets

These are prospective internal targets, not achieved results, industry standards or certification thresholds.

| Dimension | Gate / target | Scope |
|---|---|---|
| Known-truth safety | Zero false continuity, restoration and critical-floor passes | Exhaustive small-state oracle and frozen adversarial corpus |
| Semantic non-promotion | Zero upper/midpoint promotion; zero unknown-to-zero/unavailable promotion | Adapter tests and negative controls |
| Identity/accounting | 100% case/load/step/time/unit/source/parent reconciliation | Every completed adapter run |
| Temporal logic | Zero premature restoration; 100% correct right-censoring on bounded oracle cases | Synthetic known-truth tests |
| Evidence | 100% source and parent artifact accounting; zero digest/identity mismatch accepted | Byte-level independent verification |
| Interval calibration | For nominal 90% intervals, target 85–95% empirical coverage on a held-out, operator-approved set, plus weighted interval score/pinball loss | Only after real measurement data and independent calibration review |
| False rejects | Target at most 1% on a separately frozen, known-admissible positive-control set; publish confidence intervals and reasons | Research target, not a reason to relax hard gates |
| Runtime | Report p50/p95/p99; target p95 assurance overhead no more than 10% of the declared decision-loop budget | Per hardware and deployment profile |
| Planner value | Target at least 20% lower CVaR95 critical-service regret than the strongest matched baseline, without worsening any mandatory floor | Composed planner programme only |
| Restoration | Target at least 10% lower p95 time-to-sustained-restoration without worse worst-cohort service | Composed planner programme only |
| Generalization | At least 90% retention of pre-registered service-quality performance on held-out topologies and compound shocks | No train/test leakage by topology, generator, event family or weather regime |

A zero false-positive target is not enough by itself: a system that rejects everything can appear safe. Every false-reject and abstention must be counted. Conversely, do not relax a hard floor merely to improve coverage or runtime.

## 6. A South African fuel-to-service benchmark that can become externally useful

### 6.1 Start with synthetic cases, then progress to governed evidence

The first reproducible corpus should be synthetic and explicitly labelled as such. Include:
- crude cargo delay and supplier/country concentration;
- refined-product grade shortage or specification failure;
- tank measurement drift, temperature correction and contamination/quarantine;
- unusable heel, ullage and drawdown-rate limits;
- berth, discharge-arm, pipeline, pump, road/rail and terminal disruptions;
- refinery/process compatibility, yield uncertainty and ramp/commissioning constraints;
- grid curtailment, islanding, generator derating, fuel-quality mismatch and battery depletion;
- communication loss, stale measurements, timestamp drift, replayed events and credential expiry;
- common-cause combinations such as port + grid + telecom, or supplier + route + weather;
- critical-service demand profiles for health, water, telecom, transport and industrial safety boundaries.

Use explicit common-cause groups and event correlations. Do not multiply nominally independent probabilities when events share a port, supplier, grid, weather or control-system dependency.

Next, seek permissioned historical data with a frozen data dictionary, data-sharing basis, evidence-provider identity, privacy boundaries and retention rules. Minimum useful records:

| Chain segment | Candidate record | Required metadata |
|---|---|---|
| Tank / stock | Tank gauging, dip/level, temperature, density, product/grade and quantity | Tank ID, sample time, instrument ID, calibration, uncertainty, quality status, custody, source digest |
| Product quality | Laboratory assay, certificate of analysis, contamination/quarantine/release | Batch/lot, method, lab/provider, spec revision, sample chain, release authority |
| Legal/access | Ownership, liens/pledges, allocation, release and transfer authority | Scope, issuer, effective time, expiry, revocation, beneficiary and permitted quantity/rate |
| Logistics | Cargo nominations, vessel arrival, berth/arm, transfer, pipeline/pump/road/rail | Event and asset IDs, time, capacity, status, source identity, actual vs planned |
| Conversion | Refinery/process runs, throughput, yield, product slate, outages and restart | Feedstock assay, unit configuration, process model, calibration, losses and ramp |
| Generation | Fuel issue/consumption, generator output, derating, maintenance and reserve | Generator ID, fuel spec, efficiency/load curve, kW/kWh, start/ramp, calibration |
| Electricity network | Load, topology, switching, protection and power-quality observations | Frozen topology, phase, P/Q, voltage/current/frequency, sample rate, protection state |
| Beneficiary | Metered or domain-qualified output | Exact service boundary, demand denominator, critical floor, freshness, missingness and reviewer |

A stock ledger is not automatically accessible inventory; a transfer ticket is not usable fuel at a generator; generator nameplate MW is not net deliverable kW; and delivered Wh is not acceptable service quality.

### 6.2 Field evidence admission ladder

1. **Public claim:** useful for research questions only; never treated as measured inventory.
2. **Documentary record:** source bytes and issuer identified, but no independent physical measurement qualification.
3. **Reconciled accounting record:** physical observation, book quantity, product identity and custody reconciled.
4. **Instrument-qualified measurement:** raw bytes, calibration, uncertainty, time and measurement boundary verified.
5. **Operationally accessible quantity:** withdrawal rights, quality release, transfer path and sustainable rate evidenced.
6. **Service-qualified observation:** net delivered service measured/modelled at the beneficiary boundary and independently verified.
7. **Outcome validation:** operator-reviewed history or witnessed HIL/commissioning evidence establishes performance for a declared scope.

The level is claim-specific. A document may support ownership but not quality; an assay may support specification but not title; a meter may support delivered energy but not upstream fuel inventory.

## 7. South African strategic-stock source correction — same-day Hansard context

The March 2026 source register should record two separate statements in the same National Assembly sitting:

- In Question 260, the question expressly asked for crude-oil quantities and locations; the Minister's reply on printed p. 6 said the SFF held eight million barrels of oil in strategic stock and that it was not being used, to be used in a real crisis.
- Later, on printed p. 56, the Minister explicitly described the reserve as eight million barrels of crude oil and said part had been used in a previous crisis.

This second passage clarifies the commodity as crude oil and adds historical context to the use of reserve stock. It does not supply the amount or date of the earlier use, a new measurement date, site/tank locations, a tank-level ledger, usable quality, release rights or sustainable withdrawal rate. It should therefore refine the classification of the attributed ministerial claim, not become a measured inventory figure.

The 30 September 2026 6.9-million-barrel / about-30-days statement remains secondary-source-attested pending the primary parliamentary record. Newsday's report attributes the cover denominator to 200,000 barrels/day of crude-processing capacity; 6.9 million / 200,000 equals 34.5 nominal processing days at those rounded inputs. This is a request for calculation reconciliation, not proof of error. It is not 30 days of diesel demand, refined-product demand or essential-service continuity.

Cabinet's 1 July 2026 statement proposed SANPC strategic reserves equal to 60 days of net imports for both crude and refined products, with a phased move to 90 days over the long term. This is a policy proposal and a different denominator, not proof of current stock on hand.

## 8. Immediate implementation sequence

1. **Keep #7338 as the adapter authority.** Implement only against the accepted, exact versions of #6346, #6352 and #6450, plus the independent verifier contract. Do not path-depend on sibling draft branches that are absent from the SAFRB branch.
2. **Produce a bound-certificate artifact.** Preserve each original window's lower/upper delivery, floor, target, disposition, time and evidence reference. The artifact must be deterministic, content-addressed and included in the existing source/parent artifact chain; signature and role verification remain with #6405/#6427.
3. **Build the thin projection.** Map positive admitted lower bounds to known samples; map 0 < upper with zero lower to unknown and block complete scoring; only emit unavailable with independently verified zero-delivery evidence.
4. **Add independent oracle and adversarial tests.** Cover every #7338 case, including exact demand match, time/unit scale, interval partition/time shift, evidence misclassification, parent set and hard-floor masking.
5. **Run matched ablation.** Reuse exact planner outputs from the same frozen runs for the baseline, SAFRB overlay and independent-verifier arms. Preserve all incomplete/unknown cases.
6. **Start a governed data partnership.** Begin with one bounded site/service chain and one service boundary; publish a data dictionary and evidence requirements before asking for operational data.
7. **Progress the physical qualification ladder.** Keep topology, balanced AC, native three-phase, protection/transient, HIL, commissioning and shadow-operation evidence as separately signed tiers.

## 9. Research references and source boundaries

- FLARE, *Physics-Grounded Evaluation of Language-Model Agents for Feeder Restoration with Degraded Observations*, arXiv:2610.23251 (2026): https://arxiv.org/abs/2610.23251
- LARA, *LLM-based Agile Power Distribution Network Restoration from Disastrous Events*, Findings of EACL 2026: https://aclanthology.org/2026.findings-eacl.321/
- IEEE 1366-2022, *IEEE Guide for Electric Power Distribution Reliability Indices*: https://standards.ieee.org/ieee/1366/ (use the licensed normative text for formal index computation)
- IEC TS 62898-3-5:2026, microgrid HIL, commissioning and periodic testing: https://webstore.iec.ch/en/publication/84175
- South African National Assembly Unrevised Hansard, 25 March 2026: https://www.parliament.gov.za/storage/app/media/Docs/hansard/01mfgh4vqebdhkps6o35hybjq2binzrfd6.pdf
- Newsday, 5 October 2026, reporting 6.9 million barrels and 200,000 barrels/day crude-processing denominator: https://newsday.co.za/business/28619/south-africas-strategic-oil-reserves-fell-by-96-from-158-5-million-barrels-to-6-9-million-barrels/
- South African Government, Cabinet statement, 1 July 2026, draft strategic petroleum stocks policy: https://www.gov.za/news/cabinet-statements/full-statement-cabinet-meeting-1-july-2026-03-jul-2026

These papers and standards define comparison contexts, not a single common leaderboard. Paper-reported metrics must be reproduced under their published assumptions before comparison. A software or synthetic-fixture pass does not establish AC feasibility, protection coordination, HIL performance, commissioning, operator authority or field reliability.


## 10. 2026 comparator update — reproduce the strongest relevant ideas

The current comparator set should retain these separate tracks and their own claim boundaries:

- **HAPPO / OpenDSS (IEEE TPEC 2026):** heterogeneous decentralized agents with a centralized critic and a physics-informed OpenDSS environment; the paper reports IEEE-123-bus and 8,500-node tests and more than 95% of available load restored under a 2,400 kW generation cap. Reproduce the exact multi-microgrid layout, DER constraints, training seeds, cap and runtime before comparing.
- **Two-stage CoT/LoRA (Applied Energy, 15 November 2026):** reports 100% physical feasibility under its modified IEEE-123 test protocol with unseen-topology/sequence perturbations and around 44% faster average inference using soft prompts. Treat the 100% as a paper-scoped result, not a universal guarantee or an independent safety certificate; replay the exact test split and use an independent physical verifier.
- **Explainable agentic LLM restoration (arXiv:2611.22016, 17 November 2026):** uses specialized LLM agents constrained to feasible actions and deterministic AC power-flow checks; evaluates IEEE-33, IEEE-69 and IEEE-118 cases. It is a useful recent preprint comparator for multi-agent decomposition, explanation quality and receding-horizon plan evaluation, not evidence of field qualification.
- **Distributionally robust whole-process restoration (Applied Sciences, 26 October 2026):** jointly models black-start, network reconfiguration and load restoration with PV-BESS uncertainty and flexible loads. On its IEEE-118 study, the authors report every proposed schedule passing AC-OPF while a DC-only schedule had no AC operating point at 8 of 22 steps. It is a strong reason to make physical-fidelity tiers and per-step feasibility explicit.
- **Bottom-up renewable blackstart (Nature Communications review, 17 August 2026):** emphasizes deliverable inverter headroom, current/apparent-power limits, low-inertia frequency dynamics, cold-load pickup, protection admissibility and imperfect situational awareness. Installed DER capacity must not be substituted for dynamically deliverable headroom.

### What to take from this research

1. **Joint the restoration process, not just the dispatch snapshot.** Where the domain model supports it, couple black-start, network energization, load pickup, fuel availability and generator startup constraints over the same time horizon. SAFRB should qualify the resulting service trajectory; it should not solve these dynamics itself.
2. **Make AC feasibility a per-step gate.** A plan that is topologically connected or DC-feasible can still have no AC operating point. Native three-phase, voltage, reactive, thermal and frequency claims remain owned by the relevant specialist models.
3. **Measure DER deliverable headroom.** Record current/apparent-power limits, grid-forming mode, state of charge, energy duration, ramp, frequency response and cold-load pickup envelope; do not score nameplate MW as firm service.
4. **Benchmark explanation against evidence.** An LLM rationale should cite the exact constraints and evidence that made an action admissible or rejected. The explanation itself is not an independent witness.
5. **Use the assurance overlay as a common cross-planner instrument.** Keep each planner's published case track, then replay identical output artifacts through SAFRB and the independent verifier. Report changes in false-safe claims, critical-floor outcomes, uncertainty/abstention, p95/CVaR95 regret and verification overhead.

### Additional source register

- HAPPO / OpenDSS, *Heterogeneous Multi-Agent Proximal Policy Optimization for Power Distribution System Restoration*, IEEE TPEC 2026, DOI: https://doi.org/10.1109/TPEC67884.2026.11513130
- Liu et al., *Two-stage fine-tuning of large language models for distribution network service restoration via CoT reasoning*, Applied Energy 423 (15 November 2026), article 128417, DOI: https://doi.org/10.1016/j.apenergy.2026.128417
- Chowdhury and Li, *An Explainable Agentic AI Framework for Distribution System Load Restoration*, arXiv:2611.22016: https://arxiv.org/abs/2611.22016
- Lin and Xu, *Distributionally Robust Whole-Process Restoration of Power Systems with Photovoltaic–Battery Energy Storage Stations and Flexible Loads*, Applied Sciences 16(21), 10584 (26 October 2026), DOI: https://doi.org/10.3390/app162110584
- Shi, Bai and Wang, *Power distribution system blackstart restoration using renewable energy*, Nature Communications 17, 8498 (17 August 2026): https://www.nature.com/articles/s41467-026-76692-0

All reported numerical results above are claims from the named papers under their respective study assumptions. They are not SAFRB results, not a common leaderboard, and not field reliability evidence.


## 11. Industry boundary and optimizer-neutral assurance layer — 25 November 2026

The benchmark should distinguish what commercial products already do from the missing assurance problem. SAFRB is not a replacement for a microgrid EMS, a dispatch optimizer or a terminal inventory-management system.

### 11.1 Commercial capability comparison

| Industry capability | Existing product scope (public product description) | SAFRB integration boundary |
|---|---|---|
| Microgrid forecast and dispatch | Schneider Electric EcoStruxure Microgrid Advisor describes forecasting, monitoring, optimization, tariff/site constraints, previews and model-predictive control for generation, storage and EV charging | Consume frozen forecasts, candidate schedules and controller outputs; verify that every relied-upon constraint, resource and service claim is evidence-qualified |
| Local microgrid operation | Schneider Electric EcoStruxure Microgrid Operation describes real-time power management, grid/island operation, load sharing/shedding, black start and restoration | Do not replace control; independently qualify power/service intervals, transition preconditions, evidence freshness and restoration claims |
| Terminal inventory and movement | DTN TIMS describes source-system inventory normalization, movement/inventory validation and exception surfacing; DTN TABS describes customer-specific product availability visibility at terminals | Do not rebuild a terminal inventory ledger; qualify whether the asserted quantity is physically received, product/grade matched, assay released, in custody/title, contractually accessible, authorized, transferable in time and service-relevant |
| Grid and physical qualification | OpenDSS/AC solvers, protection studies, HIL and commissioning processes supply physical analyses at their declared tier | Treat their outputs as upstream evidence only after exact case/topology/configuration, solver/convergence, measurement and independent verification requirements are met |

Public product descriptions are capability statements, not comparative benchmark results. SAFRB must not claim to exceed Schneider or DTN on control, optimization or inventory-management functionality. The differentiator to test is the **vendor-neutral claimability chain from source quantity to beneficiary service**, preserving unknowns and non-compensatory hard limits.

Primary product references:
- Schneider EcoStruxure Microgrid Advisor: https://www.se.com/us/en/product-range/65896-ecostruxure-microgrid-advisor/
- Schneider EcoStruxure Microgrid Operation: https://www.se.com/us/en/product-range/65897-ecostruxure-microgrid-operation/
- DTN TIMS: https://www.dtn.com/refined-fuels/seller/tims/

### 11.2 Public-stock claimability is a benchmark dimension

The South African 2026 Estimates of National Expenditure, Vote 34, Table 34.18 (printed pp. 729–730), lists medium-term targets of 60 days of strategic crude and 14 days of refined-product supply. The preceding audited-performance cells for the two days-on-hand indicators are recorded as `-1`, with the footnote “No historical data available.” A separate CEF performance indicator lists 10.3 million crude barrels maintained as per ministerial directive.

The correct conclusion is narrowly scoped: the published table does not provide a historical achieved series for those two days-on-hand metrics. It neither proves stock is absent nor proves the target has been achieved. It also does not show product-grade, quality-release, custody, title, terminal location, drawdown rate, transfer latency or beneficiary service.

Required claimability labels:
- `POLICY_TARGET`
- `ATTRIBUTED_PUBLIC_CLAIM`
- `REPORTED_AGGREGATE`
- `AUDITED_ACCOUNTING_ACTUAL`
- `PHYSICALLY_MEASURED`
- `QUALITY_ADMITTED`
- `CUSTODY_TITLE_CONTRACT_ELIGIBLE`
- `WITHDRAWAL_AUTHORIZED`
- `ROUTE_DELIVERABLE_BY_DEADLINE`
- `GENERATOR_READY`
- `BENEFICIARY_SERVICE_PROVEN`

Every transition between labels requires a separately attributable evidence receipt. There is no implicit promotion from a policy target to actual stock, from crude barrels to refined diesel, or from nominal storage to days of critical-service continuity.

Source: National Treasury, 2026 Estimates of National Expenditure, Vote 34, Table 34.18, printed pp. 729–730: https://www.treasury.gov.za/documents/National%20Budget/2026/ene/FullENE.pdf

### 11.3 Extended benchmark corpus and metrics

Freeze a **claimability and event-transition suite** alongside the existing temporal/service oracle:
- ETA without physical receipt; partial, late, duplicated, replayed or conflicting receipt;
- wrong product/grade/terminal/batch identity;
- failed, pending, stale or disputed assay; quality quarantine and later release;
- stock physically present but custody, title, contract or withdrawal authority absent, expired or conflicting;
- encumbrance, operating heel, route capacity, transit delay, berth/pump/terminal outage;
- fuel delivered to generator but not issued; incompatible fuel grade; derating and power-cap curtailment;
- generator output exists but network voltage/frequency/reactive-power/power-quality evidence is missing or outside hard limits;
- beneficiary boundary/demand denominator changes, partial outage, restoration transient and right-censored observation;
- common-cause supplier/port/grid/telecom dependencies and correlated evidence providers;
- public policy target or ministerial statement incorrectly presented as measured accessible stock.

Report, without collapsing them:
- false-guarantee rate and 95% interval, by failure mechanism;
- false-rejection rate on separately frozen known-admissible positive controls;
- indeterminate/abstention rate and unresolved-exposure hours;
- interval coverage, interval width and calibration score on held-out permissioned measurements;
- p50/p95/p99 receipt→assay→custody/title/contract→authority→generator-ready latency;
- p50/p95/p99 evaluation and evidence-verification overhead;
- exact ledger closure, duplicate/replay rejection, time-order correctness and oracle disagreement;
- service EENS/EUE, critical-floor breach Wh, service-hours, sustained-restoration time, P95/CVaR95 regret, tail beneficiary outcomes and fuel/energy costs.

For zero observed failures in (N) independent Bernoulli trials, the rough one-sided 95% upper bound is (3/N) (rule of three). Thus 300 independent positive-control cases with zero false rejects only support an upper bound near 1%; 3,000 independent negative controls with zero false guarantees support an upper bound near 0.1%. These are not substitutes for exhaustive software-state exploration or stratified coverage. Correlated seeds/cases must be grouped by topology, common-cause event family and supplier/provider; they cannot be counted as independent trials merely because JSON rows differ.

### 11.4 Proposed frozen benchmark gates

These are research targets, not attained results, standards or field acceptance thresholds:

| Gate | Proposed target | Qualification caveat |
|---|---|---|
| Event/ledger integrity | 100% exact identity, chronology and mass-balance reconciliation; zero duplicate/replay acceptance; zero backdating | Exhaustive bounded-state checks plus synthetic adversarial corpus |
| Service safety | Zero false guarantees on all frozen hard-negative cases | Publish confidence bounds and failure-family coverage; no claim about all physical reality |
| Positive controls | At least 300 distinct known-admissible controls; target false rejection ≤1% with confidence interval | Positive controls must not reuse training templates or be tuned after the freeze |
| Generalization | ≥90% retention of pre-registered service-assurance performance on held-out topology, supplier, product/grade and compound-shock families | Grouped holdouts; no same-feeder/same-event leakage |
| Matched risk outcome | ≥20% reduction in CVaR95 critical-service regret vs strongest matched planner+validator baseline, with no mandatory floor regression | Same planner output, objective, physical model, solver budget and compute |
| Restoration outcome | ≥10% lower P95 time-to-sustained-restoration without worse worst-cohort service | Only on shared topologies and same outage/repair/control budget |
| Runtime | P95 assurance overhead ≤10% of declared control/decision-loop budget | Report hardware, artifact size, verifier tier and all failed/abstained runs |
| Evidence delay | Publish p50/p95/p99 and deadline miss rate for each availability state transition | Planned ETA and observed receipt remain distinct time series |
| Interval quality | For nominal 90% intervals, 85–95% empirical coverage on a held-out, operator-approved dataset, with weighted interval score | Only after independent metrology/data review; do not calibrate on synthetic oracle alone |

### 11.5 Optimizer- and vendor-neutral three-arm test

For each matched physical case and fixed planner output, run:

A. Planner/controller with its existing validation and acceptance logic.  
B. The exact same output passed through SAFRB's typed resource/service, uncertainty, temporal and evidence gates.  
C. The same run with SAFRB plus independent artifact/claim verification and parent/source-chain validation.

Add a fourth integration-only observation (not a new decision arm): the cost and latency of binding public commercial EMS/terminal-system exports into the same assurance interface. Proprietary black-box vendor features are not to be inferred; use only documented exports and actual permitted test access.

Hold constant: topology/revision, demand, assets, generator/fuel constraints, outage/cold-load-pickup sequence, observations, random seed, planner candidate action bytes, solver/runtime budget, and observation cutoff. Score the controller and assurance overlay separately. The benchmark winner is not whichever produces the most optimistic service claim; it is the stack with the best admissible service outcome subject to zero violations of declared hard gates.

### 11.6 Qualification ladder and standards boundary

Keep evidence maturity explicit:
1. deterministic synthetic/exhaustive software oracle;
2. frozen replay of permissioned historical evidence;
3. software-in-loop with frozen controller/planner output;
4. controller-in-loop and HIL under a qualified test plan;
5. site commissioning and witnessed acceptance;
6. periodic requalification after firmware, topology, asset, calibration, supplier, route or operating-envelope changes.

IEC TS 62898-3-5:2026 expressly concerns HIL, commissioning and periodic testing for microgrid monitoring/control/energy-management systems. It supports this progression; it does not make the present synthetic corpus IEC-conformant or field-qualified. Source: https://webstore.iec.ch/en/publication/84175

The programme should seek one permissioned, bounded site/service chain first, publish a data dictionary and evidence protocol, and keep all public-data claims distinct from physically measured inventory. The current Rust cargo/service crates are synthetic reference implementations and do not yet perform this physical/site qualification.
