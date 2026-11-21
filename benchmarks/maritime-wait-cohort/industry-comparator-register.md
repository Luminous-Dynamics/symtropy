# Maritime Wait Cohort — Industry Comparator Register

**Research snapshot:** 21 November 2026.

## World Bank / S&P Global CPPI

The World Bank's 2026 *Container Terminal Performance Improvement Guide* uses a milestone-based port-call model. Its Figure 3 and definitions distinguish:
- Port Limits;
- Pilot On Board;
- Arrive at Berth;
- First/Last Line Tied;
- Begin/End Cargo Operations;
- First/Last Line Untied;
- Leave Berth.

The Guide defines port arrival wait, steam-in, berth arrival wait, cargo handling, berth departure wait, berth occupancy, ship turnaround and port turnaround separately. This is the right discipline for our measurement system: one "wait time" label is insufficient unless its start/end milestones are explicit.

The Guide's 2025 CPPI dataset reports descriptive overall averages of 12.3 hours waiting and 4.1 hours steam-in, totaling 16.4 hours arrival time. It also reports 53.5% of total port time as cargo operations and 46.5% outside cargo operating hours. These are dataset-wide arithmetic descriptions, not universal targets or terminal-specific standards. Port-arrival and port-hours measures cross organizational boundaries.

CPPI 2025 measures time from port limits to departure from berth and considers vessel/call size; the proposed queue cohort's entry-to-first-berth wait is a distinct interval.

## S&P Global Port Performance

The commercial product page advertises:
- 180,000 port calls/year;
- 1,000+ terminals;
- 500 ports;
- total port hours, vessel waiting, cargo unloading times, berth productivity, call/vessel size and gross crane moves/hour.

S&P Global's CPPI 2025 commentary separately describes 180,000 calls across more than 400 ports combined with AIS, vessel characteristics and port reference data. These are differently worded coverage statements and should not be conflated. Source coverage and benchmark method must be version-pinned.

A valid external comparison would require legally licensed call-level data, the exact time anchors and population, call/censor treatment, workload normalization and stratification. No scraping or mirroring is permitted.

## DCSA Port Call 2.0.0

DCSA distinguishes estimated, requested, planned and actual times, with a negotiated ERP-A pattern for berth, cargo operations, pilotage, towage, mooring, bunkering and anchorage. Actual execution time is not interchangeable with planned or estimated time.

Our cohort evaluator accepts only reconciled actual event outcomes at this stage. The DCSA adapter remains responsible for profile-conformant event/facility semantics; the cohort evaluator must not guess what a provider's "arrival" or "berth" field means.

## Why the Luminous benchmark differs

The differentiating layer is a measurable evidence contract:
1. right-censored calls remain in the dataset as exposure;
2. diversion and cancellation remain explicit competing causes;
3. berth cumulative incidence is not estimated from only completed calls;
4. exact vessel-tick queue area reconciles to the stock ledger;
5. historical cutoffs prevent later observations from rewriting earlier decisions;
6. all quantiles carry an estimability decision.

This is not a claim of better port operations yet. It is a measurement-integrity advantage to validate before attempting optimization claims.
