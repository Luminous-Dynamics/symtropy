# Subterranean Utility Fabric — Lifecycle Economics Model v0.1

**Status:** Economic-method specification; no local prices, incident rates, discount rate, route quantities or commercial outcomes are asserted.  
**Snapshot:** 2026-11-12  
**Programme:** https://github.com/Luminous-Dynamics/symtropy/issues/4910

## 1. Decision rule

The question is not whether a robotic duct bank costs less per metre to install. The question is:

> For the same accepted service, risk boundary, route, time horizon and quality floor, does an alternative deliver lower risk-adjusted whole-life cost and better distributional outcomes than the best eligible local alternative?

Evaluate physical and legal eligibility first. A cheaper architecture that fails electrical, geotechnical, environmental, access, wayleave, service-continuity or safety requirements is not an economic candidate.

Do not use one weighted score to hide trade-offs. Report:
- a financial view for each participating utility / corridor operator;
- a consolidated whole-system economic view;
- a service and resilience outcome vector;
- a distributional / local-capability view;
- a Pareto frontier for alternatives that meet the same hard service floors.

## 2. Accounting boundaries

Publish two distinct ledgers / views.

### 2.1 Project or operator financial case

This view answers whether the asset owner, utility, corridor operator or concessionaire can finance, operate and maintain the project.

It includes:
- cash capex and financing;
- utility-owner and corridor-operator opex;
- maintenance, security, insurance and energy;
- replacement, major renewals and decommissioning;
- outage compensation and directly borne consequences;
- capacity / access revenue and payment obligations;
- taxes, grants and subsidies with the accounting treatment stated.

### 2.2 Consolidated whole-system economic case

This view answers whether the intervention creates net value for the affected service system and society.

It includes real resource use and changes in service outcomes:
- labour, materials, equipment, land and road-space occupation;
- traffic delay, surface disruption and reinstatement;
- water / electricity / telecommunications service lost or restored;
- repeat excavation and avoidable repair work;
- environmental effects and embodied impacts where measured;
- safety and worker exposure where there is a defensible valuation method;
- distribution of service availability, affordability and local economic participation.

**Transfer / double-count rule:** a corridor lease payment is operator revenue and tenant expenditure, not two system-wide benefits. Wages are a project cost and household income transfer; do not count the full wage bill again as net social benefit. Taxes, subsidies and grants must be treated consistently. Count real resource savings once.

## 3. Functional comparison unit

A universal "metre of tunnel" is not a sufficient unit of value. A metre of corridor with no accepted service does not satisfy the objective.

For each study, define a service vector over the analysis horizon:
- delivered electrical capacity and energy at the agreed interface;
- telecommunications / fibre capacity and availability;
- potable-water quantity, quality and continuity;
- wastewater conveyance and environmental compliance;
- stormwater performance for the declared design events;
- maximum interruption duration and restoration service level;
- served customers / premises / industrial loads;
- coverage, equity and affordability constraints.

Only compare alternatives with materially equivalent service requirements. Where services differ, report service-specific costs and outcomes; do not collapse incompatible outputs into a single artificial "service unit."

Useful normalized outputs include:
- cost per accepted route-km-year;
- cost per connected premise-year at the declared service floor;
- cost per delivered MWh-year for the electrical service;
- cost per m³ of compliant water delivered / wastewater conveyed where applicable;
- cost per service-hour restored or outage-hour avoided;
- cost per accepted, maintainable utility connection.

## 4. Lifecycle model

For architecture (a), analysis period (T), and real discount rate (r), define:

[
NPV_a =
sum_{t=0}^{T}
rac{
CAPEX_{a,t}+OPEX_{a,t}+RENEW_{a,t}+E[LOSS_{a,t}]-REV_{a,t}-RESIDUAL_{a,t}
}{
(1+r)^t
}
]

The model must state whether flows are real or nominal, the price year, currency, inflation treatment, tax treatment, financing boundary and discount-rate source. Do not insert a generic South African discount rate without identifying the owner and decision context.

### 4.1 Initial and staged capital

Separate:
- route planning, utility-record requests and subsurface investigation;
- cross-cuts, trial holes, survey and geotechnical / hydrogeological investigation;
- design, professional review and permitting / wayleave work;
- mobilization, contractor setup, traffic and public protection;
- excavation / trenchless construction / boring / jacking / pipe or duct placement;
- ducts, partitions, access nodes, chambers, service modules and sensors;
- cable / pipe / fibre installation, jointing, terminations and protection;
- reinstatement, testing, commissioning and as-built acceptance;
- staged expansion, spare capacity and future connection capex.

Include unsuccessful / aborted work, redesign and rework in the correct candidate's cost.

### 4.2 Operating and maintenance costs

Include:
- scheduled and condition-based inspections;
- robot mobilisation, charging, retrieval, cleaning, wear parts and calibration;
- human operator / inspector time and review;
- sensors, power, communications, storage and software support;
- access-node maintenance, cleaning, dewatering, sealing and corrosion control;
- security, response, insurance and incident investigation;
- cable / pipe / duct repairs and service restoration;
- spare parts, technician training and local support capability;
- utility-owner coordination and contract administration.

A robot is not "autonomous maintenance at zero labour." Count the human attention, supervision, recovery and repair capacity it actually needs.

### 4.3 Event and consequence costs

Model separately:
- theft attempt, completed theft, vandalism and sabotage;
- direct lost asset / material value;
- investigation and response;
- replacement and repair;
- service interruption and restoration;
- secondary damage to roads, other utilities, property, commerce and public services;
- common-cause incidents such as flood, fire, excavation strike or shared-node failure.

Avoid counting one event both as a generic maintenance cost and as a theft / outage event cost. Each cost item needs an owner, event ID or accounting rule and a boundary showing who bears it.

### 4.4 Renewal and end of life

Include asset-specific useful-life assumptions, inspection intervals, technology obsolescence, conductor / joint replacement, duct capacity changes, service-module replacement, decommissioning, rehabilitation, residual value and lawful material recovery.

Do not assign a common design life to every service simply because the services share a corridor. A shared corridor may remain while a cable, pipe, joint, sensor or robot is replaced several times.

## 5. Expected theft and outage loss

For each event class (s) and time period (t):

[
E[LOSS_{a,t}] = sum_s P(s mid a, exposure, controls, common causes); C(s,a,t)
]

Where:
- event probability is conditioned on route exposure, asset class, controls and the common-cause state;
- event cost is separated into direct loss, response, repair, restoration and service consequence;
- attempted events and completed events are separate observations;
- an alarm is not a prevented theft unless a response and counterfactual basis support that claim.

Estimate incidence using exposure denominators such as route-km-years or asset-years. Record incident definitions, missing reporting, changing detection probability and changes in patrol / sensor coverage. Check overdispersion and clustering before assuming a simple Poisson model. Rare catastrophic failures need scenario / tail analysis, not just the mean.

### 5.1 Common-cause dependence

Do not model every protection layer as independent. A flood may disable the access chamber, power, communications and sensors together. A contractor or credential compromise may affect several assets. A provider insolvency can remove maintenance, spares and software support simultaneously.

Use explicit joint scenarios or a defensible dependency model. Do not multiply optimistic independent component reliabilities when their failure causes are correlated.

### 5.2 Measuring deterrence effect

A pre/post incident count alone is weak evidence because enforcement, commodity prices, reporting and exposure can change simultaneously.

Prefer, where feasible:
- pre-registered matched corridors or phased rollouts;
- exposure-normalized incident and service-loss rates;
- comparable route type, service criticality, patrol intensity and reporting regime;
- difference-in-differences or other defensible quasi-experimental estimation;
- uncertainty intervals and sensitivity to unobserved confounding;
- repeat-targeting and displacement to adjacent corridors / other materials.

Randomized allocation may be impractical or unethical for critical infrastructure; a matched observational design must disclose residual confounding. Keep raw incident reports, police / utility reports, attempted events, confirmed losses and unverified alarms distinct.

## 6. Incremental break-even equations

Let (B) be the best eligible baseline and (R) a robot-first / shared-corridor candidate.

[
Delta NPV = NPV_R - NPV_B
]

The candidate is financially preferred only when (Delta NPV < 0) for the declared owner boundary and assumptions, or where a separately justified resilience / public-service requirement warrants a higher cost.

A useful decomposition is:

[
egin{aligned}
PV(benefits) ={}& PV(avoided theft and outage)\
&+PV(avoided repeat excavation and reinstatement)\
&+PV(inspection, repair, and restoration savings)\
&+PV(incremental capacity value)\
&+PV(other verified system savings)
end{aligned}
]

[
egin{aligned}
PV(costs) ={}& incremental initial CAPEX\
&+PV(incremental OPEX, renewals, financing, and decommissioning)\
&+PV(new common	ext{-}cause and provider risks)
end{aligned}
]

Break-even requires benefits to exceed costs, after transfer payments and double counting are removed.

### 6.1 Theft break-even

For an explicitly defined event class with baseline frequency (lambda_B), candidate frequency (lambda_R), exposure and average attributable loss (C_s), the avoided-event value is:

[
PV_{theft} = sum_t rac{(lambda_{B,t}-lambda_{R,t}),C_{s,t}}{(1+r)^t}
]

Use an exposure-normalized rate and uncertainty interval. Do not insert a target percentage reduction until a pilot design has a baseline and adequate sample size.

### 6.2 Shared-corridor break-even

Test the effects of:
- number of utility tenants and their timing of entry;
- duct / compartment occupancy and reserved spare capacity;
- future connection growth and uncertainty;
- repeat road opening / reinstatement rate;
- utility access rights and cost-allocation rules;
- capital cost allocated to common vs service-specific assets;
- maintenance access and service outage coupling.

The shared design may lose where there are few tenants, little repeat work, low service density or expensive access nodes. An empty spare duct is an option with a carrying cost, not automatically an economic benefit.

### 6.3 Robot-first break-even

Count benefits only where measured:
- fewer confined-space entries;
- shorter time to locate an accepted fault;
- less mobilisation or traffic disruption;
- more inspections per operator-hour;
- more repairs via existing access nodes;
- fewer failed / unnecessary excavations;
- higher first-pass maintenance acceptance;
- less service downtime.

Subtract robot capital, retrieval incidents, support / spares, calibration, operator supervision, docking energy, communications, software support and technology refresh.

## 7. Architecture-specific cost drivers

| Architecture | Likely cost advantage | Cost / risk to expose |
|---|---|---|
| A0 observed local practice | Uses existing assets and established teams | Hidden repeat cuts, theft, outage, maintenance debt and poor asset records |
| A1 protected overhead | Avoids much excavation; may be quickest | Continued exposure, access / vandalism, weather and clearance constraints |
| A2 conventional single-utility underground | Mature construction and asset practices | Independent trenches, repeat road cuts, difficult fault access and restoration |
| A3 shared partitioned duct bank | Coordinated installation and spare capacity | Upfront premium, coordination, common-node risks and occupancy uncertainty |
| A4 robot-first duct bank | Potentially lower inspection / retrieval / repair disruption | New hardware, access geometry, software, spares, training and proof burden |
| A5 accessible mini-gallery | Human access, inspection and multiple interventions | High capital, fire / ventilation / drainage / egress / security and ongoing operating costs |
| A6 reuse / brownfield | May avoid some new excavation and civil capex | Unknown condition, contamination, water ingress, rehabilitation, liability and rights |
| A7 no-build / targeted hardening | Avoids stranded capex | Residual theft / outage exposure and deferred renewal costs |

Every architecture should be allowed to win or lose by site stratum. No preselected winner.

## 8. Uncertainty and sensitivity protocol

For every quantitative input store:
- source / custodian and exact revision;
- valid period and price year;
- geographic / route scope;
- unit, timebase and conversion;
- evidence class and acquisition status;
- point estimate or distribution, basis and uncertainty;
- correlation / dependency assumptions;
- missingness, stale status and known bias.

Report at minimum:
- deterministic low / central / high sensitivities where distributions are unavailable;
- P10 / P50 / P90 lifecycle cost when a defensible probabilistic model exists;
- break-even maps across theft exposure, route density, repeat roadworks, tenant occupancy, project life and discount rate;
- one-way sensitivities plus selected joint / global sensitivity analysis;
- tail scenarios for common-cause failures, loss of operator, and catastrophic restoration;
- value of information for additional survey / cross-cut / trial installation.

Do not use invented probability distributions to make a model appear quantitative. When there is no data, report a symbolic threshold or an explicit range of scenarios.

## 9. South African model inputs

Acquire before declaring a business case:
- local tender / quotation data by method, route class, soil / rock, diameter, access, reinstatement and price date;
- wayleave cycle times, service-owner responses, cross-cut and redesign costs;
- asset exposure, theft / attempted tamper, response, replacement and service-outage history;
- route length, existing utility occupancy, asset age / condition and planned renewals;
- energy losses and cable thermal / soil data where applicable;
- local contractor capacity, equipment availability, lead times, wages and skills;
- municipal and utility fee / tariff / servitude arrangements;
- project finance cost and the actual owner decision discount rate;
- local material / embodied impact data and end-of-life recovery value;
- paid local work-hours, wages, supplier spend, SME participation, training completion and retained skills.

A national aggregate theft figure cannot substitute for local route-level exposure. A utility annual report does not reveal the counterfactual loss rate for one corridor.

## 10. Model acceptance tests

The first evaluator should be deterministic and dependency-light. It should not calculate physical feasibility or authorize construction; it receives already owner-qualified inputs.

Minimum finite test cases:
1. **Zero-benefit case:** zero theft, zero repeat roadworks and low occupancy; added robot / tunnel premium should not be justified by those absent benefits.
2. **High-repeat-work case:** repeated excavation and multiple service tenants can make shared ducts economically attractive, but only if additional capex and access costs are lower than discounted avoided works.
3. **High-theft / high-criticality case:** exposure-normalized expected loss and outage value can shift the result, with probability and severity uncertainty exposed.
4. **False alarm case:** alarms without a verified event / response do not count as prevented theft.
5. **Double-count case:** the same event cannot appear simultaneously as direct theft, generic maintenance and full outage loss without an explicit decomposition.
6. **Lease transfer case:** lease income and tenant payment cancel in the consolidated whole-system view.
7. **Wage transfer case:** wages are not automatically counted as net social surplus in addition to the labour cost.
8. **Common-cause case:** flooding disables multiple compartments / systems according to declared shared dependencies; independent reliability multiplication must fail.
9. **Reuse case:** sunk historical capex is not treated as free; rehabilitation, liability and decommissioning remain.
10. **No-build case:** a no-build / targeted hardening candidate can be the least-cost eligible choice.
11. **Staged occupancy case:** future tenants arrive late or not at all; unused capacity carries its true cost.
12. **Price-year mismatch:** inputs in incompatible price years or currencies are rejected unless an explicit documented conversion is supplied.
13. **Missing input case:** unknown cost / event rate is not converted to zero.
14. **Exact replay:** identical admitted inputs and model revision produce byte-stable normalized output.
15. **Independent oracle:** small finite examples are compared with a separately implemented hand-calculated reference.

## 11. Deliverable contract

An economics report must contain:
- exact candidate, route / service boundary and configuration revision;
- input manifest and source / evidence references;
- accounting boundary and price year;
- time horizon, discount-rate source and currency;
- cashflow breakdown by year and category;
- direct / indirect / transfer cost distinction;
- risk events, exposure denominators and dependency assumptions;
- service floor and service outcome vector;
- sensitivity ranges and unresolved inputs;
- candidate disposition and exclusion reasons;
- NPV / equivalent annual cost / service-normalized outputs;
- distributional and local-capability outcome vector;
- independent verifier result and claim ceiling.

## 12. Claim ceiling

This model is a proposed accounting and benchmark contract. It contains no South African local cost estimates, no calibrated theft probability, no forecast savings and no financing decision. It cannot establish project bankability, affordability, value for money or public approval. A credible business case requires local utility records, actual quotations, approved engineering designs and a transparent comparison against the best eligible alternative.
