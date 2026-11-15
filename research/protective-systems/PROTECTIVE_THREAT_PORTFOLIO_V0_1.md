# Defensive Threat Portfolio v0.1

**Snapshot:** 15 November 2026  
**Owner:** PROTECT-THREAT-PORTFOLIO-001 #5433  
**Parent:** PROTECTIVE-SYSTEMS-001 #5246  
**Existing method owner:** PROTECT-ROBUST-001 #5317  
**Register:** `research/protective-systems/fixtures/protective-threat-family-register-v0.1.json`

## 1. Decision statement

The programme will prioritize **defensive protection of people and essential capabilities**, not design or evaluation of offensive weapons. It will screen threat/hazard families by evidence maturity, event frequency in a specific context, asset exposure, consequence, present vulnerability, mitigation readiness and whole-life economics.

A hypothetical hazard can have a dramatic consequence and still be a poor investment target if there is no credible physical/engineering pathway, no relevant exposure and no practical mitigation. Conversely, a modest event can justify investment if it is common, affects many people or interrupts an essential service.

The register is qualitative. It does not produce invented probabilities, universal damage scores or a ranked list of military targets.

## 2. What current evidence says

### Directed energy

The Congressional Research Service's January 7, 2026 primer covers high-energy lasers and high-power microwave systems, describing active investment and potential applications. It also distinguishes their possible advantages from unresolved affordability, feasibility and utility questions in some roles. The November 2026 technical discussion by JAPCC highlights power, cooling, mobility, weather, line-of-sight and collateral constraints. These sources support a **context-specific sensor/electronics resilience workstream**, not a claim that every building or person faces a common directed-energy threat.

Sources:
- https://www.everycrsreport.com/files/2026-01-07_IF11882_97aff318dfa7ee420f43e2fc93c0e2a642284ee8.html
- https://www.japcc.org/articles/bringing-directed-energy-weapons-into-focus/

### Hypersonic and high-speed strike

The CRS Emerging Technologies primer updated November 13, 2026 includes hypersonics among technologies of interest. A separate CRS overview says current defence against hypersonic weapons remains unresolved and experts disagree about technical feasibility and affordability. This argues for **passive infrastructure resilience, service distribution, redundancy and recovery**, not an unsupported claim that Luminous can intercept such systems.

Sources:
- https://www.everycrsreport.com/files/2026-11-13_IF11105_50a12d74c49e9c88f8f93828fd81a478ce4b9bc3.html
- https://www.gao.gov/products/gao-26-107974

### EMP, GMD and PNT

DHS identifies EMP and geomagnetic disturbance as potential sources of electronic and infrastructure disruption. CISA's EMP guidance describes protection tiers tied to tolerated mission outage. Its PNT acquisitions guidance describes cascading consequences from PNT disruption and promotes multiple sources, monitoring and resilient timing. The Luminous EMSR programme already owns defensive electromagnetic and PNT monitoring, anomaly assessment and safe degraded operation.

Sources:
- https://www.dhs.gov/science-and-technology/electromagnetic-pulse-empgeomagnetic-disturbance
- https://www.cisa.gov/sites/default/files/publications/19_0307_CISA_EMP-Protection-Resilience-Guidelines.pdf
- https://www.cisa.gov/sites/default/files/2024-03/Federal%20Positioning%20Navigation%20and%20Timing%20%28PNT%29%20Services%20Acquisitions%20Guidance%20%28508%29_0.pdf
- https://github.com/Luminous-Dynamics/symthaea/issues/10775

### Chemical, biological and radiological emergencies

WHO's June 3, 2026 clinical guidance covers recognition, personal protection, decontamination and health-facility response to chemical exposure. WHO's hospital-shelter guidance focuses on maintaining health services during CBRN emergencies. The IAEA GSR Part 7 establishes a broad preparedness and response framework for nuclear or radiological emergencies, including protective actions, monitoring, communication and response capability.

Sources:
- https://www.who.int/publications/i/item/B09740
- https://www.who.int/europe/publications/i/item/WHO-EURO-2025-11046-50818-77024
- https://www.iaea.org/publications/10905/preparedness-and-response-for-a-nuclear-or-radiological-emergency

### Space-environment hazards

NASA's Meteoroid Environment Office describes meteoroid impacts as a quantifiable spacecraft risk, including potential damage to pressurized volumes and spacecraft components. NASA's orbital debris materials describe the use of risk assessment, shielding and mass-efficient design. For space habitats, this makes micrometeoroid/orbital debris exposure a much more grounded protection problem than speculative exotic orbital weapons.

Sources:
- https://www.nasa.gov/meteoroid-environment-office/overview/
- https://orbitaldebris.jsc.nasa.gov/faq/
- https://orbitaldebris.jsc.nasa.gov/protection/

### Economics

NIST's Risk Mitigation Toolkit organizes hazard-frequency, consequence and economic-evaluation resources for cost-effective mitigation. The programme should use lifecycle costs and multi-hazard co-benefits, not purchase price alone. The costs of maintenance, energy, testing, training, consumables, downtime, replacement, decommissioning and recovery must be included.

Source:
- https://www.nist.gov/services-resources/software/risk-mitigation-toolkit

## 3. Initial investment portfolio

The JSON register contains ten hazard families. The following is a provisional portfolio disposition; it is not an absolute global ranking.

| Portfolio | Initial disposition | Why |
|---|---|---|
| Physical impact and blast protection | INVEST / IMPLEMENT in exposed contexts | Established hazard class and direct fit with current PPE/vehicle/building work |
| Fire, smoke and thermal safety | INVEST / IMPLEMENT | Broad co-benefit across buildings, transport, energy storage and workplaces |
| Chemical/biological incident readiness | INVEST / IMPLEMENT where exposed | Detection, shelter, ventilation, PPE, decontamination and medical continuity have multiple uses |
| PNT, communications and cyber-physical continuity | INVEST / IMPLEMENT | Real dependencies and existing operational evidence; reuse EMSR |
| EMP/GMD resilience | PILOT / VALIDATE critical service tiers | Selective hardening and continuity can have broad benefits; full asset hardening is not automatically economical |
| Directed-energy exposure | PILOT / VALIDATE exposed sensor/electronic contexts | Emerging capability, but maturity, exposure and operational effects are context-specific |
| Radiological/nuclear preparedness | MONITOR / PREPARE | High-consequence readiness is valuable; specialized capital requires site/context evidence |
| Hypersonic/advanced strike resilience | MONITOR / PREPARE essential services | Interception economics are unresolved; redundancy and recovery are broadly useful |
| Space debris/MMOD | INVEST / IMPLEMENT for space assets | Quantifiable natural environment and established engineering test practice |
| Speculative exotic-physics claims | DEFER / MONITOR | No credible fieldable pathway established; retain only general resilience |

### First principle: fund co-benefit controls

The first defensive investments should be those that cover several credible hazard classes:
1. Redundant and recoverable power, communications and timing.
2. Safe shelter, detection, warning and essential healthcare continuity.
3. Fire/thermal protection and contamination-aware building services.
4. Configuration-bound inspection, maintenance and qualification evidence.
5. Distributed service provision and recovery capability.
6. Repairable protective assemblies, available spares and locally serviceable systems.

A specialized protective measure should be preferred only where its marginal risk reduction justifies its lifecycle cost over these broader controls.

## 4. Economic comparison method

Do not multiply ordinal values like likelihood 1–5 × severity 1–5 and call the result expected loss.

For each asset/service context, preserve a risk vector:

`(evidence maturity, event frequency range, exposure, consequence vector, vulnerability, uncertainty, mitigation readiness, lifecycle cost, co-benefits, residual risk)`

Where event frequency and loss distributions are adequately supported, estimate expected annualized loss and uncertainty. Where they are not, use bounded scenarios, sensitivity analysis, robust regret and break-even analysis.

Compare interventions by:
- incremental present-value lifecycle cost;
- expected/bounded avoided loss;
- cost per beneficiary-year protected;
- critical-service hours preserved;
- reduction in service-deficit duration and time-to-recovery;
- inspection/maintenance/training burden;
- co-benefits to other hazards;
- supply-chain repairability and option value.

Do not monetize away hard safety requirements. Report Pareto-efficient portfolios and residual risks separately.

## 4.1 Industry comparison: whole-system economics

The industry comparison must include the operational system around a protective technology, not only its headline performance.

- The CRS directed-energy primer notes a potentially low marginal energy cost per engagement only **assuming sufficient power supply**; this is not the full acquisition, integration, cooling, maintenance or support cost.
- A public U.S. Army discussion on 21 August 2026 described a year-long operational pilot intended to establish the fully burdened cost of running directed-energy systems, including maintainability, personnel and field-support requirements, while checking that operations do not disrupt the installation's other critical functions.
- GAO's April 2026 sustainment report notes that operating and support costs historically account for approximately 70% of a weapon system's total lifecycle costs in the U.S. defense acquisition context. This is not a universal fraction for protective products, but it is a warning against using purchase price as a proxy for lifecycle affordability.
- CISA's EMP guidance uses graduated protection levels tied to permitted mission outage duration, illustrating that protection intensity should follow the value and continuity requirement of the protected service.

Sources:
- CRS directed-energy primer: https://www.everycrsreport.com/files/2026-01-07_IF11882_97aff318dfa7ee420f43e2fc93c0e2a642284ee8.html
- U.S. Army operational pilot discussion (21 August 2026): https://www.army.mil/article/294836/jiatf_401_counter_uas_and_critical_infrastructure_protection_media_roundtable_august_21_2026
- GAO-26-108140 sustainment findings (23 April 2026): https://files.gao.gov/reports/GAO-26-108140/index.html
- CISA EMP protection and resilience guidelines: https://www.cisa.gov/sites/default/files/publications/19_0307_CISA_EMP-Protection-Resilience-Guidelines.pdf

Therefore, every proposed protective intervention should report power and cooling demand, operator and maintainer hours, calibration/inspection intervals, spares and consumables, support-contractor dependence, downtime, training burden, failure/repair availability, replacement and decommissioning. Compare cost per beneficiary-year or critical-service-hour preserved; never compare only purchase price or marginal energy cost.

## 5. How Symthaea, Symtropy and Mycelix should cooperate

**Symthaea:** maintain source-aware hazard hypotheses, expose uncertainty and propose the next useful evidence or assessment. Do not infer intent or assign unsupported event probability.

**Symtropy:** reuse PROTECT-ROBUST #5317 for uncertainty sets and robust multi-objective comparisons; use CIVDYN-CASCADE #3894 to evaluate service impacts and recovery over dependency networks. Reuse existing hazard and engineering solvers rather than creating a universal exotic-weapon simulator.

**Mycelix:** bind the selected asset configuration, inspection, evidence, approval and maintenance history to exact subjects. It does not certify the physical truth of an assertion.

**Sol-Atlas:** show only context and spatial data with source and permitted resolution; avoid exposing sensitive asset vulnerabilities or personal information.

## 6. Benchmark programme

### T0 — threat registry integrity
- Source, publication date, maturity, context and claim limit on every entry.
- No unsupported probability or damage figures.
- Unknown and conflicting evidence preserved explicitly.
- Ten synthetic decision fixtures exercise established hazards, rare high consequence, emerging technology, unknown frequency, unsupported exotic claims, absence of exposure, overlapping benefits, ordinal-score misuse and safety-floor violation.
- A mandatory safety-floor violation returns REJECT_INFEASIBLE, never an economic trade-off.

### T1 — ranking stability
- Change event-frequency ranges, exposed asset and consequence assumptions.
- Report rank movement, confidence and cases that become indeterminate.
- No claim of a stable top-ten ranking if small plausible changes reorder it.

### T2 — whole-life economics
- Compare no-change baseline, general resilience, specialized controls and multi-hazard portfolios.
- Match asset boundary, service objective, discount assumptions, time horizon and operating cost.
- Independent review of avoided-loss and co-benefit double-counting.

### T3 — compound scenario resilience
- Compose PROTECT-ROBUST and CIVDYN-CASCADE.
- Report hard-floor violations, nominal loss, worst-case loss, CVaR only when sample assumptions justify it, service-deficit area, sustained recovery and regret.
- Keep human safety, infrastructure service and ecological outcomes separate.

### T4 — value of information
- Ask whether another inspection, measurement, source or drill would change the portfolio decision.
- Stop gathering information when the expected decision value does not justify the cost.
- No adaptation to a new alleged threat without admitted evidence.

### T5 — independent case studies
- Start with documented fire/thermal, PNT loss, GMD/EMP continuity, shelter/healthcare continuity and MMOD/space-environment cases.
- Use public/authorized data and domain-appropriate independent review.
- Keep high-consequence, low-frequency scenarios distinct from common hazards.

## 7. What would count as beating the state of the art?

Not a higher model score on synthetic scenarios. We need matched comparisons that show at least one defensible improvement:
- lower lifecycle cost for the same hard protection/service floors;
- improved service continuity/recovery for the same lifecycle budget;
- lower residual tail risk without unacceptable nominal cost;
- more accurate prioritization under uncertainty and evidence update;
- better rank stability and fewer false confident recommendations;
- more independent replay and less evidence loss across the lifecycle.

Proposed targets (not achieved):
- 100% of funded portfolio decisions trace to admissible evidence and declared contexts.
- zero invented probabilities, unqualified damage claims or unsupported threat attributions.
- zero safety-floor compensation in multi-objective scoring.
- 100% reproducible frozen decision outputs from evidence, assumptions, cost model and method version.
- demonstrate measurable lifecycle-cost or service-resilience improvements on independently reviewed cases before claiming superiority.

## 8. Synthetic decision corpus

The frozen synthetic cases are in `research/protective-systems/fixtures/protective-threat-decision-cases-v0.1.json`. They are qualitative controls only; they contain no actual weapon performance parameters, event probabilities or damage calculations. The corpus includes explicit negative controls for unsupported exotic threats, unknown likelihood, absent asset exposure, double-counted multi-hazard benefits, multiplication of ordinal scores and safety-floor violations. `REJECT_INFEASIBLE` is a hard disposition, not a low ranking.

## 9. Safety and evidence boundary

This register does not contain weapon recipes, construction instructions, transmission parameters, target analysis, damage calculations or protection-defeat guidance. It does not authorize physical testing, procurement or human-subject research. The only positive result this portfolio can produce is a defensible *research/investment-priority disposition* for a declared asset context.

A theoretical possibility with no credible deployment path remains monitor-only. This keeps scientific curiosity alive without letting a dramatic hypothetical consume resources that could reduce real-world harm today.


## 10. Cross-owner resilience evidence bridge

The bridge contract and six synthetic conformance fixtures are now specified in:

- `PROTECTIVE_RESILIENCE_EVIDENCE_BRIDGE_V0_1.md`
- `fixtures/protective-resilience-bridge-cases-v0.1.json`

This is an adapter over CIVDYN-CASCADE (#3894), CIVDYN-RES (#3621/#3739), PROTECT-ROBUST (#5317), PROTECT-VOI (#5268) and CIVBENCH-MAD (#3731), not a duplicate simulator or optimizer. It keeps service, buffer, human-safety and ecological/physical outcomes separate; enforces no double-counting; and fails closed when owner evidence, comparable scope or probability support is absent.

NIST anchors include SP 1339 (final, 17 June 2026), SP 1800-45 (final, 24 June 2026), and SP 800-82 Rev. 4 (initial public draft, 21 September 2026; comments due 30 November 2026). Draft status and standards applicability must remain explicit.


## 11. Industry comparison and what we must prove

IN-CORE is an important external comparator. It already provides research-backed community resilience modelling that links infrastructure inventories, hazards, physical damage and functionality, repair/recovery/restoration, and social/economic impacts. The NIST Community Resilience Planning Guide already supports stakeholder-defined recovery-time performance goals. We must not claim these underlying capabilities as novel or position our current synthetic gate as a replacement.

Our research opportunity is narrower and potentially complementary: a common, fail-closed assurance layer that can consume outputs from specialist engineering/community models, preserve exact scenario and evidence lineage, enforce non-compensatory safety/service floors, keep service/buffer/ecological outcomes separate, reconcile shared benefits across a portfolio, and reproduce decisions across model owners.

ISO 22301 / South African SANS 22301 are business-continuity management-system comparators, not physical hazard simulators. The existence of a BCMS or certificate does not by itself establish the physical effectiveness of a protective assembly or a measured service-recovery trajectory. The South African Disaster Management Act 57 of 2002 and National Disaster Management Framework review (Government Gazette 48874, 30 June 2023) add a local multi-level planning context. Each real deployment must resolve local law, municipal/provincial frameworks, sector rules and exact SANS editions through competent local review.

External anchors:
- IN-CORE: https://www.nist.gov/publications/interdependent-networked-community-resilience-modeling-environment-core
- NIST Center of Excellence / service architecture: https://www.nist.gov/community-resilience/center-excellence
- NIST recovery-time planning guide: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.1190GB-16.pdf
- SABS SANS catalogue: https://www.sabs.co.za/sabs-standards
- South African Disaster Management Act: https://www.gov.za/documents/disaster-management-act
- National Disaster Management Framework review: https://www.gov.za/sites/default/files/gcis_document/202307/48874gon3635.pdf

**Beating the state of the art requires a matched result, not a novel architecture label.** On a rights-cleared independent case, compare persistence, a competent domain rule/operator plan, available external-model results (including IN-CORE only when scope-compatible), domain-specific Symtropy solvers, and the full portfolio bridge. Freeze boundary, service demand, scenarios, evidence, uncertainty, safety floors, time horizon and lifecycle cost before comparison.

Prospective materiality thresholds (not achieved): either at least 20% lower fully burdened lifecycle cost at unchanged mandatory safety/service floors, or at least 20% lower integrated critical-service deficit at equal lifecycle budget. The reviewer must first establish that the data and sample precision can resolve such effects. Otherwise publish confidence intervals and smallest detectable effects. Also require zero false-feasible decisions on the frozen negative-control set, full deterministic reproducibility, and disclosure of cases where the Luminous approach loses or abstains.


## 12. Evidence bridge E0 and updated SOTA crosswalk

The follow-on branch `research/protect-resilience-e0-oracle-v0-1` now contains the first exact E0 synthetic reference and independent Rust accounting oracle. It uses frozen service trajectories rather than creating another hazard or service simulator. The known-answer case checks exact per-stratum deficit/recovery metrics, matched scope, unique cost/benefit identities, separate buffer recovery and a mandatory safety-floor negative control.

See:
- `PROTECTIVE_RESILIENCE_EVIDENCE_BRIDGE_V0_1.md`
- `PROTECTIVE_RESILIENCE_SOTA_CROSSWALK_2026_11.md`
- `fixtures/protective-resilience-e0-reference-v0.1.json`
- `verifier/src/bin/protective_resilience_e0.rs`

The SOTA crosswalk incorporates the 7 November 2026 comparative paper by Shamsi, Helmrich and Deegan (DOI 10.1007/s10669-026-10162-2). It identifies a genuine integration/assurance opportunity while explicitly acknowledging existing IN-CORE, PRIIA, REFIT and other specialist tools. Luminous should compete on provenance-preserving interoperability, matched decision assurance, non-compensatory constraints, cost/benefit accounting, explicit abstention and practical operator usability—not by claiming to have invented community resilience modelling.

The SOTA paper's 1–5 ordinal tool-characterization ratings are not real-world risk values or a universal leaderboard. Any future claim of superiority still requires matched scopes, external reproduction, sufficient measurement precision and independent review.
