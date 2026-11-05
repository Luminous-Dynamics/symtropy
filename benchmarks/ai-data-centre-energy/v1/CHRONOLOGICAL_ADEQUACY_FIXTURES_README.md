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
- Loss-of-load periods, hours and days are distinct. A 15-minute interval contributes one period and 0.25 hours; multiple shortfalls on the same calendar day still contribute one loss-of-load day.
- A search result at the caller's maximum added-load limit is censored and must be described as a lower bound within the tested search interval.
- Zero reference risk is rejected because a finite synthetic study with no observed shortage cannot identify a positive ELCC-like increment under this method.

## Reproduction

The fixtures describe the same known-truth cases covered by unit tests in the chronological adequacy source module. Sixteen test cases are authored in V0.2. A passing Rust build/test result must come from an exact-head CI or local Rust toolchain run; fixture arithmetic and static source inspection are not substitutes for that evidence.

## Fixture C — weighted upper-tail EUE CVaR

Two scenario paths have weights 0.9 and 0.1. Reference scenario EUE values are 0.01 and 0.02 MWh, so the 90% upper-tail CVaR is 0.02 MWh. A verified 10 kW reduction in the high-stress scenario makes both flexible outcomes equal to 0.01 MWh at zero added load. Increasing the system by 10 kW restores the reference 90% CVaR to 0.02 MWh. This is a weighted finite-scenario arithmetic control, not a Monte Carlo confidence interval.

## Fixture D — event-periods versus hours and days

Two 15-minute intervals on the same day each contain shortage. They count as two event periods, 0.5 elapsed loss-of-load hours, and one loss-of-load day. This is why a single field called "LOLE hours" cannot be safely equated to event-period LOLE at arbitrary time resolution.

## PRAS comparison target

The protocol now defines a metric crosswalk to NLR PRAS v0.9.0, including event-period LOLE, event-day LOLD, EUE and upper-tail energy CVaR. PRAS is the independent bulk-system engine; Symtropy's value is the service/physical assurance and signed grid-flexibility trajectory supplied to that analysis. Matching metrics is valid only when timebase, regions, probabilities, horizon and tail conventions are aligned.
