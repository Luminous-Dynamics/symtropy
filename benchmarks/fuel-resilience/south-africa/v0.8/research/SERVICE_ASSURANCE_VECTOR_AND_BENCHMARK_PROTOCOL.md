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
