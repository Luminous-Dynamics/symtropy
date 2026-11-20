# EV-CARRIER-LIFECYCLE-001 — Module serviceability and repairability screen v0.1

**Implementation:** `tools/ev-passenger-cost-frontier/src/carrier_lifecycle.rs`  
**CLI:** `ev_carrier_lifecycle_screen`  
**Factory input:** `EV-CARRIER-MODULE-MFG-001` report  
**Lifecycle input:** `ev_carrier_lifecycle_profiles_v0_1.json`  
**Parent:** [EV-CARRIER-REF-001 #6109](https://github.com/Luminous-Dynamics/symtropy/issues/6109)  
**Specification:** [EV-CARRIER-LIFECYCLE-001 #6321](https://github.com/Luminous-Dynamics/symtropy/issues/6321)

## Purpose and claim boundary

This overlay answers a different question from factory process capacity:

> Once a carrier module is installed, how much technician effort, downtime, direct parts/labor cost and qualification evidence does it require to inspect, diagnose, repair, replace and return to service?

It consumes the factory module report; it does not recalculate factory yield, takt, capacity, equipment, floor area, labour positions or manufacturing cost. It does not create a maintenance ledger, asset registry, battery passport, reliability database or release authority.

The result is an annualized **per-module service-burden screen**. It is not a whole-life cost model or fleet-availability estimator. Whole-life cost ranking is always false in v0.1 because acquisition, warranty, spare pooling, supplier lead time, unplanned failure exposure, battery degradation, residual value, end-of-life and base vehicle service boundaries are not yet complete.

## Paradigm shift: design for restoration and reconfiguration

The product should be designed around the ability to restore useful service, not simply the lowest first-build mass or the quickest initial installation.

### 1. Make module interfaces reversible

A removable module must earn its modularity claim. Include:
- structural datums, retention and fastener access;
- HV/LV isolation, stored-energy discharge and safe verification;
- cooling, fire containment, sealing and drainage interfaces;
- connector mating cycles, protection, EMC and calibration dependencies;
- service access, lift/handling requirements and tool commonality;
- configuration identity, effectivity and post-installation test coverage.

A detachable pod is not automatically more maintainable than an integrated bay. More connectors and joints can increase inspection burden, faults, sealing risk and configuration errors.

### 2. Optimize time-to-restoration, not just time-to-swap

Measure each phase separately: isolation, access/disassembly, diagnosis, removal, repair or replacement, reinstallation/configuration, post-maintenance test and return-to-service disposition. A “10-minute swap” is not a useful metric if isolation, lifting, calibration, inspection and a failed test add hours.

### 3. Treat the module as a lifecycle option

Compare repair, replacement, remanufacture and repurpose separately. A module that can be transferred between carriers may reduce spare-pool needs and fleet downtime, but only if physical compatibility, configuration correctness, parts availability and repeated reuse are demonstrated.

### 4. Make maintenance evidence-carrying

Every performed phase needs a source reference, measurement method, evidence class, elapsed duration, crew size and skill code. A not-applicable phase requires an explicit reason. An unknown phase is not zero minutes.

The module evaluator does not create asset identity. Reuse Mycelix Mobility Commons #3709 for serial/configuration lineage and Battery Passport #6062 / refresh #6076 for applicable passport projections. The responsible economic operator retains any regulatory update responsibility.

### 5. Separate economics from authority

Symthaea can estimate degradation, predict service demand and compare repair-versus-replace counterfactuals. It does not approve maintenance. Mycelix can carry custody and asset events. It does not release a vehicle or aircraft. Return-to-service is an evidence record of an authorized human/organizational disposition; this evaluator grants no authority.

## Architecture coverage

The current factory/lifecycle matrix covers:
1. **Truck-only baseline:** zero incremental optional-module service burden only. Base truck body, driveline and traction-battery maintenance are excluded and remain with EV-BENCH-001 #2164.
2. **Detachable UAV pod:** synthetic time/labor/parts example; unplanned event rate deliberately unknown.
3. **AMR handoff module:** synthetic time/labor/parts example; unplanned event rate deliberately unknown.
4. **Integrated enclosed drone bay:** production-intent service data unknown.
5. **Mobile drone-station trailer:** production-intent service data unknown.

The module factory matrix now has four optional module configurations (pod, AMR, integrated bay and mobile trailer). Across 36 site/template/volume combinations this produces 144 factory results. The lifecycle overlay emits 144 matched module results plus 36 truck-only zero-incremental-module controls, for **180 lifecycle rows**.

No trailer manufacturing or service performance is assumed. Its structural, axle/brake, coupling, electrical, charging and service process data remain unknown pending a production-intent design and study.

## Service event and task contract

Each module profile contains declared currency and cost basis year, labour cost per hour, annual operating hours per module, evidence reference and event records.

A service event has:
- unique event identity;
- planned or unplanned event class;
- inspection, repair, replacement, remanufacture or repurpose treatment;
- explicit occurrences per module-year, or null where unknown;
- occurrence evidence class, reference and basis;
- observed event count and module-hour exposure when a measured/independent unplanned rate is used;
- parts/material cost per occurrence;
- exactly eight ordered task phases.

The required phases are:

| Sequence | Phase | What it must capture |
|---:|---|---|
| 1 | Safe isolation | de-energize, discharge, verify, lockout and safety preconditions |
| 2 | Access/disassembly | access panels, lifting, removal sequence and special tooling |
| 3 | Diagnose/inspect | fault isolation, condition inspection and measurements |
| 4 | Remove | module/part removal and identity/effectivity confirmation |
| 5 | Repair/replace | repair, replacement, remanufacture or repurpose action |
| 6 | Reinstall/configure | reconnect, torque/retention, firmware/configuration and calibration |
| 7 | Post-maintenance test | required inspection/test, outcome and evidence |
| 8 | Return-to-service disposition | recorded authorized-personnel disposition, separate from the evaluator |

Each phase is Performed, Not Applicable, Unknown or Blocked. Performed phases require elapsed minutes, crew count, skill code, method and evidence reference. Not Applicable requires a reason. Unknown and Blocked phases cannot carry fabricated durations.

A release disposition without a passing post-maintenance test and complete phase evidence is a negative control and blocks the screen. The report's `return_to_service_evidence_complete` is not authorization; it only states whether the supplied record is complete.

## Event-frequency and cost semantics

Planned event intervals/counts must be explicit. An unplanned failure rate must never be inferred from a synthetic task fixture. For measured or independently tested unplanned rates, the supplied annual rate must reconcile with observed event count, module-hour exposure and annual operating hours.

For each event, the evaluator reports:
- elapsed downtime minutes per event;
- technician-hours per event (elapsed task duration × crew size);
- labour cost per event;
- parts/material cost per event;
- direct cost per event;
- annualized downtime, technician-hours and direct service cost only when event frequency and all required inputs are complete.

Planned and unplanned annual totals are separate. If the unplanned frequency is unknown, the planned burden can still be shown, but total annual service cost, total downtime and downtime fraction remain null. Null means unknown, not zero.

The currency and cost basis year are explicit. No exchange rate, inflation, discount rate, warranty cost, residual value or salvage credit is fabricated.

The single-module downtime fraction, when complete, is the additive sum of event downtime divided by annual operating hours. It assumes no overlapping events and no spare-module substitution. It is not fleet availability, service level, route completion or asset dispatch probability.

## Manufacturing and logistics composition

- EV-CARRIER-MFG-001 #6176 / PR #6193 supplies factory module configuration, process yield, capacity margin, station resources and base factory evidence gaps.
- EV-FACTORY-DESIGN-001 #2354 / PR #2355 owns shared factory architecture, sites, shifts, general cost and production economics.
- EV-BENCH-001 #2164 owns matched EV lifecycle, battery repairability, fleet-service and robustness comparisons.
- AERO-SERVICE-LOOP-001 #4056 owns generic service-task semantics and maintainability evidence vocabulary. This EV overlay is a domain adapter, not a second maintenance ledger.
- MFB-SOTA-REPRO-001 #6247 owns published truck–drone benchmark reproduction and algorithm tournaments.
- MFB cargo #6147 and co-availability #6083 own cargo/load/release and carrier/UAV temporal feasibility.
- Mycelix Mobility Commons #3709 and Battery Passport #6062 / #6076 own physical identity, lineage and passport projections.

The factory overlay's good-module yield/capacity is carried into the service result as context. The lifecycle evaluator does not multiply module manufacturing cost into service cost or double-count the factory's shared overhead.

## Research and industry comparison

### Commercial drone logistics is already operating at meaningful scale

On 5 November 2026, Walmart reported more than **2 million Zipline deliveries** and an agreement to add 300 sites. Zipline reported 150 million commercial autonomous miles flown. This is a mature fixed-network service benchmark for delivery reliability, turnaround, support and scale. It is not a direct comparator for an unqualified truck-mounted pod, and it does not validate our concept. Source: https://corporate.walmart.com/news/2026/11/walmart-expands-drone-delivery-network-with-zipline.

### EV truck manufacturing is a scale and service-network competition

IEA's Global EV Outlook 2026 reports around **440,000 electric trucks produced in 2025**, more than twice the prior year, with over 90% produced in China. Around 3% of global electric truck production was traded. The IEA also notes Daimler and Volvo can assemble battery-electric trucks on the same lines as diesel models. Local production, warranty handling and service coverage therefore matter alongside factory cost. Source: https://www.iea.org/reports/global-ev-outlook-2026/manufacturing-and-trade.

Volvo's 2026 investor presentation describes a common truck platform and a plant where combustion, battery-electric and fuel-cell variants feed a shared main assembly line through separate pre-assembly streams. This is the right manufacturing comparator: flexible common assembly with product-specific preparation, not a bespoke line for every variant. Source: https://www.volvogroup.com/content/dam/volvo-group/markets/master/investors/260327-volvo-group-business-update-post-2025-q4.pdf.

### Battery integration creates a serviceability frontier

A 2026 Nature perspective on structural battery integration identifies the trade-off between packaging/structural efficiency and diagnostic access, intervention granularity and material recovery. It argues that lifecycle-level repairability and disassembly must be evaluated alongside energy density and manufacturing cost. Source: https://www.nature.com/articles/s44359-026-00185-y.

A 2026 open-access parametric battery-pack LCA study found that design-for-disassembly and direct-recycling pathway choices materially affect environmental outcomes. Its reported ranges are study-specific and must not be transferred as a universal saving. Source: https://doi.org/10.1016/j.resconrec.2026.108930.

### Battery passport rules reinforce the need for lineage

The European Commission states that relevant EV, LMT and industrial batteries over 2 kWh placed on the EU market require a Battery Passport from 18 February 2027. Its FAQ distinguishes simple repair from remanufacture/repurposing and describes when a new passport and predecessor linkage are needed. Applicability depends on the actual battery category and operator role; the EV lifecycle evaluator does not decide legal applicability. Sources:
- https://single-market-economy.ec.europa.eu/single-market/digital-product-passport/batteries_en
- https://single-market-economy.ec.europa.eu/single-market/digital-product-passport/eu-digital-product-passport-faq-batteries_en

## Benchmark tournament and targets

### Correctness / evidence gates
- 100% of events contain exactly eight ordered task phases.
- 100% of performed phases include time, crew, skill, measurement method and source reference.
- 100% of Not Applicable phases carry an explicit reason.
- Unknown durations, parts costs and event rates never become zero.
- Measured/independent unplanned rates reconcile with observed counts and exposure.
- 100% deterministic output under factory/profile/event input-order permutations.
- Zero return-to-service evidence completion when the post-maintenance test fails, is missing, stale or is mismatched.
- Zero return-to-service evidence completion when an earlier required phase is unknown or blocked.
- No whole-life ranking while a candidate's failure exposure, cost or maintenance evidence is incomplete.

### Maintainability performance
Once physical articles and measured trials exist, compare p50/p90/p95 for diagnosis, isolation, removal, repair, replacement, reinstallation, post-test and return-to-service. Report sample counts, technician skills, facilities, ambient conditions, censoring and confidence intervals. Do not calculate tail percentiles from a single synthetic task value.

For a detachable pod to earn a modularity claim, target:
- p95 removal + reinstallation + verification time no more than half that of a matched integrated-bay intervention for equivalent maintenance scope;
- 100% declared carrier compatibility with no unverified adapter, wiring or configuration change;
- no increase in false-install or post-test failure rate relative to the matched fixed installation;
- zero return-to-service evidence false positives in the locked fault corpus;
- lower total service cost per module-year only after unplanned event rates, spares and field exposure are measured.

The 50% time target is a proposed internal design objective, not a reported industry benchmark. It must be tested against matched work scopes and the same technician/tooling conditions.

### Whole-life economics
Do not publish a total-cost winner until all candidates have:
- equivalent mission and utilization boundary;
- compatible base vehicle maintenance treatment;
- measured/qualified event frequency and task times;
- sourced parts, labour, tooling and logistics costs;
- spare-pool and substitution assumptions;
- module manufacturing cost/yield from the factory owner;
- warranty, refurbishment, remanufacture and end-of-life boundaries.

Report both cost per module-year and cost per successful useful-service unit when the service owner supplies the denominator. Keep annual technician-hours, downtime hours, service events, replacement counts and customer-service losses distinct.

## Next increments

1. Admit real inspection/service task cards and physical-artifact time studies for pod, AMR and integrated-bay designs.
2. Add configuration/serial applicability and event lineage by consuming Mycelix Mobility Commons; do not create a parallel asset registry.
3. Add spare-pool and cross-carrier interchange scenarios, with explicit lead times, compatibility and double-booking controls.
4. Extend the module economics boundary to acquisition, warranty, remanufacture and EOL only when source evidence is available.
5. Feed measured condition and service histories to Symthaea for predictive maintenance and repair-versus-replace counterfactuals; retain independent hard gates and human authorization.
6. Couple maintenance downtime and spare availability into the shared-aircraft logistics oracle (#6219) only after the physical fleet-resource owner provides the required availability/occupancy contract.
7. Link battery replacement/repurpose/remanufacture evidence to the Mycelix Battery Passport projection; never assume every drone pack falls into a specific statutory battery category.

## Non-claims

All populated task times, labour rates and parts prices in the initial pod/AMR examples are synthetic. The integrated-bay and mobile-trailer service profiles remain unknown. The truck-only control reports zero incremental optional-module burden only; it does not represent zero vehicle maintenance.

No result establishes actual maintenance rates, field reliability, fleet uptime, supplier cost, warranty, aircraft airworthiness, road homologation, production readiness, battery passport applicability or commercial savings. No maintenance release or actuator command is emitted.
