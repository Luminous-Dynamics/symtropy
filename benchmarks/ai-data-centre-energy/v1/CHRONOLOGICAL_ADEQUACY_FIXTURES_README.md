# Chronological Adequacy V0.2 — Known-Truth Fixtures

**Fixture class:** synthetic known truth  
**Scope:** one-zone copperplate, weighted chronological scenario traces  
**Not:** real system data, annualized LOLE, production resource adequacy, transmission-constrained ELCC, or ISO/TSO accreditation

## Purpose

These fixtures verify the arithmetic and claim boundaries of the first chronological adequacy implementation. They are intentionally small enough to independently recompute by hand. Each interval is one hour; each scenario is a short synthetic horizon, not a full weather year.

## Fixture A — weighted EUE and ELCC-like search

The reference case has 100 kW load and 90 kW available supply for one hour. Reference unserved energy is:

    (100 - 90) kW × 1 h / 1000 = 0.01 MWh

The augmented scenarios have raw weights 3 and 1, normalized to 0.75 and 0.25. Their net demand reductions are 20 kW and 10 kW. For an added constant system load d (kW), weighted unserved energy is:

    0.75 × max(d - 10, 0) / 1000
  + 0.25 × max(d, 0) / 1000

For d at least 10 kW this equals (d - 7.5) / 1000 MWh. Setting it equal to reference EUE 0.01 MWh gives d = 17.5 kW. The search should recover approximately 17.5 kW within its configured load tolerance, with the EUE criterion restored.

This is a known-truth arithmetic control—not a claim that 17.5 kW is deliverable or accredited capacity.

## Fixture B — rebound defeats a dispatch-window-only claim

The reference trajectory has two stressed hours: 10 kW unserved in the first and 20 kW in the second, for 0.03 MWh total. The augmented trajectory reduces load by 20 kW in the first hour but adds 30 kW of rebound in the second. Augmented unserved energy is 0.05 MWh; rebound energy is 0.03 MWh. The evaluator must report that the augmented system is worse at zero added load and return zero positive load-carrying value.

## Mandatory interpretation

- Scenario weights are normalized for aggregation, but this library does not validate their statistical calibration.
- Scenario paths preserve the joint temporal sequence supplied by the caller. They do not prove that scenarios cover rare weather, common-mode failures, or future distributions.
- The signed net load reduction field is positive for reduction and negative for rebound/recovery.
- Loss-of-load events, periods, hours and days are distinct. A contiguous run crossing midnight counts as one event; each shortfall interval counts as one PRAS-style period; elapsed duration is counted in hours; a day with one or more shortfall intervals counts once.
- A search result at the caller's maximum added-load limit is censored and must be described as a lower bound within the tested search interval.
- Zero reference risk is rejected because a finite synthetic study with no observed shortage cannot identify a positive ELCC-like increment under this method.

## Reproduction

The fixtures describe the same known-truth cases covered by unit tests in the chronological adequacy source module. Nineteen test cases are authored in V0.3. A passing Rust build/test result must come from an exact-head CI or local Rust toolchain run; fixture arithmetic and static source inspection are not substitutes for that evidence.

## Fixture C — weighted upper-tail EUE CVaR

Two scenario paths have weights 0.9 and 0.1. Reference scenario EUE values are 0.01 and 0.02 MWh, so the 90% upper-tail CVaR is 0.02 MWh. A verified 10 kW reduction in the high-stress scenario makes both flexible outcomes equal to 0.01 MWh at zero added load. Increasing the system by 10 kW restores the reference 90% CVaR to 0.02 MWh. This is a weighted finite-scenario arithmetic control, not a Monte Carlo confidence interval.

## Fixture D — event-periods versus hours and days

Two 15-minute intervals on the same day each contain shortage. They count as two event periods, 0.5 elapsed loss-of-load hours, and one loss-of-load day. This is why a single field called "LOLE hours" cannot be safely equated to event-period LOLE at arbitrary time resolution.

## PRAS comparison target

The protocol now defines a metric crosswalk to NLR PRAS v0.9.0, including event-period LOLE, event-day LOLD, EUE and upper-tail energy CVaR. PRAS is the independent bulk-system engine; Symtropy's value is the service/physical assurance and signed grid-flexibility trajectory supplied to that analysis. Matching metrics is valid only when timebase, regions, probabilities, horizon and tail conventions are aligned.

## Fixture E — NTCSA-style contiguous loss-of-load events

The synthetic trace contains two shortage spells. The first begins on day 0 and continues across the day boundary into day 1; the next interval is adequate and closes the event. A later shortage begins a second event. The final interval has 5 kW headroom. The expected result is 2 events, 3 shortfall periods, 3 shortfall hours and 2 days containing shortfall. The primary monotonic metric is loss-of-load hours; it returns a 5 kW added-load value. The event count is a companion diagnostic only. This tests the event-spell definition only; it does not reproduce NTCSA's regional dispatch model.

## Fixture F — upper-tail shortfall-power CVaR

The reference scenario distribution has a 10% high-stress case at 20 kW shortfall and a 90% case at 10 kW. The prototype's interval-level weighted 90% tail mean is 20 kW. A 10 kW flexible reduction in the high-stress case makes both flexible outcomes 10 kW at zero added load; adding 10 kW system load restores the reference power-tail metric. The metric is in kW. It is deliberately separate from the EUE CVaR fixture (MWh), and its interval-weighted sample population is not yet an exact replication of NTCSA's 200-sample annual Monte Carlo distribution.

## Metric definitions: do not conflate the standards

- **NTCSA LOLE:** expected count of discrete contiguous shortage events. Consecutive shortfall intervals spanning midnight remain one event; the event ends once the adequacy requirement is met.
- **PRAS LOLE:** expected number of simulation periods with shortfall; time-resolution dependent.
- **NTCSA LOLH:** expected hours with shortfall.
- **PRAS LOLD:** expected number of days with a shortfall event.
- **PRAS EUE CVaR:** tail of energy-not-served outcomes (MWh).
- **NTCSA CVaR:** tail of shortage power magnitude (MW).

This prototype now represents the two LOLE concepts and two CVaR populations separately. The power CVaR remains a finite-scenario prototype statistic, not a validated replication of the NTCSA report's sampling population.


## Why contiguous event count is not used as the capacity search metric

A discrete event count can be non-monotonic in added load: as the system gets worse, a previously adequate interval between two shortage spells may become shortfall, merging two events into one. The event count can therefore decrease even while shortage duration and energy increase. V0.3 reports NTCSA-style contiguous event count, but rejects it as the primary ELCC-like load-search metric. Use a monotonic metric (period count, hours, days, EUE or a monotonic tail-severity statistic) for the one-dimensional search and retain event count as a diagnostic. The negative-control unit test verifies this guard.
