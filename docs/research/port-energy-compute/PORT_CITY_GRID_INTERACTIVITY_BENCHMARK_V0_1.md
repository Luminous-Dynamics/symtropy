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
