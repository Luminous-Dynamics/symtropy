# EV-FACTORY-AUTOMATION-001 — Manual, assisted and automated station economics v0.1

**Research date:** 20 October 2026  
**CLI:** `tools/ev-passenger-cost-frontier/src/bin/ev_automation_frontier.rs`  
**Fixture:** `tools/ev-passenger-cost-frontier/fixtures/ev_automation_breakeven_v0_1.json`  
**Input discipline:** all numbers in the sample fixture are synthetic sensitivities. No equipment vendor, integrator, plant, supplier, landlord or agency was contacted; no quotation was requested.

## Paradigm shift: automate a constraint, not a factory slogan

There is no universal answer to “manual or automated?” The right choice changes with wage, volume, cycle time, product variation, quality loss, energy, maintenance capability, line reliability and financing. The programme should choose *per station and per demand case*. Manual/assistive operations may be cheaper at launch or under variable demand; automation may pay back where work is repetitive, safety/ergonomics justify it, and demand is stable. Automation can also increase integration risk, maintenance skill requirements, spare-part dependence and downtime.

Published research on automotive reconfigurable lines treats module choice and reconfiguration policy as part of the design because demand, product variants and batch sizes change. A separate peer-reviewed study explicitly models the economics of manual versus automation-assisted kitting using capital, labor, space and quality costs and finds that break-even can be calculated for a given application. Our method builds on those decision structures; it does not reproduce or claim results from their case studies:
- Colledani et al., *Design and management of reconfigurable assembly lines in the automotive industry*: https://doi.org/10.1016/j.cirp.2016.04.123
- D'Antonio et al., *A model for planning and economic comparison of manual and automated kitting systems*: https://doi.org/10.1080/00207543.2020.1711985

## Implementation

The deterministic Rust CLI enumerates 3 illustrative regional packs × 4 annual output levels × 3 station families × 3 options = **108 site/family/volume/option scenarios**.

Station families:
- **Inbound kitting:** manual pick/scan; assistive guided picking; automated pick-to-light/material presentation.
- **Battery/HV installation:** manual controlled lift/torque; guided lift and digital torque interlocks; automated placement/fastening.
- **End-of-line test:** technician-led sequence; semi-automatic sequence/data capture; automated test and trace-linked diagnosis.

The included station values—cycle, cell capital, integration capital, positions per shift, area, electrical and thermal energy, station yield, failed-attempt cost, rework and maintenance rate—are invented sensitivities used only to exercise the equations. The regional FX, labor, utility and lease values are synthetic too.

For each option and target annual good-unit output, the model reports:
- production attempts required after the option's stated good-yield fraction;
- net available minutes per cell/year and the number of parallel cells needed;
- attempted and good-unit capacity, headroom and calculated utilization;
- staffing and paid operator hours; equipment/integration capital; footprint;
- annual labor, electricity, useful thermal energy, lease, maintenance, expected failure and rework costs;
- annualized station capital, total annualized station cost, and station cost per demanded good output;
- annual operating savings against the manual baseline, incremental capital, simple payback (where defined) and discounted NPV over the declared horizon.

Each option is evaluated at the same target good output for the same station/site. Required parallel cells are recalculated for that option, rather than comparing one manual workstation with a robot assumed to be infinitely fast. If the automated option is faster, it may need fewer cells; if quality yield is lower, it needs more attempts and bears more expected failure cost.

## Cost equation and boundaries

Let (Q) be target good station output/year, (y) option yield, (A) attempts, (m) cycle minutes/attempt, (T) available productive minutes/cell/year, (n) installed parallel cells, (C) installed station capital, (r) discount rate, (N) evaluation years.

- Attempts: (A=\lceil Q/y \rceil).
- Available productive minutes reflect days, shifts, paid time less unpaid breaks, availability and performance.
- Cells required: (n=\max(1,\lceil A m/T \rceil)).
- Installed capital: equipment cost per cell × cells + one-time integration/commissioning cost.
- Annual operating cost = loaded labor + electricity + useful thermal energy + leased footprint + equipment maintenance + expected failed-attempt cost + expected rework cost.
- Annualized station cost = annual operating cost + installed capital / declared straight-line asset life.
- Station unit cost = annualized station cost / target good output.
- Simple payback is defined only if annual operating savings are positive and capital delta is positive; if capital decreases and operating cost improves, payback is reported as zero. If operating savings are non-positive, simple payback is undefined—not invented.
- NPV of candidate vs manual = (manual annual operating cost − candidate annual operating cost) × discounted annuity factor − (candidate installed capital − manual installed capital). A positive number means the candidate has lower discounted cost under the stated inputs; it is not a financial forecast.

This v0.1 is intentionally limited to **one station family in isolation**. It does not model the entire production line's queueing, upstream/downstream synchronization, robot safety-rated control systems, depreciation/tax, financing/working capital, maintenance spare-parts supply, integration risk, changeovers, multiple product mix, scrap value or an option-combination optimizer. In particular, the station yield is a declared option property, not the validated full-vehicle yield. Its output should narrow pilots, not make a factory-investment decision.

## What the external benchmark literature tells us

- **Kitting is a real trade-off, not a reflexive automation target.** Peer-reviewed work compares manual/robotized parts retrieval using capital, labor, space and quality, and explicitly analyzes break-even; test it for the line's actual kit complexity, SKU count, replenishment route and demand. Source: https://doi.org/10.1080/00207543.2020.1711985
- **Reconfigurability matters.** Modular line research addresses demand and variant changes. Compare fixed dedicated automation with modular cells, flexible fixtures, programmable tools and duplicated manual/assistive cells at the same output and product scope. Source: https://doi.org/10.1016/j.cirp.2016.04.123
- **Measure, don't optimize to one headline KPI.** NIST emphasizes defined metrics and systems characterization; production throughput, availability, quality, maintainability and sustainability are related and can move against one another. Use a shared data boundary so the station and facility are measured against the same output basis. Source: https://www.nist.gov/programs-projects/operations-driven-performance-measurement-smart-manufacturing-systems
- **Utilities belong in the same decision.** DOE guidance states compressed-air systems are inherently inefficient, with more than 80% of input energy lost as heat in typical systems; it recommends eliminating inappropriate compressed-air uses, reducing pressure, controlling storage/supply and fixing leaks. Prefer direct electric actuation where suitable, but only after pressure, safety, ergonomics, tool requirements and serviceability are considered. Source: https://betterbuildingssolutioncenter.energy.gov/better-plants/compressed-air
- **Automated tests do not waive safety.** The battery/HV fixture options remain blocked until station hazard controls, safe states, tooling and emergency procedures receive appropriate independent validation. A spreadsheet may compare costs; it cannot certify a battery installation or EOL safety test.

## Suggested acceptance benchmarks

These are internal targets for the next measured pilot, not claims about an industry's universal best performance:

1. **Comparable options:** manual, assisted and automated alternatives meet identical output, product-quality and required test coverage. No mode can claim an advantage by dropping required inspections.
2. **Capacity:** every option's installed cells can meet planned attempts, with a provisional design headroom target of at least 15% at the intended sustained output. Run a sensitivity for degraded availability and higher rework before finalizing capacity.
3. **Economic case:** report incremental capex, operating savings, simple payback, discounted NPV and cost/good output. Require positive NPV under the conservative measured-input case before a positive financial recommendation; do not substitute nominal robot speed for payback.
4. **Quality:** compare first-pass yield, escape/false-pass risk, rework and repair hours using the same sampling/test boundary. Safety-critical defects are a hard stop, not an average cost term.
5. **Ergonomics and safe operation:** verify lifting/reach/torque hazards and access/lockout in the actual cell. A lower labor count is not sufficient justification if the residual risk is unacceptable.
6. **Robustness:** evaluate low/base/high wage and demand, utility price, asset life, uptime, yield and maintenance-spares lead time; report the frontier where manual, assisted and automated alternatives switch rank.
7. **Global repeatability:** share station schema, interface and calculation engine globally; localize the wage, utility, labor law, electrical/network, equipment support, machine safety and process approvals with dated evidence. Unknown local compliance prevents release.

## Next evolution

- **v0.2:** time distributions / variability, variant changeover, MTBF/MTTR, spare-part lead time, tool calibration, integration contingency and working-capital impacts.
- **v0.3:** discrete-event simulation of queues, buffers, starved/blocked stations, quality holds, rework loops and combined station-option sets; identify the *system* bottleneck.
- **v0.4:** connect the optimization to observed pilot-cycle/defect/energy records, keeping calibration data separate from held-out validation data.
- **v0.5:** multi-site Pareto optimization using the global factory core plus regional compliance/economic packs.

NIST's digital-twin programme cautions that validated models require common vocabularies/interfaces plus verification, validation and quantified uncertainty. Thus a global solver should share process interfaces and calculation rules, but never copy local legal/safety approvals or treat uncalibrated predictions as plant evidence: https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing

## Claim ceiling

This tool is a station-level screening model. Its synthetic result may justify measuring a specific process, but not ordering equipment, claiming SOTA, certifying safety or underwriting a factory. Unknown sources, taxes and local approvals remain unknown; no vendors or agencies will be contacted to fill evidence gaps.
