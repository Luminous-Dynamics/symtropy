# WATER-RECOVERY-VERIFICATION-001 — Independent Service Restoration Benchmark v0.1

**Tracking:** [#5069](https://github.com/Luminous-Dynamics/symtropy/issues/5069)  
**Composes with:** #4959 pump service, #5018 sparse-sensor control, #5027 community floors, #5045 observability qualification, #4963 leak diagnostics.

## Research basis and SOTA gap

Pérez-Pérez (15 October 2026), *Artificial Intelligence, Digital Twins, and Cyber-Resilience in Water Distribution Systems: A Critical Narrative Review with a WDT Benchmark Case Study*, maps 50 water cyber-physical studies and evaluates nominal-only detectors and alarm policies under a common chronological testbed protocol. In the evaluated records, cumulative alarm policies raise sample-level recall but sustain alarms and detect fewer event onsets than a fixed-threshold policy. The authors caution against generalising the numerical result beyond their protocol, and call for event performance, nominal alarm burden, causal localisation, independent validation and verified recovery.

Source: https://doi.org/10.3390/eng7100564

This makes a useful distinction: a system is not recovered because an alarm classifier stops firing, a pump restarts, or a repair ticket closes. Event-level and service-level outcomes must be proven separately.

## Paradigm shift: recovery as an evidence-gated state machine

Track:
1. disturbance onset and event lineage;
2. detection and safe degradation;
3. repair/intervention assertion;
4. sensor and telemetry recovery;
5. candidate service recovery;
6. per-area service-floor verification;
7. independent path corroboration;
8. stable dwell completion;
9. verified restoration and event closure;
10. recurrence and re-open.

Keep repair, alarm, hydraulic-service and water-quality claims distinct.

## Frozen T0 corpus

The companion schema, 13-case fixture and separate oracle use three abstract DMAs. The declared fixture profile is:
- delivered-demand fraction >= 0.80 per area;
- maximum observation age 600 seconds;
- minimum stable dwell 3,600 seconds;
- at least two independent measurement paths per area.

These constants are synthetic. They are not legal standards, Johannesburg Water operating limits or household service measurements.

Cases include closed-ticket/known-breach, one independent path, incomplete dwell, aggregate-masking, stale/missing/conflicting evidence, shared-gateway false independence, stale area binding, breach-plus-unknown, active alarm and recurrence after a previous restoration claim.

## Independence test

Two paths are independent only when:
- both have current evidence state;
- their source-family identities differ;
- they share no active failed domain across physical interface, calibration lineage, power, communications or clock.

Different serial numbers alone do not establish independence.

## Disposition semantics

At area level:
- `meets_floor`: valid, current, complete and fresh lower-bound observation >= floor, two independent paths, and stable dwell complete;
- `breached_floor`: valid, current, complete and fresh upper-bound observation < floor;
- `candidate_restored`: current lower bound >= floor, but corroboration, dwell or incident-clearance conditions remain incomplete;
- `indeterminate`: missing, stale, conflicting, incomplete or unbound evidence, or an interval that crosses the floor;
- `invalid`: unsupported policy, metric or unit contract.

Overall:
- `verified_restored`: every required area meets, no active alarm/conflict, and area/topology binding is current;
- `not_restored`: a known breach and no unknown area;
- `breached_with_indeterminate`: a known breach and a separate unknown coexist;
- `indeterminate`: no known breach but at least one unknown;
- `candidate_restored`: no breach/unknown, but one or more verification gates remain incomplete;
- `invalid`: a required record is structurally unsupported.

## Industry and benchmark position

Bentley OpenFlows WaterSight already advertises integrated SCADA, GIS, hydraulic models, IoT, AMI/CRM, real-time anomaly monitoring, burst geolocation, pump optimization and emergency response simulation. Luminous should interoperate with this operational stack, not claim to replace it. The differentiation hypothesis is evidence qualification at the restoration boundary: correct area/topology binding, independent measurements, freshness, stable dwell, recurrence lineage and reproducible receipts.

The 2026 WDT benchmark implies event-level measures alongside sample classification:
- event recall and missed event count;
- event-onset delay;
- false alarms per nominal sensor-day / DMA-month;
- alarm persistence and verified-clearance delay;
- time to candidate service recovery;
- time to independent verification;
- time below each required service floor;
- recurrence rate within predeclared post-restoration windows.

T0 evaluates semantics only; no time-series performance is measured.

## Proposed targets

Targets, not achieved:
- 100% frozen semantic known-answer agreement;
- zero verified-restored states with any required breach, unknown, stale, incomplete or unbound area;
- zero restoration claims sourced only from work-order closure, alarm clearance, controller reset or model estimate;
- zero independent-path credit for shared active failure domains;
- 100% preservation of breach-plus-unknown;
- at T1, seek >=10% reduction in median and P95 verified-restoration time against the strongest matched baseline, without more service-floor breaches, false restorations or false alarms.

## Claim ceiling

T0 validates only synthetic state-transition semantics. No hydraulic performance, repair effectiveness, real community service, potable-water quality, health safety, legal compliance or utility operating authority is established. Drinking-water quality remains a separate mandatory qualification for any potable-use assertion.
