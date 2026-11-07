# Vector-resistant, heat-resilient habitat benchmark v0.1

Status: research specification; synthetic-only; not an engineering or clinical qualification.

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/4095

## Purpose

Evaluate residential designs as coupled systems: vector exclusion, thermal comfort, ventilation, water management, accessibility, maintainability, affordability and continuity through outages. The target is a healthy living environment with reduced vector exposure, not a smart pesticide platform.

## Existing owners

This benchmark composes existing work and adds no parallel geometry or building-physics authority:

- Sol-Atlas CITY-MORPH-SAFE-001 #591 — resident safety, CPTED and life-safety.
- Sol-Atlas CITY-HEAT-RETROFIT-ATLAS-001 #593 — Johannesburg heat exposure and retrofit opportunities.
- Sol-Atlas CITY-NEIGHBOURHOOD-REFERENCE-001 #596 — neighbourhood reference demonstrator.
- Symtropy SPATIAL-DESIGN-001 #1766 — resident-led habitat design and spatial topology.
- Symtropy SPATIAL-RESILIENCE-001 #1848 — habitability trajectories under matched stress.
- Symtropy ENERGYPLUS-QUAL-001 #1900 — version-pinned whole-building thermal adapter.
- Symtropy built-environment adapter boundary #1790 — solver neutrality and validation limits.

## Research baseline

1. Abong'o et al., Nature Medicine, published 5 January 2026: [Housing modifications for heat adaptation, thermal comfort and malaria vector control in rural African settlements](https://www.nature.com/articles/s41591-025-04104-9). This randomized pilot involved 40 houses across cool-roof, cross-ventilation, mat-ceiling and control arms. Intervention houses had doors, windows and eaves screened. The paper reports a 3.3 °C daytime and 2.4 °C nighttime heat-index reduction in cool-roof houses; captured female Anopheles funestus and Culex counts were 77% and 58% lower in screened intervention houses. This is feasibility evidence from a small pilot, not a clinical endpoint or a transferable forecast.
2. Cochrane CD013398: [House modifications for preventing malaria](https://www.cochrane.org/evidence/CD013398_house-modifications-preventing-malaria). Seven African trials; the review reports reduced parasite prevalence and moderate/severe anaemia, while clinical incidence findings were mixed and indoor mosquito-density evidence less certain.
3. WHO: [Global Vector Control Response 2017–2030](https://www.who.int/publications/i/item/9789241512978). Four pillars: cross-sector collaboration; community engagement; surveillance, monitoring and evaluation; integrated tools.
4. WHO: [Malaria surveillance, monitoring and evaluation reference manual, second edition](https://www.who.int/publications/i/item/9789240112476), 10 July 2025.
5. WHO: [Guidelines for malaria](https://www.who.int/publications/i/item/guidelines-for-malaria/), version dated 10 September 2026. Pin the applicable recommendation revision and jurisdiction for each real intervention.
6. South Africa: [National Vector Control Strategy 2023–2027](https://www.health.gov.za/wp-content/uploads/2024/07/National-Vector-Control-Strategy-2023-2027-signed.pdf). Its surveillance objectives include receptivity, vector density and seasonality, insecticide resistance, intervention coverage and quality, and identification of gaps.

Study effect sizes are source observations, not benchmark thresholds or guaranteed product outcomes.

## Paradigm shift

Move from single interventions and nominal scores to a whole-life, evidence-qualified habitat:

- Passive prevention and maintainability before powered sensing or chemical control.
- Vector ingress, indoor captures, human exposure and clinical disease outcomes are separate evidence tiers.
- Ventilation, air quality, thermal safety, accessibility, fire egress and water service are hard constraints, not costs that can be traded away.
- Local species, behaviour, seasonality and ecology are explicit applicability conditions.
- A missing sample, damaged screen or stale model produces Unknown/Blocked, never a favorable zero.
- Modelled output is not measured performance; measured vector counts are not clinical efficacy.

## System boundaries

Symtropy owns deterministic candidate comparison, resource accounting and scenario replay. It consumes geometry and solver outputs only through existing qualified interfaces.

Symthaea may summarize evidence, identify contradictions and suggest the next useful measurement. It does not diagnose disease or direct treatment.

Sol-Atlas owns sourced spatial context and controlled map presentation. Coarse ward-level evidence must never be presented as household-level certainty.

Mycelix owns consent, provenance, approvals and correction/revocation records. A recorded approval does not prove physical performance.

Public-health and clinical interpretation remains with qualified and authorized professionals.

## V0 synthetic scenario matrix

Freeze paired scenarios for:
- intact screened openings;
- degraded or poorly fitted screens;
- screening that compromises required ventilation;
- screened openings plus passive roof cooling;
- alternative ventilation with an explicit outdoor vector context;
- blocked drainage and standing-water observations;
- hot-weather power and water outage;
- inaccessible hardware or egress obstruction;
- absent, stale or conflicting vector-surveillance observations;
- species mismatch;
- wrong coordinate reference or spatial-scale mismatch;
- simulated vector-count improvement with no clinical outcome evidence.

All fixtures are synthetic. No synthetic numerical output may be represented as a building or disease prediction.

## Metrics

Report separate, unit-bearing results:
- vector capture per trap-night and species, with sampling protocol and effort;
- indoor biting/exposure proxies only where an approved validated protocol exists;
- indoor operative temperature and declared comfort/heat metric, including hot-night tails;
- air-change rate, CO2 and particulate matter where actually measured;
- thermal solver convergence, calibration domain and residuals;
- energy and water use; essential service-floor breaches during outages;
- screen integrity, defect rate, maintenance time and replacement availability;
- accessibility and emergency-egress review state;
- lifecycle cost and household affordability;
- resident acceptance, use and maintenance burden;
- coverage, missingness, uncertainty and out-of-domain rate.

Do not collapse results into a single BioShield score.

## Proposed gates (not yet measured)

- 100% of frozen cases return a typed result or explicit Unknown, Blocked or OutOfDomain.
- Zero simulated airflow/vector/thermal outputs promoted to clinical efficacy.
- Zero hard-floor violations hidden by an aggregate.
- Every design result binds exact geometry, weather, solver, source, model and fixture digests.
- Independent verifier reproduces all known-answer classifications.
- Any field-validation plan is reviewed by qualified entomology, public-health and building professionals before health claims.

## Implementation sequence

1. Freeze the schema and synthetic fixture cases.
2. Add semantic conformance checks, including species, scale, freshness and evidence-tier negative controls.
3. Reuse the qualified EnergyPlus adapter for in-scope thermal outputs.
4. Register a separate airflow/ventilation adapter if the decision requires capabilities outside EnergyPlus' qualified envelope.
5. Build a low-cost screened/passive-cooling reference design and a matched alternative.
6. Obtain professional and community review before field deployment.
7. Publish field results only with preregistered methods, independent analysis and explicit confidence intervals.

## Non-goals

No autonomous pesticide dosing or insecticide choice; no pathogen manipulation, genetic vector engineering or release design; no diagnosis or treatment advice; no household infection mapping; no claim of disease elimination or clinical efficacy from simulation alone.
