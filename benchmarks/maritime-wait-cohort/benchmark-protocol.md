# Maritime Wait Cohort — Benchmark Protocol v0.1

**Status:** deterministic synthetic cohort estimator. It is not a production port-time measurement or CPPI reproduction.

## 1. Research question

How should waiting-time performance be estimated when many vessel calls are still waiting at the observation cutoff, some divert or cancel, and event observations can arrive after the historical decision snapshot?

A completed-only berth sample is not representative of the queue during congestion: the longest-waiting calls can be absent because they have not yet berthed.

## 2. Paradigm shift: model incomplete futures rather than discard them

Every vessel call starts a queue exposure interval at a declared queue-entry event. It ends at first berth entry, diversion, cancellation, or the observation cutoff if unresolved.

- Berth is the event of interest.
- Diversion and cancellation are competing events.
- Unresolved calls are right-censored.
- Outcomes observed after the declared cutoff cannot change the earlier cohort snapshot.

A diverting vessel did not berth. A cancelled call did not berth. An unresolved vessel did not wait zero hours and is not a missing row.

## 3. Input semantics

Each row contains:
- scenario/cohort ID;
- unique call ID;
- queue-entry tick;
- common observation cutoff tick;
- outcome (BERTH, DIVERTED, CANCELLED or CENSORED);
- outcome tick;
- source-observation tick.

The upstream adapter is responsible for authenticating the source and defining the exact start/end milestone. This evaluator assumes the entry/outcome events have already been qualified.

A record whose source-observation tick is later than the cohort cutoff is not visible to that historical snapshot. It is treated as a right-censored call ending at the cutoff, regardless of the later-observed outcome value.

Observed outcomes must occur on/after queue entry and on/before the cutoff, and must not be observed before they occur. A CENSORED record must be observed at its declared cutoff. Cohort cutoff must be consistent within a scenario.

The synthetic ticks have no timezone, UTC, hours or minutes semantics.

## 4. Competing-risk estimator

At each distinct observed wait duration (t), define the risk set (Y(t)) as calls still under observation immediately before that duration. Let (d_B(t)), (d_D(t)), and (d_C(t)) be berth, diversion and cancellation events; let (c(t)) be right-censorings.

Event-free survival updates as:

`S(t) = S(t^-) \left(1 - \frac{d_B(t)+d_D(t)+d_C(t)}{Y(t)}\right)`

Cause-specific cumulative incidence updates as:

`F_k(t) = F_k(t^-) + S(t^-)\frac{d_k(t)}{Y(t)}`

for (k\in\{B,D,C\}).

After the update, all competing events and censorings at that duration leave the risk set. A censor alone does not reduce survival or add cause incidence. Tied events are handled in one risk-set update; no arbitrary order is invented.

At each event-time step:

`S(t) + F_B(t) + F_D(t) + F_C(t) = 1`

within numerical tolerance.

This is a non-parametric Aalen–Johansen competing-risk estimate. It is not the same as treating diversion/cancellation as ordinary independent censoring in a Kaplan–Meier estimator. No extrapolation beyond the observed event horizon is performed. The estimator still relies on non-informative right censoring, conditional on the cohort and any stratification. Administrative end-of-window censoring may be defensible under the declared sampling design; delayed source publication may be informative if delay depends on terminal, outcome or congestion. V0.1 separates administrative censoring from reporting-lag censoring, but does not model or correct informative censoring. A real cohort must publish reporting-delay diagnostics and sensitivity analyses (or use an independently validated censoring model) before interpreting the CIF as a calibrated probability.

## 5. Berth-incidence quantiles

The p50/p90/p95 berth-incidence time is the first observed duration at which (F_B(t)) reaches 0.50/0.90/0.95 respectively.

If the berth cumulative incidence does not reach a threshold during the observed horizon, the quantile is **NOT_ESTIMABLE**. The evaluator must not substitute the percentile of completed-only berth calls or silently extrapolate.

A berth quantile of NOT_ESTIMABLE is a real result: given the observed competing outcomes and censoring, the requested cumulative probability was not reached.

## 6. Queue exposure / backlog area

For call (i), define observed queue exposure (w_i) as:
- outcome tick minus queue-entry tick for an outcome observed by the cutoff; or
- observation cutoff minus queue-entry tick if censored.

Then:

`VesselTicks = Σᵢ wᵢ = ∫ Q(t) dt`

where (Q(t)) is the number of calls in the declared queue. This equality is exact for the fixture's defined entry/exit times and unit-weight call count. The output remains vessel-ticks; convert to vessel-hours only after a source adapter proves the tick unit and boundary.

## 7. Frozen synthetic cohorts

Five cohorts and 25 calls cover:
- completed berths mixed with diversion, cancellation and censoring;
- no berth CIF crossing p50;
- an all-censored cohort;
- multiple competing outcomes at the same wait duration;
- a berth outcome observed only after the historical cutoff.

Administrative and reporting-lag censoring are counted separately.\n\nIndependent golden tests pin:
- call/outcome/censor counts;
- total vessel-tick exposure;
- selected exact risk sets;
- event-free survival and cause-specific CIF values;
- p50/p90/p95 estimability;
- probability-mass reconciliation;
- input row-order invariance.

## 8. Industry comparison

### World Bank / CPPI 2025

The World Bank's 2026 *Container Terminal Performance Improvement Guide* describes the port-call stages separately: Port Limits, pilot boarding, arrival at berth, lines fast, cargo start/completion, lines released and leave berth. It defines arrival process as waiting plus steam-in and distinguishes port-level outcomes from processes controlled by the terminal. The guide explicitly warns that port-level indicators should not be attributed to terminal operators alone.

The 2025 CPPI measures call time from arrival at port limits to departure from berth and considers vessel/call-size differences. Our queue cohort metric is narrower and only becomes comparable after the exact queue-entry/outcome boundary, port limits, anchorage definitions, censoring and workload normalization are matched. This v0.1 does not reproduce CPPI.

### S&P Global Port Performance

S&P Global describes a proprietary call-level Port Performance dataset with 180,000 annual port calls across more than 1,000 terminals and 500 ports. The CPPI 2025 research describes a dataset with 180,000 port calls across more than 400 ports, combining carrier, AIS, vessel and port-reference data. These published coverage descriptions have different scopes and should not be casually equated.

This is a potential licensed comparator only. Do not scrape, mirror or redistribute. A comparison requires lawful access to call-level event timestamps, workload fields, cohort/censoring rules and data-use rights.

### DCSA

DCSA Port Call 2.0 provides estimated/requested/planned/actual service timestamps and defines the event lifecycle for berth, cargo operations, pilotage and anchorage services. The evaluator must consume correctly classified actual outcomes; forecasts and planned windows cannot be treated as actual berth/censor events.

## 9. Promotion gates

Before any real-port metrics are published:
1. Admit rights-qualified event data, raw artifact digests and exact source field dictionaries.
2. Fix the population and queue-entry event (e.g. port limits versus designated anchorage); report alternative boundaries separately if both matter.
3. Preserve vessel/call identity, duplicate call handling, timestamp timezone/precision, source observation time and corrections.
4. Include unresolved calls as right-censored at the exact observation cutoff.
5. Keep diversion/cancellation as competing causes.
6. Validate call cohorts against independent port/terminal totals and reconcile with the conserved backlog ledger.
7. Stratify by terminal, vessel size, trade/service, cargo class and disruption regime; use time/terminal held-out windows.
8. For algorithm comparisons, replay only information available at each decision time and compare the same disturbance realizations.\n9. Measure source reporting delay by outcome, terminal and congestion regime; test censoring independence and quantify sensitivity to informative censoring.

## 10. Claim ceiling

A synthetic pass proves estimator and cutoff semantics only. It does not establish real Durban queue durations, validate a terminal's event definitions, reproduce CPPI, prove a cause of delay, infer intervention effect or authorize operational activity.

## References

- World Bank (2026), *Container Terminal Performance Improvement Guide*: https://documents1.worldbank.org/curated/en/099103026152033883/pdf/P509479-2ec7a9c5-9ed5-4252-ab11-a2f60747db87.pdf
- World Bank / S&P Global, CPPI 2025: https://www.worldbank.org/en/topic/transport/publication/cppi
- S&P Global Port Performance dataset: https://www.marketplace.spglobal.com/en/datasets/port-performance-%28275%29
- DCSA Port Call 2.0.0 Purpose & Scope: https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope
- DCSA Port Call 2.0.0 Use Cases: https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-use-cases
