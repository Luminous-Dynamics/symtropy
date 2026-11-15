# Habitat Resilience Outcome Measurement Crosswalk v0.1

**Owner:** PROTECTIVE-SYSTEMS-001 (#5246)  
**Composed owners:** VECTOR-HABITAT-001 (#4095), SPATIAL-RESILIENCE-001 (#1848), Sol-Atlas local heat/retrofit and material intelligence  
**Status:** measurement design; no local field study or measured housing outcome claimed

## 1. Research premise

A 2026 *Energy and Buildings* scoping review mapped 176 building-resilience studies and 29 indicators; it reports limited operational definitions and gaps in multi-hazard, longitudinal and Global South evidence. A 10 November 2026 *Urban Science* paper derives resilience indicators from empirical Global South disaster cases and highlights conditions omitted by administrative records, including informal settlement and implementation failures.

These studies suggest a design change: start with the services residents must retain, then identify the household, building, infrastructure and institutional conditions that determine those services. Do not begin with a weighted resilience index and assume that available municipal data captures lived outcomes.

## 2. Functional unit and measurement boundary

Primary unit: **resident-hour of safe, accessible essential service under a declared household/site hazard trace**.

Also report dwelling-day and neighbourhood-person-day outputs only where the denominator and observation coverage are explicit. Never substitute a neighbourhood aggregate for household or individual observations without a valid aggregation design.

Define the services and minimum acceptable conditions before comparing designs:
- thermal safety/comfort and recovery, including hot-night conditions;
- potable water availability and minimum hygiene function;
- ventilation and indoor air-quality safeguards;
- lighting and communications access;
- safe accessible egress and emergency access;
- food/cold-chain capability where relevant;
- vector exclusion and safe pest-control interfaces where locally relevant;
- structural, fire/smoke, flood/moisture and electrical safety.

Each service has its own threshold source, measurement method, uncertainty and hard floor. A composite index may summarize but must never let improved thermal comfort compensate for loss of egress, water safety or ventilation.

## 3. Compound-hazard scenario traces

Freeze scenario inputs and distinguish observed events, design-weather files, typical meteorological years and future projections. Do not relabel a typical-year file as an extreme event.

Minimum synthetic scenario families:
1. hot day + hot night + grid interruption;
2. heat + water interruption + constrained ventilation due to outdoor air quality;
3. heavy rain + local drainage blockage + power outage;
4. heat + high humidity + moisture accumulation;
5. fire/smoke alert + power interruption + accessible egress constraint;
6. vector pressure + water interruption + ventilation/screening trade-off.

The hazard trace is an input, not evidence that a real home experienced it.

## 4. Resident-centred and Global South measurement

Where records are unavailable, explicitly represent UNKNOWN and design an ethical observation path instead of silently filling values from municipal datasets or model inference.

Candidate local-context variables:
- tenure/security and practical permission to retrofit;
- informal/unpermitted construction conditions that change exposure or access;
- affordability, finance and monthly operating cost;
- water/power service quality and outage duration;
- local material/repair availability and journey/time to obtain a replacement;
- household composition and mobility/accessibility requirements, using minimised and consented data;
- informal communication, care and mutual-aid networks relevant to warning and recovery;
- resident-defined priority services and acceptable work disruption.

These variables are not reasons to downgrade resident protection. They are conditions the engineering design must accommodate. Do not collect household biometrics, continuous audio, precise movement histories or inferred sensitive traits by default. Use consent, minimization, retention limits and group-level reporting.

## 5. Recovery and distributional outcomes

Report trajectories, not a single snapshot:
- hours below/above the declared service floor;
- cumulative resident-hours with unavailable or unsafe service;
- time to restore each service;
- time to reach stable safe conditions after grid/water restoration;
- maintenance/replacement delay and material unavailability;
- downtime distribution across households / accessibility groups where justified;
- who bears installation, cleaning, monitoring and repair work;
- distribution of benefits and burdens, not only the mean.

A design is not resilient if it maintains service only for residents who can afford backup energy, private transport or immediate repairs.

## 6. Matched benchmark comparison

Compare equivalent dwelling function, household/service assumptions, climate trace, occupancy schedule, maintenance policy and cost boundary.

Baselines:
- current compliant/reference assembly where known and applicable;
- low-cost passive retrofit;
- passive plus service-continuity provisions;
- higher-cost technology-assisted option;
- candidate Symtropy/Symthaea designs.

Report separate Pareto axes: resident-hour service, hard-safety floors, capital cost, operating cost, maintenance/replacement time, local availability, energy/water demand and embodied impact.

## 7. Evidence hierarchy and claims

Keep separate:
- synthetic scenario result;
- model result and validation range;
- bench/assembly test;
- monitored dwelling observation;
- resident-reported service experience;
- neighbourhood outcome;
- building approval / competent professional review.

A resident-reported outage or comfort experience is valuable evidence about service, but does not replace calibrated physical measurement for a technical property. A calibrated sensor measurement does not alone establish the resident's overall well-being. Triangulate rather than collapse.

## 8. Initial gates

**H0 — semantic corpus:** all services, thresholds, missingness and hard floors are typed; UNKNOWN never becomes zero/PASS.

**H1 — replay:** frozen hazard traces and owner model versions replay deterministically; reported output units and boundaries match.

**H2 — local evidence readiness:** community/research partners, consent, privacy, sampling and data governance defined; local climate and authority references verified.

**H3 — instrumented non-loadbearing demonstrator:** independently reviewed safe assembly and sensor plan; no occupied-building intervention without permissions and professional review.

**H4 — prospective field outcome:** pre-registered observation period and analysis; compare baseline/candidate only under matched context; publish uncertainty, missingness and distributions.

## 9. Non-goals

This document does not define South African code compliance, certify a building, prescribe a construction assembly, establish clinical thresholds, or authorize an installation. The applicable SANS and municipal approval basis must be independently verified for each site.
