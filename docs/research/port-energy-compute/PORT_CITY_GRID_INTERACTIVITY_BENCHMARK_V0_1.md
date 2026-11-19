# Port-City Grid-Interactivity and Multi-Timescale Envelope Benchmark v0.1

**Issue:** PORT-CITY-GRID-INTERACTIVITY-001 (#6289)  
**Snapshot:** 19 November 2026  
**Evidence class:** synthetic contract / event-observability fixture; owner-native limits not supplied.

## Why 15-minute scheduling is not enough

ENTSO-E's May 2026 report treats data centres as grid-significant loads. It notes that UPS-based architectures can abruptly disconnect large loads during grid disturbances, and that some fast behavior may not be visible to conventional SCADA sampling. It distinguishes workload flexibility, cooling flexibility and on-site generation rather than treating them as one resource. The report also warns that theoretical flexibility is filtered by equipment, business-model and market constraints.

ASHRAE's grid-interactive AI data-centre guidance distinguishes load-based demand response from inverter-based grid support. The first changes consumption; the second requires qualified bidirectional power conversion and applicable interconnection evidence. Neither should be confused with microgrid islanding or protection qualification.

Sources:
- ENTSO-E, *Data centres and the power system: expected trends, challenges, and opportunities* (May 2026): https://eepublicdownloads.blob.core.windows.net/public-cdn-container/clean-documents/Reports/2026/FINAL_ENTSO-E_Data_Centres_260430.pdf
- ASHRAE, AI Data Center Energy Performance Framework — Grid-Interactive Design: https://www.ashrae.org/technical-resources/topics-and-initiatives/ai-data-center/grid-interactive-design-demand-flexibility
- NREL, grid-forming inverter hardware test scenario research: https://www.nrel.gov/docs/fy25osti/91714.pdf

## Paradigm shift

A service plan is not qualified just because its 15-minute average is below a MW cap.

The plan must fit a **multi-timescale connection envelope** composed from exact owner evidence:
- firm and conditional/flexible import rights;
- per-interval W/Wh and point-of-connection boundaries;
- ramp and step-change limits;
- dynamic voltage/frequency/phase/harmonic envelopes where applicable;
- inverter-based support and export qualification;
- islanding, black-start, resynchronization and protection reports;
- measurement bandwidth, sampling and event-time integrity;
- service floors and rebound/restore limits.

The responsible grid, protection, converter, vessel or equipment owner supplies the relevant dynamic curve and method. This benchmark does not invent universal voltage, frequency, ramp or trip thresholds.

## Time hierarchy

| Layer | Coordination horizon | Owner boundary |
|---|---|---|
| Site investment / grid connection | Years to months | Utility, asset owner, Sol-Atlas evidence |
| Day-ahead service planning | Hours to days | Symtropy #4582, #6211 |
| Intraday event re-planning | 15 minutes down to seconds where supported | Symtropy #6255 and owner schedules |
| Converter/equipment behavior | Equipment-defined milliseconds or faster | Grid/electrical/OEM owner; Symthaea #11028 for vessel-side transient evidence |
| Protection/interlocks | Certified local response domain | Electrical/protection/OEM owner; independent from cloud |

These are coordination categories, not normative control-loop settings.

## Synthetic aliasing counterexample

The companion fixture uses two 2-second traces sampled at 1 ms. Both are embedded in a 15-minute planning interval.

- Trace A remains at 5.0 MW.
- Trace B rises from 5.0 MW to 5.8 MW over 100 ms, remains at 5.8 MW for 400 ms, returns over 100 ms and stays at 5.0 MW.

Trace B's embedded 15-minute average is approximately 5.000444 MW, only 0.444 kW above Trace A, while its fast peak is 5.8 MW and its maximum ramp is 8 MW/s. The synthetic fixture-only event envelope is 0.5 MW maximum excursion and 5 MW/s maximum ramp, so Trace B fails that illustrative envelope. These are synthetic test values, not grid-code or equipment limits.

The result is narrow and important: **a long-interval energy schedule cannot establish fast electrical equivalence**. The test is an observability/contract check, not a transient-stability simulation.

## Separate products and rights

1. **Firm import capacity:** utility/owner-confirmed import envelope with exact PCC/feeder/asset scope and validity.
2. **Conditional/flexible connection:** additional import entitlement only under an exact current, revocable or expiring grant, with notice, duration, call count, response, service floors, compensation/settlement and rebound terms. Conditional capacity is not firm capacity.
3. **Demand response:** workload, cooling or other load changes under a service contract. Record response time, duration, available MW, energy, rebound and non-delivery.
4. **Inverter/grid support:** voltage/frequency, reactive power, export, ride-through or other services from qualified electrical resources. Demand response does not prove these capabilities.
5. **Island / black start / reconnection:** separately qualified control/protection and local authority; no cloud scheduler or Symthaea action can substitute for those functions.

## South African demand-response product references

The paired-site work should benchmark actual local programme profiles, not import generic demand-response times from another market. Public programme descriptions currently distinguish at least three products:

### Eskom Distribution Instantaneous Demand Response

The public Eskom programme page states:
- 10 MW minimum entry;
- full reduction within 6 seconds after notification;
- sustain for 10 minutes;
- up to three 10-minute events per day;
- day-ahead scheduling and event dispatch when frequency reaches a preset level;
- special metering requirements;
- standby payment and no energy payment.

Source: [Eskom Instantaneous Demand Response Programme](https://www.eskom.co.za/distribution/instantaneous-demand-response-programme/).

These are programme-page terms, not proof of eligibility or a current offer to a specific port. Confirm aggregation, participant qualification, baseline, telemetry, site connection, current contractual terms and settlement directly with Eskom.

### SAWEM instantaneous reserve definition

The NTCSA SAWEM Market Code Rev 2.3 (February 2026) defines Instantaneous Reserve as generation or demand-side managed load available to respond fully within 10 seconds to a frequency drop and sustain the response for at least 10 minutes. It includes day-ahead reserve and real-time dispatch/settlement provisions.

Sources: [NTCSA Market Code Rev 2.3 (February 2026)](https://www.ntcsa.co.za/wp-content/uploads/2026/02/Market-Code-v2.3.pdf), especially the definition of Instantaneous Reserve and reserve capacity/dispatch sections.

This is a different profile from Eskom Distribution's six-second IDR programme. Do not combine them into a single product or silently substitute one response deadline for the other. Market Code publication and reserve definitions do not establish that an individual port or data centre is registered, qualified or eligible to participate.

NERSA's September 2026 regulatory tracker (last updated 15 September) listed approval of the Market Code as in finalisation, with an end-October target. That dated tracker does not establish the subsequent final regulatory decision; confirm current approval and implementation status before making a participation or settlement claim. [NERSA Electricity Regulation Projects Dashboard, September 2026](https://www.nersa.org.za/files/files/2026/09/ELRStakeholderDashboard-September2026-Rev1-HighRes.pdf).

### NTCSA Supplemental Demand Response

The NTCSA public programme page describes:
- notice period from 30 minutes to 6 hours;
- minimum entry equal to the greater of 500 kW or 15% of average plant demand;
- minimum two-hour response duration, once per day under the published description;
- 30-minute interval remote metering as a prerequisite;
- standby and energy payments.

Source: [NTCSA Supplemental Demand Response Programme](https://www.ntcsa.co.za/supplemental-demand-response-programme-2/).

This slower product is materially different from instantaneous reserve. A 30-minute meter can support the published programme's metering requirement, but cannot alone prove a six-second IDR response. Use the exact programme's required telemetry, baseline and verification rules.

### Emergency load reduction is a separate category

NERSA-approved NRS 048-9 addresses emergency load reduction, including load shedding/curtailment and restoration. Mandatory emergency reduction under System Operator/utility authority must not be represented as voluntary contracted demand response or an optimizer's discretionary dispatch right. [NRS 048-9 Edition 3](https://www.kaya959.co.za/wp-content/uploads/2024/04/NERSA-Approved-Version-of-NRS-048-9-2023-Edition-3.pdf).

### Implications for Ngqura and Durban

- The published 10 MW Eskom IDR minimum is large relative to a single 2.3 MW demonstration module and cannot be assumed to be met by the site. Portfolio aggregation may be possible only if the programme/aggregator rules and electrical boundaries permit it.
- Ngqura's published 17.1 MW estimated spare substation capacity and Durban's 3.6 MW estimate are pre-feasibility grid-capacity figures, **not** available flexible load or contracted response. Do not subtract or equate them to IDR qualification.
- The supplemental 500 kW / 15% threshold may be a more accessible starting product for a campus or aggregated port load, but average demand, baseline rules, 30-minute metering, curtailment rights, response duration and actual eligible load must be evidenced.
- The six-second programme makes local control and telemetry requirements materially different from 15-minute scheduling. Symtropy can evaluate the cross-timescale contract and audit evidence, but the grid operator and electrical/protection owners qualify the response.

## Ownership and non-duplication

- Symtropy #4582 remains canonical for port/PCC simultaneous W, cumulative Wh, connection readiness and schedule verification.
- Symtropy #6211 remains canonical for compute job identity, useful work, objective-specific shifting, no-new-peak admission and rebound.
- Symtropy #3418 owns grid-forming, black-start, islanding, protection and microgrid resilience qualification.
- Symtropy #3400 owns the energy-system model/co-simulation interface and validity boundaries.
- Symtropy #4448 owns ship/shore interface, compatibility and commissioning/readiness evidence.
- Symtropy #6255 owns read-only multi-domain event triggers.
- Symthaea #11028 owns vessel-side multi-timescale transient/power-quality envelope; #11572 owns shore-power service.
- Mycelix #5974/#6343 owns authority, provenance and service-equivalence handoff.
- Sol-Atlas #654/#948 owns site, marine and source-quality evidence.

The integration issue creates no second PCC verifier, scheduler, inverter model, transient solver, protection engine or shore-power commissioning system.

## Comparator protocol

On identical load, service, grid and event traces compare:
- firm-connection baseline;
- conditional/flexible connection contract;
- workload-only demand response;
- storage-only response where owner-qualified;
- combined workload + cooling + storage response;
- grid-forming/island-capable microgrid using #3418 owner evidence;
- perfect-foresight scheduling as a labelled non-deployable upper bound.

Preserve vessel service, terminal throughput, compute useful work, cooling and water obligations. Use the same PCC boundary, forecast vintage, outage/event and evidence cut-off.

## Scorecard — never collapse into one scalar

**Slow energy/service:** PCC peak, import MWh, ramp, cost, carbon, renewable curtailment, storage throughput/loss, vessel/terminal service, compute useful work/deadline and cooling/water service.

**Fast electrical:** event peak/duration, step/ramp, voltage/frequency excursions, phase imbalance, harmonic distortion, current limit, UPS/converter trip, ride-through and recovery. Only the qualified electrical owner may issue pass/fail against real limits.

**Flexibility:** firm vs conditional MW, notice, response latency, response duration, maximum calls, rebound peak, non-delivery and service impact.

**Resilience:** grid/fibre/cooling faults, islanded service, black start, resynchronization, reconnection and restoration time.

**Evidence:** waveform coverage, sample rate, event observability, clock uncertainty, calibration, freshness, model/configuration and unknown disposition.

## Acceptance targets — hypotheses

- Zero cases accepted solely on 15-minute averages where a fast-envelope report is required.
- Zero conditional MW counted as firm MW.
- Zero demand-response claim promoted to inverter grid-support qualification.
- 100% of fast-event records bind exact configuration, sample rate, clock uncertainty, calibration and raw-data digest.
- Missing samples, downsampled event erasure or time uncertainty produce DATA_INCOMPLETE, never pass.
- 100% accepted service schedules preserve service floors and report the full recovery/rebound horizon.
- Local protection and interlocks remain independent under cloud/federation/optimizer loss.

## Claim ceiling

The synthetic fixture demonstrates averaging/aliasing and contract distinctions only. It is not an AC power-flow or transient-stability model, grid-code assessment, converter/UPS test, protection-coordination study, black-start qualification, islanding test, shore-power commissioning or authorization to dispatch.
