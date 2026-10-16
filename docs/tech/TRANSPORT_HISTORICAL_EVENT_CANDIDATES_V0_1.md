# Historical Transport Event Candidates v0.1

**Reviewed:** 2026-10-16  
**Status:** desk research and source-candidate qualification only; no source artifacts have been acquired, hash-frozen, or rights-cleared by this document.  
**Tracking:** [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671)  
**Panama artifact admission:** [TRANSPORT-DATA-PANAMA-001 #1677](https://github.com/Luminous-Dynamics/symtropy/issues/1677)  
**Provisional machine-readable transcription (not qualified data):** [TRANSPORT_PANAMA_EVENT_TIMELINE_PROVISIONAL_V0_1.csv](TRANSPORT_PANAMA_EVENT_TIMELINE_PROVISIONAL_V0_1.csv)  
**November prospective evaluation (draft):** [TRANSPORT_PANAMA_2026_PROSPECTIVE_EVAL_V0_1.md](TRANSPORT_PANAMA_2026_PROSPECTIVE_EVAL_V0_1.md)  
**Reservation-policy replay rules (draft):** [TRANSPORT_PANAMA_POLICY_RULES_V0_1.md](TRANSPORT_PANAMA_POLICY_RULES_V0_1.md)  
**Benchmark contract:** [TRANSPORT_BENCHMARK_LADDER_V0_1.md](TRANSPORT_BENCHMARK_LADDER_V0_1.md)  
**Global source discovery:** [GLOBAL_TRANSPORT_DATA_COVERAGE_V0_1.md](GLOBAL_TRANSPORT_DATA_COVERAGE_V0_1.md)

## Decision

**First candidate to qualify: Panama Canal drought-driven transit restrictions, 2023–2024.** It has a comparatively good official record of staged booking-slot/capacity changes, recurring operating summaries, observed monthly/daily aggregate traffic and canal-water context. It can support a capacity-policy replay if source artifacts and rights review pass. It does *not* yet support a shipment-level, end-to-end, global rerouting benefit claim: OD-level cargo, individual vessel histories, alternative route outcomes and observed customer-level service impacts have not been established here.

**Second candidate: Port of Baltimore / Francis Scott Key Bridge collapse, March–June 2024.** The event and channel-reopening timeline are well documented, with public federal/state snapshots and annual port totals. It is potentially stronger for network-severance and restoration chronology, but operationally granular rerouting/demand/recovery traces remain a prerequisite for an empirical quantitative replay.

These are source-priority judgements, not a claim that any source file has been acquired, licensed for all intended uses, or validated. If artifact review fails, retain a candidate as a labelled scenario or reject it—do not lower the gate.

## Candidate scorecard

Score the *evidence available for the specific replay question*, not the importance of the event. Current desk-research scores are provisional ordinal labels: 0 = not established, 1 = official lead found, 2 = artifact appears to support the dimension, 3 = artifact audited and fit for this exact claim. No dimension is currently at level 3 because acquisition, rights and independent reconciliation remain undone.

| Candidate | Event chronology | Capacity / operating changes | Observed outcomes | Likely initial use | Main blocker |
|---|---:|---:|---:|---|---|
| Panama Canal drought restrictions, 2023–24 | 2 | 2 | 1–2 | capacity-policy / aggregate traffic replay | Shipment-level OD, queues and alternate-route outcomes not established |
| Baltimore channel closure / recovery, 2024 | 2 | 1–2 | 1 | resource-severance stress case; later quantitative replay if granular traces exist | Fine-grained rerouting, demand and service outcomes not established |
| Suez Canal blockage, March 2021 | 1 | 0–1 | 0–1 | reserve candidate only | Need a permitted, auditable, time-resolved capacity/queue/outcome corpus |
| Red Sea route diversions, 2023–25 | 1–2 | 0–1 | 0–1 | reserve candidate for route substitution | Vessel traces, carrier decisions, cost and delay data may be commercially restricted and heavily confounded |

The numbers above are triage judgements, not measured evidence scores or source-quality ratings. Scores must be revised with artifact receipts and per-claim evidence.

## A. Panama Canal drought / operating restrictions

### Official source leads

- [Panama Canal Authority — Advisory to Shipping archive](https://pancanal.com/en/maritime-services/advisory-to-shipping/) includes dated operating advisories and monthly summaries across years.
- [Advisory A-48-2023, 30 October 2023 — reduction in transits](https://pancanal.com/wp-content/uploads/2023/01/ADV48-2023-Reduction-in-Transits-Due-to-the-Ongoing-Deficit-in-Precipitation-in-the-Canal-Watershed.pdf) describes sequential booking-slot reductions: 25 for 3–6 November 2023; 24 for 7–30 November; 22 for December; 20 for January 2024; and 18 per day from booking dates beginning 1 February 2024, subject to further adjustment. This is an announced booking policy, not a guarantee of actual daily transit capacity or individual vessel outcomes.
- [Advisory A-11-2024, 10 April 2024 — March operating summary](https://pancanal.com/wp-content/uploads/2024/01/ADV11-2024-Monthly-Canal-Operations-Summary-March-2024-1.pdf) reports March 2024 daily arrivals (average/high/low 24.5/33/12), oceangoing transits (24.1/28/21), canal-waters time (20.7/34.5/14.2 hours), in-transit time (9.4/11.1/8.0 hours), booking-slot availability/use and scheduled lock maintenance. It also notes that transit restrictions and maintenance jointly affect capacity.
- [Panama Canal Authority — FY2024 update](https://pancanal.com/en/fiscal-year-2024-update/) documents the evolving daily transit policy through 2023–2024 and contemporaneous queue snapshots.
- [Oceangoing commercial traffic by month, FY2023 vs FY2022](https://pancanal.com/wp-content/uploads/2023/11/02-Oceangoing-Commercial-Traffic-Through-the-Panama-Canal-by-Month.pdf) supplies monthly traffic, tonnage and cargo totals for a pre-/early-restriction comparison. Its reporting window ends before the most severe FY2024 restrictions, so it cannot be used as the event-period outcomes by itself.

### Preliminary source-grounded timeline: multi-phase policy plus observed outcomes

The event must be modeled as a **bitemporal sequence**: when a policy was published is different from when it applies to booking/transit dates. Monthly reports are retrospective outcomes, not inputs the planner could have known during the reported month. Later advisories can supersede earlier defaults.

| Publication date | Effective / observed window | Authority-reported fact | Correct interpretation |
|---|---|---|---|
| 30 October 2023 — [A-48-2023](https://pancanal.com/wp-content/uploads/2023/01/ADV48-2023-Reduction-in-Transits-Due-to-the-Ongoing-Deficit-in-Precipitation-in-the-Canal-Watershed.pdf) | 3–6 Nov 2023 | 25 booking slots/day | A future booking policy announced on 30 Oct |
| Same advisory | 7–30 Nov 2023 | 24 booking slots/day | Offered booking slots, not actual completed transits |
| Same advisory | Dec 2023 | 22 booking slots/day | Same distinction |
| Same advisory | Jan 2024 | 20 booking slots/day | Same distinction |
| Same advisory | Booking dates from 1 Feb 2024 | 18 booking slots/day until further notice | A-48 baseline only; must be supersession-checked |
| 8 March 2024 — [A-08-2024](https://pancanal.com/wp-content/uploads/2024/01/Advisory-to-Shipping-No.-A-08-2024.pdf) | Additional auction slots for transit dates from 18 Mar; added supers slot from 25 Mar | Panamax booking policy rises to a 27-slot total | Unlike an April retrospective, this is a dated announcement before the corresponding effective dates |
| 10 April 2024 — [A-11-2024](https://pancanal.com/wp-content/uploads/2024/01/ADV11-2024-Monthly-Canal-Operations-Summary-March-2024-1.pdf) | March 2024 outcomes | 747 oceangoing transits; average 24.1/day (high 28, low 21); canal-waters time 20.7 h (high 34.5, low 14.2); in-transit time 9.4 h (high 11.1, low 8.0) | Retrospective observed statistics, not available as March planning inputs |
| Same A-11 summary | Reported as of 9 April | Around 50 vessels outside; more than three-quarters reportedly held reservations and often arrived early; average wait for unreserved arrivals year-to-date was just under 2.5 days | Canal-authority interpretation with a specific population and time window; not a count of all vessels stuck in a queue |
| 15 April 2024 — [A-12-2024](https://pancanal.com/wp-content/uploads/2024/01/ADV12-2024-Increase-in-the-Number-of-Booking-Slots-and-Increase-in-the-Maximum-allowable-draft.28.pdf) | Booking dates 16–31 May | 31 offered slots/day; policy rises to 32/day from 1 June through 14 July; 32/day from 15 July until further notice | Announced slot distribution; actual usable capacity still depends on vessel mix, maintenance, restrictions and slot category |
| 10 May 2024 — [A-14-2024](https://pancanal.com/wp-content/uploads/2024/01/ADV14-2024-Monthly-Canal-Operations-Summary-%E2%80%93-April-2024.pdf) | April 2024 outcomes | 789 transits; average 26.3/day (high 29, low 23); canal-waters time 20.2 h average; in-transit time 9.4 h average | Retrospective monthly results; maintenance was also present |
| 10 June 2024 — [A-18-2024](https://pancanal.com/wp-content/uploads/2024/01/ADV-18-2024-Monthly-Canal-Operations-Summary-May-2024.pdf) | May 2024 outcomes | 805 transits; average 26.0/day (high 32, low 20); canal-waters time 22.9 h average; in-transit time 10.7 h average | Retrospective monthly results; slot availability/use and maintenance must remain separate fields |
| 10 June 2024 — [A-19-2024](https://pancanal.com/wp-content/uploads/2024/01/ADV-19-2024-Increase-in-the-Number-of-Neopanamax-and-Panamax-Booking-Slots-Draft-Adjustment-and-Other-Information-Regarding-the-Transit-Reservation-System.pdf) | Booking dates from 11 July, then from 22 July | Daily offered slots rise to 33 from 11 July and 34 from 22 July | Policy revisions announced on 10 June |
| 26 June 2024 — [A-20-2024](https://pancanal.com/wp-content/uploads/2024/01/ADV-20-2024-Increase-in-the-Number-of-Neopanamax-Booking-Slots-Transit-Draft-Increase-and-the-Transit-Reservation-System.pdf) | Booking dates from 5 August | Total offered booking slots rise to 35/day; allowable Neopanamax draft is also revised | Separate slot-count and vessel-draft constraints; do not collapse into one capacity scalar |

This is a **desk-researched transcription** from the Panama Canal Authority's official [advisory archive](https://pancanal.com/en/maritime-services/advisory-to-shipping/) and [statistics page](https://pancanal.com/en/statistics/). Original bytes have not yet been acquired, hashed or rights-reviewed, so this is not a frozen source artifact pack. The table includes examples of policy supersession; it is not a complete event chronology. The artifact-admission gate is tracked in [TRANSPORT-DATA-PANAMA-001 #1677](https://github.com/Luminous-Dynamics/symtropy/issues/1677).

#### New finding: raw anchorage count is a poor queue metric

The April 10 report says roughly 50 vessels were outside the Canal at that point, but also says over three-quarters had reservations and routinely arrived early; it reports unreserved-arrival waiting time separately from vessels with booked transit dates. The benchmark must therefore stratify at least **reserved / unreserved**, **arrival date / scheduled transit date**, **actual transit**, and **observed elapsed time**. A single vessel count or an aggregate average can otherwise misstate service reliability. Treat these as the Authority's retrospective statements until independently reconciled—not as shipment-level ground truth.

#### New finding: capacity recovery is multi-dimensional

Policy slot counts, realized oceangoing transits, Panamax/Neopanamax mix, draft limits, lock maintenance, booked-slot utilization and canal-waters time all evolve on different timelines. In the March report, the Authority lists multiple maintenance outages and estimates lock-specific capacity under ongoing conservation measures; April and May reports similarly record outages. A valid model needs separate typed constraints and confounders, not a single “available capacity” number inferred from daily slot announcements.

### Exact initial research question

Given only each date's already-published operating advisories, water/season context and available demand proxy, can a constrained policy model correctly represent changes to announced slot limits without turning monthly averages into daily capacity? Can it estimate a *scenario-bounded* range of possible throughput/service effects while preserving the distinction between booked slots, actual transits, transit delay and unobserved cargo rerouting?

### Admissible claim ladder

1. **G2 / policy-timeline reconstruction:** reproduce dated announced slot limits and their effective booking dates from original advisories; independent extraction agrees exactly.
2. **G2 / aggregate time-series comparison:** reconcile monthly published traffic and operating statistics using their stated definitions and units. Seasonality and maintenance must be represented or explicitly left as confounders.
3. **Scenario evaluation only:** stress bounded demand/capacity configurations and compare policies; outcomes are simulated and must be labeled as such.
4. **Not yet admissible:** claim actual shipments were rerouted, quantify global trade losses caused by the event, estimate a counterfactual optimizer benefit, or claim end-to-end reliability without observed route, demand and shipment/outcome evidence.

### Required acquisition and hostile checks

- Acquire original official advisories for the entire selected window and at least one pre-event baseline period; save exact bytes, source URL, publication/effective dates, retrieval time, SHA-256, and use/redistribution terms.
- Build a daily dated-policy series distinct from actual transits and monthly averages. Handle announced, corrected, superseded and effective-on-date policies explicitly.
- Reconcile daily/period summaries to the monthly statistics without interpolation that manufactures measurements.
- Include negative controls for applying a later advisory too early, treating slots as completed transits, mixing fiscal/calendar months, conflating Panamax and Neopanamax constraints, ignoring planned maintenance, and counting a booked vessel as observed cargo delivered.
- Search for permitted, machine-readable, individual vessel arrival/queue/transit records and OD/cargo outcomes. Until qualified, do not infer them from aggregate public figures.

## B. Port of Baltimore / Francis Scott Key Bridge collapse

### Official source leads

- [U.S. Bureau of Transportation Statistics — Port of Baltimore information spotlight, 5 April 2024](https://www.bts.gov/current-transportation-statistics/information-about-port-baltimore) documents the 26 March 2024 event and a 27 March vessel-tracking snapshot. The snapshot is an observation at a particular timestamp, not a complete vessel-time-series corpus.
- [Maryland DOT — full reopening announcement, 12 June 2024](https://www.mdot.maryland.gov/tso/pages/newsroomdetails.aspx?PageId=38&newsId=799) records full reopening following reinstatement of the federal channel.
- [U.S. Army Corps of Engineers — tentative channel reopening timeline, 4 April 2024](https://www.usace.army.mil/Media/News/Article/3731723/us-army-corps-of-engineers-develops-tentative-timeline-to-reopen-fort-mchenry-c/) provides contemporaneous restoration planning.
- [Maryland Port Administration — 2024 recovery milestone, 25 February 2025](https://content.govdelivery.com/accounts/MDMPA/bulletins/3d42609) reports 45.9 million tons of cargo handled in 2024 compared with 52.3 million tons in 2023 and says more than 25.5 million tons of 2024 cargo were handled during the last six months. These annual/half-year aggregates do not identify the causal effect of the bridge collapse or the counterfactual.

### Exact initial research question

Can a resource-severance model represent the blocked federal navigation channel, preserve the distinction between marine-channel access and ongoing terminal/truck operations, and replay the dated restoration milestones without overclaiming the effect on specific commodities or alternate ports?

### Gate

Begin as a chronology/resource-severance test. Quantitative empirical replay requires permitted dated channel-state evidence, the relevant before/during/after vessel calls or equivalent service observations, and a defensible demand/commodity or shipment-outcome record. Annual tonnage totals can be contextual checks but cannot determine per-shipment diversion, queue or optimizer benefit.

Adversarial checks: do not equate the channel reopening with full supply-chain recovery; do not mark all terminal or truck activity zero during the marine-channel closure; prevent later recovery data from leaking into the initial response plan; keep the observed DALI/stranded-vessel snapshot separate from a full AIS track.

## C. Suez blockage and Red Sea diversions

These remain lower-priority candidates until exact artifacts and legal terms are identified. Do not use general news chronology as a proxy for capacity, vessel queue duration, route selection or costs. AIS/commercial movement datasets may offer useful traces but must be reviewed for permission to store, redistribute, derive metrics and publish benchmark fixtures. Separate the canal blockage itself from the multi-week/multi-month rerouting mechanisms in Red Sea cases; they are different event models and should not be blended into one event label.

## D. Prospective temporal-transfer holdout: Panama policy change, October 2026

The Canal now supplies a potential *prospective* check, distinct from the 2023–24 drought-restriction calibration window:

- [A-19-2026, issued 9 June](https://pancanal.com/wp-content/uploads/2026/06/ADV-19-2026-Monthly-Canal-Operations-Summary-May-2026.pdf): May 2026 had 1,149 oceangoing transits, average 37.06/day, canal-waters time 31.20 h and in-transit time 11.06 h.
- [A-23-2026, issued 10 July](https://pancanal.com/wp-content/uploads/2026/07/ADV-23-2026-Monthly-Canal-Operations-Summary-June-2026.pdf): June had 975 transits, average 32.50/day, canal-waters time 23.80 h and in-transit time 11.01 h.
- [A-26-2026, issued 10 August](https://pancanal.com/wp-content/uploads/2026/08/ADV-26-2026-Monthly-Canal-Operations-Summary-July-2026.pdf): July had 1,055 transits, average 34.03/day, canal-waters time 25.38 h and in-transit time 10.87 h. The same report announced draft reductions to 14.63 m (effective 26 August) and 14.48 m (effective 3 September), while explicitly saying those draft changes did not affect daily transit count.
- [A-34-2026, issued 10 September](https://pancanal.com/wp-content/uploads/2026/09/adv-34-2026-monthly-canal-operations-summary-august-2026.pdf): August 2026 had 1,029 oceangoing transits, a reported daily average of 33.19, canal-waters time 34.90 hours on average, and in-transit time 11.27 hours.
- [A-38-2026, issued 9 October](https://pancanal.com/wp-content/uploads/2026/10/ADV-38-2026-Monthly-Canal-Operations-Summary-September-2026.pdf): September had 979 transits, daily average 32.63, canal-waters time 37.97 hours and in-transit time 11.59 hours.
- The authoritative prospective policy is [A-36-2026, issued 28 September](https://pancanal.com/wp-content/uploads/2026/04/ADV-36-2026-Increase-in-the-Number-of-Neopanamax-Booking-Slots-Draft-Adjustment-and-Transit-Reservation-System.pdf): it announces an increase from 9 to 10 Neopanamax booking slots, for 33 total daily slots across both lock systems, effective for booking dates beginning 15 October 2026. It separately sets maximum Neopanamax draft to 14.94 m (49 ft), effective immediately. The advisory also details market-segment/booking-period allocation and conditions; those constraints must not be reduced to one interchangeable slot count.

### Why this is a paradigm shift

This is an opportunity to **pre-register a prospective, temporal-transfer evaluation before October outcomes are known**, rather than repeatedly tune against a closed historical corpus. It tests whether a model learned from an older water/slot regime remains calibrated as water levels, booking policy, maintenance and vessel mix change. It is not yet a valid pass/fail experiment: future outcome reports and appropriate comparator data do not exist in this evidence snapshot. May–September values are retrospective aggregates published after each month; A-36 is a prospective rule for booking dates, not a guaranteed actual-transit count. Keep those time axes separate.

Protocol:
1. Freeze the pre-decision input snapshot and timestamp it no later than the applicable planning cutoff. A-36's policy announcement on 28 September can inform decisions after that announcement, but later October outcomes and future monthly summaries cannot be model inputs.
2. Before examining October outcomes, record the exact forecast target, baseline, unit, temporal window, model/configuration commit and all assumptions. Prefer forecasts that can actually be scored from later official observations; do not invent an October outcome now.
3. Compare at least (a) seasonal/static baseline, (b) a transparent rule-based baseline that honors known slot/draft/maintenance conditions, and (c) the historical-model/optimizer configuration. Keep predicted booking slots distinct from actual transits and from the separate reported canal-waters/in-transit times.
4. Wait for a future official report only as an evaluation data dependency; work now on source admission, forecast manifests, the verifier and negative controls. No “pass” can be claimed until held-out outcomes arrive and the exact checker reproduces the score.
5. Report forecast error/interval coverage and calibration alongside served/unserved demand and delays only if the sources actually provide those measures. Monthly aggregate averages cannot validate vessel-level queue tails or shipment-level rerouting.
6. Treat October 2026 as one prospective transfer case, not enough to establish generalization.

### Hard blocker: Gatun Lake daily history permission

The Canal's [official Gatun Lake water-level download](https://evtms-rpts.pancanal.com/eng/h2o/Download_Gatun_Lake_Water_Level_History.csv) is an attractive explanatory covariate, and the official [indicator page](https://evtms-rpts.pancanal.com/eng/h2o/index.html) links the history, projection and dashboard. However, that page explicitly limits use to informational purposes and states that copying, modifying, distributing, publishing or creating derivatives requires **prior written ACP consent**. The downloadable CSV could not be inspected in this research pass; its coverage and units are not independently verified. Record it as **permission-gated/link-only**. Do not commit the daily series or derived feature rows into this public corpus without written permission or a different legally permitted source.

### Current evidence boundary

The 2026 rows in the provisional CSV are a separate `event_id` from the 2023–24 drought record. August and September figures are retroactive aggregate observations, while the October 15 slot policy is a prospective announcement. All 63 rows remain marked as not acquired/hash-frozen, with source-use rights unreviewed except that the Gatun history link is explicitly blocked pending written permission. None is yet a qualified artifact pack or a model forecast result.

## E. Reservation-policy rules and 2027 Long-Term Slot Allocation (LoTSA)

A new event dimension is emerging: **the allocation rules themselves change over time**. A-31 modifies Neopanamax multiple/consecutive bookings and segment allocation; A-35 clarifies that the Panamax consecutive-date restriction does not apply across different booking periods; A-30 changes special handling of cancelled slots and distinguishes NetZero slot swaps from substitutions. These rules affect eligibility and allocation, not physical canal capacity.

[ A-37-2026, published 5 October ](https://pancanal.com/wp-content/uploads/2026/10/ADV-37-2026-Long-Term-Slot-Allocation-Program-2027-and-Modification-to-Transit-Reservation-System-tariffs.pdf) adds a 2027 LoTSA booking horizon from 3 January to 3 April. It schedules the Neopanamax competition for 17 November 2026 and the Panamax sealed-bid competition for 24 November; the new Panamax pilot and existing Neopanamax program each offer an average of three slots per day. The same advisory changes the late-cancellation surcharge threshold from seven to four days effective 15 October 2026.

This suggests a new, bounded benchmark: **replay public rule eligibility and policy chronology before attempting to model actual outcomes**. Use [TRANSPORT-POLICY-001 #1681](https://github.com/Luminous-Dynamics/symtropy/issues/1681) and [TRANSPORT_PANAMA_POLICY_RULES_V0_1.md](TRANSPORT_PANAMA_POLICY_RULES_V0_1.md). In particular:
- the 17/24 November competition dates are not the 2027 booking/transit horizon;
- the Neopanamax multiple/consecutive-date rule must not leak into Panamax;
- A-35's cross-period exception does not erase same-period restrictions;
- slot minima, quotas, bidding, cancellation rules, draft limits and physical transit capacity are separate constraints;
- public rule text cannot reveal private requests, bids, customer rankings or actual award outcomes.

Golden-case target: 100% of manually reviewed rule-conformance fixtures correct, zero rules used before their known-at time or outside their effective scope, zero quota/unit violations, deterministic output under ordering permutations, and every unparsed exception explicitly unsupported. Do not claim fairer access or better service until actual request/award/outcome data and an appropriate evaluation design exist.

## Benchmark protocol for whichever event passes qualification

- Freeze the event timeline, source manifests and pre-event information cutoff before planner/baseline comparisons.
- Compare identical source snapshots and demands across: (1) a reconstructible observed/static baseline, (2) a declared deterministic feasible-route baseline, and (3) event-triggered re-planning. Add a reserve/robust baseline only after its policy is specified before examining holdout results.
- Report observed vs simulated quantities separately. Metrics may include actual transits, published wait/time summaries, served/unserved demand, missed windows, service/recovery trajectory, time to threshold, route-plan churn, displaced bottlenecks, uncertainty and missingness—but only where evidence supports each quantity.
- Use one event for a case study. Do not claim generalized resilience improvement until multiple suitable independent events and predeclared held-out evaluation exist.
- Retain failed, adverse, contradictory and unfit-to-compare records with rejection reasons.
- An independent checker must verify artifact hashes, publication/effective-date cutoffs, event chronology, unit/time-grain conversions, reconciliations and computed outcomes.

## Next decision

Qualify Panama first as an **official policy + aggregate operations case**, while parallel-reviewing Baltimore as a **network-severance chronology case**. Before selecting either for an empirical performance claim, obtain and inspect the exact underlying files, record rights/allowed uses, and document gaps. If neither has adequate outcome traces, ship a reproducible capacity-policy or severance stress test and label it honestly; do not promote it to historical shipment-level validation.
