# Protective Resilience — SOTA and Industry Crosswalk (November 2026)

**Snapshot:** 16 November 2026  
**Owner:** PROTECT-RESILIENCE-MATCHED-EVAL-001 #5452  
**Parent:** PROTECT-THREAT-PORTFOLIO-001 #5433  
**Status:** literature and tool-landscape crosswalk; not a performance claim

## 1. Executive conclusion

Luminous should not attempt to build another monolithic community-resilience simulator. The current tool landscape already includes substantial, independently developed modelling systems. The highest-value opportunity is to create an **evidence-preserving, fail-closed comparison and assurance layer** that interoperates with specialist models without replacing their physics, fragility, hazard, service-cascade, recovery or socio-economic calculations.

A recent comparative study by Shamsi, Helmrich and Deegan (published 7 November 2026) identified 24 tools in its energy/water/transport scope, with seven explicitly supporting cross-sector interdependency assessment. It found no single tool that captures the full breadth of interdependency behaviour, and reported a trade-off between model complexity, data burden, computational demand, expertise and practical usability.

DOI: https://doi.org/10.1007/s10669-026-10162-2

The study's technical comparison uses ordinal 1–5 ratings for interdependency breadth, data requirements, validation strength and integration potential. These are **tool-characterization scores**, not estimates of hazard likelihood, physical safety, or a universal performance leaderboard. The paper reports IN-CORE as strongest overall on those technical dimensions in its sample, while explicitly finding that no one tool dominates every dimension.

## 2. External tool landscape

The following descriptions summarize the comparative paper's classification, not an independent Luminous re-benchmark.

| Tool | Reported role / strength | Relevant limitation or comparison caution |
|---|---|---|
| IN-CORE | Integrated community-scale hazard, infrastructure and socio-economic modelling; broad interdependency and integration capability; empirical community/post-disaster use | High data and calibration requirements; do not reproduce its hazard, DFR3 or socio-economic models |
| PRIIA | Planning and post-disaster decision support; empirical application and integration capability | Context, accessibility and input requirements must be matched |
| REFIT | Multi-sector cascading and probabilistic modelling; supports coupling with specialist tools such as EPANET and OpenDSS | Coupled-model assumptions and sector-specific input validity must travel with the result |
| Kaleidoscope | Network-oriented operational planning and post-disaster decision support | Detailed asset/geospatial data and application context matter |
| CIrcle | Participatory mapping and stakeholder-led exploration of infrastructure relations and consequences | Qualitative/participatory outputs are not interchangeable with a numerical physical simulation |
| CISIA-pro | Deterministic fault-propagation simulation | More constrained scope and model representation |
| InfraRisk | Sequential asset-level hazard, failure and recovery analysis | Narrower asset-centric focus than community-level integrated modelling |

NIST describes IN-CORE as a platform connecting hazard, DFR3 (Damage, Functionality, Repair, Recovery, Restoration), data, geospatial visualization, semantic and space services. Its architecture and technical outputs are appropriate external references for interoperability; they are not a reason to recreate its functions inside Symtropy.

References:
- Shamsi, N., Helmrich, A. & Deegan, M. (7 November 2026), *Toward system-of-systems resilience: a comparative framework for infrastructure interdependency assessment tools*: https://doi.org/10.1007/s10669-026-10162-2
- NIST Center of Excellence and IN-CORE: https://www.nist.gov/community-resilience/center-excellence
- IN-CORE project: https://in-core.org/
- IN-CORE service repository: https://github.com/IN-CORE/incore-services

## 3. What is already established — do not claim it as novel

The following capabilities already have credible external precedents:

1. Hazard-to-damage-to-functionality modelling.
2. Physical and cross-sector infrastructure dependency analysis.
3. Recovery curves and service restoration modelling.
4. Community-scale social/economic consequence assessment.
5. Stakeholder-defined performance goals and recovery-time planning.
6. Integration of multiple domain-specific simulators and data services.
7. Business-continuity management systems and operational recovery practices.

NIST's Community Resilience Planning Guide makes a particularly important distinction: desired performance goals should be driven by community social needs and the functions groups require, and established independently of any one hazard. Specified hazard scenarios are then used to evaluate anticipated building/infrastructure recovery against those goals.

This should shape our own contract:
- service floors and recovery goals are requirements established from the declared beneficiary/community context;
- hazard scenarios test whether the requirements are met;
- the scenario must not redefine the requirement merely because the candidate performs poorly;
- the resulting report must preserve the subgroup/service distribution, not just a population mean.

References:
- NIST Community Resilience Planning Guide: https://www.nist.gov/community-resilience/planning-guide
- NIST SP 1190 Volume I: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.1190v1.pdf
- NIST Community Resilience Playbook / performance goals: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.1190GB-16.pdf

## 4. The genuine gap to investigate

The 2026 comparison identifies a persistent imbalance: tools more readily represent physical infrastructure dependencies than social, economic, organizational and governance dependencies. These latter dependencies may dominate during response, restoration, cross-jurisdiction coordination and long-term recovery.

The opportunity is **not** to invent a universal agent-based society simulator. It is to ensure that decisions cannot silently ignore decision-relevant institutions and people.

Future evaluation manifests should be able to declare, where relevant:

- authority to approve and execute a protective action;
- responsibility and coordination across operators, agencies and jurisdictions;
- mutual-aid and service-provider commitments;
- workforce availability, skills and shift/relief constraints;
- procurement lead times, spares and local repair capacity;
- funding and liquidity constraints;
- permissions, access rights and land/building tenure;
- information-sharing, communications and public-warning pathways;
- distribution of service benefits and recovery burden;
- accessibility, language and locally defined essential functions.

These are dimensions to represent as evidence-backed requirements, scenario variables or explicit unknowns. They must not be assigned invented behavioural parameters. Any quantitative institutional model requires its own empirical validation and claim ceiling.

The current Symtropy owners should remain responsible for their domains:
- #3894: service dependencies and cascade trajectories;
- #3621/#3739: cross-scale service, reserve and ecological/physical recovery;
- #5317: uncertainty sets and robust decision comparison;
- #5268: decision value of new observations;
- #3731: measurement-as-distribution and benchmark design;
- Mycelix: identity, permission, evidence and lifecycle provenance;
- Sol-Atlas: rights-aware spatial and local-context evidence.

The bridge consumes declared outputs and exposes missing owner evidence; it does not duplicate the underlying models.

## 5. Proposed Luminous differentiation

The bridge should be benchmarked on assurance properties that are often left to the analyst when multiple specialist models are combined:

| Assurance dimension | Testable question |
|---|---|
| Scope identity | Are context, jurisdiction, boundary, demand, time horizon and scenario exactly matched before comparison? |
| Provenance preservation | Can each value be traced to its source, revision, method, owner and configuration? |
| Measurement semantics | Are native units and beneficiary strata preserved, with no invalid cross-service aggregation? |
| Uncertainty semantics | Are aleatory, epistemic, model-form, shift and unknown states kept distinct? |
| Missing evidence | Does the bridge abstain rather than substitute zero, safe, available or recovered? |
| Hard constraints | Are mandatory human-safety and service floors non-compensatory? |
| Recovery quality | Are service, reserve/buffer, maintenance and ecological/physical recovery separate? |
| Benefit accounting | Are shared costs and outcomes credited once, with overlapping claims reconciled? |
| Jurisdiction and rights | Are standard edition, legal scope, data rights and local approval authority explicit? |
| Reproduction | Can an independent reviewer replay the decision from frozen artifacts and exact model revisions? |
| Practical usability | Can a small operator use staged evidence levels without needing a complete city-scale digital twin? |

The core claim to test is not "Luminous has a better simulator." It is:

**Given valid outputs from competent domain models, does the Luminous bridge make cross-model investment decisions more reproducible, comparable, safety-constrained and transparent—without increasing the data and operational burden beyond the value it adds?**

## 6. Benchmark design implications

The E0 exact synthetic oracle now provides a narrow start:
- consumes fixed service trajectories rather than simulating their causes;
- checks identical scenario, boundary, demand, time and lifecycle-cost basis;
- recomputes deficit and sustained recovery independently;
- keeps service metrics separate by service ID;
- keeps a vulnerable/assisted-access beneficiary stratum visible;
- rejects duplicate cost and outcome-benefit identities;
- distinguishes service recovery from buffer recovery;
- rejects a low-cost option that violates a mandatory safety floor.

This is only evidence-accounting and metric conformance. It does not validate the upstream trajectory source or demonstrate actual resilience.

Next comparison gates:
- **E0:** exact micro-oracle, frozen expected metrics and mutation/negative controls.
- **E1:** import a rights-cleared IN-CORE or other specialist-model result, preserving source, version, assumptions, output units and claim limits.
- **E2:** compare against persistence, competent domain rules/operator plans, applicable continuity-management practices, and the specialist model where scope is matched.
- **E3:** independently reproduce a rights-cleared retrospective case.
- **E4:** only then consider prospective/physical qualification under competent review.

Match context, boundary, service demand, beneficiaries, scenarios, observation budget, lifecycle cost and evaluation horizon before computing comparisons. If a match cannot be established, mark the comparison non-comparable.

## 7. Metrics and claim discipline

Report a vector, not one resilience score:

- service deficit and time below a community-defined floor;
- sustained time to recover function;
- per-stratum minimum service and recovery;
- reserve/storage condition and replenishment runway;
- maintenance debt and repair capacity;
- human safety, accessibility and health constraints;
- environmental/physical state only where the domain model supports it;
- full lifecycle cost and fully burdened operating burden;
- uncertainty, model discrepancy, applicability and abstention;
- reproducibility and traceability.

Do not aggregate unlike outputs (for example, water volume, kWh, clinic access, ecological condition and money) into one weighted total without a defensible, community-approved construct and an independently reviewed measurement model.

The review of resilience curves by Poulin and Kane examined 273 publications and emphasizes that summary metrics depend on how performance is defined and how milestones and ensembles are handled. In particular, a single expected curve can conceal the distribution of trajectories. Our reporting should preserve scenario ensembles, strata and uncertainty rather than replacing them with a smoothed average.

Reference:
- Poulin & Kane (2021), *Infrastructure resilience curves: Performance measures and summary metrics*: https://doi.org/10.1016/j.ress.2021.107926

## 8. Standards and jurisdiction crosswalk

These references have different roles and must not be treated as interchangeable certifications.

- **NIST Community Resilience Guide:** community performance planning, social need, interdependency and desired recovery time.
- **IN-CORE:** external computational modelling and interoperability comparator.
- **ISO 22301:2019 + Amendment 1:2024:** business-continuity management-system requirements. ISO lists the 2019 edition as at stage 90.92 ("to be revised"), with a committee draft under development. A BCMS certificate is not proof of a specific physical protection or service trajectory.
- **NIST SP 1339 (final, 17 June 2026):** OT backup discipline, including integration with change management, regular creation, testing and recovery exercises.
- **NIST SP 1800-45 (final, 24 June 2026):** three representative secure remote-access architectures for water/wastewater OT; not a universal utility design.
- **NIST SP 800-82 Rev. 4 (initial public draft, 21 September 2026):** draft status only; do not call it final.
- **South African Disaster Management Act 57 of 2002 and National Disaster Management Framework review (Government Gazette 48874, 30 June 2023):** a national/provincial/municipal coordination context. Each real deployment still requires current local, sector and municipal/provincial requirements to be established by competent local review.

References:
- ISO 22301:2019 lifecycle: https://www.iso.org/standard/75106.html
- ISO 22301 Amendment 1:2024: https://committee.iso.org/standard/88412.html
- NIST SP 1339: https://csrc.nist.gov/pubs/sp/1339/final
- NIST SP 1800-45: https://csrc.nist.gov/pubs/sp/1800/45/final
- NIST OT publication status: https://csrc.nist.gov/projects/operational-technology-security/publications
- Disaster Management Act 57 of 2002: https://www.gov.za/documents/disaster-management-act
- National Disaster Management Framework review: https://www.gov.za/sites/default/files/gcis_document/202307/48874gon3635.pdf

Do not assume the international standard is identical to the current South African adopted edition. Record exact SANS identifier, edition, amendment, jurisdiction, applicability basis and currentness from the authoritative SABS catalogue before a compliance statement.

## 9. What would count as superiority

A superiority claim requires a matched test and independent review. Proposed materiality thresholds remain prospective:
- at least 20% lower fully burdened lifecycle cost with all mandatory floors preserved; or
- at least 20% lower integrated deficit for a specified service at equal lifecycle budget.

These thresholds are not universal pass marks. Pre-evaluation power/precision analysis must show the available data can resolve the target effect. Otherwise report the smallest detectable effect and confidence interval.

Also report:
- model/data preparation effort and calibration hours;
- required operator expertise and recurring maintenance burden;
- compute, review and measurement costs;
- missing/unsupported strata;
- abstentions and false-feasible rates;
- cases where Luminous loses or adds no value.

**The most credible path to beating SOTA is useful interoperability plus measurable assurance value, not rebuilding IN-CORE or presenting a higher synthetic score.**
