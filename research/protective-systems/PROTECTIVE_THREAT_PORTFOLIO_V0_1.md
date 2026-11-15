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

## 8. Safety and evidence boundary

This register does not contain weapon recipes, construction instructions, transmission parameters, target analysis, damage calculations or protection-defeat guidance. It does not authorize physical testing, procurement or human-subject research. The only positive result this portfolio can produce is a defensible *research/investment-priority disposition* for a declared asset context.

A theoretical possibility with no credible deployment path remains monitor-only. This keeps scientific curiosity alive without letting a dramatic hypothetical consume resources that could reduce real-world harm today.
