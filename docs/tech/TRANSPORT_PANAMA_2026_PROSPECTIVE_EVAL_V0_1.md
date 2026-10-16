# Panama Canal November 2026 Prospective Evaluation v0.1

**Prepared:** 2026-10-16  
**Status:** prospective evaluation draft with explicit simple baselines; **not a qualified preregistration** until original source artifacts, rights/use terms, and source-to-row extraction have been admitted.  
**Tracking:** [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671), [TRANSPORT-DATA-PANAMA-001 #1677](https://github.com/Luminous-Dynamics/symtropy/issues/1677)  
**Provisional input table:** [TRANSPORT_PANAMA_EVENT_TIMELINE_PROVISIONAL_V0_1.csv](TRANSPORT_PANAMA_EVENT_TIMELINE_PROVISIONAL_V0_1.csv)  
**Claim ceiling:** month-level out-of-time forecast check only; no causal policy effect, shipment-level reliability, or dispatch claim.

## 1. Why this is useful

The 2023–24 drought period can support historical policy/aggregate reconstruction, but tuning and testing on the same historical window can overstate transfer. The Canal's May–September 2026 reports provide a separate, newer monthly series. [A-36-2026, published 28 September](https://pancanal.com/wp-content/uploads/2026/04/ADV-36-2026-Increase-in-the-Number-of-Neopanamax-Booking-Slots-Draft-Adjustment-and-Transit-Reservation-System.pdf) announces 10 Neopanamax booking slots and 33 total daily slots for **booking dates beginning 15 October** and separately raises maximum Neopanamax draft to 14.94 m effective immediately.

This is an out-of-time forecast exercise, not a causal study of that policy. “Booking dates” are not automatically vessel-transit dates. Actual transits depend on booking allocation, vessel class, water/draft limits, maintenance, additional auctioned slots and other operational conditions.

## 2. Outcome horizon and frozen rules

- **Target month:** November 2026. Its official monthly outcome report had not been published in this 16 October source snapshot.
- **Input cutoff:** observations and advisories published no later than 16 October 2026. Each report's publication timestamp must be retained separately from the month it describes and from each rule's booking/effective window.
- **Data partition:** 2023–24 drought period and the May–September 2026 series are distinct event/time regimes. Do not silently pool or tune across them and then label the later period independent.
- **Forecast unit:** expected monthly count, expected daily transit rate, published monthly mean canal-waters time, published monthly mean in-transit time, and monthly vessel-beam class counts. The total and daily mean are mathematically linked and are *not* independent evidence.
- **Source gate:** every source row is currently provisional; all original PDFs remain unacquired/hash-unfrozen and use rights have not been fully reviewed. These calculations are a draft reference contract, not final qualified benchmark evidence.
- **Unchanged inputs:** preserve adverse outcomes, missingness, rounding differences and source conflicts. No daily observations may be manufactured from a monthly mean.

## 3. Reference baselines calculated from the published provisional figures

The following numbers are **pre-outcome reference baselines**, calculated from the values transcribed in the provisional table. Recompute them from admitted source artifacts before the evaluation is formally locked.

| Metric | B0: September persistence | B1: trailing 3-month daily-rate mean |
|---|---:|---:|
| November oceangoing transits (expected count) | 979 | 998.5914 |
| Derived daily transit rate (transits/day) | 32.6333 | 33.2864 |
| Mean canal-waters time (hours) | 37.97 | 32.75 |
| Mean in-transit time (hours) | 11.59 | 11.2433 |
| Transits: beam under 91 inches | 149 | 164.1828 |
| Transits: beam 91 to under 107 inches | 561 | 548.6129 |
| Neopanamax: beam 107 inches and over | 269 | 285.7957 |

### Exact baseline formulas

Let (d_m) be the number of calendar days in month (m), (T_m) the observed monthly total, and (C_{m,k}) the observed number in beam category (k).

- **B0 — persistence:** predict November totals and beam-category counts equal to September 2026. Predict time means equal to the September report. Daily rate for the consistency check is the frozen predicted total divided by 30.
- **B1 — rate-normalized trailing mean:** for each count (X) (total or beam category), predict (hat X_{Nov}=30 \times \frac{1}{3}\sum_{m\in\{Jul,Aug,Sep\}} X_m/d_m). For time means, predict the arithmetic mean of the three reported July, August and September means.
- Keep expected count predictions unrounded for scoring. Compare against the exact monthly integer count; separately record the report's rounded daily average and any source rounding tolerance. The expected category counts under B1 sum to the B1 expected total by construction.
- Do not call B0 or B1 “optimal.” They are intentionally simple benchmark baselines. Their predictions must be preserved unchanged once source admission completes.

## 4. Inputs used for the provisional baseline

| Period | Total transits | Beam <91 in | Beam 91–<107 in | Beam ≥107 in | Canal-waters mean (h) | In-transit mean (h) |
|---|---:|---:|---:|---:|---:|---:|
| July 2026 | 1,055 | 175 | 570 | 310 | 25.38 | 10.87 |
| August 2026 | 1,029 | 180 | 551 | 298 | 34.90 | 11.27 |
| September 2026 | 979 | 149 | 561 | 269 | 37.97 | 11.59 |

Official leads: [A-26-2026 (July, published 10 August)](https://pancanal.com/wp-content/uploads/2026/08/ADV-26-2026-Monthly-Canal-Operations-Summary-July-2026.pdf), [A-34-2026 (August, published 10 September)](https://pancanal.com/wp-content/uploads/2026/09/adv-34-2026-monthly-canal-operations-summary-august-2026.pdf), and [A-38-2026 (September, published 9 October)](https://pancanal.com/wp-content/uploads/2026/10/ADV-38-2026-Monthly-Canal-Operations-Summary-September-2026.pdf). The provisional dataset also retains May and June, but B1 uses the most recent three complete months specified above.

## 5. Model forecast entry — intentionally unfilled

Do not invent a “Symtropy forecast” before there is a reproducible policy/model implementation and a frozen configuration. Before the November result is revealed, add a versioned, immutable record with:

- model source commit and configuration digest;
- exact admissible input record IDs and manifest SHA-256;
- information cutoff and time-zone convention;
- exact predicted values and intervals for each target;
- units and metric extraction rules;
- all unknown, assumed, excluded, or rights-restricted inputs;
- independent checker revision and a reproduction receipt.

If there is no permitted hydrology input, explicitly state that. The official [Gatun Lake historical-level CSV](https://evtms-rpts.pancanal.com/eng/h2o/Download_Gatun_Lake_Water_Level_History.csv) is **link-only/permission-gated**: its [official indicator page](https://evtms-rpts.pancanal.com/eng/h2o/index.html) says copying, modifying, redistributing, publishing or creating derivatives requires prior written ACP consent. Do not ingest that time series or derive public features from it without permission.

## 6. Scoring and acceptance

- Score each target independently after the November report is published: absolute error and signed error for totals/counts; absolute and signed error in hours for time means; optionally interval coverage/width if the model emits calibrated intervals.
- Check that the three vessel-beam categories sum to total transits, and that reported daily means agree with total/day count within documented source rounding. Do not count total, daily average and each segment's sum as independent replications.
- Compare B0, B1 and any frozen model forecast against the same month and source version. Record which model is best per metric; do not blend all errors into one score without a declared decision purpose.
- If actual monthly outcomes cannot be accessed lawfully or cannot be reconciled, the result is **unscorable**, not a model failure/pass.
- One future month is a single prospective case study. It does not establish generalization, causal policy benefit, robust logistics performance or global applicability. Repeat across multiple predeclared periods or independent events before making those claims.
- Do not require the optimizer to beat a naïve forecast on every metric: its primary purpose may be feasible allocation and preservation of service. Any downstream planning-value test must specify a separate decision metric, demand model and operationally supported outcomes.

## 7. Leakage and interpretation negative controls

1. A monthly report published after the observation period is forbidden as a contemporaneous input for forecasts made during that period.
2. Booking-date policy is not a guarantee of a matching number of completed transits on transit dates.
3. Draft limits, number of slots, actual transits, booking-slot counts and canal-waters time remain distinct variables.
4. Maintenance, vessel mix, auction slots and reservations remain visible confounders; simple pre/post difference is not a causal policy estimate.
5. Source conflicts (including A-48-2023's November slot dates), superseded policies, and missing/permission-restricted inputs remain represented, not overwritten or silently dropped.

## 8. Status

This document makes baseline calculation reproducible and reduces the risk of post-hoc baseline selection. It is still **draft** until original source bytes, rights/use terms, extraction receipts and the input manifest are admitted. No November result exists here; no policy-aware forecast has been emitted; no optimizer improvement is claimed.
