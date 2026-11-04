# Grid-Service Response Deliverability Protocol

**Protocol:** GRID-RESPONSE-DELIVERY-001  
**Version:** 0.1  
**Status:** research verifier; synthetic known-truth fixtures only

## 1. Why a separate verifier is needed

The existing flexibility-event evaluator measures event-average PCC import reduction, total facility energy, completed work, energy per work, latency/deadlines, thermal limits and post-event rebound over a matched horizon. Those remain important, and this protocol does not duplicate them.

The new grid-response verifier qualifies the *dispatch signal itself*: command-to-threshold latency, sample-level delivery, sustained tracking, under-delivery energy, overshoot energy and measured ramp at the declared telemetry resolution.

A data centre is not a dispatchable grid resource merely because a workload is theoretically deferrable or a battery is installed. It is a deliverable resource only when a product-specific request can be met at a declared point of connection, with known notice, response time, ramp, duration, recovery, locational limits and non-performance risk.

## 2. Research and industry evidence

- **Inter-data-centre field demonstration (ACM Sustainability Week Companion '26, 22 June 2026).** Measures response time from a migration/switch command until power change stabilizes. The study finds only pre-launched read-only data access patterns meet the strictest evaluated Japan balancing response class in both directions, while several patterns satisfy minute-level classes. This is a strong field comparator showing that migration/data semantics change response eligibility; the result is specific to its workload and infrastructure. DOI: [10.1145/3765611.3815063](https://doi.org/10.1145/3765611.3815063).
- **LAD-Flex with Alibaba cluster traces (May 2026).** Reports that around 20% of data-centre power is associated with tasks suitable for shifting and up to 22% load shedding during flexibility windows in its one-week case study. This is trace-based workload classification and power modeling, not a universal measured operating guarantee. DOI: [10.1016/j.ijepes.2026.111940](https://doi.org/10.1016/j.ijepes.2026.111940).
- **IEA 4E Data Centres and Flexibility (1 July 2026).** Distinguishes market-serving, grid-serving and system-serving flexibility, and identifies workload flexibility, supporting infrastructure and additional flexibility assets as separate mechanisms. The protocol therefore pins each product contract instead of imposing one universal latency or ramp number. [IEA 4E report](https://www.iea-4e.org/edna/publications/data-centres-and-flexibility/).
- **IEA Powering Artificial Intelligence (14 October 2026).** Surveys policy, regulation and company strategy across 34 policy jurisdictions representing 90% of global data-centre capacity, highlighting electricity-system integration, local costs, emissions and water. The operating context varies by jurisdiction and site. [IEA report](https://www.iea.org/reports/powering-artificial-intelligence).
- **Rebound-aware demand-response accounting (2026).** A ComStock-based evaluation across 3,960 building-event-method cases finds gross-as-net reporting risk in 70.9% of cases under its materiality definition, but rebound-aware net reporting was not uniformly more accurate. This supports keeping event delivery and post-event rebound as separate, auditable quantities rather than assuming a universal rebound correction. [DOI 10.3390/su18147154](https://doi.org/10.3390/su18147154).
- **LBNL / DOE demand-flexibility performance assessment.** Establishes the importance of the timing, location, quantity and quality of a grid service and notes the need for stronger measurement, telemetry and interoperability as services move from discrete load shedding toward continuous modulation. [LBNL report](https://emp.lbl.gov/publications/performance-assessments-demand).

## 3. Product-specific contract

Every test freezes:

- service product identifier and event identifier;
- point-of-connection / measurement boundary;
- baseline method identifier and response configuration digest;
- instruction issuance time and service end time;
- requested reduction in kW;
- telemetry interval;
- minimum delivered fraction at an interval;
- maximum command-to-threshold latency;
- minimum fraction of compliant intervals;
- maximum under-delivery energy fraction;
- maximum overshoot energy fraction;
- maximum measured ramp in kW/s.

All numerical thresholds are caller-pinned to the actual service product, contract and telemetry capabilities. No single response time or ramp limit is hardcoded as universally applicable to every market or grid service.

The verifier rejects a product whose maximum response latency is shorter than the supplied telemetry interval. It cannot establish sub-sample response or transient ramp behavior from coarse readings.

## 4. Metric conventions

For each interval j:

    delivered_reduction_kw[j] = baseline_PCC_import_kw[j] - observed_response_PCC_import_kw[j]
    underdelivery_kw[j] = max(target_reduction_kw - delivered_reduction_kw[j], 0)
    overshoot_kw[j] = max(delivered_reduction_kw[j] - target_reduction_kw, 0)

Negative delivered reduction is retained. If grid import rises relative to baseline, delivery is negative and under-delivery can exceed 100% of the request. It must not be clamped to zero.

Requested, delivered, under-delivered and overshoot energies are interval-power integrals using the frozen sample duration. Delivered energy is signed; under-delivery and overshoot are non-negative decompositions.

Response latency is the first interval-end timestamp at which delivered reduction reaches the configured minimum-delivery fraction, minus instruction issuance time. It is distinct from sustained compliance: a fast initial response can subsequently fall below the requested envelope.

Measured ramp is the absolute change in delivered reduction between consecutive interval-end samples divided by the interval duration. The initial prior-to-event reduction is declared as zero. Ramp qualification is therefore resolution-limited and does not prove the instantaneous actuator ramp between samples.

## 5. Required benchmark matrix

Run the same policy set on the same product request, baseline method, measurement boundary, sample rate and held-out conditions. Include:

| Test axis | Variants |
|---|---|
| Response initiation | immediate, delayed, no response |
| Sustained tracking | full delivery, intermittent under-delivery, late degradation |
| Overshoot | none, limited, excessive |
| Ramp | within limit, exact boundary, excessive; different telemetry resolutions |
| Baseline condition | stable baseline, varying baseline, baseline revision/uncertainty |
| Dispatch product | market-serving, grid-serving, system-serving; only with their own frozen contract |
| Flexibility mechanism | temporal workload deferral, pre-launched spatial migration, storage, facility controls, combinations |
| Recovery | immediate recovery, controlled recovery, rebound peak; use the existing whole-horizon flexibility evaluator |

Report per event and over a multi-event portfolio:

- first-qualified-response latency, p50/p95/p99 across repeated events;
- maximum and average measured ramp, with telemetry resolution explicitly stated;
- interval compliance fraction and longest continuous under-delivery streak;
- requested, delivered, under-delivered and overshoot kW/kWh;
- probability of passing the frozen product contract across repeated events;
- event availability, notice time, duration, event frequency and recovery delay;
- post-event rebound peak and rebound-adjusted net energy from the separate flexibility/M&V evaluator;
- workload completed, on-time service, energy per useful work and thermal/reserve safety from their existing owners;
- measurement quality, missing samples, clock uncertainty and baseline uncertainty.

Do not aggregate market, grid and system service results into one score. A product can have different value and qualification criteria by location, duration, notification period and grid need.

## 6. Proposed research gates

These are protocol goals, not universal standards requirements or achieved results:

- 100% exact sample chronology and evidence-reference completeness in the frozen verifier suite;
- zero false passes where the measured response latency exceeds the pinned product deadline;
- zero false passes for sustained-delivery, under-delivery, overshoot or ramp constraints;
- no qualification when telemetry resolution is coarser than the product response deadline;
- across repeated real events, report at least p50/p95/p99 response latency and pass probability, not only the best event;
- test each strategy against the strongest eligible workload-only, storage-only, facility-only and combined baseline;
- qualify rebound and net benefit over the full event-plus-recovery horizon, not only the dispatch window.

## 7. Scope and interpretation

The Rust implementation is an evaluator over supplied interval-end meter samples. It does not issue dispatch, communicate with an aggregator, authenticate source bytes, establish that the counterfactual baseline is causally valid, prove a balancing-market qualification or replace local electrical protection.

The new response report is complementary to, not a replacement for, `evaluate_flexibility_event`. Require both when claiming a complete grid-flexibility event: the response verifier qualifies time-resolved delivery, while the existing event evaluator qualifies work, whole-horizon facility energy, latency/deadlines, thermal limits and rebound.

A successful synthetic fixture proves only arithmetic and verifier behavior. Operational qualification requires product-specific contracts, independently reviewed baselines, synchronized high-quality telemetry, repeated field trials and local grid/operator acceptance.
